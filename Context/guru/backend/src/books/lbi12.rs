//! Lukac, Brorsen & Irwin, "A Comparison of Twelve Technical Trading Systems" (Lukac M.S. thesis, Purdue; Traders Press reprint 2004, scan).
//! Appendix A (printed p.33-45) gives the twelve rule sets; Appendix B (p.47-59) the yearly optimal parameters.
//! The book never fixes one parameter per system: each trading year uses the parameter that "yielded the highest
//! profit over the preceding three years" (p.6, p.12) from a printed grid (Tables A.1-A.12). That three-year
//! re-optimisation is the default here; `adaptive=0` runs one fixed parameter set (labelled, not the book's procedure).
//! Futures portfolio study (12 commodities 1978-84, $100 round turn): run here on one index (instrument swap).
//! Source and checks: library/lbi12/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::VecDeque;

pub const BOOK: &str = "lbi12";
/// Admitted ids (authors/lukac/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lbi12_chl", "lbi12_par", "lbi12_drm", "lbi12_rnq", "lbi12_drp", "lbi12_mii", "lbi12_lso", "lbi12_ref", "lbi12_dmc", "lbi12_dri", "lbi12_mab", "lbi12_alx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! adapt { () => { "Default = the book's procedure (p.6, p.12): for every calendar year Y the parameter set from the printed grid with the highest profit over the three preceding calendar years (fresh one-unit simulation, points, minus opt_cost% of price per round turn; book: $100 per contract, not convertible to an index, so 0 by default) trades year Y; ties keep the first grid value. A year is traded only when three full calendar years precede it (the book starts BP/DM in 1980 'as soon as three years of data are available', p.8). When the new year's parameter holds a different position than the one carried, the carried position is closed and the new one opened at the year's first open (the book's own dominant-contract roll rule, p.4, applied to the parameter switch: book silent). The run starts flat at the selected start. adaptive=0 runs the fixed parameters below instead (not the book's procedure). Futures system run on the index (instrument swap); no limit days on an index." }; }
macro_rules! d { ($id:expr, $name:expr, $dir:expr, $rules:expr, $notes:expr) => { mk($id, $name, $dir, $rules, concat!(adapt!(), " ", $notes)) }; }

fn mk(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, dir, status: Status::Live, rules, notes, group: "Twelve technical trading systems (Lukac-Brorsen-Irwin)", session: 0,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d!("lbi12_chl", "Close Channel (CHL), 3-year re-optimised", "Long/Short", &[
            "F|\"A price channel is a time interval (present day included) which is L days in length. The highest high is the highest price in the previous (L-1) days. The lowest low is the lowest price in the previous (L-1) days.\" (p.33)",
            "E|\"Buy long when today's high or open is above the highest high in daily price channel. Sell short when today's low or open is below the lowest low.\" (p.33)",
            "E|\"if the open is above (below) the daily channel high (low) ... a long (short) position is entered (exited) at the open price, if the daily high (low) is above (below) the channel high (low) ... the entry (exit) price assumed to be just above (below) the channel high (low)\" (p.33): fill at the channel level (tick unspecified)",
            "X|\"System is always in the market -- never goes neutral.\" Reversal (OAR) is the only exit (p.33)",
            "F|Grid L = 5..60 step 5 (Table A.1, p.47)"],
            "At most one fill per bar; from flat, if both channel sides are broken on one bar the side nearer the open fills first. Same-bar re-reversal is not taken (daily data cannot order it)."),
        d!("lbi12_par", "Parabolic Time/Price (PAR), 3-year re-optimised", "Long/Short", &[
            "F|\"For the second day and thereafter ... SAR(tomorrow) = SAR(today) + AF(EP(trade) - SAR(today))\"; \"Use IF for the beginning value of the AF and increase its value by IF whenever a new high (if long) or new low (if short) for the trade is made ... Do not increase the AF above .20.\" (p.43)",
            "F|\"If long (short) never move the SAR for tomorrow above (below) the previous day's low (high) or today's low (high) ... use the lower low (higher high) between today and the previous day\" (p.43-44)",
            "F|\"When entering a new position on day 1, the SAR is the previous SIP\" (the extreme of the previous trade) (p.43)",
            "E|\"Buy long at the SAR if today's price moves above the SAR. Sell short at the SAR if today's price moves below the SAR. In the event the SAR price cannot be attained, get in at the opening price\" (p.44)",
            "X|\"System always offsets and reverses.\" (p.44)",
            "F|Grid IF = 14..24 'percent' step 1 (Table A.2, p.48); literal unit = IF/100 (default; if_scale=1000 reads it as 0.014-0.024)"],
            "The fixed-parameter rule equals Wilder's parabolic already in the vault (kftsm_parabolic, rwmts_psar) when IF = 0.02; this entry differs by the book's increment grid and three-year re-optimisation. Unit uncertainty: the printed 'percent' 14-24 taken literally gives AF 0.14-0.24 (start above the 0.20 cap for 21-24: the AF is then never increased). Start: a virtual SAR path begins at the first bar in the direction of that bar's close change; the first real position is the first reversal."),
        d!("lbi12_drm", "Directional Movement (DRM, N-day sums), 3-year re-optimised", "Long/Short", &[
            "F|PDM = high - yesterday's high if above, else 0; NDM = yesterday's low - low if below, else 0; inside/equal day: 0; outside day: only the larger counts; TR = largest of H-L, |H-C1|, |L-C1| (p.37-38)",
            "F|\"PDI_N is defined as PDM_N/TR_N where N designates the summation over N days\"; NDI_N = NDM_N/TR_N (p.38)",
            "E|\"Buy long when the PDI_N crosses above the NDI_N. Sell short when the NDI_N crosses below the PDI_N.\" (p.38)",
            "E|Extreme Point Rule: \"when the PDI_N and NDI_N cross, use the extreme point made that day as the reverse point; i.e., the high if short, the low if long\" (p.38)",
            "X|\"stay with these reversals even if not reversed and even if the indexes stay crossed contrary to your position for several days. The reversal point must be crossed before offsetting and reversing.\" (p.38)",
            "F|Grid N = 3..39 step 3 (Table A.3, p.49)"],
            "The printed sell rule 'NDI_N crosses below the PDI_N' (p.38) is read as NDI crossing ABOVE PDI, as the system description states ('sell signals are generated when the negative directional indicator crosses above the positive', p.37). Differs from Wilder's DMI in the vault (wwnc_dmi): plain N-day sums, not Wilder smoothing. Reading: every cross day resets the reverse point to that day's high (when short or flat after an up-cross) or low (when long or flat after a down-cross), as a stop from the next day; gap through = open. After a fill the new position's reverse point is the opposite extreme of the triggering cross day (rules 3-4). Outside day with equal moves: both 0. At most one fill per bar."),
        d!("lbi12_rnq", "Range Quotient (RNQ), 3-year re-optimised", "Long/Short", &[
            "F|TH = max(high, yesterday's close); TL = min(low, yesterday's close); DR = TH - TL; ADR = mean DR over N days; HH/LL = highest high / lowest low in N days (today inclusive); TR = HH - LL; RQ = (1 - ADR/TR) * 100 (p.39)",
            "F|\"A (+) sign is assigned to RQ if today's close is above the close of day 1 of the N day interval and a (-) sign if it is below.\" (p.39)",
            "E|\"Buy long, next day's open, if RQ is greater than +ET. Sell short, next day's open, if RQ is less than -ET.\" (p.39-40)",
            "X|\"Sell offset on next day's open when sign of RQ changes from (+) to (-). Buy offset on next day's open when sign of RQ changes from (-) to (+).\" (p.39-40)",
            "F|Grid N = 20..70 step 5, ET = 55..80 step 5 (Table A.4, p.50)"],
            "Close equal to day 1's close: no sign (no entry, no sign change). An exit and an opposite entry on the same signal day both act at the next open (reversal)."),
        d!("lbi12_drp", "Directional Parabolic (DRP), 3-year re-optimised", "Long/Short", &[
            "E|\"If position is long and parabolic signals short, then if DM is down (up) reverse (offset) at the parabolic stop.\" \"If position is short and parabolic signals long, then if DM is down (up) offset (reverse) at the parabolic stop.\" (p.44)",
            "E|\"If position is neutral and parabolic signals long (short) then if DM is up (down) enter the market long (short) at the parabolic stop.\" (p.44)",
            "E|\"If position is neutral and DM signals long (short) then if parabolic is long (short) enter the market long (short) at the high (low) on the day +DM crossed above (below) the -DM.\" (p.45)",
            "X|\"when in a trade, the exit and/or reverse price is the stop price of the parabolic system regardless of the position of the Directional Movement\"; \"The system does go neutral when trades are not offset\" (p.44)",
            "F|Parabolic and DM as in PAR and DRM; grid N = 3..30 step 3, IF = 14..24 'percent' step 1 (Table A.5, p.51)"],
            "The vault's Kaufman 'Directional Parabolic' (kftsm_directional_parabolic) is not testable there; this is the Lukac rule set. Readings (book silent): the parabolic runs its own always-in path; 'DM up' = PDI_N > NDI_N at the previous close; rule 4 is a stop at the cross day's high (low) from the next day, kept while neutral, DM still up (down) and the parabolic still long (short). If both entries trigger on one bar the nearer level fills. IF unit as in PAR."),
        d!("lbi12_mii", "MII Price Channel (MII), 3-year re-optimised", "Long/Short", &[
            "F|\"The Price Channel is N consecutive days of futures prices including today. The Reference Day (RD) is the first day of the price channel.\" (p.35)",
            "F|\"RDTH is the high of the RD (1st day) or the previous day's close, whichever is higher. RDTL is the low of the RD (1st day) or the previous day's close, whichever is lower.\" (p.35)",
            "E|\"Buy long if today's close is greater (above) the RDTH. Sell short if today's close is less (below) the RDTL.\" Stop Close Only: acted on at the close (p.35-36)",
            "X|\"After the initial position is taken, the system offsets and reverses (OAR).\" (p.36)",
            "F|Grid N = 5..80 step 5 (Table A.6, p.52)"],
            "Reading: 'previous day's close' = the close before the reference day (the theoretical/true high of day 1, as the book defines True High for RNQ, p.39). The introduction (p.35) says 'yesterday's high' instead: ref_mode=1 uses max(high of day 1, yesterday's high) / min(low of day 1, yesterday's low) as a labelled option."),
        d!("lbi12_lso", "L-S-O Price Channel (LSO), 3-year re-optimised", "Long/Short", &[
            "F|N = days in the channel including today; the Reference Interval (RI) = the L consecutive days at the opposite end of the channel; RIH / RIL = its highest high / lowest low (p.34)",
            "E|\"Buy long, on the close, if today's close is higher than the reference interval high within the N-day channel. Sell short, on the close, if today's close is lower than the reference interval low.\" (p.34)",
            "X|\"If long (short), place a sell (buy) stop order halfway between the RIH and RIL ... This is an intraday stop calculated everyday. If the stop is triggered the system is stopped out and the trader has a neutral position.\" (p.34-35)",
            "F|Grid N = 20..70 step 5, RI = 12..90 step 3 (Table A.7, p.53)"],
            "The printed grid includes RI >= N (e.g. corn 1982 = 55,90), which the RI definition cannot express; such pairs are skipped (no profit). The stop of day t uses day t's RI (fully in the past for RI < N); gap through = open. The stop is checked before the close rules of the same day. 'RIJ + RIL' printed for the midpoint is read with 'halfway'."),
        d!("lbi12_ref", "Reference Deviation (REF), 3-year re-optimised", "Long/Short", &[
            "F|RMA = simple average of the closes over the last N days; DRD = close - RMA; NDV = sum of DRD over the previous N days; TDV = sum of |DRD|; RDV = NDV/TDV x 100 (p.40)",
            "E|\"Buy long on tomorrow's open if RDV is greater than +ET. Sell short on tomorrow's open if RDV is less than -ET.\" (p.40)",
            "X|\"Sell offset on tomorrow's open if RDV is less than zero. Buy offset on tomorrow's open if RDV is greater than zero.\" (p.40)",
            "F|Grid N = 5..50 step 5, ET = 10..80 step 10 (Table A.8, p.54)"],
            "The fixed rule equals the vault's stad10_refdev (Brower); this entry differs by the book's grid and three-year re-optimisation."),
        d!("lbi12_dmc", "Dual Moving Average Crossover (DMC), 3-year re-optimised", "Long/Short", &[
            "E|\"Buy long on tomorrow's open when the SMA is greater than (above) the LMA. Sell short on tomorrow's open when the SMA is less than (below) the LMA.\" (p.42)",
            "X|\"System is reversing, always in the market either long or short, thus is OAR when it trades.\" (p.42)",
            "F|Grid SM = 5..25 step 5, LM = 30..65 step 5 (Table A.9, p.55)"],
            "Simple averages. The fixed rule duplicates many vault crossovers; this entry differs by the grid and the re-optimisation."),
        d!("lbi12_dri", "Directional Indicator (DRI), 3-year re-optimised", "Long/Short", &[
            "F|NPC = today's close - the close N days ago; TPC = sum of |close-to-close changes| over N days; DI = NPC / TPC x 100 (printed '(NPC + TPC) x 100'; the book's own 'expressed as a percent' and negative entry threshold require the ratio) (p.36)",
            "E|\"Buy long on tomorrow's open when today's DI becomes equal to or more positive than the positive entry threshold (+ET). Sell short on tomorrow's open when today's DI becomes equal to or more negative than (-ET).\" (p.37)",
            "X|\"Sell (offset) on tomorrow's open when today's DI becomes less than or equal to zero. Buy (offset) ... when today's DI becomes greater than or equal to zero.\" (p.37)",
            "F|Grid N = 20..60 step 5, ET = 12..90 step 3 (Table A.10, p.56)"],
            "'Becomes' read as the state on the signal day (flat and DI >= ET enters)."),
        d!("lbi12_mab", "Moving Average with % Price Band (MAB), 3-year re-optimised", "Long/Short", &[
            "E|\"Buy long on tomorrow's open when today's closing price is higher than the upper band limit. Sell short on tomorrow's open when today's closing price is below the lower band limit.\" (p.41)",
            "X|\"Sell offset on tomorrow's open when today's closing price drops below the moving average. Buy offset on tomorrow's open when today's closing price rises above the moving average.\" (p.41)",
            "F|Simple moving average of N closes; band 'centered around it based on a percent' P (p.41); grid N = 5..60 step 5, P = 5..60 percent step 5 (Table A.11, p.57)"],
            "Band = MA x (1 +/- P/100) (literal percent). Uncertainty: a literal 5-60% band rarely triggers on most markets, yet the book reports trading results for every year (e.g. corn 1978 (40,40), Table A.14 -3.00%), so the printed unit is doubtful; p_scale=1000 reads P as tenths of a percent (labelled option, not the book's text)."),
        d!("lbi12_alx", "Alexander's Filter Rule (ALX), 3-year re-optimised", "Long/Short", &[
            "F|HEP = highest high obtained while in a long trade; LEP = lowest low obtained while in a short trade; X% filter (p.45)",
            "E|\"Buy long on the close if the closing price rises X% above an LEP. Sell short on the close if the closing price falls X% below an HEP.\" (p.45)",
            "X|\"System always offsets and reverses.\" (p.45)",
            "F|Grid X = 1..20 percent step 1 (Table A.12, p.58)"],
            "Table A.12 prints chosen filters of 23 and 24 although its footnote gives the 1-20 grid; the footnote grid is used. Extremes are tracked from the bar after the entry (the entry is at the close), starting at the entry price. Before the first trade the lowest low / highest high since the start are used (book silent). 'Rises X% above' read as close >= LEP x (1 + X/100)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "lbi12_chl" => "Close channel", "lbi12_lso" => "L-S-O price channel", "lbi12_mii" => "MII price channel",
        "lbi12_dri" => "Directional indicator", "lbi12_drm" => "Directional movement", "lbi12_rnq" => "Range quotient",
        "lbi12_ref" => "Reference deviation", "lbi12_mab" => "MA with % band", "lbi12_dmc" => "Dual MA crossover",
        "lbi12_par" => "Parabolic", "lbi12_drp" => "Directional parabolic", "lbi12_alx" => "Alexander filter",
        _ => return None,
    };
    Some(("lbi12_twelve", "Twelve technical trading systems (Lukac et al.)", v))
}

/// Grid (min, max, step) per system parameter, as printed in Tables A.1-A.12; fixed-mode default = grid midpoint.
fn grid(id: &str) -> Option<Vec<(&'static str, &'static str, f64, f64, f64)>> {
    Some(match id {
        "lbi12_chl" => vec![("l", "Channel days L (fixed mode)", 5., 60., 5.)],
        "lbi12_par" => vec![("if", "Increment factor IF (printed 'percent', fixed mode)", 14., 24., 1.)],
        "lbi12_drm" => vec![("n", "DM days N (fixed mode)", 3., 39., 3.)],
        "lbi12_rnq" => vec![("n", "RQ days N (fixed mode)", 20., 70., 5.), ("et", "Entry threshold ET (fixed mode)", 55., 80., 5.)],
        "lbi12_drp" => vec![("n", "DM days N (fixed mode)", 3., 30., 3.), ("if", "Increment factor IF (fixed mode)", 14., 24., 1.)],
        "lbi12_mii" => vec![("n", "Channel days N (fixed mode)", 5., 80., 5.)],
        "lbi12_lso" => vec![("n", "Channel days N (fixed mode)", 20., 70., 5.), ("ri", "Reference interval days (fixed mode)", 12., 90., 3.)],
        "lbi12_ref" => vec![("n", "RMA days N (fixed mode)", 5., 50., 5.), ("et", "Entry threshold ET (fixed mode)", 10., 80., 10.)],
        "lbi12_dmc" => vec![("sm", "Short MA days (fixed mode)", 5., 25., 5.), ("lm", "Long MA days (fixed mode)", 30., 65., 5.)],
        "lbi12_dri" => vec![("n", "DI days N (fixed mode)", 20., 60., 5.), ("et", "Entry threshold ET (fixed mode)", 12., 90., 3.)],
        "lbi12_mab" => vec![("n", "MA days N (fixed mode)", 5., 60., 5.), ("p", "Band P percent (fixed mode)", 5., 60., 5.)],
        "lbi12_alx" => vec![("x", "Filter X percent (fixed mode)", 1., 20., 1.)],
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let g = grid(id)?;
    let mut v = vec![ParamDef { key: "adaptive", label: "1 = book's 3-year re-optimisation (default), 0 = fixed parameters", def: 1., min: 0., max: 1., step: 1. },
        ParamDef { key: "opt_cost", label: "Round-turn cost % of price used when ranking parameters (book: $100/contract)", def: 0., min: 0., max: 1., step: 0.05 }];
    for (key, label, lo, hi, st) in g {
        let mid = lo + (((hi - lo) / st / 2.).floor()) * st;
        v.push(ParamDef { key, label, def: mid, min: lo, max: hi, step: st });
    }
    match id {
        "lbi12_par" | "lbi12_drp" => v.push(ParamDef { key: "if_scale", label: "IF divisor: 100 = literal percent (default), 1000 = read as tenths", def: 100., min: 100., max: 1000., step: 900. }),
        "lbi12_mab" => v.push(ParamDef { key: "p_scale", label: "Band divisor: 100 = literal percent (default), 1000 = tenths of a percent", def: 100., min: 100., max: 1000., step: 900. }),
        "lbi12_mii" => v.push(ParamDef { key: "ref_mode", label: "0 = true high/low of day 1 (definitions, default), 1 = day 1 vs yesterday's high/low (intro)", def: 0., min: 0., max: 1., step: 1. }),
        _ => {}
    }
    Some(v)
}

// ------------------------------------------------------------------ trade helpers

fn open_t(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
/// Position book: one position, reversals allowed.
struct Book { cur: Option<Trade>, out: Vec<Trade> }
impl Book {
    fn new() -> Self { Book { cur: None, out: vec![] } }
    fn dir(&self) -> f64 { self.cur.as_ref().map_or(0., |t| t.dir) }
    fn exit(&mut self, i: usize, px: f64, sig: usize, why: &str) {
        if let Some(mut t) = self.cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); self.out.push(t); }
    }
    fn go(&mut self, dir: f64, i: usize, px: f64, sig: usize, kind: &'static str, why: &str) {
        if self.dir() == dir { return; }
        self.exit(i, px, sig, why);
        if dir != 0. { let mut t = open_t(dir, i, px, kind); t.signal_i = sig; self.cur = Some(t); }
    }
    fn done(mut self, x: &Ctx, e: usize) -> Vec<Trade> {
        if let Some(mut t) = self.cur.take() { t.exit_i = e; t.exit_px = x.c[e]; t.signal_i = e; t.reason = "open".into(); t.open = true; self.out.push(t); }
        self.out
    }
}

fn roll(v: &[f64], w0: usize, e: usize, n: usize, max: bool) -> Vec<f64> {
    // out[k] = max/min of v[w0+k+1-n ..= w0+k] (NaN until n values)
    let mut out = vec![f64::NAN; e + 1 - w0];
    let mut q: VecDeque<usize> = VecDeque::new();
    for i in w0..=e {
        while let Some(&b) = q.back() { if (max && v[b] <= v[i]) || (!max && v[b] >= v[i]) { q.pop_back(); } else { break; } }
        q.push_back(i);
        while let Some(&f) = q.front() { if f + n <= i { q.pop_front(); } else { break; } }
        if i + 1 >= w0 + n { out[i - w0] = v[*q.front().unwrap()]; }
    }
    out
}
fn sma_local(v: &[f64], w0: usize, e: usize, n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; e + 1 - w0];
    let mut s = 0.;
    for i in w0..=e { s += v[i]; if i >= w0 + n { s -= v[i - n]; } if i + 1 >= w0 + n { out[i - w0] = s / n as f64; } }
    out
}
/// Lukac DM: N-day sums of PDM/NDM/TR; returns (PDI, NDI) over the local window.
fn dm_local(x: &Ctx, w0: usize, e: usize, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = e + 1 - w0;
    let (mut p, mut m, mut tr) = (vec![0.; len], vec![0.; len], vec![0.; len]);
    for i in w0.max(1)..=e {
        let up = x.h[i] - x.h[i - 1];
        let dn = x.l[i - 1] - x.l[i];
        let (a, b) = (up.max(0.), dn.max(0.));
        let (pd, nd) = if a > 0. && b > 0. { if a > b { (a, 0.) } else if b > a { (0., b) } else { (0., 0.) } } else { (a, b) };
        p[i - w0] = pd; m[i - w0] = nd;
        tr[i - w0] = (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs());
    }
    let (mut pdi, mut ndi) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    let (mut sp, mut sm, mut st) = (0., 0., 0.);
    let first = if w0 == 0 { 1 } else { 0 }; // local index of the first bar with a true range
    for k in 0..len {
        sp += p[k]; sm += m[k]; st += tr[k];
        if k >= first + n { sp -= p[k - n]; sm -= m[k - n]; st -= tr[k - n]; }
        if k + 1 >= first + n && st > 0. { pdi[k] = sp / st; ndi[k] = sm / st; }
    }
    (pdi, ndi)
}

/// Parabolic state, one bar at a time. Returns Some((new_dir, fill)) on a reversal.
struct Sar { dir: f64, sar: f64, ep: f64, af: f64, inc: f64 }
impl Sar {
    fn start(x: &Ctx, i: usize, inc: f64) -> Sar {
        let up = i > 0 && x.c[i] >= x.c[i - 1];
        let (lo, hi) = if i > 0 { (x.l[i].min(x.l[i - 1]), x.h[i].max(x.h[i - 1])) } else { (x.l[i], x.h[i]) };
        if up { Sar { dir: 1., sar: lo, ep: x.h[i], af: inc, inc } } else { Sar { dir: -1., sar: hi, ep: x.l[i], af: inc, inc } }
    }
    fn bump(&mut self) { if self.af < 0.20 { self.af = (self.af + self.inc).min(0.20); } }
    /// Process bar i (the SAR for bar i is self.sar), then set the SAR for bar i+1.
    fn step(&mut self, x: &Ctx, i: usize) -> Option<(f64, f64)> {
        let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
        let mut flip = None;
        if self.dir > 0. && l <= self.sar {
            let px = o.min(self.sar);
            let sip = self.ep.max(h);
            self.dir = -1.; self.sar = sip; self.ep = l; self.af = self.inc;
            flip = Some((-1., px));
        } else if self.dir < 0. && h >= self.sar {
            let px = o.max(self.sar);
            let sip = self.ep.min(l);
            self.dir = 1.; self.sar = sip; self.ep = h; self.af = self.inc;
            flip = Some((1., px));
        } else {
            if self.dir > 0. {
                if h > self.ep { self.ep = h; self.bump(); }
                let mut s = self.sar + self.af * (self.ep - self.sar);
                let floor = if i > 0 { l.min(x.l[i - 1]) } else { l };
                if s > floor { s = floor; }
                self.sar = s;
            } else {
                if l < self.ep { self.ep = l; self.bump(); }
                let mut s = self.sar - self.af * (self.sar - self.ep);
                let cap = if i > 0 { h.max(x.h[i - 1]) } else { h };
                if s < cap { s = cap; }
                self.sar = s;
            }
        }
        flip
    }
}

// ------------------------------------------------------------------ the twelve simulators
// sim(x, p, s, e): start flat at bar s, trade through e (indicators may use bars before s), open trade marked at e.

fn warm(id: &str, p: &[f64]) -> usize {
    match id {
        "lbi12_ref" => 2 * p[0] as usize + 2,
        "lbi12_mab" | "lbi12_rnq" | "lbi12_dri" | "lbi12_mii" | "lbi12_lso" | "lbi12_chl" | "lbi12_drm" | "lbi12_drp" => p[0] as usize + 2,
        "lbi12_dmc" => p[1] as usize + 2,
        _ => 2,
    }
}

fn sim(id: &str, x: &Ctx, p: &[f64], s: usize, e: usize, opt: &Opt) -> Vec<Trade> {
    let w0 = s.saturating_sub(warm(id, p));
    let mut b = Book::new();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let k = |i: usize| i - w0;
    match id {
        "lbi12_chl" => {
            let n = (p[0] as usize).max(2) - 1;
            let (hh, ll) = (roll(h, w0, e, n, true), roll(l, w0, e, n, false));
            for i in s.max(w0 + 1)..=e {
                let (hi, lo) = (hh[k(i - 1)], ll[k(i - 1)]);
                if !hi.is_finite() { continue; }
                let buy = if o[i] > hi { Some(o[i]) } else if h[i] > hi { Some(hi) } else { None };
                let sell = if o[i] < lo { Some(o[i]) } else if l[i] < lo { Some(lo) } else { None };
                let d = b.dir();
                if d > 0. { if let Some(px) = sell { b.go(-1., i, px, i, "stop", "channel low broken (OAR)"); } }
                else if d < 0. { if let Some(px) = buy { b.go(1., i, px, i, "stop", "channel high broken (OAR)"); } }
                else {
                    match (buy, sell) {
                        (Some(bp), Some(sp)) => { if (o[i] - bp).abs() <= (o[i] - sp).abs() { b.go(1., i, bp, i, "stop", "") } else { b.go(-1., i, sp, i, "stop", "") } }
                        (Some(bp), None) => b.go(1., i, bp, i, "stop", ""),
                        (None, Some(sp)) => b.go(-1., i, sp, i, "stop", ""),
                        _ => {}
                    }
                }
            }
        }
        "lbi12_lso" => {
            let (n, ri) = (p[0] as usize, p[1] as usize);
            if ri >= n { return vec![]; }
            // RI of bar i = bars i-n+1 ..= i-n+ri : rolling(ri) evaluated at bar i-n+ri
            let (mx, mn) = (roll(h, w0, e, ri, true), roll(l, w0, e, ri, false));
            let at = |i: usize| -> (f64, f64) { let j = i + ri; if j < n + w0 { return (f64::NAN, f64::NAN); } let j = j - n; (mx[j - w0], mn[j - w0]) };
            for i in s..=e {
                let (rih, ril) = at(i);
                if !rih.is_finite() { continue; }
                let mid = (rih + ril) / 2.;
                let d = b.dir();
                if d > 0. && l[i] <= mid { b.exit(i, o[i].min(mid), i, "midpoint stop (neutral)"); }
                else if d < 0. && h[i] >= mid { b.exit(i, o[i].max(mid), i, "midpoint stop (neutral)"); }
                if c[i] > rih { b.go(1., i, c[i], i, "close", "close below RI low (reverse)"); }
                else if c[i] < ril { b.go(-1., i, c[i], i, "close", "close above RI high (reverse)"); }
            }
        }
        "lbi12_mii" => {
            let n = p[0] as usize;
            for i in s.max(n)..=e {
                let rd = i + 1 - n;
                if rd == 0 { continue; }
                let (th, tl) = if opt.mii_ref == 1 { (h[rd].max(h[i - 1]), l[rd].min(l[i - 1])) } else { (h[rd].max(c[rd - 1]), l[rd].min(c[rd - 1])) };
                if c[i] > th { b.go(1., i, c[i], i, "close", "close below RDTL (OAR)"); }
                else if c[i] < tl { b.go(-1., i, c[i], i, "close", "close above RDTH (OAR)"); }
            }
        }
        "lbi12_dri" | "lbi12_rnq" | "lbi12_ref" | "lbi12_mab" | "lbi12_dmc" => {
            // close-signal systems acting at the next open. sig(i) -> (exit_long, exit_short, enter: +1/-1/0)
            let len = e + 1 - w0;
            let mut ind = vec![f64::NAN; len];
            let mut aux = vec![f64::NAN; len];
            match id {
                "lbi12_dri" => {
                    let n = p[0] as usize;
                    let mut sabs = 0.;
                    for i in w0 + 1..=e { sabs += (c[i] - c[i - 1]).abs(); if i >= w0 + 1 + n { sabs -= (c[i - n] - c[i - n - 1]).abs(); }
                        if i >= w0 + n && sabs > 0. { ind[k(i)] = 100. * (c[i] - c[i - n]) / sabs; } }
                }
                "lbi12_rnq" => {
                    let n = p[0] as usize;
                    let dr: Vec<f64> = (0..=e).map(|i| if i == 0 || i < w0 { 0. } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
                    let (hh, ll) = (roll(h, w0, e, n, true), roll(l, w0, e, n, false));
                    let adr = sma_local(&dr, w0 + 1, e, n);
                    for i in w0 + n..=e {
                        let tr = hh[k(i)] - ll[k(i)];
                        let a = adr[i - w0 - 1];
                        if tr > 0. && a.is_finite() {
                            let rq = (1. - a / tr) * 100.;
                            let sg = (c[i] - c[i + 1 - n]).signum() * if c[i] == c[i + 1 - n] { 0. } else { 1. };
                            ind[k(i)] = sg * rq; aux[k(i)] = sg;
                        }
                    }
                }
                "lbi12_ref" => {
                    let n = p[0] as usize;
                    let ma = sma_local(c, w0, e, n);
                    let drd: Vec<f64> = (0..len).map(|j| c[w0 + j] - ma[j]).collect();
                    let (mut sn, mut sa, mut cnt) = (0., 0., 0usize);
                    for j in 0..len {
                        if drd[j].is_finite() { sn += drd[j]; sa += drd[j].abs(); cnt += 1;
                            if cnt > n { sn -= drd[j - n]; sa -= drd[j - n].abs(); }
                            if cnt >= n && sa > 0. { ind[j] = 100. * sn / sa; } }
                    }
                }
                "lbi12_mab" => {
                    let ma = sma_local(c, w0, e, p[0] as usize);
                    ind = ma; aux = vec![p[1] / opt.p_scale; len];
                }
                _ => {
                    let (a, z) = (sma_local(c, w0, e, p[0] as usize), sma_local(c, w0, e, p[1] as usize));
                    for j in 0..len { ind[j] = a[j] - z[j]; }
                }
            }
            let et = if p.len() > 1 { p[1] } else { 0. };
            let mut pend: Option<(f64, usize, &'static str)> = None; // (target dir, signal bar, why) to act at the next open
            for i in s..=e {
                if let Some((dir, sg, why)) = pend.take() {
                    if dir == 0. { b.exit(i, o[i], sg, why); } else { b.go(dir, i, o[i], sg, "open", why); }
                }
                if i == e { break; }
                let v = ind[k(i)];
                if !v.is_finite() { continue; }
                let d = b.dir();
                let (lx, sx, le, se) = match id {
                    "lbi12_dri" => (v <= 0., v >= 0., v >= et, v <= -et),
                    "lbi12_ref" => (v < 0., v > 0., v > et, v < -et),
                    "lbi12_rnq" => { let sg = aux[k(i)]; (sg < 0., sg > 0., v > et, v < -et) }
                    "lbi12_mab" => { let band = aux[k(i)]; (c[i] < v, c[i] > v, c[i] > v * (1. + band), c[i] < v * (1. - band)) }
                    _ => (v < 0., v > 0., v > 0., v < 0.),
                };
                let mut target = d;
                if (d > 0. && lx) || (d < 0. && sx) { target = 0.; }
                if target == 0. { if le { target = 1.; } else if se { target = -1.; } }
                if target != d {
                    let why = if target == 0. { "offset signal" } else if d != 0. { "reverse signal" } else { "entry signal" };
                    pend = Some((target, i, why));
                }
            }
        }
        "lbi12_drm" => {
            let n = p[0] as usize;
            let (pdi, ndi) = dm_local(x, w0, e, n);
            // Extreme-point rule: the last cross day's high/low; the active reverse point flips between them.
            let mut pend: Option<(f64, f64)> = None; // (direction taken when hit, level)
            let (mut xh, mut xl) = (f64::NAN, f64::NAN);
            for i in s.max(w0 + 1)..=e {
                if let Some((dir, lvl)) = pend {
                    let hit = if dir > 0. { (h[i] >= lvl).then(|| o[i].max(lvl)) } else { (l[i] <= lvl).then(|| o[i].min(lvl)) };
                    if let Some(px) = hit {
                        b.go(dir, i, px, i, "stop", "extreme-point reverse");
                        pend = Some(if dir > 0. { (-1., xl) } else { (1., xh) });
                        continue; // at most one fill per bar; a cross on a fill bar is not re-read
                    }
                }
                let (a, z, a1, z1) = (pdi[k(i)], ndi[k(i)], pdi[k(i - 1)], ndi[k(i - 1)]);
                if !(a.is_finite() && a1.is_finite()) { continue; }
                let up = a > z && a1 <= z1;
                let dn = z > a && z1 <= a1;
                if up || dn {
                    xh = h[i]; xl = l[i];
                    let d = b.dir();
                    pend = Some(if d > 0. || (d == 0. && dn) { (-1., xl) } else { (1., xh) });
                }
            }
        }
        "lbi12_drp" => {
            let n = p[0] as usize;
            let inc = p[1] / opt.if_scale;
            let (pdi, ndi) = dm_local(x, w0, e, n);
            let mut par = Sar::start(x, s.max(1).min(e), inc);
            let mut dm_up: Option<bool> = None;
            let mut r4: Option<(f64, f64)> = None; // rule-4 stop (dir, level)
            for i in s.max(1) + 1..=e {
                let flip = par.step(x, i);
                let d = b.dir();
                let mut acted = false;
                if let (Some((pd, px)), Some(up)) = (flip, dm_up) {
                    if d > 0. && pd < 0. { if up { b.exit(i, px, i, "parabolic short, DM up: offset") } else { b.go(-1., i, px, i, "stop", "parabolic short, DM down: reverse") } acted = true; }
                    else if d < 0. && pd > 0. { if !up { b.exit(i, px, i, "parabolic long, DM down: offset") } else { b.go(1., i, px, i, "stop", "parabolic long, DM up: reverse") } acted = true; }
                    else if d == 0. && ((pd > 0. && up) || (pd < 0. && !up)) {
                        let mut fill = px;
                        if let Some((rd, lvl)) = r4 { if rd == pd && ((pd > 0. && h[i] >= lvl) || (pd < 0. && l[i] <= lvl)) {
                            let rp = if pd > 0. { o[i].max(lvl) } else { o[i].min(lvl) };
                            fill = if pd > 0. { fill.min(rp) } else { fill.max(rp) }; } }
                        b.go(pd, i, fill, i, "stop", "parabolic with DM: enter"); acted = true;
                    }
                }
                if !acted && b.dir() == 0. {
                    if let Some((rd, lvl)) = r4 {
                        if rd > 0. && h[i] >= lvl { b.go(1., i, o[i].max(lvl), i, "stop", "DM cross with parabolic long: cross-day high"); acted = true; }
                        else if rd < 0. && l[i] <= lvl { b.go(-1., i, o[i].min(lvl), i, "stop", "DM cross with parabolic short: cross-day low"); acted = true; }
                    }
                }
                if acted || b.dir() != 0. { r4 = None; }
                // close of bar i: DM state and rule-4 orders for the next bar
                let (a, z) = (pdi[k(i)], ndi[k(i)]);
                if a.is_finite() && z.is_finite() {
                    let prev = dm_up;
                    if a > z { dm_up = Some(true); } else if z > a { dm_up = Some(false); }
                    let crossed = prev.is_some() && prev != dm_up;
                    if b.dir() == 0. {
                        if crossed && dm_up == Some(true) && par.dir > 0. { r4 = Some((1., h[i])); }
                        else if crossed && dm_up == Some(false) && par.dir < 0. { r4 = Some((-1., l[i])); }
                        else if let Some((rd, _)) = r4 { if (rd > 0.) != (dm_up == Some(true)) || rd != par.dir { r4 = None; } }
                    }
                }
            }
        }
        "lbi12_par" => {
            let inc = p[0] / opt.if_scale;
            let mut par = Sar::start(x, s.max(1).min(e), inc);
            for i in s.max(1) + 1..=e {
                if let Some((dir, px)) = par.step(x, i) { b.go(dir, i, px, i, "stop", "SAR penetrated (reverse)"); }
            }
        }
        "lbi12_alx" => {
            let xf = p[0] / 100.;
            let (mut hep, mut lep) = (h[s], l[s]);
            for i in s..=e {
                let d = b.dir();
                if d > 0. && i > b.cur.as_ref().unwrap().legs[0].i { hep = hep.max(h[i]); }
                if d < 0. && i > b.cur.as_ref().unwrap().legs[0].i { lep = lep.min(l[i]); }
                if d == 0. { hep = hep.max(h[i]); lep = lep.min(l[i]); }
                if d <= 0. && c[i] >= lep * (1. + xf) { b.go(1., i, c[i], i, "close", "close X% below the high extreme (reverse)"); hep = c[i]; }
                else if d >= 0. && c[i] <= hep * (1. - xf) { b.go(-1., i, c[i], i, "close", "close X% above the low extreme (reverse)"); lep = c[i]; }
            }
        }
        _ => {}
    }
    b.done(x, e)
}

pub struct Opt { if_scale: f64, p_scale: f64, mii_ref: u8 }

fn profit(ts: &[Trade], cost: f64) -> f64 {
    ts.iter().map(|t| t.dir * (t.exit_px - t.legs[0].px) - cost * t.legs[0].px).sum()
}

fn year(x: &Ctx, i: usize) -> i32 { x.r[i].date[..4].parse().unwrap_or(0) }

fn combos(g: &[(&str, &str, f64, f64, f64)]) -> Vec<Vec<f64>> {
    let mut out = vec![vec![]];
    for &(_, _, lo, hi, st) in g {
        let mut nxt = vec![];
        for base in &out { let mut v = lo; while v <= hi + 1e-9 { let mut b = base.clone(); b.push(v); nxt.push(b); v += st; } }
        out = nxt;
    }
    out
}

/// Book procedure: yearly re-optimisation over the preceding three calendar years.
pub fn adaptive(id: &str, x: &Ctx, from: usize, to: usize, cost: f64, opt: &Opt) -> Vec<Trade> {
    let g = grid(id).unwrap();
    let all = combos(&g);
    let n = x.r.len();
    // first bar index of every calendar year present
    let mut starts: Vec<(i32, usize)> = vec![];
    for i in 0..n { let y = year(x, i); if starts.last().map_or(true, |s| s.0 != y) { starts.push((y, i)); } }
    let first_ok = |y: i32| -> Option<usize> {
        // three full calendar years must precede y: data begins by the 10th of January of y-3
        let s3 = starts.iter().find(|s| s.0 == y - 3)?;
        (x.r[0].date.as_str() <= format!("{}-01-10", y - 3).as_str() || s3.1 > 0).then_some(s3.1)
    };
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    let mut first = true;
    for (yi, &(y, ys)) in starts.iter().enumerate() {
        let yend = starts.get(yi + 1).map_or(n - 1, |s| s.1 - 1);
        if yend < from || ys > to { continue; }
        let Some(ws) = first_ok(y) else { continue };
        let (yb, ye) = (ys.max(from), yend.min(to));
        let we = ys - 1;
        let mut best: Option<(f64, &Vec<f64>)> = None;
        for pv in &all {
            let pr = profit(&sim(id, x, pv, ws, we, opt), cost);
            if best.map_or(true, |(bp, _)| pr > bp + 1e-12) { best = Some((pr, pv)); }
        }
        let pv = best.unwrap().1;
        let ts = sim(id, x, pv, ws, ye, opt);
        let carried = ts.iter().find(|t| t.legs[0].i < yb && (t.open || t.exit_i >= yb)).cloned();
        let label = format!("{} {:?}", y, pv);
        let mut hold: Option<Trade> = None;
        match (cur.take(), carried) {
            (Some(mut c), Some(k)) if c.dir == k.dir => { c.exit_i = k.exit_i; c.exit_px = k.exit_px; c.reason = k.reason.clone(); c.open = k.open; c.signal_i = k.signal_i; hold = Some(c); }
            (Some(mut c), k) => {
                c.exit_i = yb; c.exit_px = x.o[yb]; c.signal_i = yb.saturating_sub(1); c.reason = format!("re-optimisation {label}: position differs"); c.open = false; out.push(c);
                if let Some(k) = k { let mut t = open_t(k.dir, yb, x.o[yb], "open"); t.signal_i = yb.saturating_sub(1); t.exit_i = k.exit_i; t.exit_px = k.exit_px; t.reason = k.reason.clone(); t.open = k.open; hold = Some(t); }
            }
            (None, Some(k)) if !first => { let mut t = open_t(k.dir, yb, x.o[yb], "open"); t.signal_i = yb.saturating_sub(1); t.exit_i = k.exit_i; t.exit_px = k.exit_px; t.reason = k.reason.clone(); t.open = k.open; hold = Some(t); }
            _ => {}
        }
        first = false;
        let mut push = |mut t: Trade, cur: &mut Option<Trade>| {
            if t.open && ye < to { t.open = false; *cur = Some(t); } else {
                if !t.open && t.reason.is_empty() { t.reason = "signal".into(); }
                t.reason = format!("{} · params {label}", t.reason); out.push(t); }
        };
        if let Some(t) = hold { push(t, &mut cur); }
        for t in ts.into_iter().filter(|t| t.legs[0].i >= yb) { push(t, &mut cur); }
    }
    if let Some(mut t) = cur { t.open = true; t.exit_i = to; t.exit_px = x.c[to]; t.reason = "open".into(); out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return None; }
    for (k, v) in p {
        let def = schema.iter().find(|d| d.key == k)?;
        if !v.is_finite() || *v < def.min || *v > def.max { return None; }
    }
    let get = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let opt = Opt { if_scale: get_or(p, "if_scale", 100.), p_scale: get_or(p, "p_scale", 100.), mii_ref: get_or(p, "ref_mode", 0.) as u8 };
    if get("adaptive") >= 0.5 {
        Some(adaptive(id, x, from, to, get("opt_cost") / 100., &opt))
    } else {
        let pv: Vec<f64> = grid(id)?.iter().map(|g| get(g.0)).collect();
        let mut ts = sim(id, x, &pv, from, to, &opt);
        for t in &mut ts { if t.reason.is_empty() { t.reason = "signal".into(); } }
        Some(ts)
    }
}
fn get_or(p: &Params, k: &str, d: f64) -> f64 { p.get(k).copied().unwrap_or(d) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i % 300) / 25, 1 + i % 25),
            open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect()
    }
    #[test]
    fn close_channel_breaks_previous_l_minus_1_days_and_reverses() {
        let mut cl = vec![100.; 30];
        cl[20] = 105.; // high 106 > previous highs 101 -> long at 101 (channel high), open 105 > 101 -> fills at open
        for v in &mut cl[25..] { *v = 90.; }
        let r = rows(&cl); let x = Ctx::new(&r, &r, &[]);
        let ts = sim("lbi12_chl", &x, &[5.], 5, 29, &Opt { if_scale: 100., p_scale: 100., mii_ref: 0 });
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 20); assert_eq!(ts[0].legs[0].px, 105.);
        assert_eq!(ts[0].exit_i, 25); assert_eq!(ts[1].dir, -1.); assert!(ts[1].open);
    }
    #[test]
    fn dri_is_npc_over_tpc_and_trades_next_open() {
        let mut cl: Vec<f64> = (0..40).map(|i| 100. + (i % 2) as f64).collect();
        for (j, v) in cl[30..].iter_mut().enumerate() { *v = 102. + j as f64; }
        let r = rows(&cl); let x = Ctx::new(&r, &r, &[]);
        let ts = sim("lbi12_dri", &x, &[5., 60.], 10, 39, &Opt { if_scale: 100., p_scale: 100., mii_ref: 0 });
        assert_eq!(ts[0].dir, 1.);
        // DI first >= 60 on bar 31 or later; entry is the next bar's open
        assert!(ts[0].legs[0].i >= 32 && ts[0].legs[0].px == x.o[ts[0].legs[0].i]);
    }
}

// Instrument admission: Lukac, Brorsen and Irwin 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lukac/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="lbi12" && crate::books::lbi12::reviewed(def.id),
    label: "Lukac, Brorsen and Irwin tested these twelve systems on 12 futures markets (1978-84), choosing each year's parameters by the best profit of the three years before; runs on one index, ETF or stock (US and India) use the same yearly re-optimisation over the book's grids and are a declared application, one unit per trade, shorts included.",
    basis: "Lukac, Brorsen and Irwin, A Comparison of Twelve Technical Trading Systems (1985; Traders Press 2004), Appendix A p.33-45 and Tables A.1-A.12",
};
