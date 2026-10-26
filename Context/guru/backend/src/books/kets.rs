//! Jeffrey Owen Katz & Donna L. McCormick, "The Encyclopedia of Trading Strategies" (McGraw-Hill, 2000). Book id `kets`.
//! Source: research drive "Jeffrey Owen Katz - The Encyclopedia Trading Strategies/...pdf" (386 pp.) and the authors'
//! companion C-Trader Toolkit CD ("Jeffrey Owen Katz - C-Trader Toolkit Cd (44 MB)/SCICON": TRMODS/CHAPxx.ZIP model
//! code, TSIM202/TRDSIM2.CPP simulator, TRAPS/TRAPSIND.C indicators). The CD code is the exact statement of every rule;
//! the book prints the same code. Transcription, page refs, oracle and verification: library/kets/.
//!
//! Every entry test uses the book's Standard Exit Strategy (SES, pp.76-77, 87): a close-only money-management stop
//! (entry -/+ 1 x ATR50), a close-only profit target (entry +/- 4 x ATR50) and an exit at the close once the trade has
//! been held 10 bars (`cb+1-entrybar >= maxhold`, TRDSIM2::update). ATR50 = simple mean of the last 50 true ranges,
//! taken on the bar BEFORE the bar being tested (the order is posted at the close of the signal bar). Fills follow
//! TRDSIM2 exactly: market orders at the open; stop/limit entries and exits gap-fill at the open when the open is
//! already through the price, otherwise fill at the price when the bar trades strictly beyond it; standard-exit
//! orders act only at the close. Orders are not placed on a bar whose NEXT bar has high == low (the book's
//! "limit-locked" filter). The book tested a 36-futures portfolio (1985-1999) with dollar-volatility sizing; here one
//! unit of the selected index (instrument swap only). Prices are not rounded to futures ticks.
use crate::connors::{Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const BOOK: &str = "kets";
/// Admitted ids (authors/katz/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["kets_cb_open", "kets_cb_limit", "kets_hhll_open", "kets_hhll_limit", "kets_hhll_stop", "kets_vb_open", "kets_vb_limit", "kets_vb_limit_long", "kets_vb_limit_adx", "kets_stoch_obos_open", "kets_stoch_obos_limit", "kets_stoch_obos_stop", "kets_rsi_obos_open", "kets_rsi_obos_limit", "kets_rsi_obos_stop", "kets_stoch_sig_open", "kets_stoch_sig_limit", "kets_stoch_sig_stop", "kets_stoch_div_open", "kets_stoch_div_limit", "kets_stoch_div_stop", "kets_cycle_open", "kets_cycle_limit", "kets_cycle_stop", "kets_rand_ses_open", "kets_rand_ses_limit", "kets_rand_ses_stop", "kets_rand_mses", "kets_rand_fixed_opt", "kets_rand_hhll_stop", "kets_rand_atr_stop", "kets_rand_mema_stop", "kets_rand_mema_shrink", "kets_rand_mema_shrink30", "kets_seas_xo_open", "kets_seas_xo_limit", "kets_seas_xo_stop", "kets_seas_mom_open", "kets_seas_mom_limit", "kets_seas_mom_stop", "kets_seas_conf_open", "kets_seas_conf_limit", "kets_seas_conf_stop", "kets_seas_inv_open", "kets_seas_inv_limit", "kets_seas_inv_stop", "kets_lunar_xo_open", "kets_lunar_xo_limit", "kets_lunar_xo_stop", "kets_lunar_mom_open", "kets_lunar_mom_limit", "kets_lunar_mom_stop", "kets_lunar_conf_open", "kets_lunar_conf_limit", "kets_lunar_conf_stop", "kets_lunar_inv_open", "kets_lunar_inv_limit", "kets_lunar_inv_stop"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// No orders before this bar: the book's price files began ~6 months (about 120 bars) before its first trade
/// (TRAPS.H START_DATE 850202 vs IS_DATE 850801), so every look-back was complete. Longer look-backs extend it.
const WARM: usize = 120;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Od { Open, Limit, Stop }

#[derive(Clone, Copy, PartialEq, Debug)]
enum M {
    Chan { hl: bool },      // ch5 T1/T3 (close vs highest close) and T4/T5 (close vs highest high)
    HhllStop,               // ch5 T6
    Vb { long_only: bool, adx: bool }, // ch5 T7, T8, T10, T12
    StochObos, RsiObos, StochSig, StochDiv, // ch7
    Seas(u8), Lunar(u8),    // ch8 / ch9: 1 momentum, 2 crossover, 3 + confirmation, 4 + confirmation & inversion
    Cycle,                  // ch10
    RandSes,                // ch13 T1-T3: random entries + SES
    RandMses(u8),           // ch13 T4 / ch14: random entries at open + intraday exits (0 fixed, 1 2-bar HHLL, 2 ATR, 3 MEMA, 4 MEMA + shrinking target)
}

type PDef = (&'static str, &'static str, f64, f64, f64, f64);

struct T {
    id: &'static str, name: &'static str, group: &'static str, m: M, ord: Od, dir: &'static str,
    pages: &'static str, defs: &'static [PDef],
    claim: Option<(&'static str, &'static str, &'static str, &'static str, &'static str)>,
}

const SES: [PDef; 3] = [
    ("pt", "SES profit target, ATR(50) units (close-only)", 4., 1., 8., 0.5),
    ("sl", "SES money-management stop, ATR(50) units (close-only)", 1., 0.5, 4., 0.5),
    ("hold", "SES maximum holding period, bars", 10., 2., 30., 1.),
];

macro_rules! t {
    ($id:expr, $name:expr, $g:expr, $m:expr, $o:ident, $dir:expr, $pg:expr, [$($d:expr),*], $c:expr) => {
        T { id: $id, name: $name, group: $g, m: $m, ord: Od::$o, dir: $dir, pages: $pg,
            defs: &[$($d,)* SES[0], SES[1], SES[2]], claim: $c }
    };
}
macro_rules! tr { // random-entry exit tests carry their own exit parameters
    ($id:expr, $name:expr, $g:expr, $m:expr, $o:ident, $pg:expr, [$($d:expr),*], $c:expr) => {
        T { id: $id, name: $name, group: $g, m: $m, ord: Od::$o, dir: "Long/Short", pages: $pg, defs: &[$($d,)*], claim: $c }
    };
}
const G5: &str = "Ch.5 Breakout models";
const G7: &str = "Ch.7 Oscillator entries";
const G8: &str = "Ch.8 Seasonality";
const G9: &str = "Ch.9 Lunar rhythms";
const G10: &str = "Ch.10 Cycle entries";
const G13: &str = "Ch.13-14 Exits (random entries)";
const SEED: PDef = ("seed", "Random-sequence selector ranseed (book stepped 1-10)", 1., 1., 10., 1.);
const MKT: PDef = ("mkt", "Market index in the seed, iseed = -(ranseed + 10 x market); book S&P 500 = 1", 1., 1., 36., 1.);

static TESTS: &[T] = &[
    // ---------------------------------------------------------------- Chapter 5
    t!("kets_cb_open", "Close-only channel breakout · entry next open (Tests 1-2)", G5, M::Chan { hl: false }, Open, "Long/Short", "pp.86-90 (PDF 102-106)",
        [("n", "Channel look-back (closes)", 80., 5., 100., 5.)], Some(("", "43% (T1, no costs)", "T2 with costs: IS $-80 / OOS $-871 per trade", "~7 bars", "36-futures portfolio IS 8/1985-12/1994; T1 (no costs) ~76% annual return vs max DD; T2 (costs) ROA IS -1.1% / OOS -13.5% (Table 5-3)"))),
    t!("kets_cb_limit", "Close-only channel breakout · limit at signal-bar midpoint (Test 3)", G5, M::Chan { hl: false }, Limit, "Long/Short", "pp.90-92 (PDF 106-108)",
        [("n", "Channel look-back (closes)", 80., 5., 100., 5.)], Some(("", "42%", "IS $1,066 / OOS $-299", "~7 bars", "36-futures portfolio; ROA IS 32.6% / OOS -10.0% (Table 5-3)"))),
    t!("kets_hhll_open", "HHLL breakout (close beyond highest high / lowest low) · next open (Test 4)", G5, M::Chan { hl: true }, Open, "Long/Short", "pp.92-94 (PDF 108-110)",
        [("n", "Look-back (highs/lows)", 85., 5., 100., 5.)], Some(("", "39%", "IS $82 / OOS $-912", "", "36-futures portfolio; ROA IS 1.2% / OOS -15.9% (Table 5-3)"))),
    t!("kets_hhll_limit", "HHLL breakout · limit at signal-bar midpoint (Test 5)", G5, M::Chan { hl: true }, Limit, "Long/Short", "pp.94-95 (PDF 110-111)",
        [("n", "Look-back (highs/lows)", 85., 5., 100., 5.)], Some(("", "IS 43% / OOS 41%", "IS $1,558 / OOS $-72 (table) ($-912 text)", "", "36-futures portfolio; ROA IS 36.3% / OOS -2.1% (Table 5-3)"))),
    t!("kets_hhll_stop", "HHLL breakout · stop at the channel edge nearer the close (Test 6)", G5, M::HhllStop, Stop, "Long/Short", "pp.95-96 (PDF 111-112)",
        [("n", "Look-back (highs/lows, incl. current bar)", 95., 5., 100., 5.)], Some(("", "IS 41% / OOS 37%", "IS $430 / OOS $-798", "", "36-futures portfolio; ROA IS 8.7% / OOS -15.5% (Table 5-3)"))),
    t!("kets_vb_open", "Volatility breakout (EMA +/- bw x ATR) · next open (Test 7)", G5, M::Vb { long_only: false, adx: false }, Open, "Long/Short", "pp.96-98 (PDF 112-114)",
        [("bw", "Band width, ATR units", 3.8, 1.5, 4.5, 0.1), ("ma", "EMA length (closes)", 5., 1., 25., 1.), ("atr", "ATR length for the band", 20., 5., 50., 1.)],
        Some(("IS 240 / OOS 112", "IS 45% / OOS 25%", "IS $4,675 / OOS $-7,371", "6 bars", "36-futures portfolio; ROA IS 27.4% / OOS -20.5% (Table 5-3)"))),
    t!("kets_vb_limit", "Volatility breakout · limit at signal-bar midpoint (Test 8)", G5, M::Vb { long_only: false, adx: false }, Limit, "Long/Short", "pp.98-99 (PDF 114-115)",
        [("bw", "Band width, ATR units", 3.7, 1.5, 4.5, 0.1), ("ma", "EMA length (closes)", 22., 1., 25., 1.), ("atr", "ATR length for the band", 41., 5., 50., 1.)],
        Some(("IS 1,244", "IS 45%", "IS $3,616 / OOS $-2,094", "7 days", "36-futures portfolio; ROA IS 48.3% / OOS -16.9% (Table 5-3)"))),
    t!("kets_vb_limit_long", "Volatility breakout · limit · long positions only (Test 10)", G5, M::Vb { long_only: true, adx: false }, Limit, "Long", "pp.100-101 (PDF 116-117)",
        [("bw", "Band width, ATR units", 2.6, 1.5, 4.5, 0.1), ("ma", "EMA length (closes)", 15., 1., 25., 1.), ("atr", "ATR length for the band", 18., 5., 50., 1.)],
        Some(("IS 1,263", "IS 48% / OOS 35%", "IS $4,100 / OOS $-1,640", "7 days", "36-futures portfolio; ROA IS 53.0% / OOS -14.6% (Table 5-3)"))),
    t!("kets_vb_limit_adx", "Volatility breakout · limit · 18-bar ADX new 6-bar high filter (Test 12)", G5, M::Vb { long_only: false, adx: true }, Limit, "Long/Short", "pp.102-103 (PDF 118-119)",
        [("bw", "Band width, ATR units", 2.6, 1.5, 4.5, 0.1), ("ma", "EMA length (closes)", 8., 1., 25., 1.), ("atr", "ATR length for the band", 34., 5., 50., 1.),
         ("adx", "ADX length", 18., 5., 40., 1.), ("adxhi", "ADX must exceed its highest of the previous N bars", 6., 2., 20., 1.)],
        Some(("IS 872 / OOS 373", "IS 47% / OOS 36%", "IS $4,570 / OOS $-2,415", "", "36-futures portfolio; ROA IS 68.3% / OOS -20.0% (Table 5-3)"))),
    // ---------------------------------------------------------------- Chapter 7
    t!("kets_stoch_obos_open", "Fast %K overbought/oversold (80/20) · next open (Test 1)", G7, M::StochObos, Open, "Long/Short", "pp.143-146 (PDF 156-159)",
        [("len", "Fast %K length", 25., 5., 25., 1.)], Some(("", "", "IS $-3,672 / OOS $-3,130", "", "36-futures portfolio; ROA IS -10.2% / OOS -23.7% (Table 7-3)"))),
    t!("kets_stoch_obos_limit", "Fast %K overbought/oversold (80/20) · limit (Test 2)", G7, M::StochObos, Limit, "Long/Short", "pp.143-146 (PDF 156-159)",
        [("len", "Fast %K length", 20., 5., 25., 1.)], Some(("", "IS 37% / OOS 36%", "IS $-2,228 / OOS $-2,504", "", "36-futures portfolio; ROA IS -10.1% / OOS -23.7% (Table 7-3)"))),
    t!("kets_stoch_obos_stop", "Fast %K overbought/oversold (80/20) · stop at close +/- 0.5 ATR (Test 3)", G7, M::StochObos, Stop, "Long/Short", "pp.143-146 (PDF 156-159)",
        [("len", "Fast %K length", 16., 5., 25., 1.)], Some(("", "", "IS $-2,586 / OOS $-2,650", "", "36-futures portfolio; ROA IS -10.1% / OOS -22.5% (Table 7-3)"))),
    t!("kets_rsi_obos_open", "RSI overbought/oversold (70/30) · next open (Test 4)", G7, M::RsiObos, Open, "Long/Short", "p.147 (PDF 160)",
        [("len", "RSI length", 25., 5., 25., 1.)], Some(("", "26-37% (range over T4-T6)", "IS $-7,073 / OOS $-3,537", "", "36-futures portfolio; ROA IS -10.1% / OOS -21.1% (Table 7-3)"))),
    t!("kets_rsi_obos_limit", "RSI overbought/oversold (70/30) · limit (Test 5)", G7, M::RsiObos, Limit, "Long/Short", "p.147 (PDF 160)",
        [("len", "RSI length", 25., 5., 25., 1.)], Some(("", "", "IS $-4,093 / OOS $-1,978", "", "36-futures portfolio; ROA IS -9.9% / OOS -18.3% (Table 7-3)"))),
    t!("kets_rsi_obos_stop", "RSI overbought/oversold (70/30) · stop (Test 6)", G7, M::RsiObos, Stop, "Long/Short", "p.147 (PDF 160)",
        [("len", "RSI length", 25., 5., 25., 1.)], Some(("", "", "IS $-6,878 / OOS $-3,824", "", "36-futures portfolio; ROA IS -9.9% / OOS -21.1% (Table 7-3)"))),
    t!("kets_stoch_sig_open", "Slow %K crosses its 3-bar signal line · next open (Test 7)", G7, M::StochSig, Open, "Long/Short", "p.147 (PDF 160)",
        [("len", "Slow %K length", 15., 5., 25., 1.)], Some(("", "", "IS $-2,656 / OOS $-2,324", "", "36-futures portfolio; ROA IS -10.3% / OOS -23.5% (Table 7-3)"))),
    t!("kets_stoch_sig_limit", "Slow %K signal-line cross · limit (Test 8)", G7, M::StochSig, Limit, "Long/Short", "p.147 (PDF 160)",
        [("len", "Slow %K length", 14., 5., 25., 1.)], Some(("", "", "IS $-1,813 / OOS $-1,330", "", "36-futures portfolio; ROA IS -10.3% / OOS -23.3% (Table 7-3)"))),
    t!("kets_stoch_sig_stop", "Slow %K signal-line cross · stop (Test 9)", G7, M::StochSig, Stop, "Long/Short", "p.147 (PDF 160)",
        [("len", "Slow %K length", 11., 5., 25., 1.)], Some(("", "", "IS $-2,026 / OOS $-1,968", "", "36-futures portfolio; ROA IS -10.2% / OOS -23.5% (Table 7-3)"))),
    t!("kets_stoch_div_open", "Fast %K / price divergence · next open (Test 13)", G7, M::StochDiv, Open, "Long/Short", "pp.149-150 (PDF 162-163)",
        [("len", "Fast %K length", 20., 5., 25., 1.), ("lb", "Divergence look-back", 15., 15., 25., 5.)], Some(("", "", "IS $-3,245 / OOS $-3,259", "", "36-futures portfolio; ROA IS -10.1% / OOS -21.7% (Table 7-3)"))),
    t!("kets_stoch_div_limit", "Fast %K / price divergence · limit (Test 14)", G7, M::StochDiv, Limit, "Long/Short", "pp.149-150 (PDF 162-163)",
        [("len", "Fast %K length", 24., 5., 25., 1.), ("lb", "Divergence look-back", 15., 15., 25., 5.)], Some(("", "", "IS $-2,443 / OOS $-3,182", "", "36-futures portfolio; ROA IS -10.1% / OOS -21.2% (Table 7-3)"))),
    t!("kets_stoch_div_stop", "Fast %K / price divergence · stop (Test 15)", G7, M::StochDiv, Stop, "Long/Short", "pp.149-150 (PDF 162-163)",
        [("len", "Fast %K length", 25., 5., 25., 1.), ("lb", "Divergence look-back", 15., 15., 25., 5.)], Some(("", "", "IS $-3,008 / OOS $-2,179", "", "36-futures portfolio; ROA IS -9.8% / OOS -19.9% (Table 7-3)"))),
    // ---------------------------------------------------------------- Chapter 8
    t!("kets_seas_xo_open", "Seasonal crossover · next open (Test 1)", G8, M::Seas(2), Open, "Long/Short", "pp.159-170 (PDF 172-183)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 5., 0., 20., 1.)], Some(("", "40-43% (all orders)", "IS $-1,127 / OOS $-300", "", "36-futures portfolio; ROA IS -9.7% / OOS -5.8% (Table 8-3)"))),
    t!("kets_seas_xo_limit", "Seasonal crossover · limit (Test 2)", G8, M::Seas(2), Limit, "Long/Short", "pp.159-170 (PDF 172-183)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 8., 0., 20., 1.)], Some(("", "40-43%", "IS $-424 / OOS $-56", "", "36-futures portfolio; ROA IS -4.1% / OOS -1.7% (Table 8-3)"))),
    t!("kets_seas_xo_stop", "Seasonal crossover · stop (Test 3)", G8, M::Seas(2), Stop, "Long/Short", "pp.159-170 (PDF 172-183)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 6., 0., 20., 1.)], Some(("", "40-43%", "IS $-179 / OOS $576", "", "36-futures portfolio; ROA IS -1.3% / OOS 8.3% (Table 8-3)"))),
    t!("kets_seas_mom_open", "Seasonal momentum threshold · next open (Test 4)", G8, M::Seas(1), Open, "Long/Short", "pp.170-171 (PDF 183-184)",
        [("avglen", "Centered MA length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 2., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 2.5, 1.5, 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (code/script); 7 centered triangular (labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $-1,069 / OOS $-952", "", "36-futures portfolio; ROA IS -9.4% / OOS -13.6% (Table 8-3)"))),
    t!("kets_seas_mom_limit", "Seasonal momentum threshold · limit (Test 5)", G8, M::Seas(1), Limit, "Long/Short", "pp.170-171 (PDF 183-184)",
        [("avglen", "Centered MA length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 1., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 2.5, 1.5, 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (code/script); 7 centered triangular (labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $-757 / OOS $-785", "", "36-futures portfolio; ROA IS -7.4% / OOS -13.7% (Table 8-3)"))),
    t!("kets_seas_mom_stop", "Seasonal momentum threshold · stop (Test 6)", G8, M::Seas(1), Stop, "Long/Short", "pp.170-171 (PDF 183-184)",
        [("avglen", "Centered MA length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 2., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 2.5, 1.5, 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (code/script); 7 centered triangular (labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $275 / OOS $-1,750", "", "36-futures portfolio; ROA IS 3.3% / OOS -18.4% (Table 8-3)"))),
    t!("kets_seas_conf_open", "Seasonal crossover + Fast %K confirmation · next open (Test 7)", G8, M::Seas(3), Open, "Long/Short", "pp.171-173 (PDF 184-186)",
        [("avglen", "Moving-average length", 15., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 7., 0., 20., 1.)], Some(("", "", "IS $-1,195 / OOS $-1,512", "", "36-futures portfolio; ROA IS -9.6% / OOS -13.8% (Table 8-3)"))),
    t!("kets_seas_conf_limit", "Seasonal crossover + confirmation · limit (Test 8)", G8, M::Seas(3), Limit, "Long/Short", "pp.171-173 (PDF 184-186)",
        [("avglen", "Moving-average length", 15., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 6., 0., 20., 1.)], Some(("", "", "IS $-832 / OOS $-3,408", "", "36-futures portfolio; ROA IS -7.2% / OOS -21.1% (Table 8-3)"))),
    t!("kets_seas_conf_stop", "Seasonal crossover + confirmation · stop (Test 9)", G8, M::Seas(3), Stop, "Long/Short", "pp.171-173 (PDF 184-186)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 9., 0., 20., 1.)], Some(("IS 292 / OOS 121", "IS 41% / OOS 44%", "IS $846 / OOS $1,677", "", "36-futures portfolio; ROA IS 5.8% / OOS 19.6% (text p.172, Table 8-3)"))),
    t!("kets_seas_inv_open", "Seasonal crossover + confirmation & inversion · next open (Test 10)", G8, M::Seas(4), Open, "Long/Short", "pp.173-174 (PDF 186-187)",
        [("avglen", "Moving-average length", 15., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 2., 0., 20., 1.)], Some(("", "", "IS $-1,669 / OOS $-2,545", "", "36-futures portfolio; ROA IS -9.6% / OOS -20.4% (Table 8-3)"))),
    t!("kets_seas_inv_limit", "Seasonal crossover + confirmation & inversion · limit (Test 11)", G8, M::Seas(4), Limit, "Long/Short", "pp.173-174 (PDF 186-187)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 4., 0., 20., 1.)], Some(("", "", "IS $-1,698 / OOS $-2,642", "", "36-futures portfolio; ROA IS -8.9% / OOS -22.6% (Table 8-3)"))),
    t!("kets_seas_inv_stop", "Seasonal crossover + confirmation & inversion · stop (Test 12)", G8, M::Seas(4), Stop, "Long/Short", "pp.173-174 (PDF 186-187)",
        [("avglen", "Moving-average length", 20., 5., 20., 5.), ("disp", "Displacement (bars ahead)", 9., 0., 20., 1.)], Some(("", "", "IS $-229 / OOS $95", "", "36-futures portfolio; ROA IS -2.2% / OOS 0.9% (Table 8-3)"))),
    // ---------------------------------------------------------------- Chapter 9
    t!("kets_lunar_xo_open", "Lunar crossover · next open (Test 1)", G9, M::Lunar(2), Open, "Long/Short", "pp.183-193 (PDF 196-206)",
        [("avglen", "Moving-average length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 8., 0., 15., 1.)], Some(("", "", "IS $-1,287 / OOS $-894", "", "36-futures portfolio; ROA IS -9.3% / OOS -14.3% (Table 9-1)"))),
    t!("kets_lunar_xo_limit", "Lunar crossover · limit (Test 2)", G9, M::Lunar(2), Limit, "Long/Short", "pp.183-193 (PDF 196-206)",
        [("avglen", "Moving-average length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 6., 0., 15., 1.)], Some(("IS 1,759", "IS 43%", "IS $-406 / OOS $-643", "", "36-futures portfolio; ROA IS -6.0% / OOS -10.2% (Table 9-1)"))),
    t!("kets_lunar_xo_stop", "Lunar crossover · stop (Test 3)", G9, M::Lunar(2), Stop, "Long/Short", "pp.183-193 (PDF 196-206)",
        [("avglen", "Moving-average length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 12., 0., 15., 1.)], Some(("", "", "IS $-886 / OOS $-702", "", "36-futures portfolio; ROA IS -5.6% / OOS -10.4% (Table 9-1)"))),
    t!("kets_lunar_mom_open", "Lunar momentum threshold · next open (Test 4)", G9, M::Lunar(1), Open, "Long/Short", "pp.193-194 (PDF 206-207)",
        [("avglen", "Centered MA length", 10., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 10., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 2., 1., 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (CD script); 7 centered triangular (book prose, labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $-2,410 / OOS $-1,316", "", "36-futures portfolio; ROA IS -10.1% / OOS -14.8% (Table 9-1)"))),
    t!("kets_lunar_mom_limit", "Lunar momentum threshold · limit (Test 5)", G9, M::Lunar(1), Limit, "Long/Short", "pp.193-194 (PDF 206-207)",
        [("avglen", "Centered MA length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 9., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 1.5, 1., 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (CD script); 7 centered triangular (book prose, labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $-1,560 / OOS $-1,942", "", "36-futures portfolio; ROA IS -9.9% / OOS -19.7% (Table 9-1)"))),
    t!("kets_lunar_mom_stop", "Lunar momentum threshold · stop (Test 6)", G9, M::Lunar(1), Stop, "Long/Short", "pp.193-194 (PDF 206-207)",
        [("avglen", "Centered MA length", 15., 5., 15., 5.), ("disp", "Displacement (bars ahead)", 9., 1., 10., 1.), ("thresh", "Threshold x 100-bar mean |momentum|", 1.5, 1., 2.5, 0.5),
         ("matype", "Smoothing: 5 centered simple (CD script); 7 centered triangular (book prose, labelled option)", 5., 5., 7., 2.)], Some(("", "", "IS $-1,288 / OOS $-372", "", "36-futures portfolio; ROA IS -8.1% / OOS -8.3% (Table 9-1)"))),
    t!("kets_lunar_conf_open", "Lunar crossover + Fast %K confirmation · next open (Test 7)", G9, M::Lunar(3), Open, "Long/Short", "pp.194-195 (PDF 207-208)",
        [("avglen", "Moving-average length", 15., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 12., 0., 15., 1.)], Some(("", "", "IS $-1,251 / OOS $-3,465", "", "36-futures portfolio; ROA IS -8.1% / OOS -20.5% (Table 9-1)"))),
    t!("kets_lunar_conf_limit", "Lunar crossover + confirmation · limit (Test 8)", G9, M::Lunar(3), Limit, "Long/Short", "pp.194-195 (PDF 207-208)",
        [("avglen", "Moving-average length", 15., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 12., 0., 15., 1.)], Some(("", "", "IS $-655 / OOS $-3,896", "", "36-futures portfolio; ROA IS -6.6% / OOS -21.3% (Table 9-1)"))),
    t!("kets_lunar_conf_stop", "Lunar crossover + confirmation · stop (Test 9)", G9, M::Lunar(3), Stop, "Long/Short", "pp.194-195 (PDF 207-208)",
        [("avglen", "Moving-average length", 12., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 5., 0., 15., 1.)], Some(("", "", "IS $234 / OOS $-2,449", "", "36-futures portfolio; ROA IS 1.8% / OOS -18.6% (Table 9-1)"))),
    t!("kets_lunar_inv_open", "Lunar crossover + confirmation & inversion · next open (Test 10)", G9, M::Lunar(4), Open, "Long/Short", "p.195 (PDF 208)",
        [("avglen", "Moving-average length", 15., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 12., 0., 15., 1.)], Some(("", "", "IS $-1,546 / OOS $-2,937", "", "36-futures portfolio; ROA IS -9.4% / OOS -20.6% (Table 9-1)"))),
    t!("kets_lunar_inv_limit", "Lunar crossover + confirmation & inversion · limit (Test 11)", G9, M::Lunar(4), Limit, "Long/Short", "p.195 (PDF 208)",
        [("avglen", "Moving-average length", 15., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 8., 0., 15., 1.)], Some(("", "", "IS $-1,078 / OOS $-3,203", "", "36-futures portfolio; ROA IS -10.0% / OOS -20.9% (Table 9-1)"))),
    t!("kets_lunar_inv_stop", "Lunar crossover + confirmation & inversion · stop (Test 12)", G9, M::Lunar(4), Stop, "Long/Short", "p.195 (PDF 208)",
        [("avglen", "Moving-average length", 12., 3., 15., 3.), ("disp", "Displacement (bars ahead)", 15., 0., 15., 1.)], Some(("", "", "IS $-998 / OOS $-2,995", "", "36-futures portfolio; ROA IS -7.8% / OOS -20.1% (Table 9-1)"))),
    // ---------------------------------------------------------------- Chapter 10
    t!("kets_cycle_open", "Wavelet filter-bank cycle turns · next open (Test 1)", G10, M::Cycle, Open, "Long/Short", "pp.211-221 (PDF 223-233)",
        [("width", "Filter bandwidth", 0.2, 0.05, 0.2, 0.05), ("pdisp", "Phase displacement, degrees", 20., -20., 20., 10.)], Some(("IS 1,312 / OOS 547", "IS 40% / OOS 34%", "IS $-1,329 / OOS $-3,741", "6 bars", "36-futures portfolio; ROA IS -10.2% / OOS -23.2% (Table 10-1)"))),
    t!("kets_cycle_limit", "Wavelet filter-bank cycle turns · limit (Test 2)", G10, M::Cycle, Limit, "Long/Short", "pp.211-221 (PDF 223-233)",
        [("width", "Filter bandwidth", 0.2, 0.05, 0.2, 0.05), ("pdisp", "Phase displacement, degrees", 20., -20., 20., 10.)], Some(("IS 1,103 / OOS 475", "IS 41% / OOS 34%", "IS $-1,037 / OOS $-3,551", "7 bars", "36-futures portfolio; ROA IS -9.5% / OOS -22.8% (Table 10-1)"))),
    t!("kets_cycle_stop", "Wavelet filter-bank cycle turns · stop (Test 3)", G10, M::Cycle, Stop, "Long/Short", "pp.211-221 (PDF 223-233)",
        [("width", "Filter bandwidth", 0.1, 0.05, 0.2, 0.05), ("pdisp", "Phase displacement, degrees", 20., -20., 20., 10.)], Some(("IS 957 / OOS 403", "IS 40% / OOS 41%", "IS $-1,245 / OOS $-944", "7 bars", "36-futures portfolio; ROA IS -8.3% / OOS -15.0% (Table 10-1)"))),
    // ---------------------------------------------------------------- Chapters 13-14 (exits, random entries)
    t!("kets_rand_ses_open", "Random entries (2.5% each side) + standard exit · open (Ch.13 Test 1)", G13, M::RandSes, Open, "Long/Short", "pp.295-300 (PDF 305-310)",
        [SEED, MKT], Some(("", "", "IS avg of 10 sequences $-2,243 (sd $304)", "", "36-futures portfolio, 10 random sequences (Table 13-1, p.332)"))),
    t!("kets_rand_ses_limit", "Random entries + standard exit · limit (Ch.13 Test 2)", G13, M::RandSes, Limit, "Long/Short", "pp.295-300 (PDF 305-310)", [SEED, MKT], None),
    t!("kets_rand_ses_stop", "Random entries + standard exit · stop (Ch.13 Test 3)", G13, M::RandSes, Stop, "Long/Short", "pp.295-300 (PDF 305-310)", [SEED, MKT], None),
    tr!("kets_rand_mses", "Random entries at open + modified standard exit (intraday stop/target) (Ch.13 Test 4)", G13, M::RandMses(0), Open, "pp.301-305 (PDF 311-315)",
        [("pt", "Profit-target limit, ATR(50) units", 4., 0.5, 8., 0.5), ("sl", "Money-management stop, ATR(50) units", 1., 0.5, 4., 0.5), ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT], None),
    tr!("kets_rand_fixed_opt", "Random entries + MSES with the optimal fixed stop/target (Ch.14, Table 14-1)", G13, M::RandMses(0), Open, "pp.311-315 (PDF 321-325)",
        [("pt", "Profit-target limit, ATR(50) units", 4.5, 0.5, 8., 0.5), ("sl", "Money-management stop, ATR(50) units", 1.5, 0.5, 4., 0.5), ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT],
        Some(("", "39%", "$-1,581", "", "36-futures portfolio IS, ARRR -1.46 (p.322)"))),
    tr!("kets_rand_hhll_stop", "Random entries + 2-bar highest-high/lowest-low trailing stop (Ch.14, Table 14-2)", G13, M::RandMses(1), Open, "pp.316-323 (PDF 326-333)",
        [("pt", "Profit-target limit, ATR(50) units", 4.5, 0.5, 8., 0.5), ("sl", "Initial stop: no tighter than this many ATR(50) units", 2.5, 0.5, 3.5, 0.5), ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT],
        Some(("IS 4,171 / OOS 1,793", "IS 28% / OOS 29%", "IS $-1,864 / OOS $-1,818", "4 bars", "36-futures portfolio; ARRR IS -2.52 / OOS -2.38 (Table 14-2)"))),
    tr!("kets_rand_atr_stop", "Random entries + dynamic ATR-based trailing stop (Ch.14, Table 14-3)", G13, M::RandMses(2), Open, "pp.316-325 (PDF 326-335)",
        [("pt", "Profit-target limit, ATR(50) units", 4.5, 0.5, 8., 0.5), ("sl", "Entry-bar stop, ATR(50) units (FIRST)", 2., 0.5, 3.5, 0.5), ("later", "Later-bar stop: close -/+ this many ATR(50) units (LATER)", 2.5, 0.5, 3.5, 0.5),
         ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT], Some(("", "42%", "", "", "36-futures portfolio IS; ARRR -1.40 (p.323)"))),
    tr!("kets_rand_mema_stop", "Random entries + MEMA dynamic stop (Ch.14, Table 14-4)", G13, M::RandMses(3), Open, "pp.316-326 (PDF 326-336)",
        [("pt", "Profit-target limit, ATR(50) units", 4.5, 0.5, 8., 0.5), ("sl", "Initial stop, ATR(50) units", 2.5, 0.5, 3.5, 0.5), ("later", "ATR offset (ATRO)", 1., 0., 3., 0.5),
         ("coeff", "Adaptation rate coefficient (COEFF)", 0.3, 0.1, 0.5, 0.1), ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT],
        Some(("", "37%", "$-1,407", "", "36-futures portfolio IS; ARRR -1.36 (p.325)"))),
    tr!("kets_rand_mema_shrink", "Random entries + MEMA stop + shrinking profit target (Ch.14, Table 14-5)", G13, M::RandMses(4), Open, "pp.326-329 (PDF 336-341)",
        [("pt", "Initial profit target, ATR(50) units (INIT)", 5.5, 2., 7., 0.5), ("ptga", "Target shrinkage coefficient (COEFF)", 0.1, 0.05, 0.5, 0.05), ("sl", "Initial stop, ATR(50) units", 2.5, 0.5, 3.5, 0.5),
         ("later", "ATR offset", 1., 0., 3., 0.5), ("coeff", "Stop adaptation rate", 0.3, 0.1, 0.5, 0.1), ("hold", "Exit at the close after this many bars", 10., 2., 30., 1.), SEED, MKT],
        Some(("", "37%", "$-1,325", "", "36-futures portfolio IS; ARRR -1.32 (Table 14-5)"))),
    tr!("kets_rand_mema_shrink30", "Random entries + MEMA stop + shrinking target, 30-day time limit (Ch.14 best exit, Table 14-6)", G13, M::RandMses(4), Open, "pp.329-332 (PDF 341-344)",
        [("pt", "Initial profit target, ATR(50) units", 5.5, 2., 7., 0.5), ("ptga", "Target shrinkage coefficient", 0.1, 0.05, 0.5, 0.05), ("sl", "Initial stop, ATR(50) units", 2.5, 0.5, 3.5, 0.5),
         ("later", "ATR offset", 1., 0., 3., 0.5), ("coeff", "Stop adaptation rate", 0.3, 0.1, 0.5, 0.1), ("hold", "Exit at the close after this many bars", 30., 2., 40., 1.), SEED, MKT],
        Some(("", "37%", "$-1,236", "6-10 bars", "36-futures portfolio IS; ARRR -1.22 (p.330)"))),
];

/// Rules text per model (quoted / paraphrased tightly from the book and the CD code, page refs in the test table).
fn model_rules(m: M) -> &'static [&'static str] {
    match m {
        M::Chan { hl: false } => &["E|\"If the current position is either short or flat and the market closes above the highest close of the last n days, then buy\"; \"long or flat and the market closes below the lowest close of the preceding n days, then sell\" (p.86; x09mod01.c: cls[cb] > Highest(cls,n,cb-1))",
            "E|A breakout signal also reverses an opposite position; re-entry in a continuing trend is allowed while new highs/lows are made (p.88)"],
        M::Chan { hl: true } => &["E|\"buys ... when today's close breaks above the highest high of the last n days, and sells ... when today's close drops below the lowest low\" (p.92; x09mod04.c: cls[cb] > Highest(hi,n,cb-1), cls[cb] < Lowest(lo,n,cb-1))"],
        M::HhllStop => &["E|Highest high hh and lowest low ll of the last n bars INCLUDING the current bar (p.96, x09mod06.c)",
            "E|If (close-ll)/(hh-ll) >= 0.5 and not long: buy stop at hh next bar; if < 0.5 and not short: sell stop at ll (\"Both orders are never posted together\", p.96)"],
        M::Vb { long_only, adx } => match (long_only, adx) {
            (false, false) => &["E|Upper band = EMA(close, ma)[yesterday] + bw x ATR(atr)[yesterday]; lower band = EMA - bw x ATR (p.97; x09mod07/08.c)",
                "E|Close above the upper band: buy (if not long); close below the lower band: sell short (if not short)"],
            (true, _) => &["E|Upper band = EMA(close, ma)[yesterday] + bw x ATR(atr)[yesterday]; lower band = EMA - bw x ATR (p.97; x09mod10.c)",
                "E|Close above the upper band: buy on a limit (if not long). Long positions only (p.100)",
                "X|Close below the lower band while long: exit long on a limit at the signal bar's midpoint (x09mod10.c exitlonglimit)"],
            (false, true) => &["E|Upper band = EMA(close, ma)[yesterday] + bw x ATR(atr)[yesterday]; lower band = EMA - bw x ATR (x09mod12.c)",
                "F|\"Trending conditions exist as long as the 18-bar ADX makes a new 6-bar high, and entries are taken only when trends exist\" (p.102): ADX[today] > highest ADX of the previous 6 bars; Wilder ADX with integer truncation as in AvgDirMov()",
                "E|Trending and close above the upper band: buy; trending and close below the lower band: sell short"],
        },
        M::StochObos => &["E|Lane's original Fast %K (x11mod01.c osctype 1). Buy when %K crosses above 20 (%K[t] >= 20 and %K[t-1] < 20); sell when %K crosses below 80 (%K[t] < 80, %K[t-1] >= 80) (pp.141-146)"],
        M::RsiObos => &["E|Classic RSI (RsiOsc type 1). Buy when RSI crosses above 30; sell when RSI crosses below 70 (p.147)"],
        M::StochSig => &["E|Lane's Slow %K (osctype 2) and a 3-bar simple moving average of it as signal line; buy when Slow %K crosses above the signal line, sell when it crosses below (p.147)"],
        M::StochDiv => &["E|Over the look-back lb: lowest close at bar i and lowest Fast %K at bar j. Buy if i < today and i > today-6, j > today-lb+1, i-j > 4 and %K turns up (%K[t] >= %K[t-1] < %K[t-2]) (pp.148-149, x11mod01.c model 3)",
            "E|Otherwise sell on the mirror image (highest close / highest %K, %K turns down)"],
        M::Seas(k) | M::Lunar(k) => {
            let lunar = matches!(m, M::Lunar(_));
            match (lunar, k) {
                (false, 1) => &["E|pchg = (close - previous close) / ATR(50), clipped to +/-2. Seasonal value of a bar = mean pchg on the same calendar date (exact match) in all earlier years (pp.157-159, 165; SeasonalAvg)",
                    "E|Smooth with a centered simple MA (avglen); threshold = thresh x 100-bar mean |smoothed|. Buy when the value at today+disp crosses above +threshold; sell when it crosses below -threshold (p.170)"],
                (false, 2) => &["E|Seasonal series as above, integrated (running sum); ma1 = SMA(series, avglen), ma2 = SMA(ma1, avglen). Buy when ma1 crosses above ma2 at bar today+disp; sell when it crosses below (pp.165, 168)"],
                (false, 3) => &["E|Seasonal crossover (ma1/ma2 at today+disp) taken only if confirmed: buy only if today's 9-bar Fast %K < 25, sell only if > 75 (pp.157, 171)"],
                (false, _) => &["E|Seasonal crossover with confirmation; if the crossover says buy but Fast %K(9) > 75 a sell is posted instead, and if it says sell but %K < 25 a buy is posted (inversion, p.173)"],
                (true, 1) => &["E|pchg = (close - previous close) / ATR(50), clipped to +/-2. Lunar value of a bar = mean pchg on the bars with the same moon phase (same offset from the same quarter phase) 2..61 lunar cycles earlier, up to 60 cases (pp.183-189, LunarAvg mode 2, flmoon)",
                    "E|Smooth with a centered MA (avglen); threshold = thresh x 100-bar mean |smoothed|; buy when the value at today+disp crosses above +threshold, sell when it crosses below -threshold (p.193)"],
                (true, 2) => &["E|Lunar series as above, integrated; ma1 = SMA(series, avglen), ma2 = SMA(ma1, avglen); buy when ma1 crosses above ma2 at bar today+disp, sell when it crosses below (p.191)"],
                (true, 3) => &["E|Lunar crossover taken only if confirmed by today's 9-bar Fast %K (< 25 for a buy, > 75 for a sell) (p.194)"],
                (true, _) => &["E|Lunar crossover with confirmation and inversion (crossover buy with %K > 75 becomes a sell; crossover sell with %K < 25 becomes a buy) (p.195)"],
            }
        }
        M::Cycle => &["E|Bank of 20 wavelet (quadrature) filters, centre periods 3..30 bars geometrically spaced, bandwidth width (pp.207-216, WAVFILT.C); power = (in-phase^2 + quadrature^2) / period",
            "F|Trade only a 'good, pure cycle': the strongest filter is not among the 3 shortest or 2 longest, and its power > 1.5 x the strongest filter more than 2 filters away (x14mod01.c)",
            "E|Phase of the dominant filter; sell when phase crosses above -(disp + 180/period) degrees, buy when phase (0-360) crosses above that +180 (predicted cycle bottom / top) (pp.215-216)"],
        M::RandSes => &["E|Random entry: each bar a uniform random number (Numerical Recipes ran2, seed -(ranseed + 10 x market)); < 0.025 = sell, > 0.975 = buy (p.296, x19mod01.c)"],
        M::RandMses(k) => match k {
            0 => &["E|Random entry at the next open (2.5% buy / 2.5% sell per bar, ran2 as in Ch.13)",
                "X|Modified standard exit: profit-target limit at entry + pt x ATR(50) and money-management stop at entry - sl x ATR(50) (shorts mirrored), both resting intraday from the entry bar; exit at the close when held > hold bars (pp.301-302, x19mod02.c / x20mod01.c)"],
            1 => &["E|Random entry at the next open (ran2 as in Ch.13)", "X|Profit-target limit at entry + pt x ATR(50)",
                "X|Stop on the entry bar = min(lowest low of the last 2 bars, entry - sl x ATR); later raised to the lowest low of the last 2 bars, never lowered (p.317, x20mod02.c model 1)", "X|Exit at the close when held > hold bars"],
            2 => &["E|Random entry at the next open (ran2 as in Ch.13)", "X|Profit-target limit at entry + pt x ATR(50)",
                "X|Entry-bar stop at entry - sl x ATR; later stop = max(previous stop, close - later x ATR) (p.318, model 2)", "X|Exit at the close when held > hold bars"],
            3 => &["E|Random entry at the next open (ran2 as in Ch.13)", "X|Profit-target limit at entry + pt x ATR(50)",
                "X|MEMA stop: entry-bar stop at entry - sl x ATR; each later bar, if (high - later x ATR) is above the stop, the stop moves up by coeff x the difference (never down) (pp.318-319, model 3)", "X|Exit at the close when held > hold bars"],
            _ => &["E|Random entry at the next open (ran2 as in Ch.13)",
                "X|MEMA stop as in the MEMA test (initial sl x ATR, ATR offset 'later', rate 'coeff')",
                "X|Shrinking profit target: starts at entry + pt x ATR; each later bar target = target - ptga x (target - close) (pp.326-327, x20mod03.c model 4)", "X|Exit at the close when held > hold bars"],
        },
    }
}

fn order_rule(o: Od) -> &'static str {
    match o {
        Od::Open => "E|Entry: market order at the next bar's open",
        Od::Limit => "E|Entry: limit order on the next bar at the signal bar's midpoint (high+low)/2 (\"retrace at least 50% of the breakout bar\", p.90)",
        Od::Stop => "E|Entry: stop order on the next bar at the signal close + (buy) / - (sell) 0.5 x ATR(50) (p.119 standard entry stop)",
    }
}
const SES_RULES: [&str; 2] = [
    "X|Standard exit, close-only: money-management stop at entry -/+ sl x ATR(50) and profit target at entry +/- pt x ATR(50), tested on the close; exit at the close once the trade has been held hold bars (pp.76, 87; TRDSIM2 stdexitcls)",
    "X|An opposite entry signal reverses the position (p.159: \"closed out either when an entry in the opposing direction occurs or when the standard exit closes the trade\")",
];

fn notes_for(t: &T) -> &'static str {
    match t.m {
        M::Seas(_) => "Defaults = the book's printed best in-sample parameters. Seasonal averages use only earlier years (the book's out-of-sample method, p.158); the book's in-sample jackknife also used LATER years and is not reproduced. Like the book, the signal reads the seasonal value of a future bar (today+disp), which is built from prior-year prices only; no orders in the last 30 bars of the loaded data (x12mod01.c). Book: 36-futures portfolio with dollar-volatility sizing; here the selected index (instrument swap only).",
        M::Lunar(_) => "Defaults = the book's printed best parameters. Lunar averages use up to 60 earlier cycles only; the CD code fills from LATER cycles when fewer than 60 earlier ones exist, which contradicts the book text ('determined only by events about 27 or more days ago', p.184) and is not reproduced. Moon phases: Numerical Recipes flmoon, US Eastern time as in the code. No orders in the last 30 bars of the loaded data. Momentum smoothing: the CD script uses type 5 (centered simple); the prose calls it centered triangular (type 7, labelled option).",
        M::Cycle => "Defaults = Table 10-1 best in-sample parameters. Causal wavelet filters on closing prices (WAVFILT.C). No orders in the last 30 bars of the loaded data (x14mod01.c). Book: futures portfolio; here the selected index.",
        M::RandSes | M::RandMses(_) => "Exit study: entries are random by design (Numerical Recipes ran2 exactly as on the CD; the sequence depends on the seed and the bars, so it cannot match the book's sequence on its futures). Market index 1 = the book's S&P 500 seed slot. Portfolio results in the book are averages over futures markets; here one index.",
        _ => "Defaults = the book's printed best in-sample parameters (optimised on the 36-futures portfolio, 8/1985-12/1994). Book: dollar-volatility sizing of futures, 3 ticks slippage + $15 commission; here one unit of the selected index (instrument swap only); costs come from the vault's cost settings.",
    }
}

static CAT: OnceLock<Vec<(&'static [&'static str], &'static str)>> = OnceLock::new();

fn rules_of() -> &'static Vec<(&'static [&'static str], &'static str)> {
    CAT.get_or_init(|| TESTS.iter().map(|t| {
        let mut v: Vec<&'static str> = model_rules(t.m).to_vec();
        match t.m {
            M::RandMses(_) => {}
            M::HhllStop => v.extend(SES_RULES),
            _ => { v.push(order_rule(t.ord)); v.extend(SES_RULES); }
        }
        if !matches!(t.m, M::RandSes | M::RandMses(_)) {
            v.push("F|No orders when the NEXT bar has high == low (book's limit-locked filter, p.88); no orders before the book's ~120-bar warm-up / look-backs are complete");
        }
        let pages: &'static str = Box::leak(format!("F|Source: {}", t.pages).into_boxed_str());
        v.push(pages);
        let rules: &'static [&'static str] = Box::leak(v.into_boxed_slice());
        (rules, notes_for(t))
    }).collect())
}

const NT_MA: &str = "the book never prints the optimised moving-average lengths of these tests (only ROA%/$TRD in Tables 6-3 and 6-6), so the default rule cannot be stated without inventing it";
const NT_OSC: &str = "the book never prints the optimised oscillator lengths / look-back of these tests (only ROA%/$TRD in Table 7-3), so the default rule cannot be stated without inventing it";

macro_rules! off {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

fn off_list() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        off!("kets_vb_stop", "Volatility breakout · stop at the band (Ch.5 Test 9)", G5, "Long/Short",
            NotTestable("the book prints the optimal band width as 'X.3' (p.99); the bandwidth digit is missing, so the default cannot be stated"),
            &["E|Bands = EMA(close, 11)[today] +/- bw x ATR(21)[today]; buy stop at the upper band if the close is in the upper half of the bands, else sell stop at the lower band (p.99, x09mod09.c)", "X|Standard exit"],
            "Printed: \"a bandwidth of X.3, a moving average length of 11, and an average true range of 21 bars\" (p.99). IS 1,465 trades, 40% wins, $931/trade; OOS 610 trades, 29%."),
        off!("kets_vb_limit_ccy", "Volatility breakout · limit · currencies only (Ch.5 Test 11)", G5, "Long/Short",
            NeedsData("daily British Pound, Deutschemark, Swiss Franc, Japanese Yen, Canadian Dollar and Eurodollar futures (the rule trades only these six markets)"),
            &["F|Trade only BP, DM, SF, JY, CD, ED futures (x09mod11.c)", "E|Volatility breakout with limit entry, Test 8 parameters bw 3.7, EMA 22, ATR 41 (p.101)", "X|Standard exit"],
            "Book: IS 268 trades, 48% wins, $3,977/trade, ROA 36.2%; OOS 102 trades, 43%, $2,106, ROA 17.7% (pp.101-102). On an index the market filter admits no bars."),
        off!("kets_ma_trend_xo", "Trend-following MA crossover, SMA/EMA/FWTMA/VIDYA x open/limit/stop (Ch.6 Tests 1-12)", "Ch.6 Moving-average models", "Long/Short", NotTestable(NT_MA),
            &["E|Buy when the fast MA crosses above the slow MA (fast >= slow today, fast < slow yesterday); sell on the opposite cross (x10mod01.c model 1)", "E|Fast 1-5, slow 5-50 optimised per test (p.120)", "X|Standard exit"],
            "Rules exact in x10mod01.c; Table 6-3 prints only portfolio ROA%/$TRD, e.g. SMA crossover IS open/limit/stop $-1,765/$-926/$-1,045."),
        off!("kets_ma_slope", "MA slope turns, 4 MA types x 3 orders (Ch.6 Tests 13-24)", "Ch.6 Moving-average models", "Long/Short", NotTestable(NT_MA),
            &["E|Buy when the MA turns up (ma[t] >= ma[t-1] < ma[t-2]); sell when it turns down (x10mod01.c model 2)", "E|MA length 3-40 optimised per test (p.120)", "X|Standard exit"], "Table 6-3: e.g. FWTMA slope with stop OOS $-91/trade (the chapter's best OOS)."),
        off!("kets_ma_ct_xo", "Counter-trend MA crossover, 4 MA types x 3 orders (Ch.6 Tests 25-36)", "Ch.6 Moving-average models", "Long/Short", NotTestable(NT_MA),
            &["E|Sell when the fast MA crosses above the slow MA; buy when it crosses below (x10mod01.c model 3)", "E|Fast 1-7, slow 5-50 optimised (p.126)", "X|Standard exit"], "Table 6-6 prints ROA%/$TRD only."),
        off!("kets_ma_sr", "MA support/resistance bounce, 4 MA types x 3 orders (Ch.6 Tests 37-48)", "Ch.6 Moving-average models", "Long/Short", NotTestable(NT_MA),
            &["E|Buy when the slow MA is rising and the fast MA crosses below it; sell when the slow MA is falling and the fast MA crosses above it (x10mod01.c model 4)", "E|Fast 1-5, slow 5-50 optimised (p.126)", "X|Standard exit"],
            "Text: SMA support/resistance with stop entry made $227/trade IS (ROA 4.2%) and $482 OOS (ROA 14.8%) (p.127); lengths not printed."),
        off!("kets_macd_sig", "MACD / 9-bar signal-line cross · open/limit/stop (Ch.7 Tests 10-12)", G7, "Long/Short", NotTestable(NT_OSC),
            &["E|Classic MACD (EMA fast - EMA slow) crosses its 9-bar EMA signal line (x11mod01.c model 2, osctype 4)", "E|Fast 3-15 step 2, slow 10-40 step 5 optimised (p.148)", "X|Standard exit"], "Table 7-3: IS $-1,808/$-1,210/$-1,476 per trade."),
        off!("kets_rsi_div", "RSI / price divergence · open/limit/stop (Ch.7 Tests 16-18)", G7, "Long/Short", NotTestable(NT_OSC),
            &["E|Divergence rule as in the Stochastic divergence tests, on the classic RSI (x11mod01.c model 3, osctype 3)", "E|RSI 5-25 and look-back 15-25 optimised (p.150)", "X|Standard exit"], "Table 7-3: IS $-2,278/$-1,529/$-1,309."),
        off!("kets_macd_div", "MACD / price divergence · open/limit/stop (Ch.7 Tests 19-21)", G7, "Long/Short", NotTestable(NT_OSC),
            &["E|Divergence rule on the classic MACD (x11mod01.c model 3, osctype 4)", "E|Fast 3-15, slow 10-40, look-back 15-25 optimised (p.150)", "X|Standard exit"],
            "The chapter's only profitable model: IS $1,393/$1,250/$2,062, OOS $140/$985/$-589 per trade, ROA IS 26.3/12.5/27.2% (Table 7-3); its parameters are not printed."),
        off!("kets_solar", "Sunspot breakout model · open/limit/stop (Ch.9 solar Tests 1-3)", G9, "Long/Short",
            NeedsData("daily sunspot numbers (Royal Observatory of Belgium / SILSO), which the vault does not have; the model's code is only on the CD and its adaptive response vector reads prices up to 10 bars after each event"),
            &["E|Sunspot count above the highest of the previous lb1 counts (or below the lowest) with no such breakout in the previous lb2 counts; direction chosen adaptively from earlier market responses; acted on disp bars later (pp.197-199)", "E|Best: open 35/15/2, limit 40/15/0, stop 35/15/2 (p.199)", "X|Standard exit"],
            "Book: with the stop order the S&P 500 made $4,991/trade IS (37 trades, 40% wins, ROA 20.4%) and $5,640 OOS (11 trades, 45%, ROA 39.5%) (pp.199-200)."),
        off!("kets_neural", "Neural-network entries: reverse Slow %K and turning-point nets (Ch.11)", "Ch.11 Neural networks", "Long/Short",
            NotTestable("trained network weights are not in the book (it prints the training code, not the nets); results depend on the authors' trained networks and training data"),
            &["E|Feed-forward nets forecasting a time-reversed Slow %K or turning points; enter when the forecast crosses a threshold (pp.227-255)"], "Not reproducible from the book text."),
        off!("kets_ga_long", "Genetically evolved long entry (open-interest rules) · open/limit/stop (Ch.12 Tests 1-3)", "Ch.12 Genetic algorithms", "Long",
            NeedsData("futures open interest: two of the three evolved rules test the open interest (decline >= 4.2% over 34 bars, rise >= 5.6% over 44 bars); index data has no open interest"),
            &["E|Buy when all three hold: open interest 34 bars ago - open interest 1 bar ago > 4.2% of the former; close 3 bars ago > close 14 bars ago + 3.46 x ATR; open interest risen >= 5.6% over 44 bars (p.278)", "X|Standard exit"],
            "Book: IS 43 trades at the open, >60% wins, > $14,000/trade; OOS 17 trades (p.274)."),
        off!("kets_ga_short", "Genetically evolved short entry (open-interest + MACD rules) · open/limit/stop (Ch.12 Tests 4-6)", "Ch.12 Genetic algorithms", "Short",
            NeedsData("futures open interest: two of the three evolved rules test the open interest; index data has no open interest"),
            &["E|Sell when open interest 1 bar ago >= 1.38 x that 38 bars ago, >= 1.75 x that 25 bars ago, and the MACD(2,50) slope is down (p.278)", "X|Standard exit"], "Book: profitable IS, lost OOS (p.274)."),
        off!("kets_neural_exit", "Neural-network exit component (Ch.15)", "Ch.15 AI exits", "Long/Short",
            NotTestable("depends on trained neural networks whose weights the book does not print"), &["X|MEMA stop/shrinking target exit plus a neural-net exit signal (pp.335-345)"], "Not reproducible from the book text."),
        off!("kets_ga_exit", "Genetically evolved rule-based exits (Ch.15)", "Ch.15 AI exits", "Long/Short",
            NotTestable("the evolved exit rules are printed in prose (long: close above its 12-bar EMA and below its 49-bar EMA with a 6-bar new high; short: close above its 16-bar EMA and 22-bar SMA with the MACD(6,10) slope down, pp.345-346), but the new-high recency window and the random-entry sequence they were evolved and tested on are not printed, so the test cannot be reproduced"),
            &["X|Rule-template exits evolved by a genetic algorithm (pp.345-351)"], "Not reproducible from the book text."),
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    let r = rules_of();
    let mut v: Vec<StrategyDef> = TESTS.iter().zip(r.iter()).map(|(t, (rules, notes))| StrategyDef {
        id: t.id, name: t.name, group: t.group, session: 0, dir: t.dir, status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes,
        claim: t.claim.map(|(tr, w, a, h, p)| Claim { trades: tr, win: w, avg: a, hold: h, period: p }),
    }).collect();
    v.extend(off_list());
    v
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let t = TESTS.iter().find(|t| t.id == id)?;
    Some(t.defs.iter().map(|&(key, label, def, min, max, step)| ParamDef { key, label, def, min, max, step }).collect())
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if let Some(t) = TESTS.iter().find(|t| t.id == id) {
        let ord = match t.ord { Od::Open => "Entry at open", Od::Limit => "Entry on limit", Od::Stop => "Entry on stop" };
        let (f, n): (&'static str, &'static str) = match t.m {
            M::Chan { hl: false } => ("kets_cb", "Close-only channel breakout"),
            M::Chan { hl: true } | M::HhllStop => ("kets_hhll", "Highest-high / lowest-low breakout"),
            M::Vb { long_only: false, adx: false } => ("kets_vb", "Volatility breakout"),
            M::Vb { long_only: true, .. } => ("kets_vb", "Volatility breakout"),
            M::Vb { adx: true, .. } => ("kets_vb", "Volatility breakout"),
            M::StochObos => ("kets_stoch_obos", "Stochastic overbought/oversold"),
            M::RsiObos => ("kets_rsi_obos", "RSI overbought/oversold"),
            M::StochSig => ("kets_stoch_sig", "Stochastic signal line"),
            M::StochDiv => ("kets_stoch_div", "Stochastic divergence"),
            M::Seas(1) => ("kets_seas_mom", "Seasonal momentum"),
            M::Seas(2) => ("kets_seas_xo", "Seasonal crossover"),
            M::Seas(3) => ("kets_seas_conf", "Seasonal crossover + confirmation"),
            M::Seas(_) => ("kets_seas_inv", "Seasonal crossover + confirmation & inversion"),
            M::Lunar(1) => ("kets_lunar_mom", "Lunar momentum"),
            M::Lunar(2) => ("kets_lunar_xo", "Lunar crossover"),
            M::Lunar(3) => ("kets_lunar_conf", "Lunar crossover + confirmation"),
            M::Lunar(_) => ("kets_lunar_inv", "Lunar crossover + confirmation & inversion"),
            M::Cycle => ("kets_cycle", "Wavelet cycle turns"),
            M::RandSes => ("kets_rand_ses", "Random entries + standard exit"),
            M::RandMses(_) => ("kets_rand_exits", "Random entries + improved exits"),
        };
        let v: &'static str = match t.id {
            "kets_vb_limit_long" => "Limit · longs only",
            "kets_vb_limit_adx" => "Limit · ADX trend filter",
            "kets_rand_mses" => "Modified standard exit",
            "kets_rand_fixed_opt" => "Optimal fixed stop/target",
            "kets_rand_hhll_stop" => "2-bar HHLL stop",
            "kets_rand_atr_stop" => "Dynamic ATR stop",
            "kets_rand_mema_stop" => "MEMA stop",
            "kets_rand_mema_shrink" => "MEMA stop + shrinking target",
            "kets_rand_mema_shrink30" => "MEMA + shrinking target, 30 days",
            _ => ord,
        };
        return Some((f, n, v));
    }
    let f = |a, b, c| Some((a, b, c));
    match id {
        "kets_vb_stop" => f("kets_vb", "Volatility breakout", "Entry on stop · not testable"),
        "kets_vb_limit_ccy" => f("kets_vb", "Volatility breakout", "Currencies only · data missing"),
        "kets_ma_trend_xo" | "kets_ma_slope" | "kets_ma_ct_xo" | "kets_ma_sr" => f("kets_ma", "Moving-average entries (Ch.6)", "Not testable"),
        "kets_macd_sig" | "kets_rsi_div" | "kets_macd_div" => f("kets_osc_nt", "Oscillator entries, unprinted parameters", "Not testable"),
        "kets_solar" => f("kets_solar", "Sunspot breakout", "Data missing"),
        "kets_ga_long" | "kets_ga_short" => f("kets_ga", "Genetically evolved entries", "Data missing"),
        "kets_neural" | "kets_neural_exit" | "kets_ga_exit" => f("kets_ai", "Neural / genetic models", "Not testable"),
        _ => None,
    }
}

// ---------------------------------------------------------------- simulator (TRDSIM2::update, one unit)

#[derive(Default, Clone, Copy)]
struct Orders {
    bo: bool, so: bool,
    bs: Option<f64>, ss: Option<f64>, bl: Option<f64>, sl: Option<f64>,
    xls: Option<f64>, xss: Option<f64>, xll: Option<f64>, xsl: Option<f64>,
    xlc: bool, xsc: bool,
    ses: Option<(f64, f64, usize)>,
}

struct Sim<'a> { o: &'a [f64], h: &'a [f64], l: &'a [f64], c: &'a [f64], pos: i8, en_i: usize, en_px: f64, kind: &'static str, out: Vec<Trade>, ord: Orders }

impl<'a> Sim<'a> {
    fn new(x: &'a Ctx) -> Self { Sim { o: &x.o, h: &x.h, l: &x.l, c: &x.c, pos: 0, en_i: 0, en_px: 0., kind: "open", out: vec![], ord: Orders::default() } }
    fn exit(&mut self, i: usize, px: f64, reason: &str, sig: usize) {
        self.out.push(Trade { dir: self.pos as f64, legs: vec![Leg { i: self.en_i, px: self.en_px, w: 1. }], exit_i: i, exit_px: px,
            reason: reason.into(), entry_kind: self.kind, signal_i: sig, open: false, exits: vec![] });
        self.pos = 0;
    }
    fn enter(&mut self, dir: i8, i: usize, px: f64, kind: &'static str, sig: usize) {
        if self.pos != 0 { self.exit(i, px, "Reversed by an opposite entry", sig); }
        self.pos = dir; self.en_i = i; self.en_px = px; self.kind = kind;
    }
    /// Execute the orders posted on the previous bar, in TRDSIM2's order.
    fn update(&mut self, i: usize) {
        let mut o = std::mem::take(&mut self.ord);
        let (op, hi, lo, cl) = (self.o[i], self.h[i], self.l[i], self.c[i]);
        let p = i.saturating_sub(1);
        if o.bo && self.pos <= 0 { self.enter(1, i, op, "open", p); }
        if o.so && self.pos >= 0 { self.enter(-1, i, op, "open", p); }
        if let Some(px) = o.bs { if self.pos <= 0 && op > px { self.enter(1, i, op, "stop", i); o.bs = None; } }
        if let Some(px) = o.ss { if self.pos >= 0 && op < px { self.enter(-1, i, op, "stop", i); o.ss = None; } }
        if let Some(px) = o.bl { if self.pos <= 0 && op < px { self.enter(1, i, op, "limit", i); o.bl = None; } }
        if let Some(px) = o.sl { if self.pos >= 0 && op > px { self.enter(-1, i, op, "limit", i); o.sl = None; } }
        if let Some(px) = o.xls { if self.pos > 0 && op < px { self.exit(i, op, "Stop (gap through at the open)", i); o.xls = None; } }
        if let Some(px) = o.xss { if self.pos < 0 && op > px { self.exit(i, op, "Stop (gap through at the open)", i); o.xss = None; } }
        if let Some(px) = o.xll { if self.pos > 0 && op > px { self.exit(i, op, "Limit exit (gap at the open)", i); o.xll = None; } }
        if let Some(px) = o.xsl { if self.pos < 0 && op < px { self.exit(i, op, "Limit exit (gap at the open)", i); o.xsl = None; } }
        if let Some(px) = o.bs { if self.pos <= 0 && hi > px { self.enter(1, i, px, "stop", i); } }
        if let Some(px) = o.ss { if self.pos >= 0 && lo < px { self.enter(-1, i, px, "stop", i); } }
        if let Some(px) = o.bl { if self.pos <= 0 && lo < px { self.enter(1, i, px, "limit", i); } }
        if let Some(px) = o.sl { if self.pos >= 0 && hi > px { self.enter(-1, i, px, "limit", i); } }
        if let Some(px) = o.xls { if self.pos > 0 && lo < px { self.exit(i, px, "Stop", i); } }
        if let Some(px) = o.xss { if self.pos < 0 && hi > px { self.exit(i, px, "Stop", i); } }
        if let Some(px) = o.xll { if self.pos > 0 && hi > px { self.exit(i, px, "Limit exit", i); } }
        if let Some(px) = o.xsl { if self.pos < 0 && lo < px { self.exit(i, px, "Limit exit", i); } }
        if o.xlc && self.pos > 0 { self.exit(i, cl, "Time limit (close)", i); }
        if o.xsc && self.pos < 0 { self.exit(i, cl, "Time limit (close)", i); }
        if let Some((pt, sl, mh)) = o.ses {
            let d = self.pos as f64;
            if self.pos != 0 {
                if i + 1 - self.en_i >= mh { self.exit(i, cl, "SES time limit (close)", i); }
                else if d * (cl - self.en_px) < -sl { self.exit(i, cl, "SES money-management stop (close)", i); }
                else if d * (cl - self.en_px) > pt { self.exit(i, cl, "SES profit target (close)", i); }
            }
        }
    }
    fn finish(mut self, to: usize) -> Vec<Trade> {
        if self.pos != 0 {
            self.out.push(Trade { dir: self.pos as f64, legs: vec![Leg { i: self.en_i, px: self.en_px, w: 1. }], exit_i: to, exit_px: self.c[to],
                reason: "open".into(), entry_kind: self.kind, signal_i: to, open: true, exits: vec![] });
        }
        self.out
    }
}

// ---------------------------------------------------------------- indicators (TRAPSIND.C / SC1 semantics, 0-based)

fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { (x.h[i] - x.l[i]).max(x.h[i] - x.c[i - 1]).max(x.c[i - 1] - x.l[i]) }).collect()
}
/// AverageS / MovAvg type 1: mean of the last p values (partial window at the start).
fn sma(v: &[f64], p: usize) -> Vec<f64> {
    (0..v.len()).map(|i| { let lo = (i + 1).saturating_sub(p); v[lo..=i].iter().sum::<f64>() / (i + 1 - lo) as f64 }).collect()
}
/// MovAvg type 5 (centered simple) and 7 (centered triangular).
fn centered(v: &[f64], m: usize, tri: bool) -> Vec<f64> {
    let n = v.len() as i64; let mc = ((m as i64) - 1) / 2; let coeff = 0.5 * (m as f64 - 1.);
    (0..n).map(|i| {
        let (mut s, mut w) = (0., 0.);
        for j in 0..m as i64 {
            let k = i - j + mc;
            if k < 0 || k >= n { continue; }
            let wt = if tri { (1. + coeff) - (j as f64 - coeff).abs() } else { 1. };
            s += wt * v[k as usize]; w += wt;
        }
        s / w
    }).collect()
}
/// XAverageS: bias-corrected exponential average.
fn xavg(v: &[f64], p: usize) -> Vec<f64> {
    let f = 2. / (1. + p as f64); let (mut s1, mut s2) = (0., 0.);
    v.iter().map(|&x| { s1 += f * (x - s1); s2 += f * (1. - s2); s1 / s2 }).collect()
}
fn highest(v: &[f64], n: usize, cb: usize) -> f64 { v[cb + 1 - n..=cb].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lowest(v: &[f64], n: usize, cb: usize) -> f64 { v[cb + 1 - n..=cb].iter().copied().fold(f64::INFINITY, f64::min) }
/// sc1lobr / sc1hibr: ties keep the current bar, else the OLDEST bar of the extreme.
fn lowest_bar(v: &[f64], m: usize, cb: usize) -> usize {
    let (mut a, mut ans) = (v[cb], cb);
    for i in cb + 1 - m..cb { if v[i] < a { a = v[i]; ans = i; } }
    ans
}
fn highest_bar(v: &[f64], m: usize, cb: usize) -> usize {
    let (mut a, mut ans) = (v[cb], cb);
    for i in cb + 1 - m..cb { if v[i] > a { a = v[i]; ans = i; } }
    ans
}
/// StochOsc type 1 (fast %K) / 2 (slow %K = fast %D).
fn stoch(x: &Ctx, m: usize, slow: bool) -> Vec<f64> {
    let n = x.c.len();
    let (mut hh, mut ll) = (vec![0.; n], vec![0.; n]);
    for i in 0..n { let lo = (i + 1).saturating_sub(m); hh[i] = highest(&x.h, i + 1 - lo, i); ll[i] = lowest(&x.l, i + 1 - lo, i); }
    let (rg, num): (Vec<f64>, Vec<f64>) = if slow {
        (sma(&(0..n).map(|i| hh[i] - ll[i]).collect::<Vec<_>>(), 3), sma(&(0..n).map(|i| x.c[i] - ll[i]).collect::<Vec<_>>(), 3))
    } else { ((0..n).map(|i| hh[i] - ll[i]).collect(), (0..n).map(|i| x.c[i] - ll[i]).collect()) };
    let mut a = vec![50.; n];
    for i in 0..n { if rg[i] > 0. { a[i] = 100. * num[i] / rg[i]; } else if i > 0 { a[i] = a[i - 1]; } }
    a
}
/// RsiOsc type 1.
fn rsi_k(c: &[f64], m: usize) -> Vec<f64> {
    let n = c.len(); let coeff = 1. / m as f64; let mut a = vec![50.; n]; let (mut au, mut ad) = (0., 0.);
    for i in 1..n {
        let d = c[i] - c[i - 1]; let (up, dn) = if d > 0. { (d, 0.) } else if d < 0. { (0., -d) } else { (0., 0.) };
        au = coeff * ((m as f64 - 1.) * au + up); ad = coeff * ((m as f64 - 1.) * ad + dn);
        a[i] = if au + ad > 0. { 100. * au / (au + ad) } else { a[i - 1] };
    }
    a
}
/// AvgDirMov: Wilder's ADX with the CD's integer truncation.
fn adx_k(x: &Ctx, len: usize) -> Vec<f64> {
    let n = x.c.len(); let mut adx = vec![0.; n]; let tiny = 1e-10; let lf = len as f64; let coeff = (lf - 1.) / lf;
    let (mut trl, mut pl, mut ml, mut av) = (0., 0., 0., 0.);
    for i in 1..n {
        let tr = (x.h[i] - x.l[i]).max(x.h[i] - x.c[i - 1]).max(x.c[i - 1] - x.l[i]);
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        let pdm = if up > dn { up.max(0.) } else { 0. };
        let mdm = if dn > up { dn.max(0.) } else { 0. };
        trl = coeff * trl + tr; pl = coeff * pl + pdm; ml = coeff * ml + mdm;
        let pdi = (100. * pl / (trl + tiny)).floor(); let mdi = (100. * ml / (trl + tiny)).floor();
        let dx = (100. * (pdi - mdi).abs() / (pdi + mdi + tiny)).floor();
        av = (((lf - 1.) * av + dx) / lf).floor();
        adx[i] = av;
    }
    adx
}

// ---------------------------------------------------------------- calendar / moon (SC1 + Numerical Recipes)

fn ymd(s: &str) -> (i64, i64, i64) { (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0)) }
/// Julian day number of a Gregorian date (NR julday).
fn jdn(y: i64, m: i64, d: i64) -> i64 {
    let a = (14 - m) / 12; let yy = y + 4800 - a; let mm = m + 12 * a - 3;
    d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
}
/// NR flmoon: Julian day and fraction of the n-th phase nph (0 new, 1 first quarter, 2 full, 3 last quarter).
fn flmoon(n: i64, nph: i64) -> (i64, f64) {
    let rad = 3.14159265 / 180.0;
    let c = n as f64 + nph as f64 / 4.0; let t = c / 1236.85; let t2 = t * t;
    let a_s = 359.2242 + 29.105356 * c; let am = 306.0253 + 385.816918 * c + 0.010730 * t2;
    let mut jd = 2415020 + 28 * n + 7 * nph;
    let mut xtra = 0.75933 + 1.53058868 * c + ((1.178e-4) - (1.55e-7) * t) * t2;
    if nph == 0 || nph == 2 { xtra += (0.1734 - 3.93e-4 * t) * (rad * a_s).sin() - 0.4068 * (rad * am).sin(); }
    else { xtra += (0.1721 - 4.0e-4 * t) * (rad * a_s).sin() - 0.6280 * (rad * am).sin(); }
    let i = if xtra >= 0. { xtra.floor() } else { (xtra - 1.).ceil() } as i64;
    jd += i;
    (jd, xtra - i as f64)
}
/// LunarEventDates: civil (US Eastern) date, as a day number, of quarter-phase event n.
fn lunar_event(n: i64) -> i64 {
    let (mut nd, frac) = flmoon(n.div_euclid(4), n.rem_euclid(4));
    let mut f = 24.0 * (frac - 5.0 / 24.0);
    if f < 0. { nd -= 1; f += 24.; }
    if f > 12. { nd += 1; }
    nd
}
/// LunarEquivDate: the day with the same offset from the same quarter phase k lunations away.
fn lunar_equiv(cur: i64, k: i64) -> i64 {
    let mut ns = ((cur as f64 - 2415020.76) / 7.38265).floor() as i64;
    while lunar_event(ns + 1) <= cur { ns += 1; }
    while lunar_event(ns) > cur { ns -= 1; }
    lunar_event(ns + 4 * k) + (cur - lunar_event(ns))
}

/// pchg = clipped ATR-normalised close change (x12mod01.c / x13mod01.c).
fn pchg(x: &Ctx, atr50: &[f64]) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { 0. } else { ((x.c[i] - x.c[i - 1]) / atr50[i]).clamp(-2., 2.) }).collect()
}
/// SeasonalAvg, all-earlier-years form: mean of v on the same calendar date in each earlier year (exact date match).
fn seasonal_avg(x: &Ctx, v: &[f64]) -> Vec<f64> {
    let key = |s: &str| { let (y, m, d) = ymd(s); y * 10000 + m * 100 + d };
    let keys: Vec<i64> = x.r.iter().map(|r| key(&r.date)).collect();
    let idx: HashMap<i64, usize> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
    let first = keys.get(2).copied().unwrap_or(i64::MAX);
    (0..keys.len()).map(|i| {
        let (mut s, mut cnt) = (0., 0.);
        for j in 1..100 {
            let sd = keys[i] - 10000 * j;
            if sd < first { break; }
            if let Some(&k) = idx.get(&sd) { cnt += 1.; s += v[k]; }
        }
        s / (cnt + 1e-20)
    }).collect()
}
/// LunarAvg mode 2 (60 cases), earlier cycles only: bar at or after the equivalent date 2, 3, ... lunations back.
fn lunar_avg(x: &Ctx, v: &[f64]) -> Vec<f64> {
    let days: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); jdn(y, m, d) }).collect();
    let first = days.get(2).copied().unwrap_or(i64::MAX);
    (0..days.len()).map(|i| {
        let (mut s, mut cnt) = (0., 0usize);
        for j in 2..1000 {
            if cnt >= 60 { break; }
            let sd = lunar_equiv(days[i], -j);
            if sd < first { break; }
            let k = days.partition_point(|&d| d < sd);
            cnt += 1; s += v[k];
        }
        s / (cnt as f64 + 1e-20)
    }).collect()
}

// ---------------------------------------------------------------- wavelet filter bank (WAVFILT.C)

fn wav_w(k: usize, per: f64, wid: f64) -> (f64, f64) {
    let w = 2.0 * std::f64::consts::PI * (k as f64 - 1.) / per; let t = wid * w; let t = (-t * t).exp();
    if t > 0.01 { (t, w) } else { (0., w) }
}
fn sprod(a: &[f64], b: &[f64]) -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() }
fn wav_remove(a: &mut [f64], b: &[f64], bp: &[f64], c: &[f64], cp: &[f64]) {
    let (ab, ac, bb, bc, cb, cc) = (sprod(a, bp), sprod(a, cp), sprod(b, bp), sprod(b, cp), sprod(c, bp), sprod(c, cp));
    let den = cb * bc - bb * cc; let bmul = (cc * ab - cb * ac) / den; let cmul = (bb * ac - ab * bc) / den;
    for i in 0..a.len() { a[i] += bmul * b[i] + cmul * c[i]; }
}
/// Returns (in-phase kernel, quadrature kernel), index 0 = the current bar.
fn build_kernel(per: f64, wid: f64) -> (Vec<f64>, Vec<f64>) {
    let m = (80.0 / wid) as usize;
    let mut ip = vec![0.; m]; let mut iq = vec![0.; m]; let mut off = vec![0.; m]; let mut tr = vec![0.; m];
    let ro = vec![1.; m]; let rt: Vec<f64> = (1..=m).map(|i| i as f64).collect();
    for i in 1..=m {
        let (g, w) = wav_w(i, per, wid); ip[i - 1] = g * w.cos(); iq[i - 1] = g * w.sin();
        let (g2, _) = wav_w(i, per, 2. * wid); off[i - 1] = g2; tr[i - 1] = g2 * i as f64;
    }
    wav_remove(&mut ip, &off, &ro, &tr, &rt);
    wav_remove(&mut iq, &off, &ro, &tr, &rt);
    let s = 1. / sprod(&ip, &ip).sqrt(); for v in ip.iter_mut() { *v *= s; }
    let s = sprod(&ip, &iq); for i in 0..m { iq[i] -= s * ip[i]; }
    let s = 1. / sprod(&iq, &iq).sqrt(); for v in iq.iter_mut() { *v *= s; }
    let mut nk = m;
    while nk > 1 && ip[nk - 1] == 0. && iq[nk - 1] == 0. { nk -= 1; }
    ip.truncate(nk); iq.truncate(nk);
    (ip, iq)
}
/// Filter-bank outputs: (periods, in-phase[k][i], quadrature[k][i], power[k][i]).
fn filter_bank(c: &[f64], width: f64) -> (Vec<f64>, Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = c.len(); let fcount = 20; let ratio = (30f64 / 3.).ln() / (fcount as f64 - 1.); let ratio = ratio.exp();
    let mut per = 3.0; let mut pers = vec![]; let (mut ips, mut iqs, mut pws) = (vec![], vec![], vec![]);
    for _ in 0..fcount {
        let (kp, kq) = build_kernel(per, width); let nk = kp.len();
        let (mut ip, mut iq, mut pw) = (vec![0.; n], vec![0.; n], vec![0.; n]);
        // 1-based: k = nkern+2 .. n  ->  0-based i = nk+1 .. n-1
        for i in nk + 1..n {
            let (mut a, mut b) = (0., 0.);
            for j in 0..nk { a += c[i - j] * kp[j]; b += c[i - j] * kq[j]; }
            ip[i] = a; iq[i] = b;
        }
        for i in 0..n { pw[i] = (ip[i] * ip[i] + iq[i] * iq[i]) / per; }
        pers.push(per); ips.push(ip); iqs.push(iq); pws.push(pw);
        per *= ratio;
    }
    (pers, ips, iqs, pws)
}

// ---------------------------------------------------------------- Numerical Recipes ran2 (TRAPSLIB.C nrc_ran2)

struct Ran2 { idum: i64, idum2: i64, iy: i64, iv: [i64; 32] }
impl Ran2 {
    const IM1: i64 = 2147483563; const IM2: i64 = 2147483399; const IA1: i64 = 40014; const IA2: i64 = 40692;
    const IQ1: i64 = 53668; const IQ2: i64 = 52774; const IR1: i64 = 12211; const IR2: i64 = 3791;
    fn new(seed: i64) -> Self { Ran2 { idum: seed, idum2: 123456789, iy: 0, iv: [0; 32] } }
    fn next(&mut self) -> f64 {
        let ndiv = 1 + (Self::IM1 - 1) / 32;
        if self.idum <= 0 {
            self.idum = if -self.idum < 1 { 1 } else { -self.idum };
            self.idum2 = self.idum;
            for j in (0..32 + 8).rev() {
                let k = self.idum / Self::IQ1;
                self.idum = Self::IA1 * (self.idum - k * Self::IQ1) - k * Self::IR1;
                if self.idum < 0 { self.idum += Self::IM1; }
                if j < 32 { self.iv[j] = self.idum; }
            }
            self.iy = self.iv[0];
        }
        let k = self.idum / Self::IQ1;
        self.idum = Self::IA1 * (self.idum - k * Self::IQ1) - k * Self::IR1;
        if self.idum < 0 { self.idum += Self::IM1; }
        let k = self.idum2 / Self::IQ2;
        self.idum2 = Self::IA2 * (self.idum2 - k * Self::IQ2) - k * Self::IR2;
        if self.idum2 < 0 { self.idum2 += Self::IM2; }
        let j = (self.iy / ndiv) as usize;
        self.iy = self.iv[j] - self.idum2;
        self.iv[j] = self.idum;
        if self.iy < 1 { self.iy += Self::IM1 - 1; }
        let temp = ((1.0 / Self::IM1 as f64) * self.iy as f64) as f32;
        let rnmx = 1.0 - 1.2e-7;
        if temp as f64 > rnmx { rnmx as f32 as f64 } else { temp as f64 }
    }
}

// ---------------------------------------------------------------- drivers

/// Generic entry-model loop (x09..x14 / x19mod01): update, post one entry order from the model's signal, post the SES.
/// `sig(cb)` returns (signal, stop price used for a stop entry).
fn drive(x: &Ctx, from: usize, to: usize, warm: usize, last30: bool, ord: Od, long_only: bool, ses: (f64, f64, usize), atr50: &[f64],
         mut sig: impl FnMut(usize, i8) -> (i8, f64)) -> Vec<Trade> {
    let n = x.c.len();
    let mut s = Sim::new(x);
    for cb in from..=to {
        s.update(cb);
        if cb >= to { break; }
        if cb < warm { continue; }
        if last30 && cb + 30 >= n { continue; }
        if x.h[cb + 1] == x.l[cb + 1] { continue; }
        let (sg, stp) = sig(cb, s.pos);
        let lim = 0.5 * (x.h[cb] + x.l[cb]);
        if s.pos <= 0 && sg == 1 {
            match ord { Od::Open => s.ord.bo = true, Od::Limit => s.ord.bl = Some(lim), Od::Stop => s.ord.bs = Some(stp) }
        } else if s.pos >= 0 && sg == -1 {
            if long_only { s.ord.xll = Some(lim); }
            else { match ord { Od::Open => s.ord.so = true, Od::Limit => s.ord.sl = Some(lim), Od::Stop => s.ord.ss = Some(stp) } }
        }
        s.ord.ses = Some((ses.0 * atr50[cb], ses.1 * atr50[cb], ses.2));
    }
    s.finish(to)
}

/// Random entries at the open with intraday exits (x19mod02.c, x20mod01-03.c).
#[allow(clippy::too_many_arguments)]
fn drive_mses(x: &Ctx, from: usize, to: usize, model: u8, atr50: &[f64], seed: i64, pt: f64, sl: f64, later: f64, coeff: f64, ptga: f64, hold: usize) -> Vec<Trade> {
    let mut s = Sim::new(x);
    let mut rng = Ran2::new(seed); rng.next();
    let (mut lim, mut stp, mut entrybar) = (0., 0., 0usize);
    for cb in from..=to {
        s.update(cb);
        if cb >= to { break; }
        if cb < WARM { continue; }
        if x.h[cb + 1] == x.l[cb + 1] { continue; }
        let r = rng.next();
        let sg: i8 = if r < 0.025 { -1 } else if r > 0.975 { 1 } else { 0 };
        let mut posted = 0i8; let mut ep = 0.;
        if s.pos <= 0 && sg == 1 { s.ord.bo = true; posted = 1; ep = x.o[cb + 1]; entrybar = cb + 1; }
        else if s.pos >= 0 && sg == -1 { s.ord.so = true; posted = -1; ep = x.o[cb + 1]; entrybar = cb + 1; }
        let atr = atr50[cb];
        if posted != 0 {
            let d = posted as f64;
            lim = ep + d * pt * atr;
            stp = ep - d * sl * atr;
            if model == 1 { stp = if posted > 0 { lowest(&x.l, 2, cb).min(stp) } else { highest(&x.h, 2, cb).max(stp) }; }
            if posted > 0 { s.ord.xll = Some(lim); s.ord.xls = Some(stp); } else { s.ord.xsl = Some(lim); s.ord.xss = Some(stp); }
        } else if s.pos > 0 {
            match model {
                1 => stp = stp.max(lowest(&x.l, 2, cb)),
                2 => stp = stp.max(x.c[cb] - later * atr),
                3 | 4 => { let t = (x.h[cb] - later * atr) - stp; if t > 0. { stp += coeff * t; } if model == 4 { lim -= ptga * (lim - x.c[cb]); } }
                _ => {}
            }
            s.ord.xll = Some(lim); s.ord.xls = Some(stp);
            if cb - entrybar >= hold { s.ord.xlc = true; }
        } else if s.pos < 0 {
            match model {
                1 => stp = stp.min(highest(&x.h, 2, cb)),
                2 => stp = stp.min(x.c[cb] + later * atr),
                3 | 4 => { let t = (x.l[cb] + later * atr) - stp; if t < 0. { stp += coeff * t; } if model == 4 { lim += ptga * (x.c[cb] - lim); } }
                _ => {}
            }
            s.ord.xsl = Some(lim); s.ord.xss = Some(stp);
            if cb - entrybar >= hold { s.ord.xsc = true; }
        }
    }
    s.finish(to)
}

fn cross_up(a: &[f64], b: &[f64], c: usize) -> bool { a[c] >= b[c] && a[c - 1] < b[c - 1] }
fn cross_dn(a: &[f64], b: &[f64], c: usize) -> bool { a[c] < b[c] && a[c - 1] >= b[c - 1] }
fn turns_up(a: &[f64], c: usize) -> bool { a[c] >= a[c - 1] && a[c - 1] < a[c - 2] }
fn turns_dn(a: &[f64], c: usize) -> bool { a[c] < a[c - 1] && a[c - 1] >= a[c - 2] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let t = TESTS.iter().find(|t| t.id == id)?;
    let n = x.c.len();
    if n < 3 || from > to || to >= n { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or_else(|| t.defs.iter().find(|d| d.0 == k).map_or(0., |d| d.2));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let atr50 = sma(&true_range(x), 50);
    let ses = (g("pt"), g("sl"), u("hold"));
    let std_stop = |cb: usize, s: i8| x.c[cb] + 0.5 * s as f64 * atr50[cb];
    let out = match t.m {
        M::Chan { hl } => {
            let nn = u("n").min(n - 1);
            let (hs, ls): (&[f64], &[f64]) = if hl { (&x.h, &x.l) } else { (&x.c, &x.c) };
            drive(x, from, to, WARM.max(nn + 1), false, t.ord, false, ses, &atr50, |cb, _| {
                let s = if x.c[cb] > highest(hs, nn, cb - 1) { 1 } else if x.c[cb] < lowest(ls, nn, cb - 1) { -1 } else { 0 };
                (s, std_stop(cb, s))
            })
        }
        M::HhllStop => {
            let nn = u("n").min(n - 1);
            drive(x, from, to, WARM.max(nn + 1), false, t.ord, false, ses, &atr50, |cb, _| {
                let (hh, ll) = (highest(&x.h, nn, cb), lowest(&x.l, nn, cb));
                let pr = (x.c[cb] - ll) / (hh - ll);
                if pr >= 0.5 { (1, hh) } else if pr < 0.5 { (-1, ll) } else { (0, 0.) } // hh == ll: NaN, no order
            })
        }
        M::Vb { long_only, adx } => {
            let (bw, ma, al) = (g("bw"), u("ma"), u("atr"));
            let atr = sma(&true_range(x), al); let ema = xavg(&x.c, ma);
            let ax = if adx { adx_k(x, u("adx")) } else { vec![] }; let ah = u("adxhi");
            drive(x, from, to, WARM.max(al + 2).max(ah + 2), false, t.ord, long_only, ses, &atr50, |cb, _| {
                let (up, dn) = (ema[cb - 1] + bw * atr[cb - 1], ema[cb - 1] - bw * atr[cb - 1]);
                let tr = !adx || ax[cb] > highest(&ax, ah, cb - 1);
                let s = if tr && x.c[cb] > up { 1 } else if tr && x.c[cb] < dn { -1 } else { 0 };
                (s, std_stop(cb, s))
            })
        }
        M::StochObos | M::RsiObos | M::StochSig => {
            let (osc, sig, lo, hi) = match t.m {
                M::StochObos => (stoch(x, u("len"), false), vec![], 20., 80.),
                M::RsiObos => (rsi_k(&x.c, u("len")), vec![], 30., 70.),
                _ => { let o = stoch(x, u("len"), true); let s = sma(&o, 3); (o, s, 0., 0.) }
            };
            let lov = vec![lo; n]; let hiv = vec![hi; n];
            let obos = t.m != M::StochSig;
            drive(x, from, to, WARM, false, t.ord, false, ses, &atr50, |cb, _| {
                let s = if obos { if cross_up(&osc, &lov, cb) { 1 } else if cross_dn(&osc, &hiv, cb) { -1 } else { 0 } }
                    else if cross_up(&osc, &sig, cb) { 1 } else if cross_dn(&osc, &sig, cb) { -1 } else { 0 };
                (s, std_stop(cb, s))
            })
        }
        M::StochDiv => {
            let osc = stoch(x, u("len"), false); let lb = u("lb").max(2);
            drive(x, from, to, WARM.max(lb + 1), false, t.ord, false, ses, &atr50, |cb, _| {
                let lim = cb as i64 - lb as i64 + 1;
                let (i, j) = (lowest_bar(&x.c, lb, cb) as i64, lowest_bar(&osc, lb, cb) as i64);
                let c = cb as i64;
                let mut s = 0;
                if i < c && i > c - 6 && j > lim && i - j > 4 && turns_up(&osc, cb) { s = 1; }
                else {
                    let (i, j) = (highest_bar(&x.c, lb, cb) as i64, highest_bar(&osc, lb, cb) as i64);
                    if i < c && i > c - 6 && j > lim && i - j > 4 && turns_dn(&osc, cb) { s = -1; }
                }
                (s, std_stop(cb, s))
            })
        }
        M::Seas(k) | M::Lunar(k) => {
            let pc = pchg(x, &atr50);
            let mut sv = if matches!(t.m, M::Seas(_)) { seasonal_avg(x, &pc) } else { lunar_avg(x, &pc) };
            let (al, disp) = (u("avglen"), g("disp").round().max(0.) as usize);
            let stc = stoch(x, 9, false);
            let (ma1, ma2) = if k == 1 {
                sv = centered(&sv, al, g("matype").round() as i64 == 7);
                let ab: Vec<f64> = sv.iter().map(|v| v.abs()).collect();
                (sma(&ab, 100), vec![])
            } else {
                for i in 1..n { sv[i] += sv[i - 1]; }
                let m1 = sma(&sv, al); let m2 = sma(&m1, al); (m1, m2)
            };
            let th = g("thresh");
            drive(x, from, to, WARM, true, t.ord, false, ses, &atr50, |cb, _| {
                let kk = cb + disp;
                let s = match k {
                    1 => { let tmp = th * ma1[kk]; if sv[kk] > tmp && sv[kk - 1] <= tmp { 1 } else if sv[kk] < -tmp && sv[kk - 1] >= -tmp { -1 } else { 0 } }
                    2 => if cross_up(&ma1, &ma2, kk) { 1 } else if cross_dn(&ma1, &ma2, kk) { -1 } else { 0 },
                    3 => if cross_up(&ma1, &ma2, kk) { if stc[cb] < 25. { 1 } else { 0 } } else if cross_dn(&ma1, &ma2, kk) { if stc[cb] > 75. { -1 } else { 0 } } else { 0 },
                    _ => if cross_up(&ma1, &ma2, kk) { if stc[cb] < 25. { 1 } else if stc[cb] > 75. { -1 } else { 0 } }
                         else if cross_dn(&ma1, &ma2, kk) { if stc[cb] > 75. { -1 } else if stc[cb] < 25. { 1 } else { 0 } } else { 0 },
                };
                (s, std_stop(cb, s))
            })
        }
        M::Cycle => {
            let (pers, ip, iq, pw) = filter_bank(&x.c, g("width"));
            let pd = g("pdisp"); let fc = pers.len();
            drive(x, from, to, WARM, true, t.ord, false, ses, &atr50, |cb, _| {
                let (mut dom, mut peak) = (0usize, -1.0);
                for k in 0..fc { if pw[k][cb] > peak { peak = pw[k][cb]; dom = k + 1; } }
                let mut s = 0;
                if dom > 3 && dom < fc - 1 {
                    let mut noise = 0.0;
                    for k in 1..=fc { if (k as i64 - dom as i64).abs() > 2 && pw[k - 1][cb] > noise { noise = pw[k - 1][cb]; } }
                    if peak > 1.5 * noise {
                        let d = dom - 1;
                        let ph = iq[d][cb].atan2(ip[d][cb]).to_degrees(); let oph = iq[d][cb - 1].atan2(ip[d][cb - 1]).to_degrees();
                        let phb = if ph < 0. { 360. + ph } else { ph }; let ophb = if oph < 0. { 360. + oph } else { oph };
                        let sell = 0.0 - (pd + 180.0 / pers[d]); let buy = 180.0 + sell;
                        if phb > buy && ophb <= buy { s = 1; }
                        if ph > sell && oph <= sell { s = -1; }
                    }
                }
                (s, std_stop(cb, s))
            })
        }
        M::RandSes => {
            let mut rng = Ran2::new(-(g("seed").round() as i64 + 10 * g("mkt").round() as i64)); rng.next();
            drive(x, from, to, WARM, false, t.ord, false, ses, &atr50, |cb, _| {
                let r = rng.next(); let s = if r < 0.025 { -1 } else if r > 0.975 { 1 } else { 0 };
                (s, std_stop(cb, s))
            })
        }
        M::RandMses(k) => drive_mses(x, from, to, k, &atr50, -(g("seed").round() as i64 + 10 * g("mkt").round() as i64),
            g("pt"), g("sl"), g("later"), g("coeff"), g("ptga"), u("hold")),
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25),
            open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn ses_is_close_only_and_counts_the_entry_bar() {
        // flat 100 with range 2 -> ATR50 = 2; long entered at the open of bar 1 is closed at the close of bar 10 (10 bars held)
        let r = rows(&vec![100.; 40]); let x = Ctx::new(&r, &r, &[]);
        let mut s = Sim::new(&x); s.ord.bo = true;
        for i in 1..=12 { s.update(i); s.ord.ses = Some((8., 2., 10)); }
        let t = s.finish(12); assert_eq!(t.len(), 1); assert_eq!(t[0].legs[0].i, 1); assert_eq!(t[0].exit_i, 10);
        assert_eq!(t[0].reason, "SES time limit (close)");
        // a low far below the stop does not stop the trade out unless the CLOSE is beyond it
        let mut r = rows(&vec![100.; 40]); r[3].low = 50.; let x = Ctx::new(&r, &r, &[]);
        let mut s = Sim::new(&x); s.ord.bo = true;
        for i in 1..=5 { s.update(i); s.ord.ses = Some((8., 2., 10)); }
        assert_eq!(s.pos, 1);
    }
    #[test]
    fn limit_and_stop_fill_like_trdsim() {
        let mut r = rows(&vec![100.; 10]); r[2].open = 98.; r[2].low = 97.; let x = Ctx::new(&r, &r, &[]);
        let mut s = Sim::new(&x); s.ord.bl = Some(99.); s.update(2); assert_eq!(s.en_px, 98.); // gap below the limit fills at the open
        let mut s = Sim::new(&x); s.ord.bl = Some(97.); s.update(2); assert_eq!(s.pos, 0); // low == limit: no fill (strict)
        let mut s = Sim::new(&x); s.ord.bs = Some(100.5); s.update(3); assert_eq!(s.en_px, 100.5);
    }
    #[test]
    fn moon_phases_match_known_dates() {
        // full moon 1999-01-01/02 (02:50 UT Jan 2 -> Jan 1 US Eastern); new moon 2000-01-06 18:14 UT
        let fm = (4 * 1215..4 * 1235).map(lunar_event).collect::<Vec<_>>();
        assert!(fm.contains(&jdn(1999, 1, 1)) || fm.contains(&jdn(1999, 1, 2)));
        let nm: Vec<i64> = (1230..1245).map(|n| lunar_event(4 * n)).collect();
        assert!(nm.contains(&jdn(2000, 1, 6)));
    }
    #[test]
    fn ran2_is_deterministic_and_uniform() {
        let mut a = Ran2::new(-11); let mut b = Ran2::new(-11);
        let va: Vec<f64> = (0..1000).map(|_| a.next()).collect(); let vb: Vec<f64> = (0..1000).map(|_| b.next()).collect();
        assert_eq!(va, vb); assert!(va.iter().all(|&v| v > 0. && v < 1.));
        let m = va.iter().sum::<f64>() / 1000.; assert!((m - 0.5).abs() < 0.05);
    }
}
