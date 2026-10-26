//! Mark Boucher, "The Science of Trading" (seminar workbook, Investment Research Associates, c.1995).
//! Book id "mbsot". Library notes: library/mbsot/README.md. Page refs are the workbook's own page labels
//! ("Timing Models - 19", pattern section "3-20", GTI section "4-18") plus the PDF page of the scan.
//! Live here: price/breadth timing models (Zweig 4% momentum, Renshaw thrust, McClellan system, Appel impulse /
//! McClellan / combo) and the pattern-recognition setups whose page gives a complete exit (SD:5 half-exit, break-even
//! and GTI trailing stop). Everything else is NeedsData (macro, sentiment, new highs/lows, futures, stock universe) or
//! NotTestable (no exit, chart judgement). The GTI (Gann Trend Indicator, revised two-day version, p.4-1..4-18) and
//! Lane's slow %D (SD:5, p.1-22) are implemented locally below.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mbsot";
/// Admitted ids (authors/boucher/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mbsot_zweig4", "mbsot_renshaw_thrust", "mbsot_renshaw_thrust_nse", "mbsot_appel_impulse", "mbsot_appel_impulse_nse", "mbsot_appel_mcclellan", "mbsot_appel_mcclellan_nse", "mbsot_appel_combo", "mbsot_appel_combo_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NYSE_AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];
const NYSE_ADU: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged"];
const NSE_ADU: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.Unchanged"];

// ------------------------------------------------------------------ rules text

const ZWEIG4: &[&str] = &[
    "E|\"Simply keep track of weekly Value Line cash closes. When a close drops 4% from a peak close, a sell signal is given. Conversely when a close is 4% or > higher than a trough weekly close, a buy signal is given.\" (Timing Models - 19, pdf p.158)",
    "E|\"Buy Signal: weekly Value Line Composite Index close rallies >= 4% from any weekly close\" / \"Sell Signal: weekly Value Line Composite Index close drops >= 4% from any weekly peak\" (Timing Models - 25, pdf p.164)",
    "X|Sell signal = exit to cash: \"you would have only been invested 32% of the time\" (p.25) - a stop-and-reverse would always be invested. p.short=1 also sells short on sell signals (not the book's default).",
    "F|Weekly close = the last daily bar of each calendar week (Mon-Sun); orders at that close. Trough / peak = lowest / highest weekly close since the last signal (start: the first weekly close of the window).",
];
const RENSHAW: &[&str] = &[
    "E|\"A Renshaw thrust occurs when the S&P rises by 2.75% in one day and 70% of issues traded advance on the NYSE.\" (Timing Models - 19, pdf p.158) Buy on that day's close. Issues traded = advances + declines + unchanged.",
    "X|\"Using a 10% ops and a 10% trailing stop\" (p.19): sell stop 10% below the entry close, raised to 10% below the highest close since entry [Book silent: trailing from closes, stop filled intraday at the stop or at the open if it gaps through; p.trail_hi=1 trails from highs].",
];
const MCCLELLAN: &[&str] = &[
    "F|\"This is the difference between the 19 day EMA and the 39 day EMA of daily (advances - declines).\" (Timing Models - 20, pdf p.159) EMA smoothing 2/(n+1) (= McClellan's 10% and 5%), started from the first breadth day [Book silent].",
    "E|\"buy when it crosses from below 0 to above 0\" (p.20). Buy on that close.",
    "X|\"exit when the two day m.a. of the McClellan Osc drops to < 0 from above\" (p.20). Sell on that close.",
];
const APPEL_IMP: &[&str] = &[
    "F|\"Compute a 6, 8, and 15 day EMA of (advances - declines).\" (Timing Models - 21, pdf p.160)",
    "E|\"Buy on a 15 day EMA reading +300 or >, on a 8 day EMA reading of +400 or >, or on a 6 day EMA reading of +500 or >.\" (p.21) Buy on that close when flat.",
    "X|\"Exit on any NYSE close that is down 1.5% or > from the highest close since trade initiation.\" (p.21) Applied to the traded index's close (instrument swap for the NYSE Composite).",
];
const APPEL_MCC: &[&str] = &[
    "F|\"Appel runs the same 19 and 39 day EMA's as an oscillator on the cumulative advance-decline line instead of on daily (advances - declines)\" (Timing Models - 21, pdf p.160)",
    "E|\"Buy when the osc > +1000.\" (p.21) Buy on that close when flat.",
    "X|\"Exit when the osc falls to below +450 and has a close of > 3/4% from the highest close since entry.\" (p.21) Both on the same close; the close is the traded index's.",
];
const APPEL_COMBO: &[&str] = &[
    "E|Entry = Appel's Impulse Signal (6/8/15-day EMA of advances - declines >= +500/+400/+300) (Timing Models - 21-22)",
    "X|\"If an Appel's Impulse Signal is followed by an Appel's McClellan Osc signal before its exit method is triggered, wait for the Appel's McClellan Osc exit method to occur before exiting. Otherwise use normal Appel Impulse Signal exit to exit all Appel Impulse Signal Buys.\" (Timing Models - 22, pdf p.161) McClellan signal = oscillator of the cumulative A-D line > +1000 on any close from the entry bar on.",
];
const CGC_UP: &[&str] = &[
    "F|Close Gap CPR up (Compu-Trac study CGCUP, p.3-15): LOW[-1] < LOW[-2]; LOW > CLOSE[-1]; CLOSE[-1] < CLOSE[-2]; CLOSE > HIGH[-1] (day 2 = swing low day, today = day 3)",
    "F|\"SD:5 was below 30 on the swing low day\" and \"We look for a GTI up trend to be established\" (p.3-20). SD:5 = Lane's slow %D (p.1-22); GTI = revised two-day Gann Trend Indicator (p.4-18), trend as of the CGC close.",
    "E|\"Buy two nearby futures on the next day's open with an ops below the swing low.\" (p.3-20) Stop one tick below day 2's low [Book silent on the offset].",
    "X|\"Take profit on 1/2 the position on the open following an sd:5 reading of 70 or more.\" (p.3-20)",
    "X|\"Take profit on the rest of the position using a break even ops after the first position is closed, and a GTI trailing ops thereafter.\" (p.3-20) GTI trailing: \"as soon as a GTI bottom is registered that is higher than our initial ops we move our stop to one point below the GTI bottom\" (p.4-18); bottoms registered after entry, effective the next day.",
];
const CGC_DN: &[&str] = &[
    "F|Close Gap CPR down (study CGCDN, p.3-15): HIGH[-1] > HIGH[-2]; HIGH < CLOSE[-1]; CLOSE < LOW[-1]; CLOSE[-1] > CLOSE[-2]",
    "F|SD:5 above 70 on the swing high day (day 2) and GTI trend down (p.3-21)",
    "E|\"Sell nearby futures short with ops above the swing high.\" (p.3-21) Sell at the next open; stop one tick above day 2's high.",
    "X|\"Exit 1/2 the position on the open following an sd reading below 30. Use a break even and then GTI trailing ops on the remaining position thereafter.\" (p.3-21)",
];
const RGP_UP: &[&str] = &[
    "F|Reversal gap up (study RGAPLO, p.3-32): CLOSE[-1] < CLOSE[-2]; LOW > HIGH[-1]; CLOSE > CLOSE[-2]; CLOSE > CLOSE[-3]; CLOSE > (HIGH - LOW)/2 + LOW",
    "F|\"A simple RGP or reversal gap up occurs while the prior day's SD:5 reading was below 30.\" (p.3-36)",
    "E|\"We buy two units of futures on the following open with our ops below the last swing low.\" (p.3-36) Last swing low = last confirmed pivot low of the swing chart (p.3-86); stop one tick below it.",
    "X|\"We exit 1/2 the position on the opening following the first day that has an SD:5 reading above 70, and use a break even ops on the rest of the trade, and a GTI trailing stop.\" (p.3-36)",
];
const RGP_DN: &[&str] = &[
    "F|Reversal gap down (study RGAPHI, p.3-32): CLOSE[-1] > CLOSE[-2]; HIGH < LOW[-1]; CLOSE < CLOSE[-2]; CLOSE < CLOSE[-3]; CLOSE < (HIGH - LOW)/2 + LOW",
    "F|\"A simple RGP or reversal gap down occurs while the prior day's SD:5 reading was above 70.\" (p.3-37)",
    "E|\"Sell two units of futures on the open with an ops above the last swing high.\" (p.3-37) Stop one tick above the last confirmed pivot high.",
    "X|\"Exit 1/2 the position on the open following a daily SD:5 reading below 30, and use a break even ops on the remainder of the position thereafter, and a GTI trailing stop to take profits on the last 1/2 of the trade.\" (p.3-37)",
];
const RGP_GTI_UP: &[&str] = &[
    "F|Reversal gap up (study RGAPLO, p.3-32) \"while the prior day's SD:5 reading was below 30 and the GTI trend is up at the time of the signal\" (p.3-39)",
    "E|\"Buy stock, option, or futures on the next day's open with an ops below the last swing low.\" (p.3-39) Stop one tick below the last confirmed pivot low.",
    "X|\"Exit on trailing stop technique, or on some other sell signal.\" (p.3-39) Coded: the GTI trailing stop on the whole position (the \"other sell signal\" is the trader's choice and is not coded).",
];
const RGP_GTI_DN: &[&str] = &[
    "F|Reversal gap down (study RGAPHI, p.3-32) while the prior day's SD:5 was above 70 and the GTI trend is down (p.3-40)",
    "E|Sell short on the next day's open with an ops one tick above the last swing high (p.3-40)",
    "X|Exit on the GTI trailing stop technique (p.3-40, as p.3-39)",
];
const OUT_UP: &[&str] = &[
    "F|Outside day up (formula set OUTDY_SD, p.3-48): HIGH > HIGH[-1]; LOW < LOW[-1]; CLOSE > CLOSE[-1]",
    "F|SD divergence with lows, \"(LN10-L)Y1I2+([SD]-[SD]N10)Y1I2 = 2\" (p.3-48): on the day before the outside day the low is below the lowest low of the 10 days before it while SD:5 is above its lowest value of those 10 days [Reading of the Compu-Trac code: Y1 = one day back, N10 = the 10 prior days]",
    "F|\"within the confines of a GTI clear up trend\" (p.3-50): GTI trend up on the outside day's close",
    "E|\"Buy futures or options when the high of the outside day is penetrated on the upside with an ops one point below the low of the outside day.\" (p.3-50); code: \"BUY STP 1 TICK>HI\" for the next day (p.3-48)",
    "X|\"Exit when SD 5 > 70 one half the position, the other half exit on GTI trailing ops technique.\" (p.3-50) Half at that day's close [Book silent on the fill; the inside-day page says \"on close\"]; GTI trailing on the rest from entry.",
];
const OUT_DN: &[&str] = &[
    "F|Outside day down (OUTDY_SD, p.3-48): HIGH > HIGH[-1]; LOW < LOW[-1]; CLOSE < CLOSE[-1]; SD divergence with highs one day back \"(H-HM10)Y1I2+([SD]M10-[SD])Y1I2 = 2\"; GTI trend down (p.3-51)",
    "E|\"Sell on a stop below the low of the outside day with an ops above the outside day's high.\" (p.3-51); code \"SEL STP 1 TICK<LO\"; ops one point above the high (as p.3-50)",
    "X|\"Exit 1/2 the position on SD:5 < 30, the other 1/2 on GTI trailing ops technique.\" (p.3-51)",
];
const IN_UP: &[&str] = &[
    "F|Inside day (formula set IN_DY_SD, p.3-58): HIGH < HIGH[-1]; LOW > LOW[-1]; and SD divergence with lows on the prior (outside) day \"(LN10-L)Y1I2+([SD]-[SD]N10)Y1I2 = 2\" [Reading: the prior day's low below the 10 days before it, SD:5 above its 10-day low]",
    "E|\"Buy two units on stop one tick above high of outside day with ops one point below low of outside day.\" (p.3-61); code \"BUY STP 1 TICK>PRIOR DY HI\" for the next day (p.3-58)",
    "X|\"Exit 1/2 position when SD 5 > 70 on close. Use GTI trailing ops on other 1/2 position.\" (p.3-61)",
];
const IN_DN: &[&str] = &[
    "F|Inside day (IN_DY_SD, p.3-58) with SD divergence with highs on the prior (outside) day",
    "E|\"Sell short two units on a stop one tick below the outside day low with an ops one point above the outside day's high.\" (p.3-62)",
    "X|\"Take 1/2 profits when SD:5 falls below 30 on the close. Use GTI trailing ops technique to exit other 1/2 of position\" (p.3-62)",
];
const GTI_NOTE: &str = "GTI (revised two-day Gann Trend Indicator, p.4-1..4-18), as coded: two closes in a row against the trend start a counter-move; in an up trend the counter-move's lowest low is registered as a GTI bottom once the price makes a new high above the leg's high by \"one point (2%)\" (p.4-18; p.gti_pct), and the trend turns down when the last GTI bottom is penetrated by the same amount (mirror for down trends; a trend change registers the old leg's extreme). The first two-day move starts the GTI. Stops: \"one point\" = 1 price unit (p.pt), one tick = 0.01 (0.05 for NSE indices). Same-bar entry and stop: stop assumed hit. The book's statistics count patterns reaching an objective / 3x risk, not this exact exit, so there is no trade list to reproduce. Book: stocks and futures; run on the indices (vault convention).";

// ------------------------------------------------------------------ catalogue

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim,
    };
    let nd = |id, name, group, why, rules: &'static [&'static str], notes, claim| d(id, name, group, "Long", Status::NeedsData(why), &[], rules, notes, claim);
    let nt = |id, name, group, dir, why, rules: &'static [&'static str], notes, claim| d(id, name, group, dir, Status::NotTestable(why), &[], rules, notes, claim);
    use Status::*;
    vec![
        // ---- Timing models: price / breadth (Live)
        d("mbsot_zweig4", "Zweig 4% weekly momentum model", "Timing models · Breadth & momentum", "Long", Live, &[], ZWEIG4,
          "Book index: Value Line Composite (cash) weekly, 1962-; also quoted on the Nasdaq. The vault has no Value Line index: run on SPX/NDX/DJI or NIFTY 50 (instrument swap, 4% kept).",
          claim("", "48%", "", "invested 32% (p.25)", "Value Line 1962-: 10.3% compound vs 10.4% S&P (p.25) / same GPA as BH with 1/3 the drawdown (p.19); Nasdaq 1963-: 18.2% vs 10.3%, max DD < 18%")),
        d("mbsot_renshaw_thrust", "Renshaw thrust (S&P +2.75% and 70% of NYSE issues advancing) · NYSE breadth", "Timing models · Breadth & momentum", "Long", Live, NYSE_ADU, RENSHAW,
          "Book: S&P since 1953. Vault: NYSE breadth 1965-2020 on SPX/SPY/DJI.", claim("20 since 1953", "", "+23% average gain", "", "S&P 1953-1995 (Timing Models - 19)")),
        d("mbsot_renshaw_thrust_nse", "Renshaw thrust · NSE breadth (India)", "Timing models · Breadth & momentum", "Long", Live, NSE_ADU, RENSHAW,
          "India mapping: NIFTY's own 2.75% day and NSE advances / (advances + declines + unchanged) >= 70% (extra_data/india/NSE_ADV_DEC.csv, 1999-).", None),
        d("mbsot_mcclellan", "McClellan Oscillator zero-cross system · NYSE breadth", "Timing models · Breadth & momentum", "Long", Live, NYSE_AD, MCCLELLAN,
          "Book table (DJIA points, 1932-1992; p.20): 1971-78 164 trades 40% winners, 1978-92 303 trades 44%. Run on DJI (1970-) / SPX for the comparison.",
          claim("303 (1978-92)", "44% (1978-92)", "+3,512 DJIA points 1978-92", "", "DJIA 1932-1992 table (Timing Models - 20)")),
        d("mbsot_mcclellan_nse", "McClellan Oscillator zero-cross system · NSE breadth (India)", "Timing models · Breadth & momentum", "Long", Live, NSE_AD, MCCLELLAN,
          "India mapping: NSE advances - declines.", None),
        d("mbsot_appel_impulse", "Appel's Impulse Signals · NYSE breadth", "Timing models · Breadth & momentum", "Long", Live, NYSE_AD, APPEL_IMP,
          "Book: NYSE / S&P since 1970.", claim("", "68%", "", "invested 11%", "NYSE since 1970; S&P +42.3% a.r. while invested (Timing Models - 21)")),
        d("mbsot_appel_impulse_nse", "Appel's Impulse Signals · NSE breadth (India)", "Timing models · Breadth & momentum", "Long", Live, NSE_AD, APPEL_IMP,
          "India mapping: NSE advances - declines; levels kept (NSE lists a similar number of issues to the NYSE of the 1970s-90s).", None),
        d("mbsot_appel_mcclellan", "Appel's McClellan Osc (cumulative A-D line, +1000 / +450)", "Timing models · Breadth & momentum", "Long", Live, NYSE_AD, APPEL_MCC,
          "Book: NYSE since 1971. Differs from apmacd_ad_stay (Appel's own +450 stay-in rule) by the +1000 entry and the 3/4% price condition.",
          claim("", "64%", "+154 NYSE points", "", "NYSE since 1971, DD 3.5%, S&P +25% a.r. while invested (Timing Models - 21)")),
        d("mbsot_appel_mcclellan_nse", "Appel's McClellan Osc · NSE breadth (India)", "Timing models · Breadth & momentum", "Long", Live, NSE_AD, APPEL_MCC, "India mapping: NSE advances - declines.", None),
        d("mbsot_appel_combo", "Appel's Combo Exit (Impulse entry, McClellan exit) · NYSE breadth", "Timing models · Breadth & momentum", "Long", Live, NYSE_AD, APPEL_COMBO,
          "Book: NYSE since 1971.", claim("", "73%", "+171 NYSE points", "invested 23%", "NYSE since 1971, DD 5.2%, S&P +39.8% a.r. while invested (Timing Models - 22)")),
        d("mbsot_appel_combo_nse", "Appel's Combo Exit · NSE breadth (India)", "Timing models · Breadth & momentum", "Long", Live, NSE_AD, APPEL_COMBO, "India mapping: NSE advances - declines.", None),
        // ---- Pattern recognition with complete exits (Live)
        d("mbsot_cgc_sd_gti_up", "CGC up, SD:5 < 30 on swing low day, GTI trend up", "Pattern recognition · Close Gap CPR", "Long", Live, &[], CGC_UP, GTI_NOTE,
          claim("774 up patterns", "74% profitable", "72% reached objective", "", "Statistical results p.3-22 (stocks and futures, pattern counts not trades)")),
        d("mbsot_cgc_sd_gti_dn", "CGC down, SD:5 > 70 on swing high day, GTI trend down", "Pattern recognition · Close Gap CPR", "Short", Live, &[], CGC_DN, GTI_NOTE,
          claim("526 down patterns", "77% profitable", "73% reached objective", "", "Statistical results p.3-22")),
        d("mbsot_rgp_sd_up", "Reversal gap up when prior day's SD:5 < 30", "Pattern recognition · Reversal gaps", "Long", Live, &[], RGP_UP, GTI_NOTE,
          claim("290 up patterns", "64% profitable", "64% reached objective", "", "Statistical results p.3-38")),
        d("mbsot_rgp_sd_dn", "Reversal gap down when prior day's SD:5 > 70", "Pattern recognition · Reversal gaps", "Short", Live, &[], RGP_DN, GTI_NOTE,
          claim("271 down patterns", "62% profitable", "62% reached objective", "", "Statistical results p.3-38")),
        d("mbsot_rgp_sd_gti_up", "RGP up with SD:5 < 30 and GTI trend up", "Pattern recognition · Reversal gaps", "Long", Live, &[], RGP_GTI_UP, GTI_NOTE,
          claim("127 up patterns", "73% profitable", "68% returned 3x risk", "", "Statistical results p.3-41")),
        d("mbsot_rgp_sd_gti_dn", "RGP down with SD:5 > 70 and GTI trend down", "Pattern recognition · Reversal gaps", "Short", Live, &[], RGP_GTI_DN, GTI_NOTE,
          claim("98 down patterns", "75% profitable", "64% returned 3x risk", "", "Statistical results p.3-41")),
        d("mbsot_outside_sd_gti_up", "Outside day up with stochastic divergence and GTI trend up", "Pattern recognition · Outside days", "Long", Live, &[], OUT_UP, GTI_NOTE,
          claim("468 up patterns", "65% profitable", "65% reached objective", "", "Statistical results p.3-52")),
        d("mbsot_outside_sd_gti_dn", "Outside day down with stochastic divergence and GTI trend down", "Pattern recognition · Outside days", "Short", Live, &[], OUT_DN, GTI_NOTE,
          claim("315 down patterns", "67% profitable", "67% reached objective", "", "Statistical results p.3-52")),
        d("mbsot_inside_sd_up", "Inside day low upward breakout with SD:5 divergence", "Pattern recognition · Inside days", "Long", Live, &[], IN_UP, GTI_NOTE,
          claim("2292 up patterns", "59% profitable", "59% reached objective", "", "Statistical results p.3-63")),
        d("mbsot_inside_sd_dn", "Inside day high downward breakout with SD:5 divergence", "Pattern recognition · Inside days", "Short", Live, &[], IN_DN, GTI_NOTE,
          claim("1671 down patterns", "58% profitable", "58% reached objective", "", "Statistical results p.3-63")),
        // ---- Pattern recognition: no exit / judgement (NotTestable)
        nt("mbsot_key_island", "Key island reversal (reaction) low / high", "Pattern recognition · Key island reversals", "Long/Short",
           "no exit rule: only the entry (open of day 4) and the stop are given; the statistics count patterns returning 3x risk; the reaction checklist (\"major trend is up\", rally of 20% \"in a stochastic or major pivot cycle\") is judged on the chart",
           &["F|\"Day 2's high is < day 1's low and day 3's low. Pattern is completed on day 3.\" (p.3-4)",
             "E|\"Buy on open of day 4 with a stop loss one point below day 2's low.\" (p.3-4); mirror \"Sell on the open of day 4 with a stop-loss 1 point above day 2's high\" (p.3-6)"],
           "", claim("126 patterns", "82% profitable", "71% returned 3x risk", "", "Statistical results p.3-10")),
        nt("mbsot_cgc_basic", "Close Gap CPR up / down (basic, with or without SD:5 on the swing day)", "Pattern recognition · Close Gap CPR", "Long/Short",
           "no exit rule (\"buy the next day's open and use an ops below the low of day 2\" only); the statistics count patterns reaching an unstated objective",
           &["E|\"The basic strategy is to buy the next day's open and use an ops below the low of day 2.\" (p.3-11); short mirror (p.3-12)"],
           "", claim("4208 patterns", "", "63% reached objective", "", "CGC with SD overbought/oversold, p.3-19")),
        nt("mbsot_rgp_basic", "Reversal gap up / down (basic)", "Pattern recognition · Reversal gaps", "Long/Short",
           "no exit rule (\"buy on the next day's open and use an ops below the last swing low\" only)",
           &["E|\"Normally we buy on the next day's open and use an ops below the last swing low.\" (p.3-29)"], "", None),
        nt("mbsot_outside_basic", "Outside day up / down (basic and with stochastic divergence)", "Pattern recognition · Outside days", "Long/Short",
           "no exit rule: only the stop entry one tick beyond the outside day and the ops are given (p.3-42, 3-43, 3-46, 3-47)",
           &["E|\"Buy on a stop one tick above the high of the outside day, with an ops one point below the low of the outside day.\" (p.3-42)"],
           "", claim("2705 patterns", "60% profitable", "", "", "Stochastic divergence outside days, p.3-49")),
        nt("mbsot_outside_sd_gti52", "Outside day + stochastic divergence + GTI trend + < 52% retracement of the previous stochastic rally", "Pattern recognition · Outside days", "Long/Short",
           "the 52% retracement is measured from the previous stochastic rally's A-B points chosen on the chart; the stochastic-cycle highs/lows depend on the previous cycle (p.1-5, 1-6) and no code is given",
           &["F|\"The reaction leading to the outside day up potential low must correct less than 52% of the previous up move.\" (p.3-53)", "E|\"Same as previous outside day strategy\" (p.3-54)"],
           "", claim("342 patterns", "72% profitable", "65% returned 3x risk", "", "p.3-55")),
        nt("mbsot_inside_basic", "Inside day low upward / high downward breakout (basic)", "Pattern recognition · Inside days", "Long/Short",
           "no exit rule (stop entry and ops only, p.3-56, 3-57)",
           &["E|\"Buy on a stop one tick above the high of the day prior to the inside day with an ops one point below the low of the day prior to the inside day\" (p.3-56)"], "", None),
        nt("mbsot_tbblbg_defs", "Breakaway gaps, breaklaps and thrust breakouts (TBBLBG)", "Pattern recognition · Breakouts", "Long/Short",
           "definitions only (no order, stop or exit attached); breakouts need judged congestion / resistance levels",
           &["F|\"A thrust up is simply a day in which the range is two or more times the average range of the last 20 days, the volume is higher than the volume of the prior day, and the close is in the top third of the range.\" (p.3-71)",
             "F|\"A 'congestion area' occurs any time prices are confined to a range that is within 1.62 times the average range of the last 20 days for 10 days or more\" (p.3-67)"], "", None),
        nt("mbsot_tr_breakout_accum", "Trading range bottom / top with accumulation indicator prior breakout and TBBLBG", "Pattern recognition · Breakouts", "Long/Short",
           "no exit rule (entry at the open after the TBBLBG and an ops below congestion only, p.3-83, 3-84)",
           &["E|\"Buy on open following tbblbg and use ops 3 points below congestion high or ops below congestion support, whichever gives the trade less risk.\" (p.3-83)"],
           "", claim("214 patterns", "71% profitable", "61% returned 3x risk", "", "p.3-85")),
        nt("mbsot_major_swing_db", "Major swing double / triple bottoms and tops with accumulation breakout on TBBLBG", "Pattern recognition · Swing patterns", "Long/Short",
           "only a stop and a \"minimum objective\" are given (no rule for when to exit at or after the objective); major swings and accumulation breakouts are judged on the chart",
           &["E|\"Buy on next day's open with ops below D or 3 points below C, whichever gives the trade lower risk. Minimum objective is the distance from B to C projected above C.\" (p.3-91)"],
           "", claim("140 patterns", "74% profitable", "66% reached objective", "", "p.3-93")),
        nt("mbsot_swing_breakout", "Short-term or major swing breakout / breakdown on TBBLBG with accumulation confirmation", "Pattern recognition · Swing patterns", "Long/Short",
           "exit depends on futures margin (\"When your open profit equals margin use a break even ops\", p.3-114) and an unspecified \"trailing ops technique\"; 3-of-4 accumulation confirmation judged",
           &["E|\"Buy on next day's open with ops below last swing low.\" (p.3-109)"], "", claim("412 patterns", "70% profitable", "64% returned 3x risk", "", "p.3-115")),
        nt("mbsot_sharp_reaction_tr", "2-3 day sharp reaction trading range after a prolonged runaway move", "Pattern recognition · Reaction patterns", "Long/Short",
           "\"prolonged runaway decline\" of six months or more and the \"several of the runaway characteristics\" are judged on the chart",
           &["E|\"Buy on next open with an ops below the last swing low. Use long-term GTI trailing ops\" (p.3-101)"], "", claim("173 patterns", "77% profitable", "65% returned 3x risk", "", "p.3-103")),
        nt("mbsot_remaining_patterns", "N corrections, dynamite trading ranges, GTI 7-14 day reaction ranges, volume-leads-price, CGC/inside day with accumulation divergence", "Pattern recognition · Remaining", "Long/Short",
           "not coded in this pass: each needs the swing-chart letter points (A-F), congestion ranges and four accumulation indicators (OBV, VOLAC and SWAD need volume) - listed as remaining in library/mbsot/README.md",
           &["F|p.3-23..3-27, 3-56..3-66 (accumulation variants), 3-74..3-80 (GTI 7-14 day TR, Compu-Trac code TR714), 3-96..3-100 (N corrections), 3-116..3-119 (dynamite TR), 3-120..3-123 (volume leads price)"],
           "", None),
        // ---- Timing models without an exit
        nt("mbsot_zweig_91", "Zweig 9/1 and double 9/1 up-volume days", "Timing models · Breadth & momentum", "Long",
           "no exit: the book only reports prices 2, 3 and 12 months later (\"61% of the time a 9/1 is followed by an average 4.1% higher prices in 2 months\")",
           &["F|\"A 9/1 occurs when up volume/(up volume + down volume) = 90% or >.\" A double 9/1 is \"two 9/1's which occur within 3 months of each other\" (Timing Models - 18)"],
           "", claim("12 double 9/1 signals", "11 of 12 up a year later", "+18.6% a year later", "", "since 1965 (Timing Models - 18)")),
        nt("mbsot_ten_day_ad", "Ten day A/D ratio", "Timing models · Breadth & momentum", "Long",
           "no exit and the threshold is garbled (\"This gets bullish on moves over 2.71% of the time...\"); only 3- and 6-month forward averages are given",
           &["F|\"Ten day A/D ratio. This gets bullish on moves over 2.71% of the time the market is higher by an average of 5.2% 3 months later\" (Timing Models - 19)"], "", None),
        nt("mbsot_tdom_seasonal", "Larry Williams day-of-month S&P and T-bond futures trades", "Timing models · Seasonality", "Long/Short",
           "entry time on the date and exit time are not stated, and the $2,000 target / $2,500 ops need the futures contract's dollar value per point (S&P and T-bond futures data are not in the vault)",
           &["E|\"S&P FUTURES BUY DATES AND RULES: 1st, 28th - exit next day or $2,000 profit. 5th, 26th - exit in four days or $2,000 profit. 29th - exit in 6 days or $2,000 profit.\" (Timing Models - 7)",
             "E|\"S&P FUTURES SELL DATES AND RULES: 2nd - exit in six days or $2,000 profit. 13th - exit in four days or $2,000 profit.\"; \"All trades assume a $2,500 ops.\" (Timing Models - 7)"],
           "", claim("", "81%", "$202,000 total", "", "S&P and T-bond futures, last 10 years (Timing Models - 7)")),
        nt("mbsot_killer_sp", "Larry Williams 'Killer' S&P day-of-week gap system", "Timing models · Futures only system", "Long/Short",
           "S&P futures system in futures points ($3,500 / $2,500 stops) and the System Writer listing is partly illegible in the scan (several gap multipliers cannot be read)",
           &["E|\"take 40% of any gap over 200 points in the S&P and add it to the open for a buy stop, and subtract it from the open for a sell stop ... use a $3,500 stop loss for longs, and a $2,500 stop loss for shorts. Exit on the first profitable open.\" (Timing Models - 28)"],
           "Larry Williams' system reprinted in the workbook; S&P 6/92 printout: 157 trades, 92%, $1,097 average (Timing Models - 29).", claim("157", "92%", "$1,097", "1 bar (winners)", "S&P 500 futures to 6/92 (Timing Models - 29)")),
        // ---- Timing models needing data the vault lacks
        nd("mbsot_monetary_models", "Monetary indicators 1-9 (Fed funds ROC, T-bill ROC, discount rate, junk-bond yield ratio, DJ20 bonds, yield curve, Zweig prime rate, simple prime rate, Zweig Fed indicator)", "Timing models · Monetary",
           "Fed funds, T-bill, discount and prime rates, Baa/Aaa yields, DJ20 bond average, long bond yields - none in the vault",
           &["E|\"Yield Curve ... Buy when yield curve climbs to 1.10; sell when yield curve drops to 1.05.\" (Timing Models - 5)",
             "E|\"Zweig Prime Rate Indicator ... Buy on any cut in the prime rate if the prime rate peak is below 8%, or on any two cuts in a row (or a cut of 1 or more) if the prime rate peak is above 8%. Sell on any increase in PR if PR's low is >= 8%; or on the second of two consecutive hikes (or a full 1% point hike) if the PR's low is less than 8%.\" (Timing Models - 6)",
             "E|\"Simple Prime Rate Method. Buy when PR declines by 2% (of its level, not 2 percentage points) over a 2 month period; Sell when PR increases 4% of its level over 2 month period.\" (Timing Models - 6)",
             "E|Zweig Fed indicator point scheme (+2 on the first discount-rate cut, -1 per hike for six months ...); \"Buy Index values of +2 or >; Sell 0 or <.\" (Timing Models - 6, 24)"],
           "", claim("", "", "", "", "Yield curve 1947-: 14.8% GPA vs 10.7%; prime rate 14.7% vs 11.6% (S&P since 1954)")),
        nd("mbsot_economic_models", "Economic gauges 1-15 (leading inflation, capacity utilization, industrial production, S&P vs GDP, help wanted, earnings, CPI/GDP, CPI fast/slow, CRB, installment debt, unemployment, Pring torque, 5-index composite)", "Timing models · Economic",
           "monthly macro series (CPI, GDP, capacity utilization, CRB, consumer installment debt, housing starts, vendor deliveries ...) - not in the vault",
           &["E|\"CPI fast and slow ROC ... Buy when the fast rate < slow rate; Exit when fast rate >= slow rate.\" (Timing Models - 13)",
             "E|\"5 Index Composite ... Buy when composite +1 or >; Exit when composite -3 or <.\" (Timing Models - 14)",
             "E|\"Consumer Installment Debt year over year % changes ... Buy on a decline below 9%; Sell on a rise above 9%.\" (Timing Models - 13)"],
           "", claim("", "19/21 signals (CPI ROC)", "", "", "CPI fast/slow since 1954: 13.6% GPA vs 10.6%; 5-index composite since 1953: 14.4%, 14/14 buys")),
        nd("mbsot_sentiment_models", "Sentiment models 1-8 (IPOs/secondaries, corporate net stock sales, household stock share, mutual fund sales, put/call, margin debt, sentiment adjusted for monetary conditions)", "Timing models · Sentiment",
           "IPO / secondary counts, flow-of-funds, mutual fund sales, put/call history, margin debt, Investors Intelligence, T-note yields, Value Line index - not in the vault",
           &["E|Sentiment adjusted for monetary conditions: 3/5-year T-note yield vs 26 weeks ago, Investors Intelligence bulls/(bulls+bears) bands, Value Line 4% weekly momentum; \"To buy wait for components 1 and 2 to occur and then buy if and when a momentum signal occurs; opposite to exit.\" (Timing Models - 17)"],
           "", claim("21 long entries", "20 of 21", "", "exposed 50%", "since 1968: 16.74% GPA vs 10.1% (Timing Models - 17)")),
        nd("mbsot_nhnl_tbill", "NH NL T-bill dividend ratio model", "Timing models · Breadth & momentum",
           "NYSE new highs / new lows, 90-day T-bill yield and S&P dividend yield - not in the vault",
           &["E|\"Buy when NH > ((T X A) - B) X SUM and P > M. Exit when NL > ((T X (-C)) + D) X SUM and P < M.\" with NH, NL, SUM = 8-day averages, T = T-bill yield / S&P dividend, A = 0.58, B = 0.79, C = 0.66, D = 2.13, M = 18-day MA of the close (Timing Models - 20)"],
           "", claim("", "", "", "", "16.1% GPA vs 10.6% BH")),
        nd("mbsot_trim_yield", "Arms Index (TRIM) 21/55 crossovers with T-bill / dividend yield spread filter", "Timing models · Breadth & momentum",
           "90-day T-bill yield and S&P dividend yield (the TRIM part exists in extra_data, the yield filter does not)",
           &["E|Arms index 21 and 55-day MA crossovers; \"Only take buy signals when the yield spread is 2 or <; and only take sell/exits when the yield spread is > 2.\" (Timing Models - 21)"],
           "", claim("", "81%", "$192,312 total", "", "S&P futures since 1982")),
        nd("mbsot_appel_bottom_fisher", "Appel's bottom-fisher combo", "Timing models · Breadth & momentum",
           "NYSE new lows - not in the vault (extra_data has advances/declines only)",
           &["E|\"1) Buy any close by NYSE cash below 24 previous closes, where no new low close was made in the prior three days and New Lows are < 50; or 2) Buy any close by NYSE cash below 49 previous closes, where no new low close was made in the prior three days and New Lows are <= 100.\" (Timing Models - 22)",
             "X|\"Exit on any NYSE cash close that is 1.5% or > below the highest close since entry.\" (p.22)"],
           "", claim("", "79%", "$174,000 total", "", "S&P futures since 1982, DD $3,600")),
        nd("mbsot_appel_total", "Appel Total Composite", "Timing models · Breadth & momentum",
           "NYSE new highs and new lows and the bottom-fisher entry - not in the vault",
           &["E|\"Enter long on any Appel Impulse Signal or on any Appel bottom-fisher combo entry.\" (Timing Models - 22)",
             "X|McClellan exit if the osc went > +450 during the trade, else the Impulse exit, \"or 3) regardless of 1 and 2 prior, exit any time both NYSE New Highs and NYSE new lows exceed 2.5% of NYSE issues trades on the same day.\" (p.22)"],
           "", claim("", "65%", "", "invested < 45%", "since 1970: 17.2% GPA vs 11.2%, max DD 6.5%")),
        nd("mbsot_renshaw_divcon", "Renshaw Div/Con and Renshaw Composite", "Timing models · Breadth & momentum",
           "weekly S&P dividend yield and 90-day T-bill rate - not in the vault",
           &["E|\"First smooth the weekly S&P dividend yield with a 2 week EMA - if the high of the EMA was 3.5% or >, and current EMA is .25 < peak and EMA of 25 weeks ago is also .25 < peak then Buy. Exit longs when the raw S&P dividend yield is 3% or less and the 90 day T-bill rate climbs to a 75 week high.\" (Timing Models - 19)",
             "E|Composite: \"Buy on either a Renshaw Div/Con signal or when the S&P is > its 10 week m.a. and a Renshaw Thrust has occurred over the last 35 weeks. Use a 10% ops, and a 10% trailing stop. Also exit on a Renshaw Div/Con sell signal.\" (p.19)"],
           "", claim("12 (Div/Con)", "11 of 12", "", "invested 78% (composite)", "Div/Con since 1953: 17% GPA; composite since 1957: 14.6%")),
        nd("mbsot_zweig_super", "Zweig Super Model, Vega timing system, Super Bull / Super Bear signals", "Timing models · Composite models",
           "prime rate, discount rate, consumer installment debt, DJ20 bonds, PPI, P/E and dividend yield - not in the vault",
           &["E|Super model points: Fed indicator 4/2/1/0, prime rate 2/0, installment debt 2/0, 4% Value Line model 2/0; buy >= 6 points, sell <= 3 (Timing Models - 23..26)",
             "E|Vega: DJ20 bond average vs a year ago (6 points), prime rate 4, Value Line 4% 4 (sell on a 2% weekly decline); \"Buy signals: +8 Sell signals: +6 or lower\" (Timing Models - 27)"],
           "", claim("", "", "", "", "Super Model S&P since 1966: 14.6% vs 9.9%; Vega since 1962: 15% vs 10.9%")),
        nd("mbsot_bond_models", "Bond timing models (seasonals, economic gauges, 5-week sentiment, VAR7 bond system)", "Timing models · Bonds",
           "bond prices / yields, prime and discount rates, Moody's AAA yields, CPI, DJ20 bond average, market sentiment polls - not in the vault",
           &["E|VAR7: six components (prime rate, discount rate, yield curve, real bond yields, short- and intermediate-term DJ20 bond momentum); \"When the sum of all these component inputs reaches +1 or > from below it, Buy Bonds. When the sum reaches -1 or < from above it, Exit Bonds\" (Timing Models - 34-35, summary)"],
           "", claim("", "87% of buys", "", "", "since 1952: 9.6% GPA, max DD 5.4% (Timing Models - 35)")),
        nd("mbsot_gold_models", "Gold stock timing models (K-ratio SD bands, GMI-bullion-Swiss franc momentum)", "Timing models · Gold",
           "Barron's Gold Mining Index, gold bullion and Swiss franc weekly - not in the vault",
           &["E|\"Buy when the weekly GMI is 8% above a weekly GMI low; and the Swiss Franc (versus dollar) is above its 9 week m.a.; and gold bullion is higher than it was 26 weeks ago. Once long exit when the weekly GMI falls 8% or more from a prior weekly GMI close\" (Timing Models - 3, summary as in The Hedge Fund Edge p.312)"],
           "Same models as mbhfe_k_ratio / mbhfe_gmi_momentum.", None),
        nd("mbsot_stock_selection", "Runaway stocks with fuel: selection criteria, simple flag / reaction patterns and exits", "Vehicle selection",
           "stock universe with earnings, RS rank, institutional ownership, debt and rating-service data - not in the vault",
           &["F|Criteria for runaway up stocks with fuel 1-9 (consistent earnings growth, EPS and RS ranks > 80, not over-valued, low debt, institutional sponsorship < 30-40% ...) (Stock Selection - 18)",
             "X|When and how to take profits and exit positions, shorts 1-8 and longs 1-9 (stochastic cycle / GTI trailing stops, half profits at RSI(9) / accumulation divergences, RS < 60, P/E > 2x S&P ...) (Stock Selection - 27)"],
           "Same method as mbhfe_flag_breakout / mbhfe_ma200_reaction.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mbsot_zweig4" => ("mbsot_mom", "Boucher momentum models", "Zweig 4% weekly"),
        "mbsot_renshaw_thrust" => ("mbsot_renshaw", "Renshaw thrust", "NYSE breadth"),
        "mbsot_renshaw_thrust_nse" => ("mbsot_renshaw", "Renshaw thrust", "NSE breadth"),
        "mbsot_mcclellan" => ("mbsot_mcc", "McClellan zero-cross", "NYSE breadth"),
        "mbsot_mcclellan_nse" => ("mbsot_mcc", "McClellan zero-cross", "NSE breadth"),
        "mbsot_appel_impulse" => ("mbsot_appel", "Appel breadth models (Boucher)", "Impulse · NYSE"),
        "mbsot_appel_impulse_nse" => ("mbsot_appel", "Appel breadth models (Boucher)", "Impulse · NSE"),
        "mbsot_appel_mcclellan" => ("mbsot_appel", "Appel breadth models (Boucher)", "McClellan +1000 · NYSE"),
        "mbsot_appel_mcclellan_nse" => ("mbsot_appel", "Appel breadth models (Boucher)", "McClellan +1000 · NSE"),
        "mbsot_appel_combo" => ("mbsot_appel", "Appel breadth models (Boucher)", "Combo exit · NYSE"),
        "mbsot_appel_combo_nse" => ("mbsot_appel", "Appel breadth models (Boucher)", "Combo exit · NSE"),
        "mbsot_cgc_sd_gti_up" | "mbsot_cgc_sd_gti_dn" => ("mbsot_cgc", "Close Gap CPR + SD:5 + GTI", if id.ends_with("up") { "up" } else { "down" }),
        "mbsot_rgp_sd_up" | "mbsot_rgp_sd_dn" => ("mbsot_rgp", "Reversal gaps + SD:5", if id.ends_with("up") { "up" } else { "down" }),
        "mbsot_rgp_sd_gti_up" | "mbsot_rgp_sd_gti_dn" => ("mbsot_rgp", "Reversal gaps + SD:5", if id.ends_with("up") { "up + GTI trend" } else { "down + GTI trend" }),
        "mbsot_outside_sd_gti_up" | "mbsot_outside_sd_gti_dn" => ("mbsot_outside", "Outside day + SD divergence + GTI", if id.ends_with("up") { "up" } else { "down" }),
        "mbsot_inside_sd_up" | "mbsot_inside_sd_dn" => ("mbsot_inside", "Inside day + SD divergence", if id.ends_with("up") { "upward breakout" } else { "downward breakout" }),
        "mbsot_key_island" | "mbsot_cgc_basic" | "mbsot_rgp_basic" | "mbsot_outside_basic" | "mbsot_outside_sd_gti52" | "mbsot_inside_basic" | "mbsot_tbblbg_defs"
        | "mbsot_tr_breakout_accum" | "mbsot_major_swing_db" | "mbsot_swing_breakout" | "mbsot_sharp_reaction_tr" | "mbsot_remaining_patterns" =>
            ("mbsot_patterns_nt", "Science of Trading patterns without a complete exit", "not testable"),
        "mbsot_zweig_91" | "mbsot_ten_day_ad" | "mbsot_tdom_seasonal" | "mbsot_killer_sp" => ("mbsot_timing_nt", "Science of Trading timing notes", "not testable"),
        _ if id.starts_with("mbsot_") => ("mbsot_macro", "Science of Trading macro / sentiment models", "needs data"),
        _ => return None,
    })
}

const PATTERNS: &[&str] = &["mbsot_cgc_sd_gti_up", "mbsot_cgc_sd_gti_dn", "mbsot_rgp_sd_up", "mbsot_rgp_sd_dn", "mbsot_rgp_sd_gti_up", "mbsot_rgp_sd_gti_dn",
    "mbsot_outside_sd_gti_up", "mbsot_outside_sd_gti_dn", "mbsot_inside_sd_up", "mbsot_inside_sd_dn"];

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pat = |hi: f64, lo: f64| vec![
        p("gti_pct", "GTI: new high / low needed to register a swing or turn the trend, % (\"one point (2%)\")", 2.0, 0.5, 5.0, 0.25),
        p("pt", "\"One point\" for stop offsets, price units", 1.0, 0.0, 50.0, 0.5),
        p("sd_hi", "SD:5 overbought level", hi, 60.0, 90.0, 1.0),
        p("sd_lo", "SD:5 oversold level", lo, 10.0, 40.0, 1.0),
    ];
    Some(match id {
        "mbsot_zweig4" => vec![p("pct", "Reversal % from the weekly trough / peak close", 4.0, 1.0, 10.0, 0.5), p("short", "Also sell short on sell signals (1 = yes; not the book)", 0.0, 0.0, 1.0, 1.0)],
        "mbsot_renshaw_thrust" | "mbsot_renshaw_thrust_nse" => vec![
            p("rise", "Index one-day rise, %", 2.75, 1.0, 5.0, 0.25), p("adv", "Advancing issues % of issues traded", 70.0, 55.0, 85.0, 1.0),
            p("stop", "Open protective stop and trailing stop, %", 10.0, 3.0, 20.0, 1.0), p("trail_hi", "Trail from highs instead of closes (1 = yes; book silent)", 0.0, 0.0, 1.0, 1.0)],
        "mbsot_mcclellan" | "mbsot_mcclellan_nse" => vec![p("fast", "Fast EMA days", 19.0, 5.0, 30.0, 1.0), p("slow", "Slow EMA days", 39.0, 20.0, 80.0, 1.0)],
        "mbsot_appel_impulse" | "mbsot_appel_impulse_nse" | "mbsot_appel_combo" | "mbsot_appel_combo_nse" | "mbsot_appel_mcclellan" | "mbsot_appel_mcclellan_nse" => vec![
            p("e15", "15-day EMA of A-D at or above", 300.0, 100.0, 600.0, 25.0), p("e8", "8-day EMA of A-D at or above", 400.0, 100.0, 800.0, 25.0),
            p("e6", "6-day EMA of A-D at or above", 500.0, 100.0, 1000.0, 25.0), p("drop", "Impulse exit: close down from the highest close, %", 1.5, 0.5, 5.0, 0.25),
            p("mcc_buy", "McClellan (A-D line) buy above", 1000.0, 300.0, 2000.0, 50.0), p("mcc_exit", "McClellan exit below", 450.0, 0.0, 1000.0, 50.0),
            p("mcc_drop", "McClellan exit: close down from the highest close, %", 0.75, 0.25, 3.0, 0.25)],
        "mbsot_cgc_sd_gti_up" | "mbsot_cgc_sd_gti_dn" | "mbsot_rgp_sd_up" | "mbsot_rgp_sd_dn" | "mbsot_rgp_sd_gti_up" | "mbsot_rgp_sd_gti_dn"
        | "mbsot_outside_sd_gti_up" | "mbsot_outside_sd_gti_dn" | "mbsot_inside_sd_up" | "mbsot_inside_sd_dn" => pat(70.0, 30.0),
        _ => return None,
    })
}

// ------------------------------------------------------------------ indicators

fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut e = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_finite() {
            e = if e.is_finite() { e + a * (v[i] - e) } else { v[i] };
        }
        out[i] = e; // a day without breadth data leaves the average unchanged
    }
    out
}

/// Lane's slow %D (SD:5): 100 x sum3(C - LL5) / sum3(HH5 - LL5), averaged over 3 days (p.1-22).
pub fn sd5(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    let n = c.len();
    let (mut up, mut rg) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 4..n {
        let hh = h[i - 4..=i].iter().cloned().fold(f64::MIN, f64::max);
        let ll = l[i - 4..=i].iter().cloned().fold(f64::MAX, f64::min);
        up[i] = c[i] - ll;
        rg[i] = hh - ll;
    }
    let mut d = vec![f64::NAN; n];
    for i in 6..n {
        let (u, r): (f64, f64) = (up[i - 2..=i].iter().sum(), rg[i - 2..=i].iter().sum());
        if r > 0.0 { d[i] = 100.0 * u / r }
    }
    let mut sd = vec![f64::NAN; n];
    for i in 8..n {
        sd[i] = (d[i - 2] + d[i - 1] + d[i]) / 3.0;
    }
    sd
}

/// Swing chart (p.3-86): returns the last confirmed pivot low and pivot high as of each bar (NaN before the first).
pub fn pivots(h: &[f64], l: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = h.len();
    let (mut plo, mut phi) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    let (mut up, mut x) = (true, 0usize);
    let (mut lo, mut hi) = (f64::NAN, f64::NAN);
    for i in 1..n {
        if up {
            if h[i] > h[x] { x = i } else if l[i] < l[x] { hi = h[x]; up = false; x = i }
        } else if l[i] < l[x] { x = i } else if h[i] > h[x] { lo = l[x]; up = true; x = i }
        plo[i] = lo;
        phi[i] = hi;
    }
    (plo, phi)
}

/// Revised two-day GTI (p.4-1..4-18). trend[i] after bar i (+1/-1/0); bot[i]/top[i] = value registered on bar i (NaN = none).
pub struct Gti { pub trend: Vec<i8>, pub bot: Vec<f64>, pub top: Vec<f64> }

pub fn gti(h: &[f64], l: &[f64], c: &[f64], pct: f64) -> Gti {
    let n = c.len();
    let k = pct / 100.0;
    let mut g = Gti { trend: vec![0; n], bot: vec![f64::NAN; n], top: vec![f64::NAN; n] };
    let (mut t, mut ext, mut ext_i, mut corr, mut cext) = (0i8, f64::NAN, 0usize, false, f64::NAN);
    let (mut last_bot, mut last_top) = (f64::NAN, f64::NAN);
    for i in 2..n {
        let up2 = c[i] > c[i - 1] && c[i - 1] > c[i - 2];
        let dn2 = c[i] < c[i - 1] && c[i - 1] < c[i - 2];
        match t {
            0 => {
                if up2 {
                    t = 1; last_bot = l[i - 2].min(l[i - 1]); g.bot[i] = last_bot; ext = h[i]; ext_i = i;
                } else if dn2 {
                    t = -1; last_top = h[i - 2].max(h[i - 1]); g.top[i] = last_top; ext = l[i]; ext_i = i;
                }
            }
            1 => {
                if l[i] <= last_bot * (1.0 - k) {
                    // trend turns down: the leg's high is the GTI top
                    t = -1; last_top = ext; g.top[i] = ext; ext = l[i]; ext_i = i; corr = false;
                } else if corr {
                    cext = cext.min(l[i]);
                    if h[i] >= ext * (1.0 + k) {
                        last_bot = cext; g.bot[i] = cext; ext = h[i]; ext_i = i; corr = false;
                    }
                } else {
                    if h[i] > ext { ext = h[i]; ext_i = i }
                    if dn2 { corr = true; cext = l[if ext_i < i { ext_i + 1 } else { i }..=i].iter().cloned().fold(f64::MAX, f64::min) }
                }
            }
            _ => {
                if h[i] >= last_top * (1.0 + k) {
                    t = 1; last_bot = ext; g.bot[i] = ext; ext = h[i]; ext_i = i; corr = false;
                } else if corr {
                    cext = cext.max(h[i]);
                    if l[i] <= ext * (1.0 - k) {
                        last_top = cext; g.top[i] = cext; ext = l[i]; ext_i = i; corr = false;
                    }
                } else {
                    if l[i] < ext { ext = l[i]; ext_i = i }
                    if up2 { corr = true; cext = h[if ext_i < i { ext_i + 1 } else { i }..=i].iter().cloned().fold(f64::MIN, f64::max) }
                }
            }
        }
        g.trend[i] = t;
    }
    g
}

// ------------------------------------------------------------------ pattern simulator

#[derive(Clone, Copy, PartialEq)]
enum Ent { Open, Stop(f64) }
#[derive(Clone, Copy, PartialEq)]
enum Half { None, NextOpen, Close }

struct Plan {
    dir: f64,
    sig: usize,
    ent: Ent,
    stop: f64,
    half: Half,
    half_ge: bool,   // long trigger "70 or more" (true) vs "above 70" (false); shorts always "below 30"
    be: bool,        // break-even ops on the rest after the half exit
    trail_now: bool, // GTI trailing from entry (else only after the half exit)
}

/// One position: entry on bar sig+1, initial ops, optional half exit on an SD:5 reading, break-even, GTI trailing ops
/// (\"as soon as a GTI bottom is registered that is higher than our ops we move our stop to one point below\", p.4-18).
fn sim(x: &Ctx, sd: &[f64], g: &Gti, pl: &Plan, hi: f64, lo: f64, pt: f64, to: usize) -> Option<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let e = pl.sig + 1;
    if e > to { return None }
    let d = pl.dir;
    let px = match pl.ent {
        Ent::Open => o[e],
        Ent::Stop(lv) => {
            if d > 0.0 && h[e] >= lv { o[e].max(lv) } else if d < 0.0 && l[e] <= lv { o[e].min(lv) } else { return None }
        }
    };
    let mut t = Trade { dir: d, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: e, exit_px: f64::NAN, reason: String::new(),
        entry_kind: if pl.ent == Ent::Open { "open" } else { "stop" }, signal_i: pl.sig, open: false, exits: vec![] };
    let mut stop = pl.stop;
    let (mut half_done, mut half_pending, mut rest) = (false, false, 1.0);
    let hit = |v: f64, lvl: f64| if d > 0.0 { v <= lvl } else { v >= lvl };
    let tighter = |cand: f64, cur: f64| if d > 0.0 { cand > cur } else { cand < cur };
    let reg = |k: usize| if d > 0.0 { g.bot[k] } else { g.top[k] };
    let off = |v: f64| if d > 0.0 { v - pt } else { v + pt };
    for j in e..=to {
        let first = j == e;
        // a gap through the ops at the open fills at the open (the entry bar's open is the entry itself)
        if !(first && pl.ent == Ent::Open) && hit(o[j], stop) {
            t.exits.push((j, o[j], rest));
            t.reason = if half_done { "trailing / break-even ops (gap)".into() } else { "initial ops (gap)".into() };
            return Some(close_out(t, j));
        }
        if half_pending {
            t.exits.push((j, o[j], 0.5));
            rest = 0.5;
            half_pending = false;
            half_done = true;
        }
        if hit(if d > 0.0 { l[j] } else { h[j] }, stop) {
            let f = if d > 0.0 { o[j].min(stop) } else { o[j].max(stop) };
            // stop-entry bar: the ops was hit after the fill (conservative), so it fills at the stop;
            // an open entry already beyond the ops is stopped at that open
            let f = if first && pl.ent != Ent::Open { stop } else { f };
            t.exits.push((j, f, rest));
            t.reason = if half_done { "trailing / break-even ops".into() } else { "initial ops".into() };
            return Some(close_out(t, j));
        }
        if !half_done && !half_pending && pl.half != Half::None && sd[j].is_finite() {
            let trig = if d > 0.0 { if pl.half_ge { sd[j] >= hi } else { sd[j] > hi } } else { sd[j] < lo };
            if trig {
                if pl.half == Half::Close {
                    t.exits.push((j, c[j], 0.5));
                    rest = 0.5;
                    half_done = true;
                } else {
                    half_pending = true;
                }
            }
        }
        // ops for the next bar
        let was_trailing = pl.trail_now || (half_done && !(t.exits.last().map_or(false, |x| x.0 == j)));
        if pl.be && half_done && tighter(px, stop) { stop = px }
        if pl.trail_now || half_done {
            if !was_trailing {
                // trailing just began (\"thereafter\"): use the latest GTI bottom/top registered since entry
                if let Some(v) = (e..=j).rev().map(reg).find(|v| v.is_finite()) {
                    if tighter(off(v), stop) { stop = off(v) }
                }
            } else if reg(j).is_finite() && tighter(off(reg(j)), stop) {
                stop = off(reg(j));
            }
        }
    }
    t.exits.push((to, c[to], rest));
    t.reason = "open".into();
    t.open = true;
    Some(close_out(t, to))
}

fn close_out(mut t: Trade, j: usize) -> Trade {
    let w: f64 = t.exits.iter().map(|e| e.2).sum();
    t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>() / w;
    t.exit_i = j;
    t.signal_i = j;
    if t.exits.len() == 1 { t.exits.clear() }
    t
}

fn lowest(v: &[f64]) -> f64 { v.iter().cloned().fold(f64::INFINITY, f64::min) }
fn highest(v: &[f64]) -> f64 { v.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }

/// Pattern on bar i (the completion bar) -> trade plan, or None.
fn pattern(id: &str, x: &Ctx, i: usize, sd: &[f64], g: &Gti, plo: &[f64], phi: &[f64], tick: f64, pt: f64, hi: f64, lo: f64) -> Option<Plan> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    if i < 12 { return None }
    let up = id.ends_with("_up");
    let f = |b: bool| if b { Some(()) } else { None };
    let plan = |dir, ent, stop, half, half_ge, be, trail_now| Plan { dir, sig: i, ent, stop, half, half_ge, be, trail_now };
    match id {
        "mbsot_cgc_sd_gti_up" => {
            f(l[i - 1] < l[i - 2] && l[i] > c[i - 1] && c[i - 1] < c[i - 2] && c[i] > h[i - 1] && sd[i - 1] < lo && g.trend[i] == 1)?;
            Some(plan(1.0, Ent::Open, l[i - 1] - tick, Half::NextOpen, true, true, false))
        }
        "mbsot_cgc_sd_gti_dn" => {
            f(h[i - 1] > h[i - 2] && h[i] < c[i - 1] && c[i] < l[i - 1] && c[i - 1] > c[i - 2] && sd[i - 1] > hi && g.trend[i] == -1)?;
            Some(plan(-1.0, Ent::Open, h[i - 1] + tick, Half::NextOpen, false, true, false))
        }
        "mbsot_rgp_sd_up" | "mbsot_rgp_sd_gti_up" => {
            let mid = (h[i] - l[i]) / 2.0 + l[i];
            f(c[i - 1] < c[i - 2] && l[i] > h[i - 1] && c[i] > c[i - 2] && c[i] > c[i - 3] && c[i] > mid && sd[i - 1] < lo && plo[i].is_finite())?;
            if id == "mbsot_rgp_sd_up" {
                Some(plan(1.0, Ent::Open, plo[i] - tick, Half::NextOpen, false, true, false))
            } else {
                f(g.trend[i] == 1)?;
                Some(plan(1.0, Ent::Open, plo[i] - tick, Half::None, false, false, true))
            }
        }
        "mbsot_rgp_sd_dn" | "mbsot_rgp_sd_gti_dn" => {
            let mid = (h[i] - l[i]) / 2.0 + l[i];
            f(c[i - 1] > c[i - 2] && h[i] < l[i - 1] && c[i] < c[i - 2] && c[i] < c[i - 3] && c[i] < mid && sd[i - 1] > hi && phi[i].is_finite())?;
            if id == "mbsot_rgp_sd_dn" {
                Some(plan(-1.0, Ent::Open, phi[i] + tick, Half::NextOpen, false, true, false))
            } else {
                f(g.trend[i] == -1)?;
                Some(plan(-1.0, Ent::Open, phi[i] + tick, Half::None, false, false, true))
            }
        }
        "mbsot_outside_sd_gti_up" | "mbsot_outside_sd_gti_dn" | "mbsot_inside_sd_up" | "mbsot_inside_sd_dn" => {
            // SD divergence evaluated one day back (Y1) against the 10 days before it (N10)
            let y = i - 1;
            let (w, ws) = (&l[y - 10..y], &sd[y - 10..y]);
            let (wh, wsh) = (&h[y - 10..y], &sd[y - 10..y]);
            let div_lo = sd[y].is_finite() && ws.iter().all(|v| v.is_finite()) && l[y] < lowest(w) && sd[y] > lowest(ws);
            let div_hi = sd[y].is_finite() && wsh.iter().all(|v| v.is_finite()) && h[y] > highest(wh) && sd[y] < highest(wsh);
            if id.starts_with("mbsot_outside") {
                let od = h[i] > h[i - 1] && l[i] < l[i - 1];
                if up {
                    f(od && c[i] > c[i - 1] && div_lo && g.trend[i] == 1)?;
                    Some(plan(1.0, Ent::Stop(h[i] + tick), l[i] - pt, Half::Close, false, false, true))
                } else {
                    f(od && c[i] < c[i - 1] && div_hi && g.trend[i] == -1)?;
                    Some(plan(-1.0, Ent::Stop(l[i] - tick), h[i] + pt, Half::Close, false, false, true))
                }
            } else {
                let id_ = h[i] < h[i - 1] && l[i] > l[i - 1];
                if up {
                    f(id_ && div_lo)?;
                    Some(plan(1.0, Ent::Stop(h[i - 1] + tick), l[i - 1] - pt, Half::Close, false, false, true))
                } else {
                    f(id_ && div_hi)?;
                    Some(plan(-1.0, Ent::Stop(l[i - 1] - tick), h[i - 1] + pt, Half::Close, false, false, true))
                }
            }
        }
        _ => None,
    }
}

fn run_patterns(id: &str, x: &Ctx, from: usize, to: usize, g_: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let sd = sd5(h, l, c);
    let g = gti(h, l, c, g_("gti_pct"));
    let (plo, phi) = pivots(h, l);
    let tick = if x.r.first().map_or(false, |r| r.ticker.starts_with("NIFTY")) { 0.05 } else { 0.01 };
    let (pt, hi, lo) = (g_("pt"), g_("sd_hi"), g_("sd_lo"));
    let mut out = vec![];
    let mut i = from.max(12);
    while i < to {
        if let Some(pl) = pattern(id, x, i, &sd, &g, &plo, &phi, tick, pt, hi, lo) {
            if let Some(t) = sim(x, &sd, &g, &pl, hi, lo, pt, to) {
                i = t.exit_i + 1;
                out.push(t);
                continue;
            }
        }
        i += 1;
    }
    out
}

// ------------------------------------------------------------------ timing models

fn mk(i: usize, px: f64) -> Trade {
    Trade { dir: 1.0, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: f64::NAN, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

fn finish(mut out: Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) -> Vec<Trade> {
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.signal_i, t.reason, t.open) = (to, x.c[to], to, "open".into(), true);
        out.push(t);
    }
    out
}

/// Weekly closes: index of the last bar of each Monday-based calendar week.
fn week_ends(x: &Ctx) -> Vec<usize> {
    let day = |s: &str| -> i64 {
        let (y, m, d): (i64, i64, i64) = (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(1), s[8..10].parse().unwrap_or(1));
        let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * m + 2) / 5 + d - 1;
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468 // days since 1970-01-01 (a Thursday)
    };
    let wk: Vec<i64> = x.r.iter().map(|r| (day(&r.date) + 3).div_euclid(7)).collect();
    (0..wk.len()).filter(|&i| i + 1 == wk.len() || wk[i + 1] != wk[i]).collect()
}

fn zweig4(x: &Ctx, from: usize, to: usize, pct: f64, short: bool) -> Vec<Trade> {
    let c = &x.c;
    let k = pct / 100.0;
    let w: Vec<usize> = week_ends(x).into_iter().filter(|&i| i >= from && i <= to).collect();
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let Some(&w0) = w.first() else { return out };
    let (mut state, mut ext) = (0i8, c[w0]); // 0 flat-after-sell (tracking trough), 1 long (tracking peak)
    for &i in &w[1..] {
        if state == 1 {
            if c[i] <= ext * (1.0 - k) {
                let mut t = cur.take().unwrap();
                (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, c[i], i, format!("weekly close {pct}% below the peak weekly close"));
                out.push(t);
                state = 0;
                ext = c[i];
                if short { let mut s = mk(i, c[i]); s.dir = -1.0; cur = Some(s) }
            } else {
                ext = ext.max(c[i]);
            }
        } else if c[i] >= ext * (1.0 + k) {
            if let Some(mut s) = cur.take() {
                (s.exit_i, s.exit_px, s.signal_i, s.reason) = (i, c[i], i, format!("weekly close {pct}% above the trough weekly close"));
                out.push(s);
            }
            state = 1;
            ext = c[i];
            cur = Some(mk(i, c[i]));
        } else {
            ext = ext.min(c[i]);
        }
    }
    finish(out, cur, x, to)
}

fn renshaw(x: &Ctx, nse: bool, from: usize, to: usize, rise: f64, adv: f64, stp: f64, trail_hi: bool) -> Vec<Trade> {
    let s = if nse { NSE_ADU } else { NYSE_ADU };
    let (Some(a), Some(dd), Some(u)) = (x.series(s[0]), x.series(s[1]), x.series(s[2])) else { return vec![] };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let k = stp / 100.0;
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let mut top = 0.0;
    for i in from.max(1)..=to {
        if let Some(t) = cur.as_mut() {
            let stop = (t.legs[0].px * (1.0 - k)).max(top * (1.0 - k));
            if l[i] <= stop {
                (t.exit_i, t.exit_px, t.signal_i) = (i, o[i].min(stop), i);
                t.reason = "10% ops / trailing stop".into();
                out.push(cur.take().unwrap());
                continue;
            }
            top = top.max(if trail_hi { h[i] } else { c[i] });
            continue;
        }
        let tot = a[i] + dd[i] + u[i];
        if i < to && c[i] / c[i - 1] - 1.0 >= rise / 100.0 && tot > 0.0 && 100.0 * a[i] / tot >= adv {
            cur = Some(mk(i, c[i]));
            top = c[i];
        }
    }
    finish(out, cur, x, to)
}

/// Daily breadth models on closes. kind: 0 McClellan zero-cross, 1 Appel impulse, 2 Appel McClellan (A-D line), 3 combo.
fn breadth(x: &Ctx, nse: bool, kind: u8, from: usize, to: usize, g_: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let s = if nse { NSE_AD } else { NYSE_AD };
    let (Some(a), Some(d)) = (x.series(s[0]), x.series(s[1])) else { return vec![] };
    let c = &x.c;
    let net: Vec<f64> = a.iter().zip(d).map(|(a, d)| a - d).collect();
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    if kind == 0 {
        let (f, sl) = (ema(&net, g_("fast").round() as usize), ema(&net, g_("slow").round() as usize));
        let osc: Vec<f64> = f.iter().zip(&sl).map(|(a, b)| a - b).collect();
        for i in from.max(2)..=to {
            if cur.is_some() {
                let (m0, m1) = ((osc[i] + osc[i - 1]) / 2.0, (osc[i - 1] + osc[i - 2]) / 2.0);
                if m1 >= 0.0 && m0 < 0.0 {
                    let mut t = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, c[i], i, "2-day MA of the oscillator dropped below 0".into());
                    out.push(t);
                }
            } else if i < to && osc[i - 1] < 0.0 && osc[i] > 0.0 {
                cur = Some(mk(i, c[i]));
            }
        }
        return finish(out, cur, x, to);
    }
    let (e6, e8, e15) = (ema(&net, 6), ema(&net, 8), ema(&net, 15));
    let mut cum = vec![f64::NAN; net.len()];
    let mut run = 0.0;
    let mut seen = false;
    for i in 0..net.len() {
        if net[i].is_finite() { run += net[i]; seen = true }
        if seen { cum[i] = run }
    }
    let (m19, m39) = (ema(&cum, 19), ema(&cum, 39));
    let mosc: Vec<f64> = m19.iter().zip(&m39).map(|(a, b)| a - b).collect();
    let imp = |i: usize| e15[i] >= g_("e15") || e8[i] >= g_("e8") || e6[i] >= g_("e6");
    let (drop, mb, mx, md) = (g_("drop") / 100.0, g_("mcc_buy"), g_("mcc_exit"), g_("mcc_drop") / 100.0);
    let (mut top, mut mcc_mode) = (0.0, false);
    for i in from..=to {
        if let Some(t) = cur.as_mut() {
            top = f64::max(top, c[i]);
            if kind == 3 && !mcc_mode && mosc[i] > mb { mcc_mode = true }
            let use_mcc = kind == 2 || (kind == 3 && mcc_mode);
            let exit = if use_mcc { mosc[i] < mx && c[i] < top * (1.0 - md) } else { c[i] <= top * (1.0 - drop) };
            if exit {
                (t.exit_i, t.exit_px, t.signal_i) = (i, c[i], i);
                t.reason = if use_mcc { "oscillator below +450 and close 3/4% under the highest close".into() } else { "close 1.5% under the highest close".into() };
                out.push(cur.take().unwrap());
            }
            continue;
        }
        let buy = if kind == 2 { mosc[i] > mb } else { imp(i) };
        if i < to && buy {
            cur = Some(mk(i, c[i]));
            top = c[i];
            mcc_mode = kind == 3 && mosc[i] > mb;
        }
    }
    finish(out, cur, x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return Some(vec![]) }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let nse = id.ends_with("_nse");
    Some(match id {
        "mbsot_zweig4" => zweig4(x, from, to, g("pct"), g("short") >= 0.5),
        "mbsot_renshaw_thrust" | "mbsot_renshaw_thrust_nse" => renshaw(x, nse, from, to, g("rise"), g("adv"), g("stop"), g("trail_hi") >= 0.5),
        "mbsot_mcclellan" | "mbsot_mcclellan_nse" => breadth(x, nse, 0, from, to, &g),
        "mbsot_appel_impulse" | "mbsot_appel_impulse_nse" => breadth(x, nse, 1, from, to, &g),
        "mbsot_appel_mcclellan" | "mbsot_appel_mcclellan_nse" => breadth(x, nse, 2, from, to, &g),
        "mbsot_appel_combo" | "mbsot_appel_combo_nse" => breadth(x, nse, 3, from, to, &g),
        _ if PATTERNS.contains(&id) => run_patterns(id, x, from, to, &g),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn lane_sd5_and_gti_and_pivots() {
        // SD:5 by hand: flat range 0..10 with closes at 10 -> %D = 100 -> SD = 100
        let r = rows(&vec![(5.0, 10.0, 0.0, 10.0); 12]);
        let x = Ctx::new(&r, &r, &[]);
        let sd = sd5(&x.h, &x.l, &x.c);
        assert!(sd[7].is_nan() && (sd[8] - 100.0).abs() < 1e-9);
        // GTI: two up closes start an up trend with bottom = min(low[i-2], low[i-1]); a 2% break of it turns the trend down
        let h = [10.0, 11.0, 12.0, 13.0, 12.5, 12.0, 12.0, 9.0];
        let l = [9.0, 10.0, 11.0, 12.0, 11.5, 11.0, 10.0, 8.0];
        let c = [9.5, 10.5, 11.5, 12.5, 12.0, 11.5, 11.0, 8.5];
        let g = gti(&h, &l, &c, 2.0);
        assert_eq!(g.trend[2], 1);
        assert_eq!(g.bot[2], 9.0);
        assert_eq!(g.trend[7], -1); // low 8.0 <= 9.0 x 0.98
        assert_eq!(g.top[7], 13.0);
        // pivots: the high of bar 3 becomes a pivot high when bar 4's low (11.5) breaks bar 3's low (12.0)
        let (_, phi) = pivots(&h, &l);
        assert!(phi[3].is_nan() && phi[4] == 13.0);
    }
    #[test]
    fn half_exit_then_break_even() {
        // long at 100 open, ops 95; SD >= 70 on bar 1 -> half out at bar 2 open (104); break-even 100 hits on bar 3
        let r = rows(&[(99.0, 100.0, 98.0, 99.0), (100.0, 103.0, 99.0, 102.0), (104.0, 105.0, 103.0, 104.0), (101.0, 101.0, 97.0, 98.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let sd = vec![50.0, 75.0, 60.0, 50.0];
        let g = Gti { trend: vec![1; 4], bot: vec![f64::NAN; 4], top: vec![f64::NAN; 4] };
        let pl = Plan { dir: 1.0, sig: 0, ent: Ent::Open, stop: 95.0, half: Half::NextOpen, half_ge: true, be: true, trail_now: false };
        let t = sim(&x, &sd, &g, &pl, 70.0, 30.0, 1.0, 3).unwrap();
        assert_eq!(t.legs[0].px, 100.0);
        assert_eq!(t.exits, vec![(2, 104.0, 0.5), (3, 100.0, 0.5)]);
        assert!((t.exit_px - 102.0).abs() < 1e-9);
    }
}
