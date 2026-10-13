//! R. A. Stevenson & R. M. Bear, "Commodity Futures: Trends or Random Walks?", Journal of Finance 25(1), March 1970,
//! p.65-81. Book id `sbcft`. Source: research drive "USB HDD 2/tRADING KNO-2/R.A.Stevenson - Commodity Futures. Trends
//! or Random Walk (Article)/Commodity Futures Trends or Random Walks.pdf" (scan, read in place; copy refused - COPY
//! PENDING, see library/sbcft/README.md). Page refs are J. Finance pages.
//! All three trading techniques are defined on one futures contract's life (July corn / July soybeans, 1957-1968):
//! entry at the contract's first trading day, exit at the last date before delivery, and filter sizes as a percent of
//! the commodity's average price over the period. They need the individual contracts, so they are NeedsData; an index
//! version would need choices the paper does not make (start date, filter base), so none is offered.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sbcft";

const FUT: &str = "needs the individual July corn and July soybean futures contracts (CBOT, 1957-1968: open/high/low/close from each contract's first trading day to delivery); the vault has no futures contracts, and the rules are tied to a contract's first trading day and delivery date";
const NOTE: &str = "Filters z = 1.5%, 3% and 5% 'of the average per bushel trading price of the commodity' (p.76); commissions ~1/2 cent per bushel; constant commitment of 10,000 bushels (p.76). A stop that is gapped through fills at the open (\"the position would be closed out at 245 cents\", p.76). Published: Tables 3 (corn) and 4 (soybeans), p.77-78.";

macro_rules! nd {
    ($id:expr, $name:expr, $dir:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "Sec. IV · Trading techniques (p.75-79)", session: 0, dir: $dir, status: Status::NeedsData(FUT),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("sbcft_tt1", "Trading Technique 1: buy and hold with a z% stop (Houthakker)", "Long",
            &["E|\"Buy the future at the opening on the first day of trading.\" (p.75)",
              "X|\"Place a stop-loss order z per cent below the purchase price. If the stop-loss order is not executed, hold the future until the last possible date prior to delivery. If the stop-loss order is executed, no further position is assumed until the opening of trading in the next July future.\" (p.75)",
              "F|Published net (Table 3 corn 1.5/3/5%: -$753 / -$2,253 / -$1,428; Table 4 soybeans: $7,424 / $4,724 / $14,426; buy-and-hold corn -$5,328, soybeans $8,547)"]),
        nd!("sbcft_tt2a", "Trading Technique 2 Plan A: z% filter with the market, trailing z% stop", "Long/Short",
            &["E|\"Starting with the closing price on the first day of trading, wait for the closing price of the future to move up or down z per cent and then establish a position with the market by buying if the future has gone up z per cent and by selling if the future has gone down z per cent (Plan A).\" (p.75)",
              "X|\"Place a stop-loss order z per cent from the price of the established position. If the price moves in the favorable direction, establish a new stop-loss order each day z per cent from the close on the preceding day. When the price moves in the unfavorable direction, maintain the stop-loss order until it is executed. The closing price on the day that a stop-loss order is executed becomes the same base as the closing quotation was on the first day of trading in the process of establishing a new position.\" (p.75)",
              "F|Published net: corn -$8,937 / $534 / $2,413; soybeans -$6,657 / $12,316 / $17,101 (1.5/3/5%)"]),
        nd!("sbcft_tt2b", "Trading Technique 2 Plan B: z% filter against the market", "Long/Short",
            &["E|\"The same procedure was also applied with the reverse reasoning (Plan B). If price moves z per cent, establish a position against the market.\" (p.75)",
              "X|Same z% stop-loss procedure as Plan A",
              "F|Published net: corn -$2,380 / $927 / -$819; soybeans $156 / -$8,874 / -$10,472"]),
        nd!("sbcft_tt3", "Trading Technique 3: z% move entry, z% target / stop (OCO)", "Long/Short",
            &["E|\"Starting with the opening quotation, when a future moves up (down) z per cent, buy (sell)\" (p.75)",
              "X|\"and hold until the future goes either up or down z per cent. By means of placing two good-till-cancelled orders, one z per cent above and one z per cent below the price at which the position was put on.\" (p.75, n.27); \"The price at which the position is closed out becomes the base to repeat the process.\" (p.76)",
              "F|Published net: corn -$8,630 / -$1,507 / -$651; soybeans -$12,269 / -$9,138 / $12,332"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "sbcft_tt1" => ("sbcft_filters", "Stevenson-Bear filter rules", "TT1 stop"),
        "sbcft_tt2a" => ("sbcft_filters", "Stevenson-Bear filter rules", "TT2 Plan A"),
        "sbcft_tt2b" => ("sbcft_filters", "Stevenson-Bear filter rules", "TT2 Plan B"),
        "sbcft_tt3" => ("sbcft_filters", "Stevenson-Bear filter rules", "TT3"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
