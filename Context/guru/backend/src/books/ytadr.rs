//! YT Strategies · Advance Decline Ratio: the team's AmiBroker AFLs in share `Jay/Old Topics/Advance Decline Ratio`
//! (AFLs/, Exit by N days/HSL- PB Optim/V1+V2/, 5DP + ADR/), coded as the AFLs are written (`ytadr_core`).
//!
//! The AFLs read `Foreign("~Advances")` / `Foreign("~Declines")`: Nifty 50 members up / down on the day. They come from
//! extra_data `NIFTY50_ADR.csv` (Advances = the `~ADVANCES` composite of the team's "NIFTY 50 C&P" database; that database
//! has no `~DECLINES`, so Declines are derived from the team's AD Ratio exploration, see the file's README). Daily bars of
//! the selected index (the AFLs SetForeign NIFTY). One position at a time, entries and exits at the AFL's prices.
#![allow(dead_code)]
use crate::books::ytadr_core::{self as core, Afl, Bars};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_adr";
pub const ADV: &str = "NIFTY50_ADR.Advances";
pub const DEC: &str = "NIFTY50_ADR.Declines";
const SERIES: &[&str] = &[ADV, DEC];
/// The main variant a sub-variant is listed under: "YT Strategies · Advance Decline Ratio · <main variant>" (the family page shows the third part as a card).
fn group_of(id: &str) -> &'static str {
    match id {
        "ytadr_longshort" => "YT Strategies · Advance Decline Ratio · Basic Long/Short",
        "ytadr_versions" | "ytadr_versions_5dp_exit" => "YT Strategies · Advance Decline Ratio · Versions (ST and RSI filters)",
        "ytadr_without_rsi" | "ytadr_without_rsi_nday" | "ytadr_two_st" => "YT Strategies · Advance Decline Ratio · Without RSI",
        "ytadr_long_2_11_5dp" | "ytadr_long_2_11_5dp_roc" => "YT Strategies · Advance Decline Ratio · 2-11 / 5DP patch",
        _ => "YT Strategies · Advance Decline Ratio · Five Day Phenomena",
    }
}

struct V {
    id: &'static str,
    afl: Afl,
    name: &'static str,
    variant: &'static str,
    dir: &'static str,
    file: &'static str,
    rules: &'static [&'static str],
}

const R_VERSIONS: &[&str] = &[
    "E|AD ratio = Nifty 50 advances / declines (declines 0 -> 50); optional: the average of the last N ratios instead",
    "E|Long: ratio <= Entry ADR Threshold; Short: ratio >= threshold (Entry Type)",
    "E|Consecutive Candles ON: N bars in a row meet it; OFF: at least N of the last Look Back bars meet it (a look-back of 2 counts as 1)",
    "F|Optional: ST filter (daily / weekly / monthly Supertrend, bullish or bearish phase), Previous N Candles colour filter",
    "E|Enter at the close",
    "X|Exit at the close after N candles, or when the ratio crosses the AD Exit Threshold; ST filter flip; PB % / HSL % (live: at the level, gaps at the open less the buffer)",
];
const R_WO_RSI: &[&str] = &[
    "E|AD ratio = Nifty 50 advances / declines (declines 0 -> 50); optional: the average of the last N ratios instead",
    "E|Long: ratio <= Entry ADR Threshold (Consecutive Candles ON: N bars in a row; OFF: N of the last Look Back bars)",
    "F|Optional: ST filter (daily / weekly / monthly Supertrend, bullish or bearish), Previous N Candles colour filter",
    "E|Enter at the close; no exit on the entry bar",
    "X|Exit at the close after N candles, after N green candles, after N up-close candles, or when the ratio >= AD Exit Threshold",
    "X|Also: ST filter flip, PB % / HSL % (live: at the level, gaps at the open less the buffer)",
];
const R_PATCH: &[&str] = &[
    "F|Trade only inside the patch: 2-11 = from Entry Day bars after a month end to Exit Day bars later; 5DP = from the Entry Day-th last bar of a month to Exit Day bars after the month end",
    "F|Optional filters: ADR (ratio <= threshold, consecutive or within the look-back), RSI, ADX, ST",
    "E|Long at the close of the first bar in the patch that passes the filters",
    "X|Exit at the close after N candles, when the ratio >= AD Exit Threshold, on the 5DP / 2-11 exit day, on an ST flip, or at PB % / HSL %",
];
const R_5DP: &[&str] = &[
    "E|ADR test: ratio <= Entry ADR Threshold on N consecutive bars, or N times within the look-back",
    "E|Buy at the close of the Entry Day-th last trading day of the month when the ADR test holds (and the candle-colour, filter, RSI / ADX / ROC tests)",
    "X|Sell at the close Exit Day bars after the month end; profit booking % and hard SL % at their levels (gaps at the open less the selling buffer)",
    "F|Optional filters: ATHC with % swing, monthly / weekly / daily Supertrend, Bull/Bear % swing",
];

const VARIANTS: &[V] = &[
    V { id: "ytadr_longshort", afl: Afl::LongShort, name: "ADR Long/Short (basic)", variant: "Basic Long/Short", dir: "Long/Short", file: "AFLs/Advance Decline LongShort.afl",
        rules: &["E|AD ratio = advances / declines (declines 0 -> the advances)", "E|Long when the ratio <= Lower Threshold, Short when >= Upper Threshold, at the close", "X|Exit at the close N candles later"] },
    V { id: "ytadr_versions", afl: Afl::Versions, name: "ADR Long/Short Versions (RSI filter)", variant: "Versions (RSI filter)", dir: "Long/Short", file: "AFLs/Advance Decline LongShort_Versions.afl", rules: R_VERSIONS },
    V { id: "ytadr_versions_5dp_exit", afl: Afl::FiveDayExit, name: "ADR Long/Short with 5DP exit", variant: "Versions with 5DP exit", dir: "Long/Short", file: "AFLs/Advance Decline LongShort_with5DayExit.afl", rules: R_VERSIONS },
    V { id: "ytadr_without_rsi", afl: Afl::WoRsi, name: "ADR Long (Without RSI, Green / Up-close exits)", variant: "Without RSI", dir: "Long", file: "AFLs/Advance Decline LongShort_Versions_Without RSI.afl", rules: R_WO_RSI },
    V { id: "ytadr_without_rsi_nday", afl: Afl::WoRsiNDay, name: "ADR Long (Without RSI, N-candle / HSL-PB copy)", variant: "Without RSI, HSL-PB copy", dir: "Long", file: "Exit by N days/HSL- PB Optim/V1+V2/Advance Decline LongShort_Versions_Without RSI.afl", rules: R_WO_RSI },
    V { id: "ytadr_two_st", afl: Afl::TwoSt, name: "ADR Long with two ST filters", variant: "Two SuperTrend", dir: "Long", file: "AFLs/Advance Decline LongShort_Versions_Two_SuperTrend.afl", rules: R_WO_RSI },
    V { id: "ytadr_long_2_11_5dp", afl: Afl::Patch, name: "ADR Long in the 2-11 / 5DP patch", variant: "2-11 / 5DP patch", dir: "Long", file: "AFLs/ADR Long 2-11 and 5DP.afl", rules: R_PATCH },
    V { id: "ytadr_long_2_11_5dp_roc", afl: Afl::PatchRoc, name: "ADR Long in the 2-11 / 5DP patch + ROC", variant: "2-11 / 5DP patch + ROC", dir: "Long", file: "AFLs/ADR Long 2-11 and 5DP ROC.afl", rules: R_PATCH },
    V { id: "ytadr_5dp", afl: Afl::FiveDp, name: "Five Day Phenomena with ADR", variant: "5DP with ADR", dir: "Long", file: "5DP + ADR/FIVE Day Phenomena with ADR.afl", rules: R_5DP },
];

fn variant(id: &str) -> Option<&'static V> {
    VARIANTS.iter().find(|v| v.id == id)
}
pub fn is_variant(id: &str) -> bool {
    variant(id).is_some()
}

const NOTES: &str = "YT strategy from the team's AmiBroker AFL (share Jay/Old Topics/Advance Decline Ratio), coded as the AFL is written; not a book method. \
Breadth: Nifty 50 advances / declines (extra_data NIFTY50_ADR, 2000-01-03 to 2024-08-06; declines derived from the team's AD Ratio exploration because the database has no ~Declines). \
The AFL loop runs over all bars and the backtest takes its Buy / Sell signals inside the period (exits before entries on a bar). AFL defaults are the parameter defaults; the team's runs used 1 share and 0.04 % brokerage.";

pub fn catalogue() -> Vec<StrategyDef> {
    VARIANTS
        .iter()
        .map(|v| StrategyDef {
            id: v.id,
            name: v.name,
            group: group_of(v.id),
            session: 0,
            dir: v.dir,
            status: Status::Live,
            needs_volume: false,
            needs_vix: false,
            needs_series: SERIES,
            book: BOOK,
            rules: v.rules,
            notes: NOTES,
            claim: None,
        })
        .collect()
}

fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef { key, label, def, min: min.min(def), max: max.max(def), step }
}

fn st_params(v: &mut Vec<ParamDef>, on: f64, pd: (f64, f64, f64, f64), fact: (f64, f64, f64)) {
    v.push(p("st_on", "ST Filter Toggle (0 Off, 1 On)", on, 0.0, 1.0, 1.0));
    v.push(p("st_tf", "ST Time Frame (0 Daily, 1 Weekly, 2 Monthly)", 0.0, 0.0, 2.0, 1.0));
    v.push(p("st_pd", "Supertrend Period", pd.0, pd.1, pd.2, pd.3));
    v.push(p("st_fact", "ST Factor", 1.0, fact.0, fact.1, fact.2));
    v.push(p("st_avg", "ST Avg Val (0 Close, 1 Open, 2 High, 3 Low, 4 OC/2, 5 HL/2, 6 HLC/3, 7 OHLC/4)", 5.0, 0.0, 7.0, 1.0));
    v.push(p("st_dir", "ST Entry Filter (0 Bullish, 1 Bearish)", 0.0, 0.0, 1.0, 1.0));
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = variant(id)?;
    let mut out = Vec::new();
    match v.afl {
        Afl::LongShort => {
            out.push(p("entry_type", "Entry Type (0 Long, 1 Short)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("thr_up", "Upper Threshold (Short)", 0.0, 0.0, 50.0, 0.1));
            out.push(p("thr_dn", "Lower Threshold (Long)", 0.0, 0.1, 2.0, 0.1));
            out.push(p("exit_type", "Type of Candle (0 N Candle, 1 AD Ratio)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("exit_n", "N Candles (Keep 0 for OFF)", 0.0, 1.0, 10.0, 0.25));
        }
        Afl::Versions | Afl::FiveDayExit | Afl::WoRsi | Afl::WoRsiNDay | Afl::TwoSt => {
            let nday = v.afl == Afl::WoRsiNDay;
            let wo = matches!(v.afl, Afl::WoRsi | Afl::WoRsiNDay | Afl::TwoSt);
            match v.afl {
                Afl::Versions | Afl::FiveDayExit => st_params(&mut out, 0.0, (5.0, 3.0, 9.0, 2.0), (1.0, 6.0, 1.0)),
                Afl::WoRsiNDay => st_params(&mut out, 1.0, (5.0, 1.0, 10.0, 1.0), (0.0, 10.0, 0.5)),
                Afl::TwoSt => st_params(&mut out, 0.0, (5.0, 5.0, 5.0, 1.0), (1.0, 6.0, 0.5)),
                _ => st_params(&mut out, 0.0, (5.0, 5.0, 5.0, 1.0), (1.0, 5.0, 0.5)),
            }
            if v.afl == Afl::TwoSt {
                out.push(p("st2_on", "ST 2 Filter Toggle (0 Off, 1 On)", 0.0, 0.0, 1.0, 1.0));
                out.push(p("st2_tf", "ST 2 Time Frame (0 Daily, 1 Weekly, 2 Monthly)", 0.0, 0.0, 2.0, 1.0));
                out.push(p("st2_pd", "Supertrend 2 Period", 5.0, 5.0, 5.0, 1.0));
                out.push(p("st2_fact", "ST 2 Factor", 1.0, 1.0, 6.0, 0.5));
                out.push(p("st2_avg", "ST 2 Avg Val (0 Close, 1 Open, 2 High, 3 Low, 4 OC/2, 5 HL/2, 6 HLC/3, 7 OHLC/4)", 5.0, 0.0, 7.0, 1.0));
                out.push(p("st2_dir", "ST 2 Entry Filter (0 Bullish, 1 Bearish)", 0.0, 0.0, 1.0, 1.0));
            }
            if !wo {
                out.push(p("rsi_on", "RSI FILTER (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
                out.push(p("rsi_pd", "RSI Period", 14.0, 1.0, 80.0, 1.0));
                out.push(p("rsi_dir", "RSI Value (0 Below, 1 Above)", 0.0, 0.0, 1.0, 1.0));
                out.push(p("rsi_thr", "RSI Threshold", 1.0, 1.0, 100.0, 1.0));
            }
            out.push(p("prev_on", "Previous N Candles Filter (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("prev_color", "Candle Color (0 Green, 1 Red, 2 Any)", 0.0, 0.0, 2.0, 1.0));
            out.push(p("prev_n", "Previous N Candles", 0.0, 0.0, 20.0, 1.0));
            out.push(p("avg_on", "Average ADR Filter (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            let (amax, tmax) = if matches!(v.afl, Afl::WoRsi) { (20.0, 1.5) } else { (100.0, 50.0) };
            out.push(p("avg_n", "N Previous Candle Avg", 1.0, 1.0, amax, 1.0));
            out.push(p("avg_thr", "Average Threshold", 0.0, 0.1, tmax, 0.1));
            out.push(p("entry_type", if wo { "Entry Type (0 Long, 1 Short = AFL error)" } else { "Entry Type (0 Long, 1 Short)" }, 0.0, 0.0, 1.0, 1.0));
            out.push(p("consec", "Consecutive Candles (0 OFF, 1 ON)", if nday { 1.0 } else { 0.0 }, 0.0, 1.0, 1.0));
            let (lb, ne, thr) = match v.afl {
                Afl::WoRsi => ((0.0, 50.0), (0.0, 20.0), (0.0, 2.0)),
                Afl::WoRsiNDay => ((0.0, 10.0), (0.0, 6.0), (0.0, 1.2)),
                Afl::TwoSt => ((2.0, 10.0), (1.0, 6.0), (0.1, 2.0)),
                _ => ((1.0, 10.0), (1.0, 6.0), (0.1, 50.0)),
            };
            out.push(p("lookback", "Look Back Candles (when Consecutive Candles is OFF)", 1.0, lb.0, lb.1, 1.0));
            out.push(p("n_entry", "N Consecutive Candles", 1.0, ne.0, ne.1, 1.0));
            out.push(p("thr", "Entry ADR Threshold", 0.0, thr.0, thr.1, 0.1));
            match v.afl {
                Afl::FiveDayExit => out.push(p("exit_type", "Type of Candle (0 N Candles, 1 AD Ratio, 2 5DP Entry)", 0.0, 0.0, 2.0, 1.0)),
                Afl::WoRsi | Afl::TwoSt => out.push(p("exit_type", "Type of Candle (0 N Candles, 1 AD Ratio, 2 Green Candle, 3 UpClose)", 0.0, 0.0, 3.0, 1.0)),
                _ => out.push(p("exit_type", "Type of Candle (0 N Candles, 1 AD Ratio)", 0.0, 0.0, 1.0, 1.0)),
            }
            let nmax = match v.afl {
                Afl::FiveDayExit => 10.0,
                Afl::Versions => 50.0,
                _ => 20.0,
            };
            out.push(p("exit_n", "N Exit Candles (Keep 0 for OFF)", 0.0, if v.afl == Afl::TwoSt { 1.0 } else { 0.0 }, nmax, 1.0));
            let (xmin, xmax, xstep) = match v.afl {
                Afl::WoRsiNDay => (0.0, 5.0, 0.5),
                Afl::TwoSt => (1.0, 5.0, 0.5),
                Afl::WoRsi => (0.0, 20.0, 0.1),
                _ => (0.0, 50.0, 0.1),
            };
            out.push(p("exit_ad", "AD Exit Threshold", 0.0, xmin, xmax, xstep));
            out.push(p("exit_at", "Exit at (0 Live, 1 Close)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("pb", "PB %", 0.0, 0.0, 50.0, 0.1));
            out.push(p("hsl", "HSL %", 0.0, 0.0, if nday { 6.0 } else { 20.0 }, if nday { 1.0 } else { 0.1 }));
            out.push(p("buffer", "Buffer %", if nday { 0.25 } else { 0.0 }, 0.0, 10.0, 0.1));
        }
        Afl::Patch | Afl::PatchRoc => {
            let roc = v.afl == Afl::PatchRoc;
            st_params(&mut out, 0.0, (5.0, 3.0, 9.0, 2.0), (1.0, 6.0, 1.0));
            out.push(p("rsi_on", "RSI FILTER (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("rsi_pd", "RSI Period", 14.0, if roc { 4.0 } else { 1.0 }, if roc { 22.0 } else { 80.0 }, if roc { 2.0 } else { 1.0 }));
            out.push(p("rsi_dir", "RSI Value (0 Below, 1 Above)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("rsi_thr", "RSI Threshold", 1.0, if roc { 30.0 } else { 1.0 }, if roc { 70.0 } else { 100.0 }, if roc { 5.0 } else { 1.0 }));
            out.push(p("adx_on", "ADX FILTER (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("adx_pd", "ADX Period", 14.0, 1.0, 80.0, 1.0));
            out.push(p("adx_dir", "ADX Value (0 Below, 1 Above)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("adx_thr", "ADX Threshold", 1.0, 1.0, 100.0, 1.0));
            if roc {
                out.push(p("roc_on", "ROC FILTER (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
                out.push(p("roc_pd", "ROC Period", 14.0, 1.0, 80.0, 1.0));
                out.push(p("roc_dir", "ROC Value (0 Below, 1 Above)", 0.0, 0.0, 1.0, 1.0));
                out.push(p("roc_thr", "ROC Threshold", 1.0, -100.0, 100.0, 1.0));
            }
            out.push(p("patch", "Patch to Trade (0 2-11 Patch, 1 5DP Patch)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("entry_day", "Entry Day", 5.0, if roc { 1.0 } else { 0.0 }, if roc { 8.0 } else { 40.0 }, 1.0));
            out.push(p("exit_day", "Exit Day", 5.0, if roc { 1.0 } else { 0.0 }, if roc { 15.0 } else { 40.0 }, 1.0));
            out.push(p("adr_on", "ADR FILTER (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("entry_type", "Entry Type (0 Long, 1 Short; trades stay long, only the PB / HSL tests turn)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("consec", "Consecutive Candles (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("lookback", "Look Back Candles (when Consecutive Candles is OFF)", 1.0, 1.0, 10.0, 1.0));
            out.push(p("n_entry", "N Consecutive Candles", 1.0, 1.0, 6.0, 1.0));
            out.push(p("thr", "Entry ADR Threshold", 0.0, 0.1, 50.0, 0.1));
            out.push(p("exit_type", "Type of Candle (0 N Candles, 1 AD Ratio, 2 5DP Exit, 3 2-11 Exit)", 0.0, 0.0, 3.0, 1.0));
            out.push(p("exit_n", "N Exit Candles (Keep 0 for OFF)", 0.0, 0.0, 10.0, 1.0));
            out.push(p("exit_ad", "AD Exit Threshold", 0.0, 0.0, 50.0, 0.1));
            out.push(p("exit_at", "Exit at (0 Live, 1 Close)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("pb", "PB %", 0.0, 0.0, 50.0, 0.1));
            out.push(p("hsl", "HSL %", 0.0, 0.0, 20.0, 0.1));
            out.push(p("buffer", "Buffer %", 0.0, 0.0, 10.0, 0.1));
        }
        Afl::FiveDp => {
            out.push(p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0));
            out.push(p("penny", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0));
            out.push(p("turn", "TURNOVER Filter (0 NO, 1 YES; an index has no turnover)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("buffer", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05));
            out.push(p("pt", "Profit Booking %", 3.0, 0.0, 50.0, 1.0));
            out.push(p("hsl", "Hard SL %", 1.0, 0.0, 50.0, 1.0));
            out.push(p("entry_mode", "Entry Type (0 ADR of N consecutive candles below threshold, 1 ADR within N candles below threshold)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("lookback", "Look Back Candles", 1.0, 1.0, 10.0, 1.0));
            out.push(p("n_entry", "Candles Below Threshold", 1.0, 1.0, 6.0, 1.0));
            out.push(p("thr", "Entry ADR Threshold", 0.0, 0.1, 50.0, 0.1));
            out.push(p("entry_day", "Entry Day", 5.0, 1.0, 8.0, 1.0));
            out.push(p("exit_day", "Exit Day", 5.0, 1.0, 8.0, 1.0));
            out.push(p("candle", "CANDLE COLOR (0 GREEN, 1 RED, 2 RANDOM)", 0.0, 0.0, 2.0, 1.0));
            out.push(p("candle_days", "Candle color lookback days", 2.0, 1.0, 5.0, 1.0));
            out.push(p("filter", "Filter Type (0 OFF, 1 ATHC, 2 ST, 3 Bull/Bear)", 0.0, 0.0, 3.0, 1.0));
            out.push(p("athc_exit", "ATHC Swing EXIT %", 20.0, 14.0, 24.0, 2.0));
            out.push(p("flt_tf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0));
            out.push(p("st_point", "ST CALCULATION POINT (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
            out.push(p("flt_pd", "ST FILTER PERIOD", 5.0, 5.0, 25.0, 5.0));
            out.push(p("flt_fact", "ST FILTER FACTOR", 1.0, 1.0, 10.0, 1.0));
            out.push(p("bear", "Swing % For Bear Mkt", 23.0, 10.0, 55.0, 5.0));
            out.push(p("bull", "Swing % For Bull Mkt", 44.0, 10.0, 60.0, 5.0));
            out.push(p("rsi_on", "RSI (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("rsi_pd", "RSI PERIOD", 14.0, 6.0, 22.0, 4.0));
            out.push(p("rsi_val", "RSI VALUE (RSI above it)", 50.0, 30.0, 70.0, 5.0));
            out.push(p("adx_on", "ADX (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("adx_pd", "ADX PERIOD", 8.0, 6.0, 22.0, 4.0));
            out.push(p("adx_val", "ADX VALUE (ADX above it)", 20.0, 5.0, 30.0, 5.0));
            out.push(p("roc_on", "ROC (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("roc_ud", "ROC THR (0 ABOVE, 1 BELOW)", 0.0, 0.0, 1.0, 1.0));
            out.push(p("roc_day", "ROC DAYS", 1.0, 1.0, 8.0, 1.0));
            out.push(p("roc_thr", "ROC THRESHOLD", 1.0, 1.0, 5.0, 1.0));
        }
    }
    Some(out)
}

fn date_num(s: &str) -> i32 {
    let b = s.as_bytes();
    if b.len() < 10 {
        return 0;
    }
    s[0..4].parse::<i32>().unwrap_or(0) * 10000 + s[5..7].parse::<i32>().unwrap_or(0) * 100 + s[8..10].parse::<i32>().unwrap_or(0)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = variant(id)?;
    let schema = params(id)?;
    let (Some(adv), Some(dec)) = (x.series(ADV), x.series(DEC)) else { return Some(Vec::new()) };
    let f = |a: &[f64]| a.iter().map(|&v| v as f32).collect::<Vec<f32>>();
    let d: Vec<i32> = x.r.iter().map(|r| date_num(&r.date)).collect();
    let (o, h, l, c, a, dc) = (f(&x.o), f(&x.h), f(&x.l), f(&x.c), f(adv), f(dec));
    let bars = Bars { d: &d, o: &o, h: &h, l: &l, c: &c, adv: &a, dec: &dc };
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let s = core::signals(&bars, v.afl, &g);
    if s.error.is_some() {
        return Some(Vec::new());
    }
    let out = core::trades(&bars, &s, from, to)
        .into_iter()
        .map(|t| Trade {
            dir: t.dir as f64,
            legs: vec![Leg { i: t.ei, px: t.ep as f64, w: 1.0 }],
            exit_i: t.xi,
            exit_px: t.xp as f64,
            reason: t.why.to_string(),
            entry_kind: "close",
            signal_i: t.xi,
            open: t.open,
            exits: Vec::new(),
        })
        .collect();
    Some(out)
}

/// The settings the grid card runs with: the team's first reference sheet (adr-reference.ts), because the AFL defaults
/// (Entry ADR Threshold 0) never fire. Page runs keep the AFL defaults and every parameter stays editable.
/// Sub-variants with no team setting at all return `None` and stay on the AFL defaults.
pub fn card_params(id: &str) -> Option<Params> {
    let v: &[(&str, f64)] = match id {
        // best row (net profit) of the sheet "Below Thre_Exit by N Days_WOF" in ADR_Below Threshold_Long only_Optim.xlsx: Lower Threshold 1, N Candles 8, 680 trades
        "ytadr_longshort" => &[("entry_type", 0.0), ("thr_dn", 1.0), ("exit_type", 0.0), ("exit_n", 8.0)],
        "ytadr_versions" => &[("st_on", 1.0), ("lookback", 1.0), ("st_tf", 1.0), ("st_fact", 2.5), ("thr", 0.2), ("consec", 1.0), ("n_entry", 1.0), ("exit_n", 16.0)],
        // best row (net profit) of "ADR_Exit By N Candle HSL + PB Optim Daily OF V3.xlsx" Sheet1: 340 trades
        "ytadr_without_rsi_nday" => &[("st_on", 1.0), ("st_tf", 0.0), ("st_fact", 3.0), ("consec", 0.0), ("lookback", 10.0), ("n_entry", 1.0), ("thr", 0.7), ("exit_type", 0.0), ("exit_n", 18.0), ("pb", 4.0), ("hsl", 5.0)],
        // best row of "Entry Uisng AVG ADR_ Monthly Bear & Daily - Bull.xlsx" Full Patch: 131 trades (the first ST factor is not in the workbook; 1 is the only value that gives 131 trades)
        "ytadr_two_st" => &[("st_on", 1.0), ("st_tf", 2.0), ("st_dir", 1.0), ("st_fact", 1.0), ("st2_on", 1.0), ("st2_tf", 0.0), ("st2_dir", 0.0), ("st2_fact", 2.0), ("avg_on", 1.0), ("avg_n", 2.0), ("avg_thr", 1.3), ("consec", 0.0), ("lookback", 6.0), ("n_entry", 2.0), ("exit_type", 1.0), ("exit_ad", 5.0)],
        // no team file for this AFL: BORROWED from Versions "Best setting A" with the AFL's own 5DP Entry exit (135 trades); the team will replace it once it verifies the strategy
        "ytadr_versions_5dp_exit" => &[("st_on", 1.0), ("lookback", 1.0), ("st_tf", 1.0), ("st_fact", 2.5), ("thr", 0.2), ("consec", 1.0), ("n_entry", 1.0), ("exit_type", 2.0)],
        // no team file for this AFL: BORROWED from the 2-11 / 5DP ROC team setting (same AFL without the ROC filter)
        "ytadr_long_2_11_5dp" => &[("thr", 0.1), ("adr_on", 1.0), ("patch", 1.0), ("entry_day", 8.0), ("exit_day", 3.0), ("exit_type", 2.0)],
        "ytadr_without_rsi" => &[("st_on", 0.0), ("exit_type", 3.0), ("consec", 1.0), ("lookback", 1.0), ("thr", 0.7), ("n_entry", 1.0), ("exit_n", 4.0)],
        "ytadr_long_2_11_5dp_roc" => &[("thr", 0.1), ("adr_on", 1.0), ("patch", 1.0), ("entry_day", 8.0), ("exit_day", 3.0), ("exit_type", 2.0)],
        "ytadr_5dp" => &[("entry_day", 5.0), ("exit_day", 2.0), ("pt", 0.0), ("hsl", 0.0), ("thr", 50.0), ("candle", 2.0)],
        _ => return None,
    };
    Some(v.iter().map(|(k, x)| (k.to_string(), *x)).collect())
}

/// Why a run gives no trades although the data is there (the AFL as written stops with an error for these settings).
pub fn afl_error(id: &str, p: &Params) -> Option<&'static str> {
    let v = variant(id)?;
    (matches!(v.afl, Afl::WoRsi | Afl::WoRsiNDay | Afl::TwoSt) && p.get("entry_type").copied().unwrap_or(0.0) >= 0.5)
        .then_some("Entry Type Short: the AFL's short branch reads Close0, which it never defines (AmiBroker stops with an error); only Long runs.")
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    variant(id).map(|v| ("ytadr", "Advance Decline Ratio", v.variant))
}

/// The AFL file of a variant (relative to the share folder `Jay/Old Topics/Advance Decline Ratio`).
pub fn afl_file(id: &str) -> Option<&'static str> {
    variant(id).map(|v| v.file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_params_and_a_family() {
        assert_eq!(catalogue().len(), 9);
        for d in catalogue() {
            assert!(params(d.id).is_some_and(|p| !p.is_empty()), "{}", d.id);
            assert_eq!(family_of(d.id).unwrap().0, "ytadr");
            assert_eq!(d.group.split(" · ").count(), 3, "{}", d.id);
            assert_eq!(d.needs_series, SERIES);
        }
        assert!(params("ytmr_monday_reversal_t1").is_none());
    }

    #[test]
    fn card_settings_are_valid_keys() {
        for id in ["ytadr_versions_5dp_exit", "ytadr_long_2_11_5dp", "ytadr_longshort", "ytadr_without_rsi_nday", "ytadr_two_st", "ytadr_versions", "ytadr_without_rsi", "ytadr_long_2_11_5dp_roc", "ytadr_5dp"] {
            let keys: Vec<_> = params(id).unwrap().into_iter().map(|d| d.key).collect();
            for k in card_params(id).unwrap().keys() {
                assert!(keys.iter().any(|x| x == k), "{id} {k}");
            }
        }
        assert!(card_params("ytmr_monday_reversal_t1").is_none());
    }

    #[test]
    fn defaults_follow_the_afls() {
        let def = |id: &str, k: &str| params(id).unwrap().into_iter().find(|p| p.key == k).unwrap().def;
        assert_eq!(def("ytadr_without_rsi_nday", "st_on"), 1.0);
        assert_eq!(def("ytadr_without_rsi_nday", "consec"), 1.0);
        assert_eq!(def("ytadr_without_rsi_nday", "buffer"), 0.25);
        assert_eq!(def("ytadr_5dp", "pt"), 3.0);
        assert_eq!(def("ytadr_longshort", "thr_dn"), 0.0); // default below the Param minimum is kept
    }
}
