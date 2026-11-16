//! Leslie Payne (eBay seller lmp175), "Opening Gap Strategy" for TradeStation 8.x, E-mini futures (2007). Book id `lpgap`.
//! Source copies: Books/Leslie Payne/ (Gap_Strategy_with_pictures.doc, Gap_Strategy_Notes.doc,
//! Instructions_Table_for_the_Opening_Gap_Strategy_New.doc, Gap_Statistics.doc, Novice_vs_Professional_Gaps.doc,
//! eBay listing PDF). GAP_FUTURES.ELD is a compiled/protected TradeStation archive (no readable code).
//! Transcription and checks: library/lpgap/.
//!
//! Daily-bar mapping: TimeToStart (930) = the session open, gap = open - prior close; TimeToClose = the close.
//! Gap sizes, targets and stops are in index POINTS as in the documents (YM = Dow points); other indices use the
//! same points (instrument swap only). Same-bar order on a daily bar is unknown: if the target and the stop are both
//! inside the day's range the stop is assumed first (conservative, vault convention).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lpgap";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "lpgap_fade", name: "Opening Gap Strategy · fade the gap to yesterday's close", group: "Opening Gap Strategy",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Gaps are created when the opening price is greater or lower than yesterday's closing price\" (Gap Strategy doc)",
                "F|GapSizeMin: \"Set to 10 points the program ignores all gaps less than 10 points\"; GapSizeMax: \"ignores all gaps greater than 50 points\" (Instructions Table). Defaults = the published performance report: \"Minimum Gap: 9 points, Maximum Gap: 150 points\"",
                "E|\"will enter a long position on gaps down or a short position on gaps up when user specific conditions are met\" (Gap Strategy doc); \"The program enters a trade at TimeToStart\" (Notes; 930 = the open)",
                "X|Target: \"Sets the profit target equal to Gap minus [margin] ... Target is Gap Size if both are set to 0\" (Instructions Table): default margin 0 = yesterday's close",
                "X|Stop: \"Sets a Protective Stop by percentage of the gap ... if the opening gap is 50 points and StopByGapPct is set to 50% the stop is 25 points\"; report default \"Stop Percent: 95% of gap\"",
                "X|\"It will exit the trade either at target, at a specified stop loss or at the end of day\" (Gap Strategy doc); TimeToClose = the close",
                "X|Optional MaxGapToFill / MaxGapLimitPct: on gaps >= MaxGapToFill the target is MaxGapLimitPct of the gap (Instructions Table); 0 = off",
            ],
            notes: "Designed for E-mini futures on 1-minute bars (YM/NQ/ES), gap measured at 9:29 ET against the futures' prior close. Here: cash index open vs prior close, fills on the daily bar. Default parameter set is the documents' own performance report (YM, 10 contracts, 30 days): min gap 9, max gap 150, stop 95% of gap; target margin not listed there, so the program's stated zero-default (full gap fill) applies. The chart screenshots show an undocumented input list (930,1610,10,9,140,...) whose order is unknown; not used. Take1stHalf/SetBackHalfBE two-lot exits are optional and not modelled. Same-day stop+target = stop first. DJI/SPX cash opens before ~1999 are often stale (open = prior close), so few gaps there.",
            claim: claim("26", "88.46%", "$578.63 (10 YM contracts)", "intraday", "30 days to 2007, YM futures, report image") },
        StrategyDef { id: "lpgap_fade_spx", name: "Opening Gap Strategy · fade the gap, SPX 1-minute adaptation", group: "Opening Gap Strategy",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Gaps are created when the opening price is greater or lower than yesterday's closing price\"; \"enter a long position on gaps down or a short position on gaps up\" (Gap Strategy doc); GapSizeMin 9 / GapSizeMax 150 points and Stop Percent 95% of the gap are the printed performance report's settings (Instructions Table, report image)",
                "E|\"The program enters a trade at TimeToStart\" (930 = the open) at the open price of the session's first minute",
                "X|Target = the gap size (\"Target is Gap Size if both [margin inputs] are set to 0\") = yesterday's close; stop = 95% of the gap beyond the entry; \"exit the trade either at target, at a specified stop loss or at the end of day\" (Gap Strategy doc)",
            ],
            notes: "Declared adaptation (R4 #339/#340, lane D1), always UNVERIFIED: the documents describe E-mini futures (the report: 10 YM contracts, 30 days to 2007, 26 trades, 88.46%); here the real SPX 1-minute bars (2008-; real opens from 2014, clamp_open) with the gap points literal in S&P points, no costs, one lot (the two-lot Take1stHalf / SetBackHalfBE options are not modelled). Unstated or open details, each a labelled reading: (1) the instrument: the report's points are Dow points on YM; the same 9 / 150-point gap band and 95% stop are applied to SPX points (the documents give no S&P report); (2) the gap reference: the futures gap is measured at 9:29 ET against the futures' prior close; here the first SPX minute's open against the prior SPX daily close; (3) TimeToClose: the session's last 1-minute close; (4) the stop and target both work from the first minute (orders are generated intrabar at the entry); inside a minute the price runs open, the nearer extreme, the farther extreme, close, a stop or limit fills at its level. Not the old daily-bar card lpgap_fade (no admitted instrument, unchanged).",
            claim: None },
        StrategyDef { id: "lpgap_novice_pro", name: "Novice vs professional gaps", group: "Opening Gap Strategy", session: 0, dir: "Long/Short",
            status: Status::NotTestable("discretionary: 'several down bars' / 'several up bars' and overbought/oversold are never quantified"),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"An upside gap that occurs after several down bars is considered to be professional ... An upside gap that occurs after several up bars tends to be novice driven with a higher probability of filling the gap\" (Novice_vs_Professional_Gaps.doc)",
                     "X|No entry, exit or count of bars is given"],
            notes: "Interpretive guidance for choosing which gaps to fade; no mechanical rule or exit.", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "lpgap_fade" => Some(("lpgap", "Opening Gap Strategy", "Fade to prior close")),
        "lpgap_novice_pro" => Some(("lpgap", "Opening Gap Strategy", "Novice vs professional · discretionary")),
        "lpgap_fade_spx" => Some(("lpgap", "Opening Gap Strategy", "Fade to prior close · SPX 1-minute")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lpgap_fade_spx" => Some(vec![p("gap_min", "GapSizeMin (S&P points; report 9)", 9.0, 0.0, 200.0, 1.0), p("gap_max", "GapSizeMax (S&P points; report 150)", 150.0, 10.0, 1000.0, 10.0),
            p("stop_pct", "StopLossByPct (% of gap; report 95)", 95.0, 5.0, 200.0, 5.0)]),
        "lpgap_fade" => Some(vec![
            p("gap_min", "GapSizeMin (points)", 9.0, 0.0, 200.0, 1.0),
            p("gap_max", "GapSizeMax (points)", 150.0, 10.0, 1000.0, 10.0),
            p("stop_pct", "StopLossByPct (% of gap; >0 wins over points)", 95.0, 0.0, 200.0, 5.0),
            p("stop_pts", "StopLossByPts (points)", 0.0, 0.0, 500.0, 5.0),
            p("margin_pct", "TargetMarginByPct (% of gap; >0 wins over points)", 0.0, 0.0, 90.0, 5.0),
            p("margin_pts", "TargetMarginByPts (points)", 0.0, 0.0, 100.0, 1.0),
            p("maxgap_fill", "MaxGapToFill (points, 0 = off)", 0.0, 0.0, 1000.0, 10.0),
            p("maxgap_pct", "MaxGapLimitPct (% of gap)", 0.0, 0.0, 100.0, 5.0),
            p("target_first", "Daily-bar assumption, not from the source: 0 = stop first when stop and target share a day (default), 1 = target first (upper bound)", 0.0, 0.0, 1.0, 1.0),
        ]),
        _ => None,
    }
}

pub const SPX: &[&str] = &["lpgap_fade_spx"];
/// The gap fade on the real SPX minutes: entry at the first minute's open when |open - prior daily close| is inside the band,
/// target = the prior close, stop = `stop_pct` of the gap beyond the entry, both live from the first minute, out at the last close.
fn fade_minutes(x: &Ctx, from: usize, to: usize, gmin: f64, gmax: f64, spct: f64) -> Vec<Trade> {
    use crate::books::n1intra::{minute, Ord};
    #[derive(Clone)]
    struct S { pos: Option<f64>, x: Option<(f64, &'static str)> }
    let mut out = vec![];
    for i in from.max(1)..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let (o, pc) = (m[0].o, x.c[i - 1]);
        let (gap, size) = (o - pc, (o - pc).abs());
        if gap == 0.0 || size < gmin || size > gmax { continue; }
        let d = if gap < 0.0 { 1.0 } else { -1.0 };
        let (tgt, stp) = (o + d * size, o - d * size * spct / 100.0);
        let orders = |s: &S| -> Vec<Ord> { if s.x.is_some() { vec![] } else { vec![Ord { up: d > 0.0, lv: tgt, tag: 1 }, Ord { up: d < 0.0, lv: stp, tag: 2 }] } };
        let fill = |s: &mut S, t: u8, px: f64| { s.x = Some((px, if t == 1 { "target" } else { "stop" })); };
        let mut st = S { pos: Some(o), x: None };
        for q in &m {
            minute(q, &mut st, &orders, &fill, d < 0.0);
            if st.x.is_some() { break; }
        }
        let (xp, why) = st.x.unwrap_or((m[m.len() - 1].c, "end of day"));
        out.push(crate::intraday::day_trade(d, i, o, i, xp, why, "open", false));
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "lpgap_fade_spx" {
        if to >= x.c.len() || from > to { return None; }
        let g = |k: &str| super::jb624::getp(id, p, k);
        return Some(fade_minutes(x, from, to, g("gap_min"), g("gap_max"), g("stop_pct")));
    }
    let defs = params(id)?;
    if from >= to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def);
    let (gmin, gmax) = (g("gap_min"), g("gap_max"));
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        let gap = x.o[i] - x.c[i - 1];
        let size = gap.abs();
        if gap == 0.0 || size < gmin || size > gmax {
            continue;
        }
        let dir = if gap < 0.0 { 1.0 } else { -1.0 };
        // target distance from the entry (open), in points
        let dist = if g("maxgap_fill") > 0.0 && size >= g("maxgap_fill") {
            size * g("maxgap_pct") / 100.0
        } else if g("margin_pct") > 0.0 {
            size * (1.0 - g("margin_pct") / 100.0)
        } else if g("margin_pts") > 0.0 {
            // "must be less than the day's gap size or the program will reset it to 1 point less than the day's gap size"
            if g("margin_pts") < size { size - g("margin_pts") } else { 1.0 }
        } else {
            size
        };
        let stop_d = if g("stop_pct") > 0.0 { size * g("stop_pct") / 100.0 } else { g("stop_pts") };
        let (o, tgt, stp) = (x.o[i], x.o[i] + dir * dist, x.o[i] - dir * stop_d);
        let stop_hit = if dir > 0.0 { x.l[i] <= stp } else { x.h[i] >= stp };
        let tgt_hit = if dir > 0.0 { x.h[i] >= tgt } else { x.l[i] <= tgt };
        let (px, why) = if tgt_hit && stop_hit && g("target_first") >= 1.0 {
            (tgt, "target (stop also in range: target assumed first)")
        } else if stop_hit {
            (stp, if tgt_hit { "stop (target also in range: stop assumed first)" } else { "stop" })
        } else if tgt_hit {
            (tgt, "target")
        } else {
            (x.c[i], "end of day · close")
        };
        out.push(Trade { dir, legs: vec![Leg { i, px: o, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
            entry_kind: "open", signal_i: i, open: false, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn gap_down_fills_to_prior_close_and_small_gaps_are_ignored() {
        let r = vec![row("2020-01-01", 100., 101., 99., 100.), row("2020-01-02", 80., 101., 79., 90.),
                     row("2020-01-03", 95., 96., 94., 95.), row("2020-01-06", 120., 121., 119., 120.5)];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lpgap_fade", &x, 0, 3, &Params::new()).unwrap();
        // day 1: gap -20 -> long 80, target 100 hit (stop 80-19=61 not hit)
        assert_eq!(ts[0].dir, 1.0);
        assert_eq!(ts[0].exit_px, 100.0);
        // day 2: gap +5 < 9 ignored; day 3: gap +25 -> short 120, stop 143.75 no, target 95 no -> close
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[1].dir, -1.0);
        assert_eq!(ts[1].exit_px, 120.5);
    }
}
