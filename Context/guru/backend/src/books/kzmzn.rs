//! "Metastock Zone" (a web collection of MetaStock list posts and Traders' Tips, in the K.A.L.O.S.Z archive folder: research share
//! USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/Metastock Zone.pdf). Book id `kzmzn`. Four
//! System Tester systems: Key Reversals (Scorpio / Equis), StochRSI (Baude, after Chande), StoRSI (Karnish) and Onno's Binary Wave;
//! lane K9 added seven more (Psychological Index, SectorBets' Coppock signals, Rejczak's WIG 20 system, the Stochastic Simianer
//! test, Ken's DeMark setup test, Estes' Momentum Time, Thamrin's 1-2-3 reversal; section "lane K9 systems" below).
//! Lane K8 (titles loop, 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv};
use super::rbtie::{mk, target_trades};
use super::msws::{simulate, sma, stoch, Exec, Sig};

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["kzmzn_key_rev", "kzmzn_stochrsi", "kzmzn_storsi", "kzmzn_binary_wave"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) || K9.contains(&id) }

macro_rules! common { () => { " Fills at the signal day's close (the MetaStock System Tester default, no delay). MetaStock built-ins as documented: RSI = Wilder's, Mo(n) = 100 x C / C n bars ago, Mov(..,E) = exponential average. A forum/vendor sample with no printed results; ETFs, US/India indices and stocks are a declared application." } }

const KEY: &str = concat!("[Declared readings] Massimiliano Scorpio's key reversal as coded by Equis (S&C Dec 2001 Traders' Tips): buy at the close of a day with a lower low, a higher close and a close below its high; close the long at the close of the first later day whose high is at or above the previous high; Maximum Loss stop 2% below the entry price, hit inside the day (fill at the stop, or the open on a gap). Exits are tested from the day after entry; no new entry on an exit day. Long only.", common!());
const SRSI: &str = concat!("[Declared readings] StochRSI(n, s) = 100 x Sum(RSI(n) - LLV(RSI(n), n), s) / Sum(HHV(RSI(n), n) - LLV(RSI(n), n), s) for (14,3), (70,15), (252,54). Buy at the close when StochRSI-14 crosses from below 20 to 20 or above, and either StochRSI-70 or StochRSI-252 is at least 70 (rule #1), or StochRSI-70 is at least 50 and its largest one-day rise in the last 21 days is more than 3 points (rule #2, as coded: HHV(ROC(..,1,Points),21) > 3); the English rules require the StochRSI-14 cross in both (the MSW code's operator grouping drops it from rule #2). Sell: the poster's example 'CLOSE > Ref(CLOSE,-3)' under 'Sell Rule: Whatever you decide upon', labelled the author's example exit, tested from the day after entry. Long only.", common!());
const STO: &str = concat!("[Declared readings] StoRSI = Mov((RSI(8) - LLV(RSI(8),8)) / (HHV(RSI(8),8) - LLV(RSI(8),8)), 3, E) x 100, the exponential average starting from the simple average of its first 3 values. Enter long on Cross(17, StoRSI) (falls below 17), enter short on Cross(StoRSI, 83); each entry reverses the other side (no close rules printed). Karnish's suggested 13-day average trend filter is advice, not coded; he says the 'skeleton system' will not trade distant history well.", common!());
const BIN: &str = concat!("[Declared readings] The binary wave from the list reply: +1 after RSI(14) < 30 or Mo(12) < 90, -1 after RSI(14) > 70 or Mo(12) > 110, otherwise it keeps its value (the nested If favours the buy when both fire); the position follows the wave (stop-and-reverse).", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "Metastock Zone systems", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "kzmzn", rules, notes, claim: None };
    let mut v = vec![
        d("kzmzn_key_rev", "Key Reversals (Scorpio): lower low, higher close, close below the high; out when the high reaches yesterday's, 2% max loss", "Long", &[
            "E|\"Enter Long: L < Ref(L,-1) AND C>Ref(C,-1) AND C < H\"", "X|\"Close Long: H >=Ref(H,-1)\"; \"STOPS Maximum Loss: LONG ONLY 2.0 Percent\""], KEY),
        d("kzmzn_stochrsi", "StochRSI system: StochRSI-14 crosses up through 20 with StochRSI-70/252 strength; example exit close above the close 3 days back", "Long", &[
            "F|\"StochRSI-14 (Sum(RSI(14)-LLV(RSI(14),14),3) / Sum(HHV(RSI(14),14) - LLV(RSI(14),14),3))*100\"; likewise 70/15 and 252/54",
            "E|\"BUY Rule #1: EITHER STOCHRSI-70 OR STOCHRSI-252 >= 70.0 AND STOCHRSI-14 CROSSES FROM BELOW 20.0 TO EQUAL OR ABOVE 20.0\"; Rule #2: STOCHRSI-70 >= 50 and has risen by at least 3 points ... during any one of the past 21 trading days",
            "X|\"Sell Rule: Whatever you decide upon. CLOSE > Ref(CLOSE,-3)\""], SRSI),
        d("kzmzn_storsi", "StoRSI (Karnish): long when StoRSI falls below 17, short when it rises above 83, always in", "Long/Short", &[
            "E|\"Enter Long: Cross(17,(Mov((RSI(8)-LLV(RSI(8),8)) / (HHV(RSI(8),8)- (LLV(RSI(8),8))),3,E)*100))\"",
            "X|\"Enter Short: Cross((Mov((RSI(8)-LLV(RSI(8),8)) / (HHV(RSI(8),8)- (LLV(RSI(8),8))),3,E)*100),83)\""], STO),
        d("kzmzn_binary_wave", "Onno's Binary Wave: long after RSI(14) < 30 or Mo(12) < 90, short after RSI(14) > 70 or Mo(12) > 110", "Long/Short", &[
            "E|\"ENTER LONG: RSI(14)<30 OR Mo(12)<90\"", "X|\"ENTER SHORT: RSI(14)>70 OR Mo(12)>110\"; \"ValueWhen(1,signal<>0,signal)\""], BIN),
    ];
    v.extend(k9_catalogue());
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kzmzn_key_rev" => Some(("kzmzn_key_rev", "Metastock Zone key reversals", "2% max loss")),
        "kzmzn_stochrsi" => Some(("kzmzn_stochrsi", "Metastock Zone StochRSI", "14/70/252")),
        "kzmzn_storsi" => Some(("kzmzn_storsi", "Metastock Zone StoRSI", "17/83")),
        "kzmzn_binary_wave" => Some(("kzmzn_binary_wave", "Metastock Zone binary wave", "RSI 14, Mo 12")),
        _ => k9_family_of(id),
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kzmzn_key_rev" => Some(vec![p("stop", "Maximum loss, % (2)", 2., 0.5, 10., 0.5)]),
        "kzmzn_storsi" => Some(vec![p("lo", "Buy below (17)", 17., 1., 50., 1.), p("hi", "Sell above (83)", 83., 50., 99., 1.)]),
        "kzmzn_stochrsi" | "kzmzn_binary_wave" => Some(vec![]),
        _ => k9_params(id),
    }
}

fn sum_n(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n && v[i + 1 - n..=i].iter().all(|x| x.is_finite()) { v[i + 1 - n..=i].iter().sum() } else { f64::NAN }).collect()
}

fn stoch_rsi(c: &[f64], n: usize, s: usize) -> Vec<f64> {
    let r = rsi(c, n);
    let (hh, ll) = (hhv(&r, n), llv(&r, n));
    let num: Vec<f64> = (0..c.len()).map(|i| r[i] - ll[i]).collect();
    let den: Vec<f64> = (0..c.len()).map(|i| hh[i] - ll[i]).collect();
    let (sn, sd) = (sum_n(&num, s), sum_n(&den, s));
    sn.iter().zip(&sd).map(|(a, b)| if a.is_finite() && b.is_finite() && *b != 0. { 100. * a / b } else { f64::NAN }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if K9.contains(&id) { return k9_run(id, x, from, to, p); }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c, n_) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let mut w = vec![0i8; n_];
    match id {
        "kzmzn_storsi" => {
            let r = rsi(c, 8);
            let (hh, ll) = (hhv(&r, 8), llv(&r, 8));
            let raw: Vec<f64> = (0..n_).map(|i| if hh[i].is_finite() && hh[i] != ll[i] { (r[i] - ll[i]) / (hh[i] - ll[i]) } else { f64::NAN }).collect();
            let k: Vec<f64> = ema(&raw, 3).iter().map(|v| v * 100.).collect();
            let (lo, hi) = (g("lo"), g("hi"));
            for i in 1..n_ { w[i] = if k[i] < lo && k[i - 1] >= lo { 1 } else if k[i] > hi && k[i - 1] <= hi { -1 } else { w[i - 1] }; }
            return Some(target_trades(x, &w, from, to, false, "StoRSI crossed"));
        }
        "kzmzn_binary_wave" => {
            let r = rsi(c, 14);
            for i in 1..n_ {
                let mo = if i >= 12 { 100. * c[i] / c[i - 12] } else { f64::NAN };
                w[i] = if r[i] < 30. || mo < 90. { 1 } else if r[i] > 70. || mo > 110. { -1 } else { w[i - 1] };
            }
            return Some(target_trades(x, &w, from, to, false, "binary wave flipped"));
        }
        _ => {}
    }
    let (s14, s70, s252) = if id == "kzmzn_stochrsi" { (stoch_rsi(c, 14, 3), stoch_rsi(c, 70, 15), stoch_rsi(c, 252, 54)) } else { (vec![], vec![], vec![]) };
    let mx = if id == "kzmzn_stochrsi" { let rise: Vec<f64> = (0..n_).map(|i| if i > 0 { s70[i] - s70[i - 1] } else { f64::NAN }).collect(); hhv(&rise, 21) } else { vec![] };
    let (mut out, mut pos) = (vec![], None::<(usize, f64)>);
    for i in from..=to {
        if let Some((e, ep)) = pos {
            if i > e {
                if id == "kzmzn_key_rev" {
                    let st = ep * (1. - g("stop") / 100.);
                    if l[i] <= st { out.push(mk(1., e, ep, "close", i, o[i].min(st), "maximum loss", false, i)); pos = None; continue; }
                    if h[i] >= h[i - 1] { out.push(mk(1., e, ep, "close", i, c[i], "high at or above yesterday's", false, i)); pos = None; continue; }
                } else if i >= 3 && c[i] > c[i - 3] { out.push(mk(1., e, ep, "close", i, c[i], "close above the close 3 days back", false, i)); pos = None; continue; }
            }
        }
        if pos.is_none() && i < to && i >= 1 {
            let entry = if id == "kzmzn_key_rev" { l[i] < l[i - 1] && c[i] > c[i - 1] && c[i] < h[i] }
                else { s14[i] >= 20. && s14[i - 1] < 20. && (s70[i] >= 70. || s252[i] >= 70. || (s70[i] >= 50. && mx[i] > 3.)) };
            if entry { pos = Some((i, c[i])); }
        }
    }
    if let Some((e, ep)) = pos { out.push(mk(1., e, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

// ===================================================================== lane K9 systems (1 Nov 2026)
// To-code rows 147, 150, 153-155, 157-158 (rule text authors/batch-c/C3w/mszone-tocode.txt); independent check
// authors/titles-1030/K9w/k9_ref.py (header). They run through the MetaStock System Tester simulator of module msws.
pub const K9: &[&str] = &["kzmzn_psych", "kzmzn_coppock", "kzmzn_wig20", "kzmzn_simianer", "kzmzn_seq_setup", "kzmzn_momentum_time", "kzmzn_123_reversal"];

macro_rules! common9 { () => { " Fills at the signal day's close (the MetaStock System Tester default, trade delay 0). One position at a time; a position closes only by its close rule, checked from the day after a close-filled entry; a close and an opposite entry on the same day reverse the position. MetaStock built-ins as documented: Mov(..,E) = exponential average (alpha 2/(n+1), started from the simple average of its first n values), Mov(..,S) = simple average, Stoch(k,s) = 100 x Sum(C - LLV(L,k), s) / Sum(HHV(H,k) - LLV(L,k), s), Cross(a,b) = a above b today and at or below it yesterday. A mailing-list sample with no printed test unless noted; indices, ETFs and US/India stocks are a declared application." } }

const K9_PSY: &str = concat!("[Declared readings] Glen Wallace's code for the Futures (June 2000, p.48) Psychological Index: PsychIndex = 100 x (number of up closes in the last LookBack days) / LookBack. Enter long (and close a short) when it is at or below the lower threshold, enter short (and close a long) when it is at or above the upper one: always in after the first signal. LookBack 12, upper 75, lower 25 are 'the standard' settings the article describes (his Input() defaults); the System Tester version uses opt1-opt3, whose printed ranges (1-15) cannot hold 75/25, so the indicator defaults are used. The poster warns it 'will give poor results in a strongly trending market'.", common9!());
const K9_COP: &str = concat!("[Declared readings] SectorBets' Coppock curve CCT = (10 x (ROC14 + ROC11) of today + 9 x (ROC14 + ROC11) of yesterday + ... + 1 x those 9 days back) / 2, ROC in percent. Enter long (close short) when CCT rises and either today's close is above yesterday's open after a black candle yesterday (close below open) or today's close is above yesterday's close after a white candle (close above open); enter short (close long) on the printed mirror with a falling CCT. The printed Enter Short / Close Long and Enter Long / Close Short pairs are the same conditions, so the system is stop and reverse. Black() = close below open, White() = close above open (MetaStock built-ins). Daily bars (the author: 'monthly, weekly, daily, hourly').", common9!());
const K9_WIG: &str = concat!("[Declared readings] Pawel Rejczak's system for WIG 20 futures, as printed: enter long when the close is more than 1% above yesterday's and above its 5-day exponential average; close the long below the 13-day exponential average; shorts are the mirror (close more than 1% below yesterday's and below the 5-day average; cover above the 13-day). Trades at the close, as the author says ('zawieranie transakcji na close'). His test on FW20 continuation futures (16 Jan 1998 - 26 May 2000, 3+3 points commission) is in the gate's book field; the vault holds no WIG 20 futures, so it is not compared.", common9!());
const K9_SIM: &str = concat!("[Declared readings] The 'Stochastic Simianer System Test' from an Equis list reply: Enter long (close short) when Stoch(5,3) crossed above 20 on this or any of the previous 9 days (Alert(..,10)) and today Stoch(5,3) crosses above its 3-day exponential average; enter short (close long) when it crossed below 80 within the last 10 days and today crosses below its average. Printed close and entry pairs are identical, so it is stop and reverse. Simianer's own three-stochastic exploration on the same page is a buy filter only and is not coded.", common9!());
const K9_SEQ: &str = concat!("[Declared readings] Ken's 'SeqSETUP, V day System Test' (DeMark setup): a buy setup is 9 closes in a row each below the close 4 days earlier with the validation day (the close 9 days back above the close 13 days back); a sell setup is the mirror. The page lists the two formulas under 'System Test': here a buy setup enters long (closing a short) and a sell setup enters short (closing a long), stop and reverse. Validation day on (as printed; 'If you don't want validation day, don't use it': off is an option). The code uses a strict '>' on the validation day although Ken's text says '>='; the code is followed. Not the TD Sequential countdown cards.", common9!());
const K9_MOM: &str = concat!("[Declared readings] Richard Estes' Momentum Time = (H - T55 + (L - T55)) / (H - T55 - (L - T55)), T55 = Mov(C,55,T); buy when it crosses above 0 with the close above Mov(C,21,T); sell (close the long) when it crosses below 0. Long only. Mov(..,T) = MetaStock's triangular average: a simple average of a simple average, each over (n+1)/2 days rounded up (28 for 55, 11 for 21), as MetaStock documents it; a day with high = low has no value. Jim Greening's S/C version on the same page is not coded (its exit level is the optimiser variable OPT1, -55 to -13).", common9!());
const K9_REV: &str = concat!("[Declared readings] Yuwono Thamrin's corrected '1-2-3 Reversal System': enter long after two higher closes in a row with Stoch(9,3) rising and its 3-day simple average below 50; enter short after two lower closes with Stoch(9,3) falling and its average above 50. No close rules are printed, so each entry reverses the other side. The poster says 'I don't trade and learn this system': a list example.", common9!());

fn k9_catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "Metastock Zone systems", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "kzmzn", rules, notes, claim: None };
    vec![
        d("kzmzn_psych", "Psychological Index: long at 25% up days or fewer in 12, short at 75% or more, always in", "Long/Short", &[
            "F|\"UpDay:= If(CLOSE > Ref(CLOSE,-1), 1, 0); PsychIndex:= Sum(UpDay,LookBack) / LookBack * 100;\"; \"the standard lookback period as 12 bars with a level of 75% or higher being an indication of an overbought market and 25% or lower being an oversold market\"",
            "E|\"Enter Long / Close Short ... PsychIndex <= LThreshold\"", "X|\"Enter Short / Close Long ... PsychIndex >= UThreshold\""], K9_PSY),
        d("kzmzn_coppock", "Coppock curve signals (SectorBets): rising CCT with a candle reversal, stop and reverse", "Long/Short", &[
            "F|\"Coppock Curve - CCT: (ROC( CLOSE,14 ,percent )*10 + ROC(CLOSE,11,percent)*10 + ROC(Ref(CLOSE,-1),14,percent)*9+ROC(Ref(CLOSE,-1),11,percent)*9 + ... + ROC(Ref(CLOSE,-9),14,percent)+ROC(Ref(CLOSE,-9),11,percent))/2\"",
            "E|\"Enter Long: Fml(\"Coppock Curve - CCT\") > Ref(Fml(\"Coppock Curve - CCT\"), -1) AND ((Close > Ref(Open,-1) AND Ref(Black(),-1)) OR (Close > Ref(Close,-1) AND Ref(White(),-1)))\"",
            "X|\"Close Long: Fml(\"Coppock Curve - CCT\") < Ref(Fml(\"Coppock Curve - CCT\"),-1) AND ((Close < Ref(Open,-1) AND Ref(White(),-1)) OR (Close < Ref(Close,-1) AND Ref(Black(),-1)))\"; Enter Short / Close Short are the printed mirrors"], K9_COP),
        d("kzmzn_wig20", "Simple Fut WIG 20 system (Rejczak): 1% up close above the 5-day EMA, out under the 13-day EMA; shorts mirrored", "Long/Short", &[
            "E|\"Enter Long: C>1.01*Ref(C,-1) AND C>Mov(C,5,E)\"", "X|\"Close Long: C<Mov(C,13,E)\"",
            "E|\"Enter Short: C<0.99*Ref(C,-1) AND C<Mov(C,5,E)\"", "X|\"Close Short: C>Mov(C,13,E)\""], K9_WIG),
        d("kzmzn_simianer", "Stochastic Simianer system test: Stoch(5,3) crosses its 3-day EMA within 10 days of a 20/80 cross, stop and reverse", "Long/Short", &[
            "E|\"Enter Long: Alert(Cross(Stoch(5,3),20),10) AND Cross(Stoch(5,3),Mov(Stoch(5,3),3,E))\"",
            "X|\"Close Long: Alert(Cross(80,Stoch(5,3)),10) AND Cross(Mov(Stoch(5,3),3,E),Stoch(5,3))\"; Enter Short = the Close Long rule, Close Short = the Enter Long rule (as printed)"], K9_SIM),
        d("kzmzn_seq_setup", "DeMark 9-day setup test (Ken): long on a buy setup with validation day, short on a sell setup", "Long/Short", &[
            "E|\"{BUY SETUP} C < Ref(C,-4) AND Ref(C,-1) < Ref(C,-5) AND ... AND Ref(C,-8) < Ref(C,-12) AND {Validation Day} Ref(C,-9) > Ref(C,-13)\"",
            "X|\"{SELL SETUP} C > Ref(C,-4) AND Ref(C,-1) > Ref(C,-5) AND ... AND Ref(C,-8) > Ref(C,-12) AND {Validation Day} Ref(C,-9) < Ref(C,-13)\"; \"*System Test* (If you don't want validation day, don't use it.)\""], K9_SEQ),
        d("kzmzn_momentum_time", "Momentum Time (Estes): buy when it crosses above 0 over the 21-day triangular average, sell when it crosses below 0", "Long", &[
            "F|\"Momentum Time (HIGH - Mov(C,55,T)+(LOW - Mov(C,55,T)))/(HIGH - Mov(C,55,T)-(LOW - Mov(C,55,T)))\"",
            "E|\"Buy: Cross(Fml( \"MOMENTUM time\" ),0) and c>mov(c,21,t)\"", "X|\"Sell: Cross(0, Fml( \"MOMENTUM time\" ))\""], K9_MOM),
        d("kzmzn_123_reversal", "1-2-3 Reversal (Thamrin): two higher closes with a rising Stoch(9,3) under 50 buys, the mirror sells, always in", "Long/Short", &[
            "E|\"Enter Long: (Ref(C,-2) < Ref(C,-1)) AND (Ref(C,-1) < C) AND (Ref(Stoch(9,3),-1) < Stoch(9,3)) AND (Mov(Stoch(9,3),3,S) < 50)\"",
            "X|\"Enter Short: (Ref(C,-2) > Ref(C,-1)) AND (Ref(C,-1) > C) AND (Ref(Stoch(9,3),-1) > Stoch(9,3)) AND (Mov(Stoch(9,3),3,S) > 50)\"; no close rules printed"], K9_REV),
    ]
}

fn k9_family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "kzmzn_psych" => ("kzmzn_psych", "Metastock Zone Psychological Index", "12 days, 25/75"),
        "kzmzn_coppock" => ("kzmzn_coppock", "Metastock Zone Coppock signals", "CCT 14/11, candle reversal"),
        "kzmzn_wig20" => ("kzmzn_wig20", "Metastock Zone WIG 20 system", "1%, EMA 5/13"),
        "kzmzn_simianer" => ("kzmzn_simianer", "Metastock Zone Stochastic Simianer test", "Stoch 5/3, alert 10"),
        "kzmzn_seq_setup" => ("kzmzn_seq_setup", "Metastock Zone DeMark setup test", "9-day setup, validation day"),
        "kzmzn_momentum_time" => ("kzmzn_momentum_time", "Metastock Zone Momentum Time", "T55, T21"),
        "kzmzn_123_reversal" => ("kzmzn_123_reversal", "Metastock Zone 1-2-3 reversal", "Stoch 9/3, 50"),
        _ => return None,
    })
}

fn k9_params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    K9.contains(&id).then(|| match id {
        "kzmzn_psych" => vec![p("n", "Lookback days (standard 12)", 12., 2., 100., 1.), p("hi", "Upper threshold % (standard 75)", 75., 50., 100., 1.),
            p("lo", "Lower threshold % (standard 25)", 25., 0., 50., 1.)],
        "kzmzn_simianer" => vec![p("alert", "Alert window days (printed 10)", 10., 1., 30., 1.)],
        "kzmzn_seq_setup" => vec![p("vday", "Validation day: 1 = on (printed), 0 = off", 1., 0., 1., 1.)],
        _ => vec![],
    })
}

fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i >= 1 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
/// MetaStock Mov(v,n,T): simple average of a simple average, each over ceil((n+1)/2) days.
fn tma(v: &[f64], n: usize) -> Vec<f64> { let k = (n + 2) / 2; sma(&sma(v, k), k) }
/// Long/short system whose close rules are the opposite entries (stop and reverse).
fn rev(el: Vec<bool>, es: Vec<bool>) -> Sig { Sig { xl: es.clone(), xs: el.clone(), el, es } }

fn signals(id: &str, x: &Ctx, g: &dyn Fn(&str, f64) -> f64) -> Option<Sig> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let all = |f: &dyn Fn(usize) -> bool| -> Vec<bool> { (0..n).map(f).collect() };
    Some(match id {
        "kzmzn_psych" => {
            let lb = g("n", 12.).round().max(1.) as usize;
            let up: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else if c[i] > c[i - 1] { 1. } else { 0. }).collect();
            let pi: Vec<f64> = sma(&up, lb).iter().map(|v| v * 100.).collect();
            let (hi, lo) = (g("hi", 75.), g("lo", 25.));
            rev(all(&|i| pi[i] <= lo), all(&|i| pi[i] >= hi))
        }
        "kzmzn_coppock" => {
            let roc = |k: usize| -> Vec<f64> { (0..n).map(|i| if i >= k { (c[i] - c[i - k]) / c[i - k] * 100. } else { f64::NAN }).collect() };
            let (r14, r11) = (roc(14), roc(11));
            let cct: Vec<f64> = (0..n).map(|i| {
                if i < 9 { return f64::NAN; }
                let mut s = 0.;
                for k in 0..10 { let w = (10 - k) as f64; s += r14[i - k] * w; s += r11[i - k] * w; }
                s / 2.
            }).collect();
            let (black, white) = (|i: usize| c[i] < o[i], |i: usize| c[i] > o[i]);
            let up = all(&|i| i >= 1 && cct[i] > cct[i - 1] && ((c[i] > o[i - 1] && black(i - 1)) || (c[i] > c[i - 1] && white(i - 1))));
            let dn = all(&|i| i >= 1 && cct[i] < cct[i - 1] && ((c[i] < o[i - 1] && white(i - 1)) || (c[i] < c[i - 1] && black(i - 1))));
            rev(up, dn)
        }
        "kzmzn_wig20" => {
            let (e5, e13) = (ema(c, 5), ema(c, 13));
            Sig { el: all(&|i| i >= 1 && c[i] > 1.01 * c[i - 1] && c[i] > e5[i]), xl: all(&|i| c[i] < e13[i]),
                  es: all(&|i| i >= 1 && c[i] < 0.99 * c[i - 1] && c[i] < e5[i]), xs: all(&|i| c[i] > e13[i]) }
        }
        "kzmzn_simianer" => {
            let w = g("alert", 10.).round().max(1.) as usize;
            let s = stoch(x, 5, 3);
            let d = ema(&s, 3);
            let (k20, k80) = (vec![20.; n], vec![80.; n]);
            let (cu, cd) = (all(&|i| cross(&s, &k20, i)), all(&|i| cross(&k80, &s, i)));
            let alert = |v: &[bool], i: usize| v[(i + 1).saturating_sub(w)..=i].iter().any(|b| *b);
            rev(all(&|i| alert(&cu, i) && cross(&s, &d, i)), all(&|i| alert(&cd, i) && cross(&d, &s, i)))
        }
        "kzmzn_seq_setup" => {
            let vd = g("vday", 1.) >= 0.5;
            let buy = all(&|i| i >= 13 && (0..9).all(|k| c[i - k] < c[i - k - 4]) && (!vd || c[i - 9] > c[i - 13]));
            let sell = all(&|i| i >= 13 && (0..9).all(|k| c[i - k] > c[i - k - 4]) && (!vd || c[i - 9] < c[i - 13]));
            rev(buy, sell)
        }
        "kzmzn_momentum_time" => {
            let (t55, t21) = (tma(c, 55), tma(c, 21));
            let mt: Vec<f64> = (0..n).map(|i| { let (a, b) = (h[i] - t55[i], l[i] - t55[i]); if a - b == 0. { f64::NAN } else { (a + b) / (a - b) } }).collect();
            let z = vec![0.; n];
            Sig { el: all(&|i| cross(&mt, &z, i) && c[i] > t21[i]), xl: all(&|i| cross(&z, &mt, i)), es: vec![false; n], xs: vec![false; n] }
        }
        "kzmzn_123_reversal" => {
            let s = stoch(x, 9, 3);
            let m = sma(&s, 3);
            rev(all(&|i| i >= 2 && c[i - 2] < c[i - 1] && c[i - 1] < c[i] && s[i - 1] < s[i] && m[i] < 50.),
                all(&|i| i >= 2 && c[i - 2] > c[i - 1] && c[i - 1] > c[i] && s[i - 1] > s[i] && m[i] > 50.))
        }
        _ => return None,
    })
}

fn k9_run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let sig = signals(id, x, &g)?;
    Some(simulate(x, from, to, &sig, Exec::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn key_reversal_entry_and_exit() {
        // bar 1: lower low, higher close, close below high -> buy 10.5; bar 2: high 10.8 < 11 -> hold; bar 3: high 11.2 >= 10.8 -> out at 11
        let b = [(10., 11., 9.5, 10.), (10., 10.9, 9., 10.5), (10.5, 10.8, 10.35, 10.6), (10.6, 11.2, 10.4, 11.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("kzmzn_key_rev", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(1, 10.5, 3, 11.)]);
    }
    #[test]
    fn every_system_trades_and_reverses_as_printed() {
        let r: Vec<Row> = (0..700).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 13.).sin() + 4. * (f / 3.1).cos() + 0.03 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.6 * (f / 2.3).sin(), high: c + 1.2, low: c - 1.1, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in K9 {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 3, "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            if *id == "kzmzn_momentum_time" { assert!(t.iter().all(|z| z.dir > 0.)); }
            assert!(t.iter().all(|z| z.legs[0].px == x.c[z.legs[0].i]), "{id}");
        }
        // triangular average of 55 = SMA28 of SMA28
        let v: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert!((tma(&v, 55)[54] - 27.).abs() < 1e-9 && tma(&v, 55)[53].is_nan());
        assert!(run("kzmzn_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Metastock Zone systems (K.A.L.O.S.Z archive folder) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kzmzn" && crate::books::kzmzn::reviewed(def.id),
    label: "MetaStock list and Traders' Tips systems with no printed results; ETFs, US/India indices and stocks are a declared application.",
    basis: "Metastock Zone.pdf (K.A.L.O.S.Z archive folder) PDF p.665-666, 772-774, 1101-1105; independent check authors/titles-1030/K8w/k8_ref.py. Lane K9 systems: pages e-st-Psychological_Index, Coppock_Curve-Signal_Formulas, Simple_Fut_WIG_20_System, Stochastic_Simianer_System_Test, SeqSETUP_SeqINTERSECTION, 08_Momentum_Time_S_C, 1-2-3_Reversal_System; independent check authors/titles-1030/K9w/k9_ref.py",
};
