//! Marvin Appel, Investing with Exchange Traded Funds Made Easy (FT Press / Pearson, 2007).
//! Every timing model in the book switches between two asset classes (large/small caps, growth/value,
//! EAFE/S&P 500, S&P 500/Treasury bills) or holds a fixed multi-asset mix. The vault holds single equity
//! indices only (no Russell 1000/2000 or style indices, no EAFE, no REIT fund average, no bond or
//! Treasury yield series), so every rule is NeedsData with the exact gap. Page refs are printed pages.
//! Sources: library/maetf/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "maetf";


const RATES_NOTE: &str = "[Adapted: 3 unstated details + 1 declared adaptation, R4 #400/#414] Lane K3 (25 Nov 2026): the Treasury yields the card needed are now held (FRED DTB3 3-month bill 1954-, DGS10 10-year 1962-, daily). The rule as printed: Ch.11 monthly checks on the Federal Reserve monthly average yields; out of stocks when the 10-year yield is higher than six months earlier (p.197), when the 90-day bill yield exceeds the 10-year yield by any amount (p.200), or either one (composite, p.203); otherwise hold the S&P 500. Unstated, one per item: (1) a month's yield = the mean of the daily values published in that calendar month, i.e. of the yield days that exist (bond-market holidays are not filled; the H.15 monthly average); (2) the month's reading is acted on at the open of the second trading day of the next month: the Federal Reserve H.15 print of the month's last business day is published on the afternoon of the first trading day of the next month (publication lag, R4 #372), so the book's 'at the start of each month' cannot be the first open; (3) 'higher than six months previously' = strictly above the monthly average six calendar months earlier. Declared adaptation: out of stocks = flat, the Treasury-bill interest is not credited (price return only, no costs). The book's S&P 500 total-return results (Table 11.3: 11.5%/yr vs 10.4% buy and hold, drawdown 23% vs 45%, 1/1965-3/2006) use total returns and T-bill returns, so they are not compared here.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status: Status::NeedsData(why), needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    let mut v = vec![
        d("maetf_one_decision", "One-decision portfolio (annual rebalance)", "Ch 7 · The one-decision portfolio (p.115-129)", "Long",
            "needs monthly total returns of the REIT mutual-fund average, Russell 2000 Value, Lehman (Barclays) Aggregate fixed-income index and 90-day Treasury bills; not in the vault (it has S&P 500 price data only)",
            &["E|\"20 percent S&P 500, 20 percent REITs, 10 percent small-cap value, 20 percent investment-grade bonds, 30 percent cash (90-day U.S. Treasury bills)\" (p.118)",
              "X|\"I recommend rebalancing after holding for one year and one day\" (p.126); less-conservative version without cash: 2/7 S&P 500, 2/7 REITs, 1/7 small-cap value, 2/7 bonds (p.123)"],
            "Book: 1981-2005 calendar-year returns in Table 7.1 (p.120); 10.7%/yr compounded, drawdown 11.2% (5/1980-2/2006)."),
        d("maetf_small_large", "Small- vs large-cap yearly switch (Russell 2000 / Russell 1000)", "Ch 8 · When to live large (p.138-143)", "Long",
            "needs Russell 1000 and Russell 2000 total-return index history (from 1979); the vault's IWM starts 2000 and has no Russell 1000, and the engine runs one ticker at a time",
            &["E|\"Invest for the coming year in whichever of the two alternatives returned more in the previous year.\" (p.138)",
              "X|Each year-end the choice is repeated: hold whichever returned more over the calendar year just ended, switch if the winner changed (p.139)",
              "F|\"On the last trading day of the year, calculate the total returns for each of these indexes. Take whichever of the two returned more as your investment selection for the coming year.\" (p.139)"],
            "Book: 1980-3/2006, 19 of 27 selections correct, 15.3%/yr vs 13.1% (R1000) and 12.6% (R2000), max drawdown -21% (Table 8.1, p.140-141)."),
        d("maetf_growth_value", "Growth vs value relative-strength switch (10% rule)", "Ch 9 · Boring bargains or hot prospects? (p.151-173)", "Long",
            "needs growth and value style index total returns (Russell 2000 Growth/Value, Russell 1000 or S&P 500/Barra Growth/Value); not in the vault",
            &["F|\"Growth versus value rule: A disparity of 10 percent in relative performance between growth and value defines the current long-term trend.\" (p.162)",
              "E|Switch when the relative strength ratio rises 10% off its most recent trough (or falls 10% off its peak) (p.163-165)"],
            "Book applies it to small-cap and large-cap value vs growth (p.166-170)."),
        d("maetf_eafe_sp", "EAFE vs S&P 500 switch (15% rule)", "Ch 10 · When is it safe to drink the water? (p.181-187)", "Long",
            "needs MSCI EAFE (US$) total-return history; not in the vault",
            &["E|\"when the S&P 500 index lags the EAFE by more than 15 percent, it is time to invest abroad. Conversely, when the EAFE lags the S&P 500 by 15 percent or more, invest in the United States.\" (p.181-182)",
              "F|Measured from the latest peak or trough of the EAFE/S&P 500 ratio (Figure 10.4, p.183)",
              "X|Switch back to the other market when the ratio has moved 15% against the holding from its latest extreme (Figure 10.3-10.4, p.182-183)"],
            "Book: trade list in Tables 10.2a/b (p.184-185), 9 of 17 switches successful."),
        d("maetf_rising_rates", "Out of stocks while 10-year yields are rising", "Ch 11 · An indicator of rising interest rates (p.196-197)", "Long",
            "needs monthly average 10-year constant-maturity Treasury yields (Federal Reserve H.15); Treasury yield data is not in the vault",
            &["X|Switch from the S&P 500 into Treasury bills when \"10-year Treasury yields being higher than six months previously, based on monthly average data from the Federal Reserve\" (p.197)",
              "E|Otherwise hold the S&P 500"],
            "Book: S&P 500 1965-2006 (Figure 11.2)."),
        d("maetf_yield_curve", "Yield curve indicator", "Ch 11 · The rules for the yield curve indicator (p.199-202)", "Long",
            "needs monthly average 90-day Treasury bill and 10-year Treasury note yields (Federal Reserve H.15); Treasury yield data is not in the vault",
            &["F|\"At the start of each month, you check the Federal Reserve website for the monthly data on 90-day Treasury bill yields (secondary market) and on 10-year constant maturity Treasury note yields\" (p.200)",
              "X|\"if the 90-day yield exceeds the 10-year yield by any amount, you should exit your equity investments\" (p.200)",
              "E|\"Do not re-enter the stock market until the Fed's monthly data indicates that the yield curve has returned to normal.\" (p.200)"],
            "Book: nine inversions 1965-3/2006 listed in Table 11.2 (p.202); S&P 500 -13.9% vs T-bills +58.6% during them."),
        d("maetf_composite_rates", "Composite interest rate indicator", "Ch 11 · Composite interest rate indicator (p.203-204)", "Long",
            "needs monthly average 90-day T-bill and 10-year Treasury yields (Federal Reserve H.15) and T-bill returns; Treasury yield data is not in the vault",
            &["X|\"The investor should be in Treasury bills if the yield curve is inverted or if 10-year Treasury yields are higher than they were six months earlier.\" (p.203)",
              "E|\"Otherwise, the investor holds the S&P 500. Check these conditions each month\" (p.203)"],
            "Book: S&P 500 1/1965-3/2006, 11.5%/yr vs 10.4% buy and hold, drawdown 23% vs 45% (Table 11.3, p.204)."),
        d("maetf_ultimate", "Ultimate ETF investment program", "Ch 13 · The ultimate ETF investment program (p.223-235)", "Long",
            "combines the REIT/fixed-income core, the EAFE/S&P 500 switch, the composite interest-rate indicator and the growth/value switch: needs all of their data (REITs, bonds, EAFE, style indices, Treasury yields)",
            &["F|\"50 percent of your investment capital should be invested for the long term, equally balanced between REITs and U.S. investment-grade bonds\" (p.227)",
              "E|25%: EFA if the Chapter 10 model favors foreign stocks; else SPY unless \"the yield curve is inverted or interest rates are rising\", then money market (p.227-228)",
              "E|25%: the growth/value model of Chapter 9 picks the style ETF (p.228-230)"],
            "Book results: Table 13.3 and p.231-232."),
    ];
    // Lane D1 pass 2 (R4 #339/#340): the two switches whose rule is exact and price-only run on the held ETF stand-ins (rotation account, SPY calendar).
    for x in &mut v {
        match x.id {
            "maetf_small_large" => { x.status = Status::Live; x.notes = "Declared adaptation (R4 #339/#340/#343, lane D1), always UNVERIFIED: the book switches between the Russell 2000 and Russell 1000 total-return indexes (1980-2006); here IWM (the Russell 2000 tracker, from May 2000) against SPY, and price returns replace total returns (the held series carry no dividends); both are substitutions, not counted. Engine conventions, labelled: the switch is filled at the year's last close; the first decision is the first year-end with a full calendar year of both ETFs behind it (end 2001). Counted card-specific readings (2 of 4): (1) the year's return is measured from the previous year's last close to this year's last close; (2) a tie goes to SPY. Small sample: 25 yearly decisions, 9 switches of holding on one pair."; }
            "maetf_eafe_sp" => { x.status = Status::Live; x.notes = "Declared adaptation (R4 #339/#340/#343, lane D1), always UNVERIFIED: the book uses the MSCI EAFE and S&P 500 indexes from 1970; here EFA (the EAFE ETF the book itself names, from Aug 2001) and SPY; the substitution is not counted. Engine conventions, labelled: the ratio EFA/SPY is checked at each month's last close (the book: once each month) and the switch is filled at that close. Counted card-specific readings (3 of 4): (1) '15 percent' is multiplicative on the ratio from its latest extreme (more than 1.15 x its trough to move to EFA, 0.85 x its peak or less to move back); (2) after a switch the extreme restarts from that month's ratio; (3) the run starts in the S&P 500 with its first extreme taken from the first month-end both ETFs exist. Small sample: 5 switches in 25 years. Start state: the book is in the S&P 500 from 6/30/2001 (Point B, reached from a peak in 2/2000 that our data does not cover); EFA starts 27 Aug 2001, so our run starts in the S&P 500 at the 31 Aug 2001 month-end. In the book's printed overlap (Table 10.2a, p.184) EAFE is favored from 6/30/2002 and no switch back is printed to the end of the book data in 2005/06; ours switches to EFA on 31 May 2002 (a month earlier: EFA instead of the index and our own trough) and also holds it until Oct 2008."; }
            "maetf_rising_rates" | "maetf_yield_curve" | "maetf_composite_rates" => { x.status = Status::Live; x.needs_series = &["DGS10", "DTB3"]; x.notes = RATES_NOTE; }
            _ => {}
        }
    }
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("maetf_").then_some(("maetf_alloc", "ETF asset-allocation models", "Model"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

fn month_avgs(dates: &[&str], v: &[f64]) -> (Vec<String>, Vec<f64>) {
    let (mut keys, mut avg): (Vec<String>, Vec<f64>) = (vec![], vec![]);
    let (mut sum, mut n) = (0.0, 0usize);
    for i in 0..dates.len() {
        let k = &dates[i][..7];
        if keys.last().map_or(true, |l| l != k) {
            if let Some(a) = avg.last_mut() { *a = if n > 0 { sum / n as f64 } else { f64::NAN }; }
            keys.push(k.to_string()); avg.push(f64::NAN); sum = 0.0; n = 0;
        }
        if v[i].is_finite() { sum += v[i]; n += 1; }
    }
    if let Some(a) = avg.last_mut() { *a = if n > 0 { sum / n as f64 } else { f64::NAN }; }
    (keys, avg)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !matches!(id, "maetf_rising_rates" | "maetf_yield_curve" | "maetf_composite_rates") || from > to || to >= x.c.len() { return None; }
    let (y10, y3) = (x.series("DGS10")?, x.series("DTB3")?);
    let dates: Vec<&str> = (0..=to).map(|i| x.r[i].date.as_str()).collect();
    let (keys, a10) = month_avgs(&dates, &y10[..=to]);
    let (_, a3) = month_avgs(&dates, &y3[..=to]);
    // verdict[m]: Some(true) = out of stocks on the complete month m; None = not enough data (stay flat)
    let verdict: Vec<Option<bool>> = (0..keys.len()).map(|m| {
        let inv = (a3[m].is_finite() && a10[m].is_finite()).then(|| a3[m] > a10[m]);
        let ris = (m >= 6 && a10[m].is_finite() && a10[m - 6].is_finite()).then(|| a10[m] > a10[m - 6]);
        match id { "maetf_rising_rates" => ris, "maetf_yield_curve" => inv, _ => ris.zip(inv).map(|(r, i)| r || i) }
    }).collect();
    // position held from the open of each bar. The H.15 print of month m-1's last business day is published on the afternoon of the first
    // trading day of month m, so month m-1's verdict is only known after that day's close (R4 #372): bar 0 of month m still holds the
    // position decided by month m-2's verdict, and the new position starts at the open of the SECOND trading day of month m.
    let mut pos = vec![0i8; to + 1];
    let (mut m, mut k) = (0usize, 0usize);
    for i in 0..=to {
        if i > 0 && dates[i][..7] != dates[i - 1][..7] { m += 1; k = 0; } else if i > 0 { k += 1; }
        while keys[m] != dates[i][..7] { m += 1; k = 0; }
        let v = if k == 0 { m.checked_sub(2) } else { m.checked_sub(1) };
        pos[i] = if v.is_some_and(|v| verdict[v] == Some(false)) { 1 } else { 0 };
    }
    // target_trades(next_open = true) executes tgt[i] at the open of bar i+1: shift the by-bar positions left by one
    let tgt: Vec<i8> = (0..=to).map(|i| if i < to { pos[i + 1] } else { pos[i] }).collect();
    Some(super::rbtie::target_trades(x, &tgt, from, to, true, "monthly rates signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && matches!(def.id, "maetf_rising_rates" | "maetf_yield_curve" | "maetf_composite_rates"),
    label: "Marvin Appel's yield-based S&P 500 switches (Investing with Exchange Traded Funds Made Easy, Ch.11) run on the S&P 500 and SPY (and, as a declared application, the Dow DJI and DIA) with the FRED 3-month bill and 10-year yields; there is no Indian yield series, so they are US-only (R4 #414).",
    basis: "Marvin Appel, Investing with Exchange Traded Funds Made Easy (2007), Ch.11 pp.196-204; FRED DTB3 and DGS10 (extra_data/us)",
};
