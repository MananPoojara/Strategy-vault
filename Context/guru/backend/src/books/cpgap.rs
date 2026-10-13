//! Capstone Trading Systems, "Seven Trading Systems for the S&P Futures" (e-book, (c) 2010) with the TradeStation code
//! (ts-code.txt; same code in the e-book Appendix). Book id `cpgap`. Source: research drive "USB HDD 1/TradeStation
//! Systems & Indicators/Seven Trading Systems for the S&P Futures (capstonetradingsystems.com)/", copied to
//! Books/Capstone Trading Systems/. Refs: e-book chapter.page of the per-chapter PDFs (Ch3-Ch10) and the Appendix.
//! All seven are day trades on a 1-minute @ES.D chart: entries after the first minute or inside clock windows, dollar
//! stop/target inside the day, exit on the close. A daily bar cannot give the first-minute close, the time-window
//! limit/stop fills or the stop-vs-target order, so every system is NeedsData and `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cpgap";

const GF: &str = "needs 1-minute E-mini S&P (@ES.D day session) bars 2005-2010: entry is the close of the first 1-minute bar ('This Bar on Close' on the day's first bar), then the $500 stop (10 ES points) / $200 target (4 points) and the exit on the close are evaluated on the 1-minute path; daily data cannot give the first-minute close or which of stop/target came first. India: would need NIFTY futures 1-minute bars";
const GFR: &str = "needs 1-minute bars of E-mini S&P plus the day-session opens of E-mini Russell, E-mini Midcap and E-mini Dow (data2-data4) and the daily @ES.D for the SMA100 filter (data5), 2005-2010: entry on the first 1-minute bar's close, $500 stop / $200 target on the 1-minute path, exit on the close";
const GC: &str = "needs 1-minute E-mini S&P (@ES.D day session) bars 2005-2010: entries are stop/limit orders placed only inside a clock window (Tim+269 to Tim+669 minutes, or open to Tim+614, or Tim+269 to Tim+569) at OpenD +/- 50% of the gap or the 12/24-bar (minute) extreme, with an intraday dollar stop/target and exit on the close; none of this exists on daily bars";
const NOTE: &str = "Book test: E-mini S&P, 1-minute @ES.D on data1 and daily @ES.D on data2, 05/31/2005-05/28/2010 (Ch3, Ch4, Ch9, Ch10). Dollar stops/targets are per ES contract ($50/point). The SMA100 filter compares 'C of data2' with 'Average(C of data2,100)[1]' on the daily chart. No Live version: an open-to-close daily approximation would change the entry price and drop the intraday stop/target order, which the book states exactly.";

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $why:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::NeedsData($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("cpgap_gap_fill_1", "Gap Fill I (open inside yesterday's range, fade the gap)", "Ch3 · Trading System 1: Gap Fill I", "Long/Short", GF,
            &["E|Short: \"If Date<>Date[1] and Open>Close[1] + 0 and Open<HighD(1) and C of data2<Average(C of data2,100)[1] Then Sell Short (\"GapFill I SE\") This Bar on Close;\" (ts-code.txt, Appendix)",
              "E|Long: \"If Date<>Date[1] and Open<Close[1] - 0 and Open>LowD(1) and C of data2>Average(C of data2,100)[1] Then Buy (\"GapFill I LE\") This Bar on Close;\"",
              "F|\"If the daily close is above the 100 day moving average then long trades are taken. If the daily close is below the 100 day moving average then short trades are taken.\"; position taken \"after the first minute of trading\" (Ch3)",
              "X|\"SetStopLoss(500); SetProfitTarget(200); SetExitonClose;\" - \"a $500 stop loss (10 points in the E-mini S&P), a $200 profit target (4 points in the E-mini S&P), or exit at the close of the trading day\" (Ch3)",
              "F|Published (Ch3 Figure 11, @ES.D 1 min 06/01/05-05/28/10): 392 trades, 75.00% profitable, avg trade $53.76, profit factor 1.60, net $21,075"],
            claim("392", "75.00%", "$53.76", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_2", "Gap Fill II (open outside yesterday's range, fade the gap)", "Ch4 · Trading System 2: Gap Fill II", "Long/Short", GF,
            &["E|Short: \"If Date<>Date[1] and Open>HighD(1) and C of data2<Average(C of data2,100)[1] Then Sell Short (\"GapFill II SE\") cntrcts contracts This Bar on Close;\" (ts-code.txt)",
              "E|Long: \"If Date<>Date[1] and Open<LowD(1) and C of data2>Average(C of data2,100)[1] Then Buy (\"GapFill II LE\") cntrcts contracts This Bar on Close;\"",
              "X|\"SetStopLoss(500); SetExitonClose;\" - \"a $500 stop loss (10 points in the E-mini S&P) or exit at the close of the trading day\" (Ch4)",
              "F|Published (Ch4 Figure 16, 06/01/05-05/28/10): 165 trades, 50.30% profitable, avg trade $80.38, profit factor 1.41, net $13,262.50; without the SMA100 rule avg trade $29 (Figure 18)"],
            claim("165", "50.30%", "$80.38", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_combo", "Gap Fill Combination (Gap Fill I + II in one system)", "Ch7 · Trading System 3: Gap Fill Combination", "Long/Short", GF,
            &["E|Scenarios One-Four exactly as Gap Fill I and Gap Fill II (ts-code.txt System 3)",
              "X|\"SetStopLoss(500); If OpenD(0)>CloseD(1) and OpenD(0)<HighD(1) Then SetProfitTarget(200); If OpenD(0)<CloseD(1) and OpenD(0)>LowD(1) Then SetProfitTarget(200); SetExitonClose;\" - the $200 target applies only to Gap Fill I entries (Ch7)",
              "F|Published (Ch7 Figure 20, 06/01/05-05/28/10): 557 trades, 67.68% profitable, avg trade $61.65, profit factor 1.51, net $34,337.50"],
            claim("557", "67.68%", "$61.65", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_fill_rank", "Gap Fill Rank (Gap Fill I only when ES has the weakest/strongest open of four index futures)", "Ch8 · Trading System 4: Gap Fill Rank", "Long/Short", GFR,
            &["F|\"D1=(Open-Close[1])/Close[1]\" for ES and D2-D4 for data2-data4 (E-mini Russell, E-mini Midcap, E-mini Dow); \"Condition1=D1<D2 and D1<D3 and D1<D4; Condition2=D1>D2 and D1>D3 and D1>D4;\" (ts-code.txt System 4)",
              "E|Short: Gap Fill I Scenario One \"and C of data5<Average(C of data5,100)[1] and Condition1\"; Long: Scenario Two \"and C of data5>Average(C of data5,100)[1] and Condition2\", This Bar on Close",
              "X|\"SetStopLoss(500); SetProfitTarget(200); SetExitonClose;\"",
              "F|Published (Ch8 Figure 26, 06/01/05-05/28/10): 45 trades, 84.44% profitable, avg trade $123.61, profit factor 4.35, net $5,562.50"],
            claim("45", "84.44%", "$123.61", "intraday", "06/01/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_1", "Gap Continue I (open inside range: midpoint stop or 12-minute-low limit, noon-4pm)", "Ch9 · Trading System 5: Gap Continue I", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim+269 and Time<Tim+669 and OpenD(0)>CloseD(1) + PTS and OpenD(0)<HighD(1) and EntriesToday(date)<2 and C<(OpenD(0) + .5*AbsValue(OpenD(0)-CloseD(1))) Then Buy Next Bar at OpenD(0) + .5*AbsValue(OpenD(0)-CloseD(1)) Stop;\" and, if C is above that level, \"Buy Next Bar at Lowest(L,12)[1] Limit\"; PTS = 4 (ts-code.txt System 5)",
              "E|Short mirrored: \"OpenD(0)<CloseD(1) - PTS and OpenD(0)>LowD(1)\", \"Sell Short Next Bar at OpenD(0) - .5*AbsValue(OpenD(0)-CloseD(1)) Stop\" or \"Highest(H,12)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 1100, StpLs 2000 - \"$2000 stop loss ... 40 points in the E-mini S&P), a $1100 profit target ... 22 points in the E-mini S&P), or exit at the close\" (Ch9 rule 8)",
              "F|Published (Ch9 Fig.32 page, E-mini S&P 05/31/05-05/28/10): 184 trades, 57.07% profitable, avg trade $133.15, profit factor 1.92, net $24,500; E-mini Russell 139 trades, 66.91%, avg $265.25"],
            claim("184", "57.07%", "$133.15", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_2", "Gap Continue II (open outside range: 24-minute extreme limit, open-3:45pm)", "Ch10 · Trading System 6: Gap Continue II", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim and Time<Tim+614 and OpenD(0)>HighD(1) + PTS and EntriesToday(date)<2 and C>OpenD(0) Then Buy (\"GapContinue II LE\") cntrcts contracts Next Bar at Lowest (L,24)[1] Limit;\" PTS = 0 (ts-code.txt System 6)",
              "E|Short: \"OpenD(0)<LowD(1) - PTS and EntriesToday(date)<2 and C<OpenD(0) Then Sell Short ... Next Bar at Highest(H,24)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 550, StpLs 800 (Ch10)",
              "F|Published (Ch10 Figure 39, E-mini S&P 05/31/05-05/28/10): 315 trades, 59.68% profitable, avg trade $92.14, profit factor 1.73, net $29,025"],
            claim("315", "59.68%", "$92.14", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
        nd!("cpgap_gap_continue_2b", "Gap Continue II Bonus (gap held, noon-3pm CST, one entry)", "Ch10 · Trading System 7: Gap Continue II Bonus", "Long/Short", GC,
            &["E|Long: \"If Time>=Tim+269 and Time<Tim+569 and OpenD(0)>HighD(1) + PTS and EntriesToday(date)<1 and C>OpenD(0) and LowD(0)>HighD(1) Then Buy (\"GapContinue IIB LE\") cntrcts contracts Next Bar at Lowest (L,24)[1] Limit;\" (ts-code.txt System 7)",
              "E|Short: \"OpenD(0)<LowD(1) - PTS and EntriesToday(date)<1 and C<OpenD(0) and HighD(0)<LowD(1) Then Sell Short ... Next Bar at Highest(H,24)[1] Limit\"",
              "X|\"SetProfitTarget(PrfTg); SetStopLoss(StpLs); SetExitonClose;\" PrfTg 550, StpLs 800",
              "F|Published (Ch10 Figure 40, E-mini S&P 05/31/05-05/28/10): 122 trades, 62.30% profitable, avg trade $110.25, profit factor 2.27, net $13,450"],
            claim("122", "62.30%", "$110.25", "intraday", "05/31/2005-05/28/2010, E-mini S&P 1-min")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cpgap_gap_fill_1" => ("cpgap_gap_fill", "Capstone Gap Fill", "I (inside range, $200 target)"),
        "cpgap_gap_fill_2" => ("cpgap_gap_fill", "Capstone Gap Fill", "II (outside range, no target)"),
        "cpgap_gap_fill_combo" => ("cpgap_gap_fill", "Capstone Gap Fill", "Combination"),
        "cpgap_gap_fill_rank" => ("cpgap_gap_fill", "Capstone Gap Fill", "Rank (4 index futures)"),
        "cpgap_gap_continue_1" => ("cpgap_gap_continue", "Capstone Gap Continue", "I (inside range)"),
        "cpgap_gap_continue_2" => ("cpgap_gap_continue", "Capstone Gap Continue", "II (outside range)"),
        "cpgap_gap_continue_2b" => ("cpgap_gap_continue", "Capstone Gap Continue", "II Bonus (gap held)"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_needsdata_and_owned() {
        let c = super::catalogue();
        assert_eq!(c.len(), 7);
        for s in &c {
            assert!(s.id.starts_with("cpgap_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
