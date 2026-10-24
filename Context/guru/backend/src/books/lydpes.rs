//! Tom Lydon & John F. Wasik, "iMoney: Profitable ETF Strategies for Every Investor" (FT Press 2008).
//! Book id `lydpes`. Text PDF; page refs are printed pages. Source and checks: library/lydpes/.
//! One mechanical rule set (Ch.12, p.149-152): own what is above its 200-day moving average, sell on a
//! drop below it or an 8% dip off the highs, re-enter on a move back above the average or a 5% gain off
//! the low. Model portfolios (p.157-162) are static weights with no trading rule.
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lydpes";
/// Admitted ids (authors/lydon/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lydpes_ma200_8pct"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("lydpes_ma200_8pct", "200-day MA with 8% trailing exit and 5% re-entry (long / cash)", "Ch.12 · iMoney ETF Portfolios", "Long", Status::Live,
          &["E|\"1. Allocate your assets to those areas that are above their trend lines.\" (p.150); trend line = \"the 200-day moving average as the basis for all of its decisions\" (p.149)",
            "X|\"2. Sell whatever positions fall below those trend lines or those that dip 8 percent off their highs, whatever comes first.\" (p.151); \"If the price drops below the long-term trend line or dips to 8 percent off its recent highs, it's time to sell.\" (p.149)",
            "E|Re-entry: \"hang on to the cash until that area moves above its 200-day moving average or gains 5 percent from its recent low.\" (p.152)",
            "F|[Read, B] Daily closes, fill at that close (time of day not given). 'Moving average' = simple (type not given; p.sma=0 = exponential, not the default). 'Moves above' / 'fall below' = the close crossing the average. 'Their highs' = the highest close since the position was bought; 'recent low' = the lowest close since it was sold (the book gives no other window)."],
          "Long or cash, cash earns 0. A first entry needs a cross above the average (the 5% rule only applies after a sale). Book instruments: ETFs for asset classes, sectors and regions; vault = index swap (NIFTY 50, SPX, ...). The plain 200-day MA part alone duplicates apta_ma200_dji / lydetf_ema200 (p.sma=1); the 8% / 5% rules make this a different system. The book prints no results.",
          claim("-", "-", "no results printed", "-", "-")),
        d("lydpes_model_portfolios", "Model ETF portfolios (aggressive to conservative)", "Ch.12 · iMoney ETF Portfolios", "Long",
          Status::NotTestable("static allocations (Tables 12.1-12.5, p.157-162) with no rebalancing date or rule ('Feel free to mix and match', p.156); would also need VO/EFA/EEM/EEB/ILF/TLT/SHY history"),
          &["E|e.g. Aggressive: SPY 10%, VO 15%, IWM 15%, QQQQ 10%, EFA 20%, EEM 10%, EEB 5%, ILF 10%, TLT 5% (Table 12.1, p.157)"],
          "Listed so the pipeline knows these pages were read.", None),
        d("lydpes_short_etfs", "Hedging with inverse / ultra ETFs", "Ch.10 · Hedging Your Bets", "Short",
          Status::NotTestable("no entry or exit rule; 'be sure to have a stop-loss in place' without a size (p.134)"),
          &["E|\"You can use short ETFs for hedging an entire portfolio or focusing on a particular sector\" (p.134)"],
          "Listed so the pipeline knows these pages were read.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lydpes_ma200_8pct" => ("lydpes_ma", "Lydon & Wasik 200-day trend rule", "SMA 200 / 8% / 5%"),
        "lydpes_model_portfolios" | "lydpes_short_etfs" => ("lydpes_nt", "Lydon & Wasik (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "lydpes_ma200_8pct").then(|| vec![
        p("n", "Moving average days", 200., 20., 300., 10.),
        p("dip", "Sell on a dip of % off the high", 8., 2., 20., 1.),
        p("gain", "Re-enter on a gain of % off the low", 5., 1., 20., 1.),
        p("sma", "Simple (1) or exponential (0) average", 1., 0., 1., 1.),
    ])
}

fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut e = f64::NAN;
    v.iter().map(|&x| { e = if e.is_finite() { e + k * (x - e) } else { x }; e }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let len = g("n").round().max(1.) as usize;
    let (dip, gain) = (g("dip") / 100., g("gain") / 100.);
    let c = &x.c;
    let ma = if g("sma") >= 0.5 { sma(c, len) } else { ema(c, len) };
    let ok = |i: usize| i > 0 && ma[i].is_finite() && ma[i - 1].is_finite();
    let mut out = vec![];
    // state: holding (real or virtual), extreme since entry/exit, whether a sale has happened yet
    let (mut hold, mut real, mut ext, mut sold) = (false, None::<Trade>, f64::NAN, false);
    for i in 0..=to {
        if hold {
            ext = ext.max(c[i]);
            let below = ok(i) && c[i] < ma[i] && c[i - 1] >= ma[i - 1];
            let dipped = c[i] <= (1. - dip) * ext;
            if below || dipped {
                if let Some(mut t) = real.take() {
                    t.exit_i = i; t.exit_px = c[i]; t.signal_i = i;
                    t.reason = if below { "close fell below the 200-day average".into() } else { "close 8% off the high since entry".into() };
                    out.push(t);
                }
                hold = false; sold = true; ext = c[i];
            }
        } else {
            ext = if ext.is_finite() { ext.min(c[i]) } else { c[i] };
            let above = ok(i) && c[i] > ma[i] && c[i - 1] <= ma[i - 1];
            let rebound = sold && c[i] >= (1. + gain) * ext;
            if above || rebound {
                hold = true; ext = c[i];
                if i >= from && i < to {
                    real = Some(Trade { dir: 1., legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(),
                        entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                }
            }
        }
    }
    if let Some(mut t) = real { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ema_seed() {
        let e = ema(&[10., 20.], 3);
        assert_eq!(e[0], 10.);
        assert!((e[1] - 15.).abs() < 1e-12);
    }
}
