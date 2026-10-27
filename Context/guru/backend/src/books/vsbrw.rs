//! Vijay Singal, *Beyond the Random Walk: A Guide to Stock Market Anomalies and Low-Risk Investing*
//! (Oxford University Press, 2004). Book id `vsbrw`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/Vijay Singal - Beyond Random Walk/Beyond the Random Walk.pdf" (scanned, 402 pdf pages,
//! read in place: the copy into Books/ was refused, see library/vsbrw/README.md "COPY PENDING").
//! Live: the two index trades of Ch 2 "The December Effect: The Trading Process and Implementation" (p.34-37):
//! buy the S&P 500 / Nasdaq 100 at the close of the seventh last trading day of December and sell at the close of
//! the second last trading day (the book's stated strategy, Tables 2.5-2.6), and the full-month alternative the
//! chapter also reports (position at the end of November, held to the end of December: Table 2.6 "S&P 500 Dec.
//! Return"). Every other anomaly in the book needs individual stocks, mutual-fund NAVs, insider filings, index
//! change announcements, merger terms or currency futures (NeedsData), or is explicitly declared untradable
//! (NotTestable). India = instrument swap only (same calendar positions on the Indian index).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "vsbrw";
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["vsbrw_dec5", "vsbrw_decmonth"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const DEC_NOTE: &str = "Trading days are counted from the exchange calendar of the ticker being tested (the December bars of \
that year). A year is used only when its December is complete in the data (a later month exists), so the 7th-last day is known. \
Book instruments: SPDR, March S&P 500 futures, Vanguard/Dreyfus/Scudder S&P 500 index funds and the Nasdaq 100 index (Tables 2.5-2.6, \
1988-2001); here SPX / NDX price indexes (no dividends, so S&P rows run ~0.05-0.1 pt below the book's total returns) and NIFTY 50 \
(instrument swap). All fills at the close. The book reports gross five-day returns and a 0.1% round-trip cost for SPDRs (p.36).";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("vsbrw_dec5", "December effect: seventh-last to second-last trading day of December", "Ch 2 · The December effect (p.34-37)", "Long", Live,
          &["E|\"Buy SPDR, go long in S&P 500 index futures, buy an index mutual fund, or buy the Nasdaq 100 toward the close of trading on the seventh last trading day in December.\" (p.36, rule 1)",
            "X|\"Close the position toward the close of trading on the second last trading day in December. Buying on the seventh last trading day and selling on the second last trading gives the five-day return for capturing the December effect.\" (p.36-37, rule 2)",
            "F|Five-day December return \"measured from the close of the seventh trading day before the end of the year to the close of the second trading day before the end of the year\" (Table 2.3 note, p.29); \"excluding the last day\" (Tables 2.5-2.6 notes, p.35)."],
          DEC_NOTE, claim("14 (1988-2001)", "", "S&P 500 +1.88% / Nasdaq 100 +3.17% per year", "5 days", "Table 2.6 p.35: S&P 500 Dec. 5-Day Return 1988-2001 = 1.88%; Nasdaq 100 = 3.17% (3.18% 1993-2001)")),
        d("vsbrw_decmonth", "December effect: whole month of December", "Ch 2 · The December effect (p.34-37)", "Long", Live,
          &["E|\"There are two alternatives for trading on the December effect: either take a position at the beginning of December or take a position six trading days prior to the end of December\" (p.34): position taken at the close of the last trading day of November.",
            "X|\"results for both the full month and the five-day period are reported\" (p.34): Table 2.6 \"S&P 500 Dec. Return\" = total return for the month of December (p.35) -> sell at the close of the last trading day of December.",
            "F|Read: Table 2.6's month column matches SPX from the last November close to the last December close (e.g. 1991 SPX +11.16% vs book 11.43% total return, 1996 -2.15% vs -1.98%), not to the second-last day (Table 2.5's futures column uses the second-last day, p.35)."],
          DEC_NOTE, claim("14 (1988-2001)", "", "S&P 500 +2.64% per year", "1 month", "Table 2.6 p.35: S&P 500 Dec. Return 1988-2001 = 2.64% (futures, Table 2.5: 2.09%)")),
        d("vsbrw_january", "January effect (small loser stocks)", "Ch 2 · The January effect (p.23-37)", "Long", NotTestable("the book states the January effect cannot be arbitraged and gives no trading rule (\"it is not possible to arbitrage the January effect, though investors can change their trading patterns\", p.23, 33, 37); the anomaly itself needs the bottom size-decile loser stocks (CRSP), not an index"),
          &["F|\"Small loser stocks are known to appreciate considerably in January\" (p.23); \"the January effect occurs only in the bottom 20 percent of all stocks\" (p.33)",
            "X|Advice only: \"Instead of selling losers in December, they should wait until January ... investors should advance their purchases, if any, of small-cap stocks from January to December\" (p.37)"], "", None),
        d("vsbrw_weekend", "Weekend effect (Friday minus Monday return)", "Ch 3 · The weekend effect (p.40-55)", "Long", NotTestable("the book says \"it is not possible to generate large tradable profits from the weekend effect based on financial instruments available at the present time\" (p.49) and \"No efficient trading strategies exist\" (p.53); its stock screen (p.48-52) has no entry/exit rule and needs Nasdaq short-interest and option-volume data"),
          &["F|\"The weekend effect is best defined as a Friday's return minus the following Monday's return\" (p.40); Table 3.2 (p.50): Russell 2000 weekend effect 0.167% 1992-2001",
            "X|Advice only: \"whenever it is possible to alter the timing of trades, investors should sell on Fridays and buy on Mondays\" (p.53)"], "", None),
        d("vsbrw_drift", "Short-term price drift after news", "Ch 4 · Short-term price drift (p.56-77)", "Long/Short", NeedsData("individual stocks with large one-day price moves and the news type behind each move (earnings, analyst ratings, other news); the vault has no stock news feed"),
          &["F|Table 0.1 (p.xii): daily, individual stocks; short-selling only for stocks with negative news; 30-38% annualized in real-time data"], "", None),
        d("vsbrw_industry_mom", "Momentum in industry portfolios", "Ch 5 · Momentum in industry portfolios (p.78-103)", "Long", NeedsData("monthly returns of sector / industry mutual funds (Fidelity Select and Rydex sector funds); no sector funds in the vault"),
          &["F|Table 0.1 (p.xii): mutual funds, 10-12 transactions a year, about 10% per year abnormal return in real-time data"], "", None),
        d("vsbrw_fund_nav", "Mispricing of mutual funds (stale international NAVs)", "Ch 6 · Mispricing of mutual funds (p.104-133)", "Long", NeedsData("daily NAVs of international mutual funds and the 4 p.m. US market move; not in the vault"),
          &["F|Table 0.1 (p.xiii): event based, mutual funds, 15-50 transactions a year, 5-7% annualized"], "", None),
        d("vsbrw_insiders", "Trading by insiders", "Ch 7 · Trading by insiders (p.134-162)", "Long/Short", NeedsData("SEC insider-trading filings (Form 4) and the individual stocks' prices"),
          &["F|Table 0.1 (p.xiii): daily, individual stocks, 30-57% annualized in real-time data"], "", None),
        d("vsbrw_sp500_changes", "Changes to the S&P 500 index", "Ch 8 · Changes to the S&P 500 index (p.163-195)", "Long/Short", NeedsData("S&P 500 addition/deletion announcement and effective dates with the added/deleted stocks' prices"),
          &["F|Table 0.1 (p.xiii): event based, individual stocks, about 50 transactions a year"], "", None),
        d("vsbrw_merger_arb", "Merger arbitrage", "Ch 9 · Merger arbitrage (p.196-231)", "Long/Short", NeedsData("merger announcements, deal terms (cash/stock ratio) and target/acquirer prices"),
          &["F|Table 0.1 (p.xiii): event based, individual stocks, short selling only for stock mergers, 16% annualized"], "", None),
        d("vsbrw_international", "International investing and the home bias", "Ch 10 · International investing (p.232-258)", "Long", NotTestable("portfolio diversification advice; no trading rule (Table 0.1: abnormal return \"Not available\")"),
          &["F|Table 0.1 (p.xiii): continuous, mutual funds / ADRs / ETFs"], "", None),
        d("vsbrw_forward_bias", "Bias in currency forward rates", "Ch 11 · Bias in currency forward rates (p.259-283)", "Long", NeedsData("currency futures prices and short-term interest rates by country (the book trades CME currency futures, Jan 2000-Jun 2002)"),
          &["F|\"A trading strategy based on the forward rate bias is constructed to identify the currency whose interest rate is most different from the dollar interest rate. The currency with the higher interest rate is bought using currency futures.\" (p.281)",
            "F|\"The annual excess return is 9.3 percent with the simple trading strategy, and 11.5 percent with the refined trading strategy.\" (p.281)"], "", None),
        d("vsbrw_other", "Other possible mispricings (Value Line, IPOs, spin-offs, buybacks, ...)", "Ch 13 · Other possible mispricings (p.298-317)", "Long/Short", NotTestable("short descriptions of academic findings without trading rules (p.298-317)"),
          &["F|\"This chapter briefly discusses many other mispricings that have not been disproved by academic research\" (p.298)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "vsbrw_dec5" => ("vsbrw_december", "Singal December effect", "Last five days"),
        "vsbrw_decmonth" => ("vsbrw_december", "Singal December effect", "Whole month"),
        "vsbrw_january" | "vsbrw_weekend" => ("vsbrw_calendar", "Singal calendar anomalies", ""),
        "vsbrw_drift" | "vsbrw_industry_mom" | "vsbrw_fund_nav" | "vsbrw_insiders" | "vsbrw_sp500_changes" | "vsbrw_merger_arb"
        | "vsbrw_international" | "vsbrw_forward_bias" | "vsbrw_other" => ("vsbrw_other", "Singal other anomalies", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "vsbrw_dec5" => Some(vec![
            p("entry_from_end", "Buy at the close of the Nth last trading day of December", 7.0, 2.0, 22.0, 1.0),
            p("exit_from_end", "Sell at the close of the Nth last trading day of December", 2.0, 1.0, 21.0, 1.0),
        ]),
        "vsbrw_decmonth" => Some(vec![]),
        _ => None,
    }
}

fn ym(d: &str) -> (i32, u32) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0)) }

fn trade(x: &Ctx, a: usize, b: usize, why: &str) -> Trade {
    Trade { dir: 1.0, legs: vec![Leg { i: a, px: x.c[a], w: 1.0 }], exit_i: b, exit_px: x.c[b], reason: why.into(), entry_kind: "close", signal_i: b, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    // December bar ranges per year, only for years whose December is complete (a later bar exists).
    // The exchange calendar is known in advance, so the full bar list is used; trades are cut at `to` below.
    let mut decs: Vec<(usize, usize)> = vec![]; // (first Dec bar, last Dec bar)
    let mut i = 0;
    while i < n {
        if ym(&x.r[i].date).1 == 12 {
            let y = ym(&x.r[i].date).0;
            let a = i;
            while i + 1 < n && ym(&x.r[i + 1].date) == (y, 12) { i += 1; }
            if i + 1 < n { decs.push((a, i)); }
        }
        i += 1;
    }
    let mut out = vec![];
    for &(a, b) in &decs {
        let cnt = b + 1 - a;
        let (ei, xi, why) = match id {
            "vsbrw_dec5" => {
                let (e, k) = (g("entry_from_end").round() as usize, g("exit_from_end").round() as usize);
                if e == 0 || k == 0 || k >= e || e > cnt { continue; }
                (b + 1 - e, b + 1 - k, "close of the 2nd last trading day of December")
            }
            "vsbrw_decmonth" => {
                if a == 0 || ym(&x.r[a - 1].date).1 != 11 { continue; } // last November bar must exist
                (a - 1, b, "close of the last trading day of December")
            }
            _ => return None,
        };
        if ei < from || ei > to || ei == to { continue; }
        let mut t = trade(x, ei, xi, why);
        if xi > to { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; }
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(dates: &[&str]) -> Vec<Row> {
        dates.iter().enumerate().map(|(k, d)| Row { ticker: "T".into(), date: d.to_string(), open: 100.0 + k as f64, high: 101.0 + k as f64, low: 99.0 + k as f64, close: 100.0 + k as f64, volume: None }).collect()
    }
    #[test]
    fn seventh_to_second_last_december_day() {
        let mut d: Vec<String> = vec!["2001-11-29".into(), "2001-11-30".into()];
        for day in [3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 24, 26, 27, 28, 31] { d.push(format!("2001-12-{day:02}")); }
        d.push("2002-01-02".into());
        let dr: Vec<&str> = d.iter().map(String::as_str).collect();
        let r = rows(&dr);
        let x = Ctx::new(&r, &[], &[]);
        let n = r.len();
        let t = run("vsbrw_dec5", &x, 0, n - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(x.r[t[0].legs[0].i].date, "2001-12-20"); // 7th last: 20,21,24,26,27,28,31
        assert_eq!(x.r[t[0].exit_i].date, "2001-12-28");
        let m = run("vsbrw_decmonth", &x, 0, n - 1, &Params::new()).unwrap();
        assert_eq!((x.r[m[0].legs[0].i].date.as_str(), x.r[m[0].exit_i].date.as_str()), ("2001-11-30", "2001-12-31"));
    }
}
