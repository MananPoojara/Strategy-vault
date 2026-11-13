//! Lane K80 (13 Nov 2026; authors/titles-1030/K80.tsv): Orville Saari, "Momentum Trend Trader" (e-book, momentumtrend.com,
//! 2003-2004). Book id "osmtt". Catalogue only: the book introduces a momentum line (a MACD with signal length 1, 39/78 bars)
//! and two moving averages of the bar midpoints (27/54), and says itself that it is not a trading system: "It is not a black
//! box trading system. It is up to you the trader to decide where you enter and exit." (PDF p.3). No setup has an exit.
//! Source copy: research share 'USB HDD 2/tRADING KNO-2/Orville Saari - Momentum Trend Trader (85 MB) (momentumtrend.com)/
//! Momentum Trend Trader.pdf' (121 pages, text layer, read in full; the folder's videos were not opened). PDF pages cited.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SRC: &str = "Saari, Momentum Trend Trader (e-book 2003-2004); Momentum Trend Trader.pdf read in full (PDF pages cited).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules| StrategyDef { id, name, group, session: 0, dir, status: Status::NotTestable(why), rules,
        notes: SRC, needs_volume: false, needs_vix: false, needs_series: &[], book: "osmtt", claim: None };
    vec![
        d("osmtt_simple_setup", "Simple but Explosive Setup: new high, pullback while the momentum line keeps rising", "Momentum line · setups", "Long/Short",
          "Held: no thresholds and no exit. 'New high' is 'compared to a few bars earlier', the momentum must move 'in your favor the faster it is moving the better', and examples are judged ('questionable', 'not quite so clear'); 'What method or setup a trader uses to enter or exit a trade is totally at his or her discretion' (PDF p.17, 31-43).",
          &["F|Momentum line = MACD with inputs 39 and 78 and a signal length of 1 (one line), 3-minute chart in the examples (PDF p.14-15)",
            "E|\"The market makes a new high, and then pulls back. All the while the momentum indicator continues to move higher\" (PDF p.31-32); sell the mirror (p.33)"]),
        d("osmtt_zero_cross", "Momentum line crossing zero", "Momentum line · reading momentum", "Long/Short",
          "Held: the zero cross is shown as a reading of momentum ('true momentum is when the line is moving away from the zero line'), not as an order with an exit (PDF p.16, 22-23).", &[]),
        d("osmtt_stoch_momentum", "Oversold stochastic turn with rising longer-term momentum", "Momentum line · stochastics", "Long/Short",
          "Held: no stochastic settings, levels or exit are given; the chart arrows are picked by eye (PDF p.27-29).", &[]),
        d("osmtt_trend_change", "Trend change in price backed by momentum", "Momentum line · trend", "Long/Short",
          "Held: swing highs / lows are defined (two lower highs either side, or a close beyond a 9-bar average, PDF p.44-46), but the entry is a judged 'possible trend change' with shorter momentum lengths chosen per case, and there is no exit (PDF p.47-52).", &[]),
        d("osmtt_high_time_lines", "27 / 54 bar averages of the midpoint: pullbacks to the 'high time price'", "Section II · New Approach", "Long/Short",
          "Held: entries are 'possible buying opportunities' at pullbacks that touch a line, judged with the trend; no stop or exit (PDF p.60-75).",
          &["F|\"The price used for each bar was the mid price, high plus low divided by two\"; simple averages of 54 and 27 bars (PDF p.61)"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
