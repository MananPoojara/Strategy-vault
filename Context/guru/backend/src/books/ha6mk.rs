//! Lane K81 (13 Nov 2026; authors/titles-1030/K81.tsv): Howard Abell, "The Sixth Market: The Electronic Investor Revolution".
//! Book id "ha6mk". Source copy: research share 'Trading Books/Howard Abell - The Sixth Market. The Electronic Investor
//! Revolution/files/' (HTML e-book, one file per printed page: page_163.html ...). Read: contents (page_v), ch.13 'Reliable
//! Chart Setups' (p.163-177) in full. Reader row: authors/o3/O3.tsv (RULES-COMPLETE).
//! Catalogue only: the six TradeCast setups are mechanical, but every one exits at a fixed dollar amount on Nasdaq stocks (a
//! 1-point stop on the revival trades, a target of at least 3 points on all six). Decision #166: fixed-$ amounts on US stocks
//! stay withheld while the vault holds only split-adjusted stock prices; indices and ETFs are no home for a stock's $3 either.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SRC: &str = "Abell, The Sixth Market, ch.13 'Reliable Chart Setups' (TradeCast setups, p.163-177).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, why, rules| StrategyDef { id, name, group: "Ch.13 · Reliable Chart Setups (TradeCast)", session: 0, dir, status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ha6mk", rules, notes: SRC, claim: None };
    vec![
        d("ha6mk_revival_buy", "Revival Buy: down day closing in its bottom 25%, then up day in its top 25% above the 20-day average", "Long",
          "Held: the initial protective stop is '1 point below your entry point' and the exit target '3 points about the entry point ... or at 50 percent of the average daily range ... whichever is greater' (p.165): fixed dollar amounts on Nasdaq stocks (decision #166: withheld on split-adjusted US stock prices). The average-daily-range length is not stated.",
          &["E|\"You set an entry point that is 1/8 of a point higher than the high on the last trading day\" (p.165)",
            "X|\"you set your initial protective stop (IPS) at 1 point below your entry point\" (p.165)"]),
        d("ha6mk_revival_sell", "Revival Sell: up day closing in its top 25%, then down day in its bottom 25% below the 20-day average", "Short",
          "Held: stop '1 point above the entry point' and target '3 points below the entry point, or 50 percent of the average daily range below the entry point, whichever is greater' (p.167): fixed dollar amounts on Nasdaq stocks (decision #166).",
          &["E|\"You set an entry point at 1/8 below the low of the last trading day\" (p.167)"]),
        d("ha6mk_wedge_buy", "Wedge Buy: two inside days, close above the 20-day average, buy 1/8 over the last high", "Long",
          "Held: the stop is mechanical (1/8 below the last day's low, p.170), but the exit target is at least 3 points above the entry (p.170): a fixed dollar amount on a Nasdaq stock (decision #166).",
          &["E|\"You set an entry point at 1/8 above the high of the last trading day\" (p.170)",
            "X|\"You set your initial protective stop (IPS) 1/8 below the last day's low\" (p.170)"]),
        d("ha6mk_wedge_sell", "Wedge Sell: two inside days, close below the 20-day average, sell 1/8 under the last low", "Short",
          "Held: the exit target is at least 3 points below the entry (p.172): a fixed dollar amount on a Nasdaq stock (decision #166).",
          &["E|\"You set an entry point at 1/8 below the low of the last trading day\" (p.172)"]),
        d("ha6mk_smart_buy", "Smart Buy: three lower highs and lows on black bars, higher than average volume, buy 1/8 over the last high", "Long",
          "Held: the target is at least 3 points above the entry (p.175, a fixed dollar amount on a Nasdaq stock, decision #166), and the volume average ('higher than average volume', p.173) has no length.",
          &["E|\"For at least the previous three trading days, you see consecutive lower highs and lower lows ... the closing price each day is below or equal to the opening price\" (p.173)"]),
        d("ha6mk_smart_sell", "Smart Sell: the reverse of the Smart Buy", "Short",
          "Held: stated only as 'just the reverse of the smart buy' with 'prices ... rising steadily' (p.176), no entry, stop or volume detail; and the Smart Buy's 3-point target is a fixed dollar amount (decision #166).",
          &["E|\"The smart sell is just the reverse of the smart buy\" (p.176)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "TradeCast chart setups (Abell)"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { id.starts_with("ha6mk_").then(Vec::new) }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
