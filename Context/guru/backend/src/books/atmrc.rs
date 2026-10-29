//! Active Trader "Risk Control And Money Management" binder (Misc Numbers; research drive Trading Knowledge II/
//! @NewsLetters-Mags/Active Trader Magazine/Misc Numbers/Binder - Risk Control And Money Management.pdf, read in place).
//! Book id "atmrc". Only the columns with complete mechanical rules are coded: Thomas Stridsman's 2002 swing-pattern series
//! (TradeStation code printed). Notes, page refs, oracle: library/atmrc/README.md. Own simulator (no jkbwts::sim).
use crate::books::atm03::{done, finish, new_trade, stop_dn, stop_up};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atmrc";
/// Admitted ids (authors/activetrader/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["atmrc_swing_stop", "atmrc_swing_trail"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const ENTRY: &str = "E|\"The strategy goes long today if the following criteria are met (reverse the rules for short trades): 1. the market breaks above yesterday's high; 2. yesterday's close was lower than yesterday's open; and 3. yesterday's high was below the high two days prior to yesterday.\" Code: \"If High < High[2] and Close < Open and Open Next Bar < High Then Buy Next Bar at High Stop\"; short: \"If Low > Low[2] and Close > Open and Open Next Bar > Low Then Sell Next Bar at Low Stop\"";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atmrc_swing_stop", "Stridsman swing pattern · 1% stop, 4.5% target, 9-day time exit", "Looking for a target, May 2002 (binder p.79-83)", "Long/Short", Live,
          &[ENTRY,
            "X|\"ExitLong Next Bar at EntryPrice * LongStop Stop; ExitLong Next Bar at EntryPrice * LongTarget Limit\" (short mirrored); \"If BarsSinceEntry = BarsInTrade+1 Then ExitLong Next Bar at Market\"",
            "F|\"it is probably best to stick with the 1-percent stop and the nine-day trade length\" with \"a profit target of 4.5 percent\" (p.82-83); \"we have decided to feature the stop-loss version\" (Jun 2002 p.87)"],
          "The printed code's random entry gate (EntryTrigger = Random(2)) is the articles' sampling method: \"If you use the system in live trading, you need to get rid of the EntryTrigger variable\" (Jun 2002 p.86), so every signal is taken while flat. Entry fills at the setup bar's high/low (the code requires the next open inside it). Stop and target from the bar after entry; both inside one bar = stop first [B]; gap = open. Time exit: market order at the open after BarsSinceEntry = 9 (trade length = BarsInTrade + 1 = 9 in the code's output). Book: 30 largest Nasdaq-100 stocks, last 10 years (to 2002).",
          c("n/a", "n/a", "~+0.5%", "<= 9 days", "30 Nasdaq-100 stocks, ~1992-2002: about 0.5% average profit per trade (surface charts)")),
        d("atmrc_swing_trail", "Stridsman swing pattern · 1.6% trailing stop from the highest high, 4% target, 9-day time exit", "Happy trails, Jun 2002 (binder p.84-87)", "Long/Short", Live,
          &[ENTRY,
            "X|\"Top = High\" at the setup bar; while long \"Top = MaxList(Top, High); ExitLong Next Bar at Top * LongStop Stop; ExitLong Next Bar at EntryPrice * LongTarget Limit\" (short: Bottom = MinList(Bottom, Low)); \"If BarsSinceEntry = BarsInTrade+1 Then ExitLong Next Bar at Market\"",
            "F|\"the system will work the best with a 1.6-percent trailing stop, a 4-percent profit target and a nine-day time-based stop\" (p.85)"],
          "As the stop version; the trailing stop is 1.6% under the highest high since the setup bar (including the entry bar and later bars, updated at each close). Book: 30 largest Nasdaq-100 stocks; expected average profit about 0.45% per trade.",
          c("n/a", "n/a", "~+0.45%", "<= 9 days", "30 Nasdaq-100 stocks, ~1992-2002: about 0.45% average profit per trade (surface charts)")),
        d("atmrc_meander_shirley", "Dan Shirley's Meander version (QQQ) · fade 2-SD open-to-high/low moves against the 45-day MA slope", "Keeping track of the odds, Sep 2002 (binder p.11-12)", "Long/Short",
          NotTestable("the code is run with a $250 stop on 500 QQQ shares, an absolute $0.50-per-share stop sized for QQQ that has no index equivalent without rescaling; the exit limit is also issued while flat on the setup bar and the column does not say whether it may fill on the entry bar"),
          &["E|\"If (AverageFC(Close,MaLength) - AverageFC(Close[1],MaLength)) > 0 then Buy next bar at open of next bar - (Value2*Mlt) limit\", Value2 = Stddev((open - Low),5), Mlt = 2, MaLength = 45; short mirrored with Value1 = Stddev((High-open),5)",
            "X|\"If Marketposition = 1 then Sell next bar at open of next bar + (Value1*Mlt) limit\" (short mirrored); \"{Run with Stop loss at 250, shares = 500 and fees and slippage of .0225/share}\""],
          "Book: 360 days of QQQ to 5/31/2002 (Table 1).", None),
        d("atmrc_doubly_adaptive", "Doubly adaptive profit objectives (Stridsman sample strategy, INTC) · ATR targets switched by ADX", "Doubly adaptive, Dec 2000 (binder p.17-20)", "Long",
          NotTestable("the entry filter's sentence is broken by the binder layout ('...at an increasing rate, as defined by the 14-day' / 'ADX indicator') so its exact test cannot be confirmed, and the order used when a contracted 1.5 x ATR target is already passed ('the system tells us to exit immediately') is not given"),
          &["E|\"the entry rule is to go long (using a limit order) if the market opens above yesterday's high and then falls back to test that level again ... the market also needs to be trading above its 50-day moving average and at an increasing rate, as defined by the 14-day\"",
            "X|\"As long as the strength of the trend continues to increase (rising ADX), both the profit target and the stop-loss level will be four times the average true range, ATR, (as measured at the time of the entry) from the entry price. But as soon as the trend shows any signs of weakness (declining ADX), both the stop-loss and the profit target will be reduced to 1.5 times the 20-day ATR from the entry price.\""],
          "Book: Intel 1/1988-10/2000, more than 50% winners, profit factor close to 2.", None),
        d("atmrc_other", "Other Risk Control columns (MAE/MFE studies, DDF stops, random-entry trailing stops, position sizing, Kelly, drawdown estimates)", "Binder p.2-74", "Long/Short",
          NotTestable("money-management studies without a complete stand-alone system: the DDF and dynamic-sizing systems are intraday (and the DDF rules are not printed), the trailing-stop study uses a random entry, the MAE/MFE studies use intraday MACD systems or unprinted systems"),
          &["F|Directional Day Filter (Clayburg, Nov 2001 / Feb 2002): 5-minute S&P futures, rules not printed", "F|Exit, trade left (Stridsman, May 2000): four trailing stops tested with a random entry", "F|Dynamic position sizing (Shirley, Jun 2003): 10-minute SPY breakout with NYSE net volume (= atm03_intraday_misc)"],
          "See library/atmrc/README.md for the page-by-page triage.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    match id { s if s.starts_with("atmrc_") => Some(("atmrc", "Active Trader Risk Control binder", v)), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atmrc_swing_stop" => vec![p("stop", "Stop-loss %", 1., 0.2, 2., 0.2), p("tgt", "Profit target %", 4.5, 0.5, 5., 0.5), p("len", "Trade length (BarsSinceEntry for the market exit)", 9., 1., 10., 1.)],
        "atmrc_swing_trail" => vec![p("trail", "Trailing stop % from the highest high", 1.6, 0.2, 2., 0.2), p("tgt", "Profit target %", 4., 0.5, 5., 0.5), p("len", "Trade length (BarsSinceEntry for the market exit)", 9., 1., 10., 1.)],
        "atmrc_meander_shirley" | "atmrc_doubly_adaptive" | "atmrc_other" => vec![],
        _ => return None,
    })
}

/// The swing pattern with a stop (fixed from the entry price, or trailing from the extreme since the setup bar), a profit
/// limit and a market exit at the open after BarsSinceEntry = len. Orders are generated at each close for the next bar.
fn swing(x: &Ctx, from: usize, to: usize, stop: f64, trail: bool, tgt: f64, len: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut out, mut cur, mut ext) = (vec![], None::<Trade>, f64::NAN);
    for j in from.max(2) + 1..=to {
        let i = j - 1;
        if let Some(t) = cur.as_ref() {
            let (e, d, k) = (t.legs[0].px, t.dir, t.legs[0].i);
            // orders generated at the close of i (BarsSinceEntry = i - k)
            if i - k == len {
                let t = cur.take().unwrap();
                out.push(done(t, j, o[j], i, "time exit at the open"));
            } else {
                let (sl, tp) = if d > 0. { ((if trail { ext } else { e }) * (1. - stop / 100.), e * (1. + tgt / 100.)) }
                               else { ((if trail { ext } else { e }) * (1. + stop / 100.), e * (1. - tgt / 100.)) };
                // an open beyond either level fills there first; otherwise, both inside the bar = stop first [B]
                let hit = if d > 0. {
                    if o[j] >= tp { Some((o[j], "profit target (open)")) } else { stop_dn(x, j, sl).map(|p| (p, "stop")).or_else(|| stop_up(x, j, tp).map(|p| (p, "profit target"))) }
                } else if o[j] <= tp { Some((o[j], "profit target (open)")) } else { stop_up(x, j, sl).map(|p| (p, "stop")).or_else(|| stop_dn(x, j, tp).map(|p| (p, "profit target"))) };
                if let Some((px, why)) = hit { let t = cur.take().unwrap(); out.push(done(t, j, px, j, why)); }
            }
            if let Some(t) = cur.as_ref() { ext = if t.dir > 0. { ext.max(h[j]) } else { ext.min(l[j]) }; }
            continue;
        }
        // flat at the close of i: setup on bar i, entry stop on bar j (the code reads "Open Next Bar")
        if h[i] < h[i - 2] && c[i] < o[i] && o[j] < h[i] && h[j] >= h[i] {
            cur = Some(new_trade(1., j, h[i], "stop"));
            ext = h[i].max(h[j]);
        } else if l[i] > l[i - 2] && c[i] > o[i] && o[j] > l[i] && l[j] <= l[i] {
            cur = Some(new_trade(-1., j, l[i], "stop"));
            ext = l[i].min(l[j]);
        }
    }
    finish(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    Some(match id {
        "atmrc_swing_stop" => swing(x, from, to, g("stop"), false, g("tgt"), u("len")),
        "atmrc_swing_trail" => swing(x, from, to, g("trail"), true, g("tgt"), u("len")),
        "atmrc_meander_shirley" | "atmrc_doubly_adaptive" | "atmrc_other" => vec![],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn swing_long_target_hand_checked() {
        // bar 2: high 10.5 < high of bar 0 (11), close < open -> buy stop 10.5 on bar 3 (opens 10.2); bar 4 reaches 10.5 x 1.045
        let b = [(10., 11., 9., 10.), (10., 10.8, 9.5, 10.), (10.4, 10.5, 9.8, 9.9), (10.2, 10.6, 10.1, 10.5), (10.6, 11.2, 10.5, 11.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("atmrc_swing_stop", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i), (1., 3, 10.5, 4));
        assert!((t[0].exit_px - 10.5 * 1.045).abs() < 1e-9);
    }
}
