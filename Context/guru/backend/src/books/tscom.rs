//! TradeSystem Inc. (trade-system.com) TradeStation systems recovered from RARs (code read with the read-only libarchive reader):
//! - Aberration Plus (Keith Fitschen): `T:/Aberration Plus for TS (trade-system.com)/Open Code.rar` -> `Open Code.txt`
//!   (system + profit-target / band / risk indicators + the Abb.StdDev function);
//! - Aztec (Keith Fitschen, 1999-2000): `T:/Aztec for TS (trade-system.com)/Open Code.rar` -> `Open Code.txt`;
//! - I-Master: `T:/I-Master for TS (trade-system.com)/Open Code.rar` (catalogued, not testable);
//! - Double Play: `T:/Double Play System for TS (trade-system.com)/Open Code.rar` (catalogued, needs intraday data).
//! (`T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`.) Deviation Dictates (same vendor folder) is the
//! xAvTR system already in the vault as dmst2_xavtr and is not repeated. Book id "tscom". Notes: library/tscom/README.md.
use crate::books::ajmisc::{first_ok, ord, Sim, Ty, K};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tscom";
/// Admitted ids (authors/lanea/vendor1/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["tscom_aberration_plus"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const ABP: &str = "tscom_aberration_plus";
const AZT: &str = "tscom_aztec";

fn def(id: &'static str, name: &'static str, group: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(ABP, "Aberration Plus (80-bar 2-sigma band, stepped MA exits)", "Aberration Plus (Open Code.txt)", Status::Live, &[
            "F|\"Avg.Out = Average(Close,Length)\"; Avg.Out2..5 = Average(Close, Length - int(Length/8)*k), k=1..4 (Length 80 -> 80/70/60/50/40); \"Sigma = Abb.stddev(Close,Length)*2\"; \"Vol.av = (Sigma+Sigma[1]+Sigma[2])/3\"",
            "E|\"if MarketPosition < +1 and Close > Avg.Out + Sigma and Sigma > Vol.av and lps = 0 and Sigma*BigPointValue < Risk then ps = close + Sigma; crit = 1; Buy Next Bar at market\" (Risk 99999)",
            "E|\"if MarketPosition > -1 and Close < Avg.Out - Sigma and Sigma > Vol.av and sps = 0 and Sigma*BigPointValue < Risk then Sell Short Next Bar at market; ps = close - sigma; crit = 1\"",
            "X|Long: \"Close < Avg.Out and crit=1\" (lps=0) / \"Close < Avg.Out2 and crit=2\" ... \"Close < Avg.Out5 and crit=5\" / \"crit=6\" (lps=1) -> Sell Next Bar at market; short mirror with sps",
            "F|\"if MarketPosition = +1 and Close > ps then crit = crit+1; ps = Close + Sigma\" (short mirror); \"if sps = 1 and Close > Avg.Out then sps = 0; if lps = 1 and Close < Avg.Out then lps = 0\""],
            "Keith Fitschen / TradeSystem Inc. code, daily futures. Abb.stddev (in the RAR) = sample standard deviation (divides by Length-1), keeping the previous value when the sum of squares is <= 0.0001. \
Statements run in the code's order every bar (state lps/sps/ps/crit persists), so e.g. a short signal while long resets crit/ps before the long's crit test - kept literal. \
Risk is in dollars per contract: Sigma x BigPointValue with p.bpv = 1 (TradeStation's BigPointValue for a cash index); the default Risk 99999 is the code's 'off' value. \
Run on daily cash indices (instrument swap). No published results in the RAR. India: instrument swap; index shorts hypothetical."),
        def(AZT, "Aztec (mode 2: close-only trend filter, 20-bar sigma breakouts)", "Aztec (Open Code.txt)", Status::NotTestable("its stops and filters are in dollars per contract and the code never states the contract point value; reading $1 as 1 index point is our choice, not the vendor's"), &[
            "F|Mode 2 (default): track HighSoFar/LowSoFar of closes; \"IF pos1 > -1 AND Close < highsofar - abb.stddev(c,NumbarL2)*Pct2 then pos1 = -1; pos2 = 1; highsofar = lowsofar = close\"; up mirror (NumbarL2 70, Pct2 2.4)",
            "F|\"BuyPoint = Close + Abb.stddev(Close,NumBars2)*1.5; SellPoint = Close - ...\" (NumBars2 20), at least 300/BigPointValue away; \"Sigma = Abb.stddev(Close,NumBars2)[1]*1.5*SStop2\" (SStop2 2.0), at least 300/BigPointValue",
            "E|\"MarketPosition = 0 and pos1=1 and pos2 = 1 -> Buy Next Bar at Market\"; \"MarketPosition = 0 and pos1=1 and close > BuyPointP -> Buy\"; \"MarketPosition = -1 and pos1 = 1 -> Buy\" (reverse); short mirror",
            "X|\"MarketPosition = 1 and pos1 = 1 and close < SellPointP -> Sell Next Bar at Market\"; \"MarketPosition = 1 and barssinceentry > 1 and Close < Longstop -> Sell\" (LongStop = EntryPrice - Sigma set on the entry bar); short mirror",
            "E|Mode 1 (p.mode=1, 'Daily Bar Chart, Reversal'): Buy/Sell Short next bar at Close +/- 1.5 x Abb.stddev(8) stop (min 300.1/BPV) when flat or when the open position's close is beyond its stop; stop = EntryPrice -/+ 1.9 x 1.5 x stddev(8)[1] (min 300.1/BPV), exit next open when the close crosses it"],
            "Keith Fitschen code; Mode input default 2 ('Daily Bar Chart, Trend Filter'); p.mode=1 runs the code's Mode 1 block. Dollar floors (300 / 300.1) use p.bpv = 1 (TradeStation BigPointValue of a cash index), so on index points the floors are 300 points: \
on SPX/NIFTY the 1.5-sigma levels are usually replaced by close +/- 300 and the stops are 300 points away - this is what the code does on a cash-index chart; p.bpv sets a futures point value instead (e.g. 250 for S&P). \
Abb.stddev = sample stdev with the previous value kept when the sum of squares <= 0.0001. 'CurrentBar = 1' initialisation = the first bar after the 70-bar warm-up. BuyPointP starts at 0 (code). Daily cash indices (instrument swap); India: instrument swap; index shorts hypothetical."),
        def("tscom_imaster", "I-Master (New Profit Center: open-relative stops, MarketEnergy filter)", "I-Master (Open Code.txt)",
            Status::NotTestable("the counter-trend entries need MarketEnergy(20), a function whose code is not in the source (and NextOpen20 comes from the bundled Nxtopn20.dll)"), &[
            "E|\"if marketposition<>1 and Close < Open and Close < Close[29] and Range < 1.618*Average(Range,80) and MarketStrength > -.50 then Buy Next Bar at NxOpen + 2*(close-low) + .05 stop\" (MarketStrength = marketenergy(20))",
            "X|\"Stopp = 3*Average(Range,80)[2]; ProfitTarget = 4*Average(Range,80)[2]\"; exit next bar at market when the close is beyond EntryPrice -/+ Stopp or +/- ProfitTarget"], ""),
        def("tscom_double_play", "Double Play (ZProbability of data2 highs/lows, close limit)", "Double Play (Open Code.txt)",
            Status::NeedsData("intraday bars of the traded market plus a data2 series: entries only after 'Sess1StartTime + 100', day-trade exit 15 minutes before the session end"), &[
            "E|\"if ProbHigh < HighTrigger and Time > (Sess1StartTime + 100) ... then Buy Next Bar at close limit\" (ProbHigh = ZProbability(High of Data2, 52), HighTrigger .02)",
            "X|\"If DayTrade = \"Y\" and MarketPosition = 1 and Time >= (Sess1EndTime - 15) then Sell this Bar Close\""], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        ABP => ("tscom_aberration_plus", "Aberration Plus (Fitschen)", "code"),
        AZT => ("tscom_aztec", "Aztec (Fitschen)", "code"),
        "tscom_imaster" => ("tscom_imaster", "I-Master (trade-system.com)", "code"),
        "tscom_double_play" => ("tscom_double_play", "Double Play (trade-system.com)", "code"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        ABP => vec![p("length", "Length", 80., 20., 200., 5.), p("risk", "Risk ($ per contract; 99999 = off)", 99999., 100., 99999., 100.), p("bpv", "BigPointValue (1 = cash index)", 1., 1., 500., 1.)],
        AZT => vec![p("mode", "Mode (2 trend filter, 1 reversal)", 2., 1., 2., 1.), p("numbars2", "NumBars2", 20., 5., 60., 1.), p("sstop2", "SStop2", 2., 0.5, 5., 0.1),
            p("numbarl2", "NumbarL2", 70., 20., 200., 5.), p("pct2", "Pct2", 2.4, 0.5, 5., 0.1), p("numbars1", "NumBars1 (mode 1)", 8., 3., 40., 1.), p("sstop1", "SStop1 (mode 1)", 1.9, 0.5, 5., 0.1),
            p("bpv", "BigPointValue (1 = cash index)", 1., 1., 500., 1.)],
        _ => return None,
    })
}

/// Abb.stddev (Open Code.txt): sample stdev over n bars; keeps the previous value when SumSqr <= 0.0001.
pub fn abb_sd(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(2);
    let mut out = vec![f64::NAN; v.len()];
    for i in n - 1..v.len() {
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let ss: f64 = w.iter().map(|x| (x - m) * (x - m)).sum();
        out[i] = if ss > 0.0001 { (ss / (n - 1) as f64).sqrt() } else if i > 0 { out[i - 1] } else { f64::NAN };
    }
    out
}

fn aberration_plus(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let len = g("length", 80.).max(8.) as usize;
    let (risk, bpv) = (g("risk", 99999.), g("bpv", 1.));
    let st8 = len / 8;
    let avgs: Vec<Vec<f64>> = (0..5).map(|k| sma(&x.c, len - st8 * k)).collect();
    let sig: Vec<f64> = abb_sd(&x.c, len).iter().map(|v| v * 2.).collect();
    let n = x.c.len();
    let st = first_ok(n, from.max(2), &[&avgs[0], &sig]).max(len + 1);
    let mut s = Sim::new();
    let (mut lps, mut sps, mut ps, mut crit) = (0, 0, 0f64, 0);
    for i in st..to {
        let c = x.c[i];
        let sg = sig[i];
        let vol_av = (sig[i] + sig[i - 1] + sig[i - 2]) / 3.;
        let (le, se) = (avgs[0][i] + sg, avgs[0][i] - sg);
        let mp = s.pos;
        let mut q = vec![];
        if mp < 1 && c > le && sg > vol_av && lps == 0 && sg * bpv < risk { ps = c + sg; crit = 1; q.push(ord(K::Buy, Ty::Mkt, "le.AberPlus")); }
        if mp > -1 && c < se && sg > vol_av && sps == 0 && sg * bpv < risk { q.push(ord(K::Short, Ty::Mkt, "se.AberPlus")); ps = c - sg; crit = 1; }
        const LX: [&str; 6] = ["lx.AvgOut", "lx.AvgOut2", "lx.AvgOut3", "lx.AvgOut4", "lx.AvgOut5", "lx.profitstop"];
        const SX: [&str; 6] = ["sx.AvgOut", "sx.AvgOut2", "sx.AvgOut3", "sx.AvgOut4", "sx.AvgOut5", "sx.profitstop"];
        if mp == 1 {
            for k in 0..6 {
                if crit == k + 1 && (k == 5 || c < avgs[k][i]) { q.push(ord(K::XL, Ty::Mkt, LX[k])); lps = if k == 0 { 0 } else { 1 }; }
            }
        }
        if mp == -1 {
            for k in 0..6 {
                if crit == k + 1 && (k == 5 || c > avgs[k][i]) { q.push(ord(K::XS, Ty::Mkt, SX[k])); sps = if k == 0 { 0 } else { 1 }; }
            }
        }
        if mp == 1 && c > ps { crit += 1; ps = c + sg; }
        if mp == -1 && c < ps { crit += 1; ps = c - sg; }
        if sps == 1 && c > avgs[0][i] { sps = 0; }
        if lps == 1 && c < avgs[0][i] { lps = 0; }
        s.bar(x, i + 1, &q);
    }
    s.done(x, to)
}

fn aztec(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let bpv = g("bpv", 1.);
    let n = x.c.len();
    let mut s = Sim::new();
    let mut mp_prev = 0i8;
    let (mut long_stop, mut short_stop) = (0f64, 0f64);
    if g("mode", 2.).round() as i32 == 1 {
        let sd = abb_sd(&x.c, g("numbars1", 8.) as usize);
        let ss1 = g("sstop1", 1.9);
        let st = first_ok(n, from.max(1), &[&sd]) + 1;
        for i in st..to {
            let (c, mp) = (x.c[i], s.pos);
            let (mut bp, mut sp) = (c + sd[i] * 1.5, c - sd[i] * 1.5);
            if sd[i] * 1.5 * bpv < 300.1 { bp = c + 300.1 / bpv; sp = c - 300.1 / bpv; }
            let mut sigma = sd[i - 1] * 1.5 * ss1;
            if sigma * bpv < 300. { sigma = 300.1 / bpv; }
            if mp == 1 && mp_prev != 1 { long_stop = s.ep() - sigma; }
            if mp == -1 && mp_prev != -1 { short_stop = s.ep() + sigma; }
            let mut q = vec![];
            if mp == -1 && c < short_stop { q.push(ord(K::Buy, Ty::Stop(bp), "Long.Rev1")); }
            if mp == 0 { q.push(ord(K::Buy, Ty::Stop(bp), "Long.Ent1")); }
            if mp == 1 && c > long_stop { q.push(ord(K::Short, Ty::Stop(sp), "Short.Rev1")); }
            if mp == 0 { q.push(ord(K::Short, Ty::Stop(sp), "Short.Ent1")); }
            if mp == 1 && c < long_stop { q.push(ord(K::XL, Ty::Mkt, "L.StopOut1")); }
            if mp == -1 && c > short_stop { q.push(ord(K::XS, Ty::Mkt, "S.StopOut1")); }
            mp_prev = mp;
            s.bar(x, i + 1, &q);
        }
        return s.done(x, to);
    }
    let sd20 = abb_sd(&x.c, g("numbars2", 20.) as usize);
    let sd70 = abb_sd(&x.c, g("numbarl2", 70.) as usize);
    let (ss2, pct2) = (g("sstop2", 2.), g("pct2", 2.4));
    let st = first_ok(n, from.max(1), &[&sd70, &crate::books::ajmisc::back(&sd20, 1)]);
    if st >= to { return vec![]; }
    let (mut hsf, mut lsf) = (x.c[st], x.c[st]);
    let (mut pos1, mut pos2) = (0i32, 0i32);
    let (mut bpp, mut spp) = (0f64, 0f64);
    for i in st..to {
        let (c, mp) = (x.c[i], s.pos);
        if c > hsf { hsf = c; }
        if c < lsf { lsf = c; }
        if pos1 > -1 && c < hsf - sd70[i] * pct2 { pos1 = -1; pos2 = 1; hsf = c; lsf = c; }
        if pos1 < 1 && c > lsf + sd70[i] * pct2 { pos1 = 1; pos2 = 1; hsf = c; lsf = c; }
        let (mut bp, mut sp) = (c + sd20[i] * 1.5, c - sd20[i] * 1.5);
        if sd20[i] * 1.5 * bpv < 300. { bp = c + 300. / bpv; sp = c - 300. / bpv; }
        let mut sigma = sd20[i - 1] * 1.5 * ss2;
        if sigma * bpv < 300. { sigma = 300. / bpv; }
        if mp == 1 && mp_prev != 1 { long_stop = s.ep() - sigma; }
        if mp == -1 && mp_prev != -1 { short_stop = s.ep() + sigma; }
        let mut q = vec![];
        if mp == 0 && pos1 == 1 && pos2 == 1 { q.push(ord(K::Buy, Ty::Mkt, "Long.Ent2")); }
        if mp == 0 && pos1 == 1 && c > bpp { q.push(ord(K::Buy, Ty::Mkt, "Long.Entr2")); }
        if mp == 1 && pos1 == -1 { q.push(ord(K::Short, Ty::Mkt, "Short.Rev2")); }
        if mp == 1 && pos1 == 1 && c < spp { q.push(ord(K::XL, Ty::Mkt, "Long.Ex2")); }
        if mp == 0 && pos1 == -1 && pos2 == 1 { q.push(ord(K::Short, Ty::Mkt, "Short.Ent2")); }
        if mp == 0 && pos1 == -1 && c < spp { q.push(ord(K::Short, Ty::Mkt, "Short.Entr2")); }
        if mp == -1 && pos1 == 1 { q.push(ord(K::Buy, Ty::Mkt, "Long.Rev2")); }
        if mp == -1 && pos1 == -1 && c > bpp { q.push(ord(K::XS, Ty::Mkt, "Short.Ex2")); }
        if mp == 1 && s.bse(i) > 1 && c < long_stop { q.push(ord(K::XL, Ty::Mkt, "L.StopOut2")); }
        if mp == -1 && s.bse(i) > 1 && c > short_stop { q.push(ord(K::XS, Ty::Mkt, "S.StopOut2")); }
        pos2 = 0;
        bpp = bp; spp = sp;
        mp_prev = mp;
        s.bar(x, i + 1, &q);
    }
    s.done(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    match id { ABP => Some(aberration_plus(x, from, to, p)), AZT => Some(aztec(x, from, to, p)), _ => None }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abb_sd_sample_and_flat_fallback() {
        let v = [1., 2., 3., 4., 4., 4., 4.];
        let s = abb_sd(&v, 3);
        assert!((s[2] - 1.).abs() < 1e-12); // sample stdev of 1,2,3
        assert_eq!(s[6], s[5]); // 4,4,4: sum of squares 0 -> previous value
    }
}

// Instrument admission: Lane A vendor1 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/vendor1/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="tscom" && crate::books::tscom::reviewed_lanea(def.id)) || (def.book=="alfar" && crate::books::alfar::reviewed_lanea(def.id)) || (def.book=="axsym" && crate::books::axsym::reviewed_lanea(def.id)) || (def.book=="barra" && crate::books::barra::reviewed_lanea(def.id)) || (def.book=="ltcoat" && crate::books::ltcoat::reviewed_lanea(def.id)) || (def.book=="pwadcs" && crate::books::pwadcs::reviewed_lanea(def.id)) || (def.book=="ltsfut" && crate::books::ltsfut::reviewed_lanea(def.id)) || (def.book=="qsfund" && crate::books::qsfund::reviewed_lanea(def.id)) || (def.book=="rvecl" && crate::books::rvecl::reviewed_lanea(def.id)),
    label: "Vendor TradeStation code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "TradeStation vendor system code (trade-system.com, alfaranda.com, i8i.com, Baramundi/Intervest, longtermtrading.com, pwafutures.com, LTS-Futures, Quadriga Superfund, Ralph Vince Eclipse) - research share 'TradeStation Systems & Indicators'",
};
