//! Samuel K. Tennis (VISTA Research and Trading, 1998), skt.Binary Wave system, recovered from
//! `T:/Samuel K.Tennis - Binary Wave Indicators & System (vista.com)/Code.rar` -> `Binary.Wave.Strategy.txt`,
//! `Binary.Test.Function.txt`, `X.Day.Exit.Strategy.txt` (research drive, read in place).
//! Book id "sktbw". Helpers from jcfeed. Notes: library/sktbw/README.md.
use super::jcfeed::{gp, Sim};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sktbw";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["sktbw_binary_wave"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const BW: &str = "sktbw_binary_wave";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: BW, name: "Binary Wave (5+15-bar close-vs-midpoint wave, turn entries, 3-bar exit)", group: "skt.Binary Wave (Code.rar)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|skt.Binary.Test: \"if Close > (High + Low) * 0.50 then Return = +1; if Close < (High + Low) * 0.50 then Return = -1\" (else 0)",
            "F|\"Fast.Wav = Summation(User.Fnc, Fast.Len)\" (5, once BarNumber > 5); \"Slow.Wav = Summation(User.Fnc, Slow.Len)\" (15); bWav.Sum = Fast.Wav + Slow.Wav",
            "F|\"if bWav.Sum crosses above 0 or crosses below 0 then Max.Hi = -999999; Max.Lo = +999999; Trend = 0\"; Max.Hi / Max.Lo = highest positive / lowest negative bWav.Sum since",
            "E|\"if Trend = 0 and Max.Hi[0] > se.Waves and Max.Hi[3] = Max.Hi[4] and Mro(bWav.Sum > bWav.Sum[1], 4, 1) = +3 then Trend = -1; Sell Short This Bar on the close\" (se.Waves -7)",
            "E|\"if Trend = 0 and Max.Lo < le.Waves and Max.Lo[3] = Max.Lo[4] and Mro(bWav.Sum < bWav.Sum[1], 4, 1) = +3 then Trend = +1; Buy This Bar on the close\" (le.Waves +7)",
            "X|\"if MarketPosition = +1 and Max.Lo < Max.Lo[1] then Sell This Bar on the close\"; \"if MarketPosition = -1 and Max.Hi > Max.Hi[1] then Buy to Cover This Bar on the close\"",
            "X|IncludeSystem \"skt.X Day Exit\", ex.nBars (3): \"if MarketPosition = +1 and BarsSinceEntry >= Bar.Stop then Sell Next Bar at market\" (and cover shorts)",
        ],
        notes: "Strategy inputs are the defaults: Fast.Len 5, Slow.Len 15, le.Waves +7, se.Waves -7, ex.nBars 3. The commented study block uses Max.Hi > 7 and Max.Lo < -7; the strategy's signs make those two tests almost always true - kept literal (set p.se_waves=7, p.le_waves=-7 for the study's thresholds). \
TradeStation semantics used: 'crosses above 0' = above 0 now and below 0 on the most recent earlier bar where the sum was not 0; Mro(test,4,1)=3 = test false today, 1 and 2 bars ago and true 3 bars ago; BarNumber = bar count from the first bar of data; vars start at 0 (Max.Hi, Max.Lo, Trend). \
Order handling: the X-day exit (included first) is a next-open order; then the close entries (held direction ignored, opposite reverses), then the close exits for the position held before the close. \
Timeframe/market not stated (generic study by VISTA Research); run on daily bars [Assumption]. Not the MetaStock 'Binary Waves TS' of mstt (different inputs). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == BW).then_some((BW, "Binary Wave (S.K. Tennis)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == BW).then(|| vec![p("fast", "Fast.Len", 5., 2., 20., 1.), p("slow", "Slow.Len", 15., 5., 50., 1.), p("le_waves", "le.Waves (Max.Lo below)", 7., -20., 20., 1.),
        p("se_waves", "se.Waves (Max.Hi above)", -7., -20., 20., 1.), p("nbars", "ex.nBars (exit after N bars; 0 = off)", 3., 0., 20., 1.)])
}

fn run_bw(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let (fl, sl) = (gp(p, "fast", 5.).max(1.) as usize, gp(p, "slow", 15.).max(1.) as usize);
    let (lew, sew, nb) = (gp(p, "le_waves", 7.), gp(p, "se_waves", -7.), gp(p, "nbars", 3.).max(0.) as usize);
    let n = x.c.len();
    let f: Vec<f64> = (0..n).map(|i| { let m = (x.h[i] + x.l[i]) * 0.5; if x.c[i] > m { 1. } else if x.c[i] < m { -1. } else { 0. } }).collect();
    let (mut fw, mut sw) = (0., 0.);
    let mut b = vec![0f64; n];
    let (mut mh, mut ml) = (vec![0f64; n], vec![0f64; n]);
    let (mut hi, mut lo, mut trend) = (0f64, 0f64, 0i8);
    let mut sim = Sim::new();
    let mut pend: Option<i8> = None; // next-open exit for this side
    // most recent earlier value of b that is not 0
    let prior_nz = |b: &[f64], i: usize| (0..i).rev().map(|k| b[k]).find(|&v| v != 0.);
    for i in 0..=to.min(n - 1) {
        if let Some(side) = pend.take() { sim.exit(side, i, x.o[i], i.saturating_sub(1), &format!("lx/sx.nDays ({nb} bars) next open")); }
        if i + 1 > fl { fw = f[i + 1 - fl..=i].iter().sum(); }
        if i + 1 > sl { sw = f[i + 1 - sl..=i].iter().sum(); }
        b[i] = fw + sw;
        let pz = prior_nz(&b, i);
        if (b[i] > 0. && pz.is_some_and(|v| v < 0.)) || (b[i] < 0. && pz.is_some_and(|v| v > 0.)) { hi = -999999.; lo = 999999.; trend = 0; }
        if b[i] > 0. && b[i] > hi { hi = b[i]; }
        if b[i] < 0. && b[i] < lo { lo = b[i]; }
        mh[i] = hi; ml[i] = lo;
        let mp = sim.pos();
        let trade = i >= from;
        // X-day exit (IncludeSystem, evaluated first): next bar at market
        if trade && nb > 0 && mp != 0 && i - sim.entry_i() >= nb { pend = Some(mp); }
        if i >= 4 {
            let mro3 = |up: bool| (0..3).all(|k| !(if up { b[i - k] > b[i - k - 1] } else { b[i - k] < b[i - k - 1] }))
                && (if up { b[i - 3] > b[i - 4] } else { b[i - 3] < b[i - 4] });
            if trend == 0 && mh[i] > sew && mh[i - 3] == mh[i - 4] && mro3(true) {
                trend = -1;
                if trade { sim.enter(-1, i, x.c[i], i, "close", "se.bWave"); }
            }
            if trend == 0 && ml[i] < lew && ml[i - 3] == ml[i - 4] && mro3(false) {
                trend = 1;
                if trade { sim.enter(1, i, x.c[i], i, "close", "le.bWave"); }
            }
        }
        if trade && i >= 1 {
            if mp == 1 && ml[i] < ml[i - 1] { sim.exit(1, i, x.c[i], i, "lx.MaxLo"); }
            if mp == -1 && mh[i] > mh[i - 1] { sim.exit(-1, i, x.c[i], i, "sx.MaxHi"); }
        }
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != BW || from >= to || to >= x.r.len() { return None; }
    Some(run_bw(x, from, to, p))
}
