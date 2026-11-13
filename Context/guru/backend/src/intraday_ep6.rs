//! EP6 (lane K59, 5 Nov 2026, decisions #240/#243): the shared minute-order resolver for daily-bar rules.
//!
//! A daily rule walks each bar along a price path (its declared convention, e.g. TradeStation's open -> nearer extreme ->
//! other extreme -> close). A day is ORDER-DEPENDENT when the rule's events that day (fills of entry / stop / target levels)
//! differ between the two possible extreme orders. Only on such days, and only for the ids in `IDS` with `p.order` = 1, the
//! day is walked again on the real 1-minute path of the same ticker (intraday_w2b::day: the minute file, the cleaner's
//! excluded days and the rejected NIFTY 50 days; the same loader intraday_ep::day_bars uses). Each minute is one more bar:
//! a gap from the previous price to the minute's open (an order the gap passes fills at the minute's open), then the minute's
//! own open -> nearer extreme -> other extreme -> close. A minute whose events differ between its two extreme orders is
//! AMBIGUOUS (#175: the order inside one minute is unknown); it takes the same convention inside the minute (nearer extreme
//! first) and is counted. A day without minutes keeps the module's declared convention (counted as 'by convention').
//! Days that are not order-dependent are walked exactly as before, so their results do not change.
use crate::connors::{ParamDef, Params, Trade};
use crate::intraday::Bar1;

/// One straight piece of a day's price path, from `a` to `b`. `gap`: the price jumped (no trade in between), so an order
/// the jump passes fills at `b`, not at its level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg { pub a: f64, pub b: f64, pub gap: bool }

impl Seg {
    pub fn up(&self) -> bool { self.b > self.a }
    /// A level the segment reaches (excluding its start, including its end), in the direction it moves.
    pub fn inside(&self, lv: f64) -> bool { lv.is_finite() && if self.up() { lv > self.a && lv <= self.b } else { lv < self.a && lv >= self.b } }
    /// Fill price of an order at `lv` reached on this segment.
    pub fn fill(&self, lv: f64) -> f64 { if self.gap { self.b } else { lv } }
}

/// A bar's path O -> H -> L -> C (`high_first`) or O -> L -> H -> C.
pub fn bar_segs(o: f64, h: f64, l: f64, c: f64, high_first: bool) -> [Seg; 3] {
    let p = if high_first { [o, h, l, c] } else { [o, l, h, c] };
    [Seg { a: p[0], b: p[1], gap: false }, Seg { a: p[1], b: p[2], gap: false }, Seg { a: p[2], b: p[3], gap: false }]
}

/// TradeStation's daily-bar path (lc3::high_first): the extreme nearer the open first, ties the high.
pub fn ts_high_first(o: f64, h: f64, l: f64) -> bool { h - o <= o - l }

/// How one day was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Day {
    /// Not order-dependent (both extreme orders give the same events), or the resolver is off: the declared convention.
    Plain,
    /// Order-dependent, decided on the real 1-minute path; no minute had two events in an unknown order.
    Minutes,
    /// Order-dependent, no minutes that day: the module's declared convention.
    Convention,
    /// Order-dependent, decided on the 1-minute path, but at least one minute reached two levels in an unknown order (#175).
    Ambiguous,
}

impl Day {
    /// Note appended to the reason of a trade an order-dependent day decided (empty for Plain).
    pub fn note(self) -> &'static str {
        match self {
            Day::Plain => "",
            Day::Minutes => " · same-day order: real 1-minute path",
            Day::Convention => " · same-day order: daily-bar convention (no minutes that day)",
            Day::Ambiguous => " · same-day order: 1-minute path, ambiguous minute (nearer extreme first)",
        }
    }
    /// The worse of two marks for one trade (Ambiguous > Convention > Minutes > Plain).
    pub fn worst(self, o: Day) -> Day { self.max(o) }
}

/// Resolve bar `i` of `x` for a rule whose bar walk is `walk(state, path) -> events`. `st` is the state before the
/// intrabar part of the bar (after the open fills); on return it is the state after it. `high_first` = the module's own
/// daily order for this bar (its declared convention). `daily(state, high_first)` is the module's daily intrabar step
/// (for path simulators: `walk` on `bar_segs`). Events must compare equal exactly when the outcomes are the same.
pub fn resolve<S: Clone>(x: &crate::connors::Ctx, i: usize, on: bool, high_first: bool, st: &mut S,
    daily: impl Fn(&mut S, bool) -> Vec<Ev>, walk: impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> Day {
    resolve_conv(x, i, on, st, |s| { daily(s, high_first); }, &daily, walk)
}

/// `resolve` for a module whose daily convention is not a bar-path order (e.g. 'the stop first'): `probe(state, high_first)`
/// is the rule walked on the bar's path in either order (it only decides whether the day is order-dependent), and `conv`
/// is the module's own daily step, used on every day that is not resolved on minutes.
pub fn resolve_conv<S: Clone>(x: &crate::connors::Ctx, i: usize, on: bool, st: &mut S, conv: impl Fn(&mut S),
    probe: impl Fn(&mut S, bool) -> Vec<Ev>, walk: impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> Day {
    if !on { conv(st); return Day::Plain; }
    let (mut a, mut b) = (st.clone(), st.clone());
    if probe(&mut a, true) == probe(&mut b, false) { conv(st); return Day::Plain; }
    let Some(m) = crate::intraday_w2b::day(x, i).filter(|m| !m.is_empty()) else { conv(st); return Day::Convention };
    let (amb, s) = walk_minutes(&m, x.o[i], st.clone(), &walk);
    *st = s;
    if amb { Day::Ambiguous } else { Day::Minutes }
}

/// One fill of a bar walk: what happened (module code), at which price, and whether the path was moving up (the level is
/// touched from below: a buy stop, a sell limit, a short's stop) or down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ev { pub k: u8, pub px: f64, pub up: bool }

/// Walk the day's minutes from the daily open `open`: returns (any ambiguous minute, final state). A minute whose fills
/// differ between its two extreme orders reached two levels inside it. #175: when the minute OPENED THROUGH exactly one of
/// those levels (a level that was not live at the open, such as the stop of an entry filled later in the minute), that
/// level's side came first and the minute is not ambiguous; otherwise it is ambiguous and the nearer extreme goes first.
pub fn walk_minutes<S: Clone>(m: &[Bar1], open: f64, mut s: S, walk: &impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> (bool, S) {
    let (mut prev, mut amb) = (open, false);
    walk(&mut s, &[]); // a module's bar-open step (if it has one) runs once, before the first minute
    for z in m {
        if z.o != prev { walk(&mut s, &[Seg { a: prev, b: z.o, gap: true }]); }
        let hf = ts_high_first(z.o, z.h, z.l);
        let (mut s1, mut s2) = (s.clone(), s.clone());
        let e1 = walk(&mut s1, &bar_segs(z.o, z.h, z.l, z.c, hf));
        let e2 = walk(&mut s2, &bar_segs(z.o, z.h, z.l, z.c, !hf));
        s = if e1 == e2 { s1 } else {
            let mut lv: Vec<(f64, bool)> = e1.iter().chain(&e2).map(|e| (e.px, e.up)).collect();
            lv.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            lv.dedup();
            let through: Vec<bool> = lv.iter().filter(|&&(p, up)| if up { z.o >= p } else { z.o <= p }).map(|&(_, up)| up).collect();
            if through.len() == 1 { if through[0] == hf { s1 } else { s2 } } else { amb = true; s1 }
        };
        prev = z.c;
    }
    (amb, s)
}

/// The ids whose same-day order is resolved on minutes by default (#243; M6.tsv over 5%). Wave 1: the Larry Williams
/// lc3-family simulators (lwlts, lwdtfo on lwlts::simulate, lwnl, lwicw).
pub const IDS: &[&str] = &[
    "lwlts_mon_gap_t500", "lwlts_mon_gap_t1000", "lwlts_oops_sell_ma9", "lwlts_oops_buy_ma9", "lwlts_oops_sell_thu", "lwlts_oops_buy",
    "lwlts_vb_sp_40_200", "lwlts_talon_sp", "lwlts_hh1", "lwlts_hh5", "lwlts_hh10", "lwlts_hh15", "lwlts_hh20", "lwdtfo_vb_open",
    "lwnl_jul97_td4_buy", "lwnl_jan_rally", "lwnl_jul97_td16_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_mar_short", "lwnl_proj_contra",
    "lwnl_jul97_td17_buy", "lwnl_aug97_td2_sell_7d", "lwnl_pullback", "lwnl_aug97_td7_sell_bail", "lwnl_nov97_td17_buy_7d",
    "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td18_buy_7d", "lwicw_oops_tdom_buy", "lwicw_oops_tdom_sell",
    // wave 2: btp2 (books/btp.rs sim; its daily convention is 'the stop first', not a bar-path order: resolve_conv)
    "btp2_wide_range_exhaustion_gap_long", "btp2_momentum_ma_long", "btp2_momentum_ma_short", "btp2_gipsons_long",
    "btp2_8day_hl_reversal_short", "btp2_8day_hl_reversal_long", "btp2_1234_long", "btp2_1234_short",
    // wave 3 (lane K62): Bernstein day-of-week stop entries (jb624::entry_bar), the rrsim / run_ts_pp TradeStation engines,
    // the ajmisc Sim, Luisi's narrow-range day trade
    "jb624_fri_mon", "jbcdt2_dbo_mon", "jbcdt2_dbo_thu", "jbeft_dow_fm97", "jbeft_dow_wt",
    "bfmmx_esl4", "bfmmx_nql4", "bfmmx_s4", "mtenl_dx2", "tsvega_stoch", "jkppnt_code",
    "axsym_gap_breakout", "ltcoat_coat_tails", "scluisi_nr",
    // the STAD engines (books::stad, books::stad2: the 10% bounce stays the daily convention)
    "stad2_adxdiff", "stad5_stochob", "stad6_adxmac", "stad6_asi",
    // the Active Trader TSL simulator (books::atm00 levels_ep6: stop and target on one bar)
    "atm01_five_for_five", "atm01_deep_pockets",
    // Dave Landry's swing patterns (books::dlost manage_ep6: entry, stop and the 2-for-1 half-off target)
    "dlost_simple_pb", "dlost_tko", "dlost_dtko", "dlost_tpp", "dlost_bow_tie",
    // Burns inside-day breakout (books::jbid: both day-2 stops; stop and target 1 on one day)
    "jbid_inside_day",
    // wave 4 (lane K62): a position's stop and target on one bar (intraday_ep6::stop_target)
    "rwtlc_adx_hivol_ct", "rwmts_bb_fade", "jcmtt_propulsion",
    // wave 5 (lane K63): Collins's TradeStation order engine (books::acbff engine_ep6)
    "acbff_3rd_fade", "acbff_contract", "acbff_cup_3day", "acbff_fade_rev",
    // Bierovic's path-ordered simulator (books::tbabc sim_ep6)
    "tbabc_micro_m", "tbabc_micro_w", "tbabc_six_long",
    // Rensink's weekly breakout (books::jrfsp St::walk)
    "jrfsp_weekly_breakout",
    // Wilder's Trend Balance Point (stop_target) and Reaction Trend (books::wwnc rt_system_ep6)
    "wwnc_tbp", "wwnc_rt",
    // Gettess TT30 on the Krutsinger order walk (books::jkbwts sim_ep6)
    "lgtt30_tt30",
    // Pruitt & Hill's pattern breakout on the pbwts order simulator (books::pbwts sim_ep6)
    "putg_pattern_vbo",
    // Pierce's NR4 stop entries (books::sprfs nr4_trades_ep6: the entry bar's two stops and the protective stop)
    "sprfs_nr4ib", "sprfs_hvr_nr4ib", "sprfs_hvr_nr4",
    // Chong's pyramid (books::rcmm day_walk), Lien's CCI trade (books::kllien cci_walk)
    "rcmm_20", "rcmm_55", "klhpt_cci_right",
    // part B (lane K63): Lindsay's Trident (stop_target), Dorfman's 3-EMA turn (books::vrsdd manage_ep6)
    "lcttr_minor", "rdhft_ema3_turn",
    // Stridsman's Meander (books::strmm sim_ep6: the stop and the target on one bar)
    "strmm_meander",
    // Bowden's ABC swing trade (books::dbnotp held_walk and the entry-bar walk)
    "dbnotp_abc_stock", "dbnotp_abc_currency", "dbnotp_abc_index", "dbnotp_abc_trail",
    // Stridsman's Gold Digger column (books::atm00: the stop, the target and the intrabar lock-in)
    "atm00_gold_digger",
    // Ross's ZULU gap reversal (books::joe_ross_rules zulu: held bars via stop_target, the entry bar walked on the minutes)
    "jr_zulu",
    // part C (lane K63): Conway & Behle's Acme V (books::cbpst bar_walk: the Trade Manager's stop, targets, time exit and
    // the entry stops along the path)
    "cbpst_v",
    // Landry's Explosion Gap Pivot (books::dl10b day_walk: the entry stop, the stop and the half-off target along the path)
    "dl10b_egp",
    // Connors's Windows (books::cwin sim_ep6: the entry day's stop entry and its same-day protective stop)
    "cwin_single_window", "cwin_double_window", "cwin_first_window", "cwin_weekly_window",
    // Street Smarts (books::ss sim: the entry bar's stop entries, stop-and-reverse, stop and target via entry_walk; held
    // bars' stop and target via stop_target)
    "ss_turtle_soup_plus_one", "ss_holy_grail", "ss_three_day_unfilled_gap", "ss_id_nr4", "ss_hist_vol_crabel",
    // part D (lane K63): Connors's ATS patterns (books::ats sim: the entry bar's stop entry, the protective stop and the
    // 2-for-1 half-off target via ats_walk)
    "ats_1234", "ats_8day_hl_reversal", "ats_gipsons", "ats_two_for_one",
    // Bryant's MiniMax II (books::bfmm: held bars' stop (with the reversing entry stop) and target via stop_target)
    "bfmm_mm2_es", "bfmm_mm2_nq",
    // Stridsman's Meander I (books::strtw meander_walk: the entry limit, the risk stop and the VS exit on one day)
    "strtw_meander1_long", "strtw_meander1_short",
    // Connors's 5-week seminar Windows (books::ws5 M3: the 3-day exit procedure walked point by point; a held day on minutes)
    "ws5_single_window", "ws5_double_window", "ws5_momentum_windows_any",
    // lane K73 (R4 #276 (7)): Collins's Forever Long (books::acbff engine_ep6: the limit entry and its five-point target)
    "acbff_forever_long",
    // lane K74 (R4 #277): Harris's S&P patterns (books::mhstp simulate_ep6: the 10-point target and stop via stop_target), SPX only
    "mhs_sp830825_1", "mhs_sp830120_1", "mhs_sp830330_1",
    // lane K74 (R4 #277 wave 3): the Original Turtle Rules (books::covel breakout_ep6: entry, adds and unit exits along the path)
    "cov_turtle_s1", "cov_turtle_s2",
    // lane K75 (R4 #281 (3)): StockCharts full-gap-down long (books::stkch same_day: a trailing-stop exit and a new gap entry on one bar)
    "stkch_gap_fgd_long",
    // lane R2E (R4 #305 (3)): Radge's Smash Bar (books::radge: the entry bar's stop entry and its initial stop on the minutes)
    "radge_smash"];

/// #243 (c): ids whose convention result fails the pre-set #177 tests against the resolved result on the minute window:
/// only the minute window is published (resolved), the earlier years are withdrawn.
pub const WINDOW_ONLY: &[&str] = &["btp2_8day_hl_reversal_long", "btp2_8day_hl_reversal_short", "btp2_wide_range_exhaustion_gap_long",
    "lwicw_oops_tdom_buy", "lwlts_oops_buy", "lwlts_oops_buy_ma9", "lwlts_vb_sp_40_200", "lwnl_aug97_td7_sell_bail", "lwnl_mar_short",
    "lwnl_proj_contra", "lwnl_pullback",
    // wave 3 (lane K62)
    "dlost_simple_pb", "dlost_tko", "ltcoat_coat_tails", "scluisi_nr", "stad2_adxdiff", "stad5_stochob", "tsvega_stoch",
    // wave 5 (lane K63)
    "acbff_3rd_fade", "lgtt30_tt30", "rcmm_55", "wwnc_tbp",
    "dbnotp_abc_index", "strmm_meander", "atm00_gold_digger", "jr_zulu",
    "cwin_single_window", "cwin_double_window", "cwin_first_window", "ss_three_day_unfilled_gap", "ss_holy_grail",
    "ss_hist_vol_crabel", "ats_8day_hl_reversal", "strtw_meander1_long", "strtw_meander1_short",
    // lane K74 (R4 #277): the Turtle systems (S1 flags the sign test on SPX, S2 the 3x average test on NIFTYBANK; K74w/t243.tsv)
    "cov_turtle_s1", "cov_turtle_s2",
    // lane K75 (R4 #281 (3)): SPY 2000-2025 compounded +84.9% resolved vs +5.9% convention (test c; K75w/t243.out)
    "stkch_gap_fgd_long",
    // lane R2E (R4 #305 (3)): 30-37% of Smash Bar trades are order-dependent on the daily bar; only the minute window publishes
    "radge_smash"];

/// Decisions #231/#234 (lane K63, R4 #255): EP6 minute-window results on NSE cash-index minutes that are India-better than
/// tradable SPY over the same dates on both magnitude tests (average trade > 3x and > 0.003 pp above; compounded > 10x, or
/// above +100% when SPY's is not positive): those India results are withdrawn (K63w/india231.tsv).
pub const INDIA231: &[(&str, &str)] = &[("ltcoat_coat_tails", "NIFTY 50"), ("ltcoat_coat_tails", "NIFTYBANK"), ("stad2_adxdiff", "NIFTY 50"), ("wwnc_tbp", "NIFTY 50"),
    ("lgtt30_tt30", "NIFTY 50"), ("lgtt30_tt30", "NIFTYBANK"), ("ss_hist_vol_crabel", "NIFTY 50"),
    // lane K74 (K74w/t179.tsv): Turtle System 1 on NSE minutes vs SPY over the same dates (+0.75% vs -0.23% a unit, +1,786% vs -77%)
    ("cov_turtle_s1", "NIFTY 50"), ("cov_turtle_s1", "NIFTYBANK")];
const INDIA231_NOTE: &str = "The India minute-window result is withdrawn (decisions #231, #234, #255, lane K63): NSE cash-index minutes are dominated by stale component prices; there is no tradable India minute series and this result is more than 10x the tradable SPY result over the same dates.";

/// #243: ids HELD. The five thin ids (M6: over 5% order-dependent, fewer than 10 trades inside the minute window), and EP6 ids
/// whose resolved path has more than 5% ambiguous-minute trades (#175).
pub const HELD: &[(&str, &str)] = &[
    ("btp2_gipsons_short", THIN), ("dl10b_rgs", THIN), ("jr_ledge_tbtb", THIN), ("ws5_gap_window_intraday", THIN), ("ws5_outside_window", THIN),
    ("lwlts_mon_gap_t1000", CASH177B), ("lwlts_oops_buy_ma9", CASH177B), ("lwlts_oops_sell_thu", CASH177B), ("lwlts_talon_sp", CASH177B), ("lwlts_hh5", CASH177B), ("lwlts_hh20", CASH177B), ("lwnl_aug97_td7_sell_bail", CASH177B), ("lwnl_nov97_td17_buy_7d", CASH177B),
    ("lwnl_aug97_td2_sell_7d", AMB), ("lwnl_jan_rally", AMB), ("lwnl_jul97_td17_buy", AMB), ("lwnl_jul97_td4_buy", AMB), ("lwnl_nov97_td18_buy_7d", AMB),
    // wave 3 (lane K62): Bernstein's S&P-point day-of-week stops (#246 (a): #177b on the resolved versions)
    ("jb624_fri_mon", CASH177B_K62), ("jbcdt2_dbo_mon", CASH177B_K62), ("jbcdt2_dbo_thu", CASH177B_K62),
    ("jbeft_dow_fm97", CASH177B_K62), ("jbeft_dow_wt", CASH177B_K62)];
const CASH177B_K62: &str = "Held (decisions #177b, #246, lane K62): the SPX result is withheld. On tradable SPY with the book's S&P points scaled by the trailing 250-day SPX/SPY ratio, the minute-resolved rule loses over 2014-2025 while SPX gains (opposite sign, the cash-vs-tradable check), and SPX is this rule's only ticker, so no results are published.";
/// #246 (a), lane K62: daily rules whose RESOLVED result fails #179 on SPX (SPX better than SPY in both check windows): the SPX
/// result is withdrawn and, as #179 rules for SPX-flagged rules, the NIFTY 50 / NIFTYBANK results are held; SPY and the rest stay.
pub const SPX179: &[&str] = &["bfmmx_nql4", "cwin_single_window"]; // + lane K63 (wave 5)
const SPX179_NOTE: &str = "The SPX daily result is withdrawn (decisions #179, #246, lane K62): with the same-day order resolved on 1-minute bars, SPX beats tradable SPY over the same dates in both check windows (2014-01-02..2017-06-30 and 2017-07-01..2020-09-11); SPX daily open/high/low come from stale component prints, so the SPY result carries the S&P rule.";
const SPX179_INDIA: &str = "Not published on this index (decisions #179, #246, lane K62): the same rule fails the tradable-vs-cash check on SPX/SPY with its same-day order resolved on 1-minute bars, and India has no tradable index series to check it on, so the NIFTY result is held.";
/// #246 (a): SPX-point rules whose RESOLVED result fails #177b (SPX vs tradable SPY with the points scaled, 2014-2025); SPX is
/// their only ticker, so nothing is published.
const CASH177B: &str = "Held (decisions #177b, #246, lane K59): the SPX result is withheld. On tradable SPY with the book's S&P points scaled by the trailing 250-day SPX/SPY ratio, the minute-resolved rule fails the cash-vs-tradable check over 2014-2025 (opposite sign, or SPX more than 3x SPY), and SPX is this rule's only ticker, so no results are published.";
const AMB: &str = "Held (decision #243, lane K59): on SPX's real 1-minute bars one of its 11-12 trades in the minute window (8.3-9.1%) is decided inside a single minute that reaches two of its levels without opening through exactly one of them, above decision #175's 5% limit, so the same-day order cannot be resolved and no results are published.";
const THIN: &str = "Held (decision #243, lane K59): more than 5% of its trades enter at a stop or limit level and reach an exit level the same day, so the daily bar cannot say which came first (#240), and fewer than 10 of its trades fall inside the 1-minute window, so the order cannot be resolved on real minutes. No results are published.";
const WINDOW_NOTE: &str = "Not published on this ticker (decision #243 (c), lane K59): the daily-bar convention changes this rule's result materially against the real 1-minute order, so only the minute-resolved window of the 1-minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK) is published.";

/// The hold or minute-window refusal for (id, ticker), if any (instrument_policy::tradable_check_issue).
pub fn held_note(id: &str, ticker: &str) -> Option<&'static str> {
    if let Some(&(_, n)) = HELD.iter().find(|h| h.0 == id) { return Some(n); }
    let nse = if ticker == "NIFTY BANK" { "NIFTYBANK" } else { ticker };
    if INDIA231.contains(&(id, nse)) { return Some(INDIA231_NOTE); }
    if SPX179.contains(&id) {
        if ticker == "SPX" { return Some(SPX179_NOTE); }
        if matches!(ticker, "NIFTY 50" | "NIFTYBANK" | "NIFTY BANK") { return Some(SPX179_INDIA); }
    }
    (WINDOW_ONLY.contains(&id) && !matches!(ticker, "SPY" | "SPX" | "NIFTY 50" | "NIFTYBANK" | "NIFTY BANK")).then_some(WINDOW_NOTE)
}

/// The resolver is on for this run (an EP6 id and p.order = 1, the default).
pub fn on(id: &str, p: &Params) -> bool { IDS.contains(&id) && p.get("order").copied().unwrap_or(1.0) >= 0.5 }

/// The labelled switch, appended to every EP6 id's parameters.
pub fn add_param(id: &str, v: &mut Vec<ParamDef>) {
    if IDS.contains(&id) {
        v.push(ParamDef { key: "order", label: "Same-day order of the stop/limit levels: 1 = real 1-minute path where minutes exist (#243), 0 = the daily-bar convention (book check)", def: 1.0, min: 0.0, max: 1.0, step: 1.0 });
    }
}

/// #243 (c): the run window clamped to the ticker's minute file for WINDOW_ONLY ids with the resolver on (None: the window
/// and the minute file do not overlap); otherwise the window unchanged.
pub fn window(id: &str, x: &crate::connors::Ctx, from: usize, to: usize, p: &Params) -> Option<(usize, usize)> {
    if !(WINDOW_ONLY.contains(&id) && on(id, p)) { return Some((from, to)); }
    // no minutes loaded (a unit-test context, or a ticker without minute files, which held_note already refuses): unchanged
    let Some(m) = x.minutes else { return Some((from, to)) };
    let bad = |r: &crate::Row| r.ticker == "NIFTY 50" && r.date.as_str() >= crate::intraday_w2b::NIFTY_BAD_FROM; // rejected minutes
    let (a, b) = (x.r.partition_point(|r| r.date.as_str() < m.first()), x.r.partition_point(|r| r.date.as_str() <= m.last() && !bad(r)));
    let (f, t) = (from.max(a), to.min(b.checked_sub(1)?));
    (f <= t).then_some((f, t))
}

/// A position's protective stop and profit target working on bar `t` (lane K62, wave 4): with `on`, a bar whose range holds
/// both levels while the open is through neither is order-dependent and is walked on the real 1-minute path (the first level
/// the minutes reach, at the level or a gapped minute's open, reasons `why`); every other bar, and days without minutes, keep
/// the module's own daily rule `conv` (e.g. the stop first). Returns the exit (price, reason) and how the day was decided.
pub fn stop_target(x: &crate::connors::Ctx, t: usize, dir: f64, stop: f64, tgt: f64, on: bool, why: (&'static str, &'static str),
    conv: impl Fn() -> Option<(f64, &'static str)>) -> (Option<(f64, &'static str)>, Day) {
    let gap = |lv: f64, with: bool| lv.is_finite() && if (dir > 0.0) == with { x.o[t] >= lv } else { x.o[t] <= lv };
    if !on || !stop.is_finite() || !tgt.is_finite() || gap(stop, false) || gap(tgt, true) { return (conv(), Day::Plain); }
    let walk = |s: &mut Option<(f64, &'static str)>, segs: &[Seg]| -> Vec<Ev> {
        for g in segs {
            if s.is_some() { break; }
            let (lv, k, w) = if g.up() != (dir > 0.0) { (stop, 0u8, why.0) } else { (tgt, 1u8, why.1) };
            if g.inside(lv) { let px = g.fill(lv); *s = Some((px, w)); return vec![Ev { k, px, up: g.up() }]; }
        }
        vec![]
    };
    let mut st = None;
    let day = resolve_conv(x, t, true, &mut st, |z| *z = conv(), |z, hf| walk(z, &bar_segs(x.o[t], x.h[t], x.l[t], x.c[t], hf)), walk);
    (st, day)
}

/// Append the day's mark to a trade's reason.
pub fn mark(t: &mut Trade, d: Day) { if d != Day::Plain { t.reason.push_str(d.note()); } }

/// Re-mark a trade closed on a bar decided as `d`: the worse of its existing mark (from its position) and `d`.
pub fn remark(t: &mut Trade, d: Day) {
    if d == Day::Plain { return; }
    let old = [Day::Ambiguous, Day::Convention, Day::Minutes].into_iter().find(|k| t.reason.ends_with(k.note())).unwrap_or(Day::Plain);
    if old != Day::Plain { let n = t.reason.len() - old.note().len(); t.reason.truncate(n); }
    mark(t, old.worst(d));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn b(o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m: 0, o, h, l, c } }
    // a tiny walker: long buy-stop entry at `e`, then a sell stop at `s`; events = (code, price)
    #[derive(Clone)]
    struct St { e: f64, s: f64, long: Option<f64>, out: Vec<(f64, f64)> }
    fn walk(st: &mut St, segs: &[Seg]) -> Vec<Ev> {
        let mut ev = vec![];
        for g in segs {
            let mut g = *g;
            loop {
                let hit = match st.long { None if g.up() && g.inside(st.e) => Some((0u8, st.e)), Some(_) if !g.up() && g.inside(st.s) => Some((1, st.s)), _ => None };
                let Some((k, lv)) = hit else { break };
                let px = g.fill(lv);
                if k == 0 { st.long = Some(px); st.e = f64::NAN; } else { st.out.push((st.long.take().unwrap(), px)); }
                ev.push(Ev { k, px, up: g.up() });
                g.a = lv;
            }
        }
        ev
    }
    #[test]
    fn minute_order_decides_and_ambiguity_is_counted() {
        let st = St { e: 101.0, s: 99.0, long: None, out: vec![] };
        // daily bar 100 / 102 / 98 / 100: TS path (open nearer the low... h-o = 2 = o-l: high first) enters and stops.
        let minutes = [b(100.0, 100.5, 98.0, 99.5), b(99.5, 102.0, 99.4, 101.5)]; // the low came first: entered, not stopped
        let (amb, s) = walk_minutes(&minutes, 100.0, st.clone(), &walk);
        assert!(!amb && s.long == Some(101.0) && s.out.is_empty());
        // one minute reaches both levels: ambiguous; inside it the nearer extreme (the high) first, so entered then stopped
        let (amb, s) = walk_minutes(&[b(100.0, 102.0, 98.0, 99.5)], 100.0, st.clone(), &walk);
        assert!(amb && s.out == vec![(101.0, 99.0)]);
        // a gap through the entry level fills at the minute's open
        let (amb, s) = walk_minutes(&[b(100.0, 100.2, 99.9, 100.0), b(101.5, 101.8, 101.2, 101.6)], 100.0, st, &walk);
        assert!(!amb && s.long == Some(101.5));
    }
    #[test]
    fn switch_and_window_defaults() {
        let mut p = Params::new();
        assert!(on("lwlts_hh1", &p) && !on("lwlts_tdm6_sp", &p));
        p.insert("order".into(), 0.0);
        assert!(!on("lwlts_hh1", &p));
        let mut v = vec![];
        add_param("lwlts_tdm6_sp", &mut v);
        assert!(v.is_empty());
        add_param("lwlts_hh1", &mut v);
        assert_eq!(v[0].key, "order");
        assert!(Day::Ambiguous.worst(Day::Minutes) == Day::Ambiguous && Day::Plain.note().is_empty());
    }
    #[test]
    fn held_ids_publish_nothing() {
        let all = crate::connors::catalogue();
        for (id, _) in HELD {
            let d = all.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            for t in ["SPY", "SPX", "QQQ", "NIFTY 50"] {
                assert!(crate::instrument_policy::check(d, t, false).is_some_and(|i| i.note.starts_with("Held (decision")), "{id} {t}");
            }
        }
    }
    fn minutes(name: &str, days: &[(&str, &[&str])]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("ep6{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in *lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> crate::Row { crate::Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn lwlts_oops_entry_day_decided_on_minutes() {
        // Monday opens 95 under Friday's low 99: buy stop 99, $2,000 = 8-point stop at 91 armed on the entry day. The daily bar
        // (h-o = o-l: high first) enters and stops; the minutes put the low (90) before the fill, so the trade is not stopped.
        let r = vec![row("2020-01-02", 100., 101., 99., 100.), row("2020-01-03", 100., 101., 99., 100.), row("2020-01-06", 95., 100., 90., 96.)];
        let mut x = crate::connors::Ctx::new(&r, &r, &[]);
        let conv = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &[("order".to_string(), 0.0)].into_iter().collect()).unwrap();
        assert!(conv.len() == 1 && conv[0].exit_px == 91.0 && !conv[0].reason.contains("same-day"), "{conv:?}");
        let none = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &Params::new()).unwrap();
        assert!(none.len() == 1 && none[0].exit_px == 91.0 && none[0].reason.ends_with(Day::Convention.note()), "{none:?}");
        x.minutes = Some(minutes("oops", &[("2020-01-06", &["09:30,95,95.5,90,91.5", "09:31,91.5,100,91.5,99.5", "09:32,99.5,99.6,95.5,96"])]));
        let t = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &Params::new()).unwrap();
        assert!(t.len() == 1 && t[0].open && t[0].legs[0].px == 99.0 && t[0].reason.ends_with(Day::Minutes.note()), "{t:?}");
        // a card that is not an EP6 id is untouched: lwlts_mon_gap_close has no same-day order question here either way
        assert!(crate::books::lwlts::params("lwlts_mon_gap_close").unwrap().iter().all(|d| d.key != "order"));
    }

    #[test]
    fn k62_bernstein_entry_day_and_luisi_day_trade_on_minutes() {
        // Friday closes above its open (high 101): Monday buy stop 101.15, stop 6.8 points under the fill (94.35). Monday's daily
        // bar (h-o 3 < o-l 6: high first) enters and is stopped; the minutes put the low (94) before the fill: entered, not stopped.
        let r = vec![row("2020-01-02", 100., 100.5, 99.5, 100.), row("2020-01-03", 100., 101., 99.8, 100.8), row("2020-01-06", 100., 103., 94., 99.)];
        let mut x = crate::connors::Ctx::new(&r, &r, &[]);
        let p0: Params = [("order".to_string(), 0.0)].into_iter().collect();
        let conv = crate::books::jbcdt2::run("jbcdt2_dbo_mon", &x, 0, 2, &p0).unwrap();
        assert!(conv.len() == 1 && (conv[0].exit_px - 94.35).abs() < 1e-9 && !conv[0].open && !conv[0].reason.contains("same-day"), "{conv:?}");
        let none = crate::books::jbcdt2::run("jbcdt2_dbo_mon", &x, 0, 2, &Params::new()).unwrap();
        assert!(none.len() == 1 && none[0].reason.ends_with(Day::Convention.note()), "{none:?}");
        x.minutes = Some(minutes("dbo", &[("2020-01-06", &["09:30,100,100.2,94,95", "09:31,95,103,95,102", "09:32,102,102,99,99"])]));
        let t = crate::books::jbcdt2::run("jbcdt2_dbo_mon", &x, 0, 2, &Params::new()).unwrap();
        assert!(t.len() == 1 && t[0].open && (t[0].legs[0].px - 101.15).abs() < 1e-9 && t[0].reason.ends_with(Day::Minutes.note()), "{t:?}");
        // Luisi: yesterday's range 0.5 (<= 0.6% of 100.8); today trades both its high 101 and its low 100.5. Daily reading: the
        // level nearer the open (101, open 100.9) and the stop at half the range (100.75) counts; the minutes go low first.
        let r = vec![row("2020-01-02", 100., 100.5, 99.5, 100.), row("2020-01-03", 100.6, 101., 100.5, 100.8), row("2020-01-06", 100.9, 101.6, 100.2, 101.5)];
        let mut x = crate::connors::Ctx::new(&r, &r, &[]);
        let d = crate::books::scluisi::run("scluisi_nr", &x, 0, 2, &p0).unwrap();
        assert!(d.len() == 1 && d[0].dir > 0.0 && d[0].exit_px == 100.75, "{d:?}");
        x.minutes = Some(minutes("luisi", &[("2020-01-06", &["09:30,100.9,100.9,100.2,100.3", "09:31,100.3,101.6,100.3,101.5"])]));
        let m = crate::books::scluisi::run("scluisi_nr", &x, 0, 2, &Params::new()).unwrap();
        // the low came first: short at 100.5, stopped at 100.75 on the way up
        assert!(m.len() == 1 && m[0].dir < 0.0 && m[0].legs[0].px == 100.5 && m[0].exit_px == 100.75 && m[0].reason.ends_with(Day::Minutes.note()), "{m:?}");
    }
}
