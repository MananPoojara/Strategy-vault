//! Marvin Appel, Higher Returns from Safe Investments (FT Press / Pearson, 2010).
//! A bond / income book. Its only mechanical timing rules trade high-yield bond and municipal bond
//! mutual funds by their total-return NAV, and its option rule writes S&P 500 calls: the vault has
//! neither fund NAVs nor option prices, so these are NeedsData. Running bond-fund stops on an equity
//! index would be an asset-class swap (not allowed; the book itself says the stops do not suit most
//! stock investments, p.87). Page refs are printed pages. Sources: library/mahrs/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mahrs";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("mahrs_hy_stops", "High-yield bond fund 3% monthly buy and sell stops", "Ch 7 · Risk management: the stop loss (p.84-89)",
            NeedsData("needs month-end total-return NAV (distributions reinvested) of high-yield bond mutual funds, e.g. Delaware Delchester DETWX, Northeast Investors Trust NTHEX; fund data is not in the vault"),
            &["X|\"check the value of those shares (including reinvested interest distributions) at the end of each month, and to sell your shares any time they lose 3% of their most recent peak value\" (p.84)",
              "E|\"reenter the shares ... as soon as the shares rise 3% off of the most recent month-end low\" (p.86)"],
            "Book: DETWX 1988-2009 7.2%/yr, drawdown 17% vs 4.3%/yr buy and hold, 20 transactions; NTHEX 6.7%/yr, drawdown 15% (p.87-89). Weekly evaluation mentioned as better but not specified (p.90)."),
        d("mahrs_muni_stop", "Nuveen High Yield Municipal 6% weekly stops", "Ch 8 · Earn 7% per year, free of federal income tax (p.100-101)",
            NeedsData("needs weekly total-return NAV of the Nuveen High Yield Municipal Bond Fund (NHMAX); fund data is not in the vault"),
            &["X|\"evaluate the total return of your investment ... at the end of each week. You should sell your shares and move to cash whenever the fund slips by more than 6% from its peak weekly value.\" (p.101)",
              "E|\"Then you stay out of the fund until its total return lifts it at least 6% off of its weekly low point.\" (p.101)"],
            "Book: losses in 2007-2008 cut from 45% to below 10% (p.101); the author warns the rule was fitted to one episode (p.102)."),
        d("mahrs_covered_calls", "S&P 500 covered call writing (BXM)", "Ch 12 · Using options to earn income (p.153-166)",
            Live,
            &["E|\"you would buy shares of the S&P 500 SPDR (SPY). For every 100 shares of SPY you purchase, you would sell one call option.\" (p.162)",
              "E|\"you would write the SPY call with the nearest expiration date whose strike price is closest to (but not below) the current price of the shares of SPY\" (p.162); \"There is one options expiration date each month, on the third Friday.\"",
              "X|Write a new call each month (\"sell two covered calls each month\", p.163): the expiring call settles on its last trading day and the next one is written at that close [convention: the book does not give the time of day]"],
            "Options phase 2: real option prices, no pricing model. Position = one unit of the underlying (bought at the close) plus one short call: the call is the nearest monthly expiry after today and the lowest listed strike at or above the close. Written at the close only if that call traded that day (US: 16:00 mid (bid+ask)/2, volume > 0 [labelled: no bid/ask edge]; India: traded close); otherwise the next day is tried. On its last trading day the call settles at intrinsic value on the close (a call above the strike is exercised: same value) and the next month's call is written at that close; each month is one trade. Daily value = underlying move minus the call's own price change (traded close / mid; India settlement price on untraded days, labelled). Return per month = (underlying points + call premium kept) / underlying close at entry. Dividends are not included (the vault's prices are not total-return; CBOE BXM includes them). Book: SPY 1988-2009 via the CBOE BXM index; here SPY, QQQ, IWM (option data 2008/2011-2025) and NIFTY 50 / NIFTY BANK with NSE index options (declared application; the index itself stands in for an index fund)."),
        d("mahrs_high_dividend", "50 highest-dividend-yield large stocks (O'Shaughnessy)", "Ch 11 · Equity ETFs for dividend income (p.141-142)",
            NeedsData("needs a US large-company stock universe with yearly dividend yields; not in the vault"),
            &["E|\"all large company U.S. stocks compared with the 50 with the highest-dividend yield (evaluated once each year)\" (p.141, citing What Works on Wall Street)"],
            "A cited study (1952-2003, 13.6%/yr vs 11.7%), not the author's own rule."),
        d("mahrs_portfolios", "Bond ladders and lifetime-income portfolios", "Ch 4, 13 (p.45-49, p.167-175)",
            NotTestable("portfolio construction (ladders, allocation percentages) with no entry or exit timing"),
            &["F|e.g. \"Allocate 25% of Your Capital to High-Yield Bond Fund Trading\" (p.172)"], "Not a trading rule."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("mahrs_").then_some(("mahrs_income", "Income investing rules", "Rule"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mahrs_covered_calls").then(Vec::new)
}

/// Options phase 2: covered call writing (p.161-163), one trade per monthly cycle; see the card note.
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Result<Vec<crate::options::Pos>, String>> {
    use crate::options::{self as opt, Contract};
    if id != "mahrs_covered_calls" { return None; }
    let Some(chain) = x.options else { return Some(Err(crate::books::oiss::NO_CHAIN.into())) };
    let mut w = opt::Window::new(chain, x, 8);
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        let today = opt::ymd(&x.r[i].date);
        let day = w.at(i);
        let pick = opt::expiries(day).into_iter().find(|&e| opt::ltd(x, e) > today).and_then(|e| {
            opt::strikes(day, e, false).into_iter().find(|s| *s >= x.c[i]).map(|strike| Contract { expiry: e, put: false, strike })
        });
        let Some((c, px0)) = pick.and_then(|c| w.quote(i, c).and_then(|q| q.fill_close()).map(|v| (c, v))) else { i += 1; continue };
        let (j, mut pnl, why, open, marks) = opt::follow(&mut w, x, c, -1.0, i, px0, to, &mut |_| None);
        for (k, v) in pnl.iter_mut().enumerate() { *v += x.c[i + k] - x.c[i]; }
        out.push(opt::Pos { dir: 1.0, s0: x.c[i], entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · long the underlying at {:.2}, sold {} call exp {} at {px0:.2} on {} · {}", x.c[i], c.strike, c.expiry, x.r[i].date, marks.text()) });
        if open || j == i { break; }
        i = j;
    }
    Some(Ok(out))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    option_positions(id, x, from, to, p).map(|r| crate::options::account(r.unwrap_or_default(), from, to, 0.0).trades)
}
