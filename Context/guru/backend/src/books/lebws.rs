//! Chuck LeBeau's three workshop handouts: "Futures Strategies for Stock Traders" (Live@TAG workbook, INO,
//! 2000), "Precise Exits & Entries" (Traders Library workshop slides, 2008) and "On Average" (TAG XIV,
//! New Orleans 1992). Only one complete rule set is stated: the ATR version of the "simple buy-only system for
//! shares of ABC stock" (Futures Strategies, p.3). Everything else is an indicator lesson, an exit without an
//! entry, an entry without an exit, or a chart illustration. Sources and verification: library/lebws/.
use crate::connors::{sma, true_range, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "lebws";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes, claim: Option<Claim>| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        make("lebws_atr_abc", "Simple ATR system (ABC stock)", "Futures Strategies for Stock Traders (2000) · ATR, p.3-4", "Long", Status::Live,
            &["E|\"Buy when the price rises 1 ATR from the open.\" (p.3) Buy stop at today's open + 1 x ATR(20) [ATR = simple 20-day average true range through yesterday: \"Assume that the 20-day ATR is 1.5 dollars\" (p.3); \"we normally use twenty days to calculate the ATR\" (p.8)]",
              "X|\"Take a profit whenever the profit reaches 6 ATRs.\" (p.3) Limit at entry + 6 x ATR(20)",
              "X|\"Take a loss whenever the loss from our entry point reaches 2 ATRs.\" (p.3) Stop at entry - 2 x ATR(20)"],
            "The handout's own example of turning a dollar system into an ATR system (\"over-simplified system ... an entry, a profit objective and a stoploss\"). It is a stock example; run here on the index (instrument swap). Book silent [class B]: one position at a time, entry only when flat at the open; the profit and loss exits start the day after entry and use each day's ATR through the prior close (as LeBeau's own TradeStation code recomputes ATR exits daily); a gap through an exit fills at the open; if one day touches both, the stop is taken first. p.4 says the ATR stop is '1 ATR' ($6 at a $3 ATR), which contradicts p.3's 2 ATRs and its own $6; p.3's 2 ATRs is the default and 1 ATR is reachable with p.stop_k. No results are printed.",
            None),
        make("lebws_adx15", "ADX crosses above 15 entry", "Futures Strategies for Stock Traders (2000) · charts 1-19", "Long",
            Status::NotTestable("entry only: \"When ADX crosses from below 15 to above 15 - time to buy!\" (charts 1-4, DMI(18)) is paired on different charts with different exits (close below the 20-day MA, Chandelier 3 ATRs, Yo Yo 2 ATRs, Parabolic AF 0.015) and with discretionary re-entries (\"buy dips as long as plus DI above minus DI\"); the text never states one complete system or whether +DI must exceed -DI"),
            &["E|\"When ADX crosses from below 15 to above 15 - time to buy!\" (charts 1-2, p.9-10)",
              "X|\"when the ADX is above 35 and starts to roll over its time to be closing out the position\" (p.2): 'roll over' is not defined"],
            "Chart illustrations on US stocks (Cisco, Dell, IBM, Microsoft, Oracle, GE, Intel, Seagate, Home Depot, EMC), 1998-2000. Not coded: completing it would mean choosing an exit and a DI rule the handout does not state.", None),
        make("lebws_band_dips", "Buy dips at the 2-ATR band when ADX declines", "Futures Strategies for Stock Traders (2000) · chart 6, p.14", "Long",
            Status::NotTestable("chart annotation only: \"Buy on dips when ADX declining (20 day MA - band is 2 ATRs)\" with 'Sell' arrows near the upper band; the sell rule, the ATR length and what 'declining' means are not stated"),
            &["E|\"Buy on dips when ADX declining (20 day MA - band is 2 ATRs)\"; \"Buy on strength when ADX rises\" (chart 6, General Electric)"],
            "Judgement from a chart; no complete rule set.", None),
        make("lebws_atr_breakouts", "ATR breakout entry triggers", "Precise Exits & Entries (2008) · Applications of ATR", "Long",
            Status::NotTestable("entries only, explicitly not fixed: \"These numbers are just examples. Do your research to find best numbers for you.\" No exit is paired with them"),
            &["E|\"Buy if price moves 0.60 ATRs above the Open price. (Opening price breakout)\"",
              "E|\"Buy if price moves 1.75 ATRs above previous Close. (Volatility breakout)\"",
              "E|\"Buy if price moves 2.25 ATRs above current 20-day moving average. (Band breakout)\"",
              "E|\"Buy if price moves 0.25 ATRs above highest high of last 20 days (Channel breakout)\""],
            "Slide 'Applications of ATR'. The exit examples on the next slide (\"Exit if price drops 2 ATRs below your entry\", \"take profits when open profit reaches 4 ATRS or more\") are not attached to any of these entries.", None),
        make("lebws_atr_direction", "ATR trend direction (up / down / sideways)", "Precise Exits & Entries (2008) · Applications of ATR", "Long/Short",
            Status::NotTestable("a market-direction classifier, not a trading rule: no entry, exit or strategy is attached (\"Pick appropriate periods for your time frame\")"),
            &["F|\"If current price is more than two ATRs above 10 days ago price then trend is UP. ... more than two ATRs below ... DOWN. ... less than two ATRs in either direction over 10 days then trend is SIDEWAYS.\""],
            "Used in the workshop to pick a strategy family; the strategies themselves are only named (\"Buy on breakouts\", \"Buy on dips\", \"RSI, Stochastics, Williams %R, Bands\").", None),
        make("lebws_chandelier_steps", "Adjusting the Chandelier Exit", "Precise Exits & Entries (2008) · Exit Strategies", "Long",
            Status::NotTestable("exit only, with no entry; the base multiple is printed as \"3 (?)\" and the anchor as \"highest high or highest close\""),
            &["X|\"A stop is placed 3 (?) Average True Ranges from the highest high or highest close since entry of the trade.\"",
              "X|\"Start new trades with default exit of 3 ATRs from entry.\" \"when open profit reaches four ATRs, reduce Chandelier to two ATRs.\" \"when open profit reaches six ATRs, reduce Chandelier to one ATR.\""],
            "Same exit family as lebtc_chandelier / lebtc_yoyo / lebtc_modified_parabolic (also not testable).", None),
        make("lebws_wilder_adx25", "Wilder ADX above 25 entry", "On Average (TAG XIV, 1992) · charts 9-10, p.8", "Long/Short",
            Status::NotTestable("entry direction and exit are not stated: \"Wilder suggests entering the market when the ADX rises above 25\"; \"any subsequent ADX downturn is a good spot to take some profits\""),
            &["E|\"In his book, New Concepts in Technical Trading Systems, J. Welles Wilder suggests entering the market when the ADX rises above 25.\" (chart 9)",
              "X|\"Wilder suggests that after the ADX rises above the DMI lines, any subsequent ADX downturn is a good spot to take some profits.\" (chart 10)"],
            "The handout's tested system (p.14: 3/12 MA crossovers with Parabolic exits, then only when the ADX is rising; 371 trades / 151 trades) is the CAFM 'simple system', already in the vault as lebcafm_simple_system (NeedsData). Lambert's CCI rule (p.15) = lebcafm_cci_lambert; the TTB Parabolic + 18-day DMI system (chart 13) = lebcafm_adx_parabolic. Printed page 20 (steps 3-5 of the seven steps) is missing from the PDF.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("lebws", "LeBeau workshop handouts");
    Some(match id {
        "lebws_atr_abc" => (f.0, f.1, "ATR system (ABC stock)"),
        "lebws_adx15" => (f.0, f.1, "ADX 15 entry"),
        "lebws_band_dips" => (f.0, f.1, "Band dips"),
        "lebws_atr_breakouts" => (f.0, f.1, "ATR breakouts"),
        "lebws_atr_direction" => (f.0, f.1, "ATR direction"),
        "lebws_chandelier_steps" => (f.0, f.1, "Chandelier steps"),
        "lebws_wilder_adx25" => (f.0, f.1, "Wilder ADX 25"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "lebws_atr_abc" => vec![
            p("atr_n", "ATR length (days)", 20., 2., 60., 1.), p("entry_k", "Entry: ATRs above the open", 1., 0.1, 3., 0.05),
            p("target_k", "Profit target: ATRs above entry", 6., 0.5, 15., 0.25), p("stop_k", "Loss stop: ATRs below entry", 2., 0.25, 6., 0.25),
        ],
        "lebws_adx15" | "lebws_band_dips" | "lebws_atr_breakouts" | "lebws_atr_direction" | "lebws_chandelier_steps" | "lebws_wilder_adx25" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    match id {
        "lebws_atr_abc" => {
            let a = sma(&true_range(x.r), g("atr_n").round().max(1.) as usize);
            let (ek, tk, sk) = (g("entry_k"), g("target_k"), g("stop_k"));
            let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for i in from.max(1)..=to {
                let at = a[i - 1];
                if at.is_nan() { continue; }
                if let Some(t) = cur.as_ref() {
                    let ep = t.legs[0].px;
                    let (stop, tgt) = (ep - sk * at, ep + tk * at);
                    let hit = if o[i] <= stop { Some((o[i], "Loss stop (gap through, at the open)")) }
                        else if o[i] >= tgt { Some((o[i], "Profit target (gap through, at the open)")) }
                        else if l[i] <= stop { Some((stop, "Loss stop: entry - 2 ATR")) }
                        else if h[i] >= tgt { Some((tgt, "Profit target: entry + 6 ATR")) }
                        else { None };
                    if let Some((px, why)) = hit {
                        let mut t = cur.take().unwrap();
                        t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
                        out.push(t);
                    }
                    continue; // flat at the open is required for an entry order (none on an exit day)
                }
                let stop_in = o[i] + ek * at;
                if h[i] >= stop_in {
                    cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: stop_in, w: 1. }], exit_i: i, exit_px: stop_in, reason: String::new(),
                        entry_kind: "stop", signal_i: i - 1, open: false, exits: vec![] });
                }
            }
            if let Some(mut t) = cur {
                t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
                out.push(t);
            }
            Some(out)
        }
        _ => Some(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn abc_by_hand() {
        // 20 flat bars with TR 1 -> ATR 1. Bar 20 opens 100, high 101.5: buy stop 101 fills.
        // Bar 21: ATR through bar 20 = (19 + 1.5) / 20 = 1.025; low 98.9 <= stop 101 - 2.05 -> exit at 98.95.
        let mut v: Vec<(f64, f64, f64, f64)> = (0..20).map(|_| (100., 100.5, 99.5, 100.)).collect();
        v.push((100., 101.5, 100., 101.));
        v.push((100.5, 100.6, 98.9, 99.));
        let rows: Vec<Row> = v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{i:05}"), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let ts = run("lebws_atr_abc", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i), (20, 101., 21));
        assert!((ts[0].exit_px - 98.95).abs() < 1e-9);
    }
}
