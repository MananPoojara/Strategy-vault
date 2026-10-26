//! William Dunnigan, "New Blueprints for Gains in Stocks and Grains" (1956), Parts I-III (stock market),
//! Traders' Masterclass reprint pp.1-93. Part IV ("Gains in Grains") is module wdgig. Refs/oracle: library/wdnbp/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wdnbp";
/// Admitted ids (authors/dunnigan/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["wdnbp_ndb", "wdnbp_ndb_rule2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NDB_NOTE: &str = "DJIA rule (\"We have applied the method to the Dow-Jones industrial average back to 1921\", p.62); runs on DJI/SPX and NIFTY 50 (instrument swap). Swings on closing prices: a Top is set when the close falls 4% below the highest close since the last Bottom; a Bottom when the close rises 4% above the lowest close since the last Top. Rule 1 low/high = the extreme close of the latest Downswing/Upswing (the swing must exist, i.e. its 4% has been reached); it tests either of the two Bottoms (Tops) set before it. Signals are acted on only when they reverse the position; stop-and-reverse, no stops (\"a trade would be held until a reversal signal is given ... no stops have been used\", p.63). Fill at the signal day's close = the book's test (\"signal prices used in this study\"); next_open=1 = the book's practical execution (\"A trade would be made on the morning following the day of the signal\"). The book prints no trade list or totals for this barometer (only \"The evidence favors the New Dow Barometer by a large margin\").";

const ORTHODOX_GAP: &str = "Hamilton/Rhea Dow Theory needs the Dow-Jones Rail (Transportation) average for confirmation (not in the vault) and its tools are undefined: \"double tops\", \"lines\" (\"a month or more\", \"at the most four points\", Hamilton inconsistent per Moment/Cowles), \"refuse to make new highs\", \"secondary rally\" - judgement, not mechanical (p.38-41, 52-55).";
const MOMENT_GAP: &str = "Moment's rules as quoted are incomplete for a program: the \"primary price change since the end of the last preceding secondary reaction\" needs a starting secondary that the quote does not define, and the penetration point moves when the average \"hesitates for a few days\" (no number of days). His full paper (with the 1897-1932 signal list) is \"now out of print\" and not reproduced (p.59-60). The book itself says Rule 2 of the New Dow Barometer approximates it.";
const SQRT_GAP: &str = "Offered as an untested suggestion (\"Traders using orthodox Dow principles may well make tests using the foregoing suggestions\", p.71): price basis not fixed (closing vs high-low is left open, Table 3.1 example: \"high-low prices, perhaps\"), and the stop/quick-profit columns of Table 3.1 are only \"may prove to be a good guide\". Table 3.1 itself is an image and lists stocks priced up to ~$200.";
const TABLE31_GAP: &str = "Research suggestions with options left open (\"perhaps\", \"if you should not buy on the basis of the above\"), pyramiding and quick-profit fractions (\"half or more\") at the trader's choice (p.74-75).";
const QUICK_GAP: &str = "Table 3.2 is a profit-taking table only (net gain per share by US-stock price level incl. 1950s commissions); it needs a separate timing method (\"if followed faithfully in conjunction with a reasonably good timing method\", p.77) - no entry rule.";
const FAST_GAP: &str = "List of ~170 'fast-moving' 1950s NYSE stocks by group (p.78-81) - a US stock universe the vault does not have; selection, not a trading rule.";
const THRUST_STOCKS_GAP: &str = "Weekly Thrust Method for individual US stocks: signals are the Gains in Grains rules read in dollars (\"think of dollars instead of cents\"), with TOB/TOT ($1/2 below to $1 above), Thrust limits and loss/quick-profit/break-even columns taken from the Work Sheet by stock price level ($ table, p.93). Needs weekly data on individual US stocks priced inside that table (the 1950s fast-movers); an index level (thousands of points) is outside the dollar table.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("wdnbp_ndb", "New Dow Barometer (Rules 1 and 2)", "Part III · A New Dow Barometer", "Long/Short", Live, &[
            "F|\"A downswing is a decline of 4% or more. The low price in a Downswing is a bottom. An upswing is a rally of 4% or more. The high price in an Upswing is a top. (Using closing prices, we chart each movement amounting to 4% or more.)\" p.63",
            "E|Buy Rule 1: \"If the current low price in a Downswing is within the range '2 1/2% above to 1 1/2% below' either of the last two Bottoms, buy on a 5% rise above said low price.\" p.63",
            "E|Buy Rule 2: \"If the above rule does not give a buy signal, buy if closing prices rise 2% above the last Top.\" p.63",
            "E|Sell Rule 1: \"If the current high price in an Upswing is within the range '2 1/2% below to 1 1/2% above' either of the last two Tops, sell on a 5% decline below said high price.\" p.63-64",
            "E|Sell Rule 2: \"If the above rule does not give a sell signal, sell if closing prices decline 2% below the last Bottom.\" p.64",
            "X|\"The assumption has been made that a trade would be held until a reversal signal is given ... no stops have been used\"; long and short (\"trading in both the long and short sides of the market\") p.62-63",
        ], NDB_NOTE),
        d("wdnbp_ndb_rule2", "New Dow Barometer · Rule 2 only (\"conventional method\")", "Part III · A New Dow Barometer", "Long/Short", Live, &[
            "F|Swings of 4% or more on closing prices p.63",
            "E|\"buy if closing prices rise 2% above the last Top\"; \"sell if closing prices decline 2% below the last Bottom\" p.63-64",
            "X|Held until the reversal signal; the book compares \"the conventional method (Rule 2) and the New Dow Barometer (Rule 1 and 2)\" p.62",
        ], NDB_NOTE),
        d("wdnbp_dow_orthodox", "Orthodox (Hamilton) Dow Theory checklist", "Part III · The old Dow Theory", "Long/Short", NotTestable(ORTHODOX_GAP), &[
            "E|\"The Final confirmation of the inauguration of a major bull market is given when: Both averages rise above the high points established in the preceding secondary rally.\" p.40",
        ], ORTHODOX_GAP),
        d("wdnbp_moment", "Samuel Moment's modified Dow Theory", "Part III · Moment's tested improvement", "Long/Short", NotTestable(MOMENT_GAP), &[
            "F|\"A secondary reaction is a movement in a bull or bear market that retraces 33 1/3% or more of the primary price change since the end of the last preceding secondary reaction.\" p.59",
            "E|Reverse when the last secondary low/high is penetrated by 1 point (average under 100), 1% (100-150), 1.5% (151-200), 2% (over 201) p.59-60",
            "X|\"A signal remains in force until reversed.\" p.60",
        ], MOMENT_GAP),
        d("wdnbp_sqrt_dow", "Square-root Dow principles (0.15 swings, 0.05 penetration)", "Part III · Square roots with the Dow principles", "Long/Short", NotTestable(SQRT_GAP), &[
            "F|\"An upswing is a rise of 0.15 or more [on a square-root scale] ... A downswing is a decline of 0.15 or more\" p.71",
            "E|\"Cover shorts and buy when prices rise 0.05 or more above the preceding [top]\"; \"Sell longs and go short when prices decline 0.05 or more below the preceding bottom\" p.71",
        ], SQRT_GAP),
        d("wdnbp_table31", "Table 3.1 experiment (square-root New Dow principles)", "Part III · Square roots with New Dow principles", "Long", NotTestable(TABLE31_GAP), &[
            "E|\"Get an Original Buy Signal (reversal signal) on a rise of $2 1/8 above a Double Bottom or an Ascending Bottom\" (stock near $50) p.75",
        ], TABLE31_GAP),
        d("wdnbp_quick_profits", "Quick profits in fast-moving stocks (Table 3.2)", "Part III · Quick profits", "Long", NotTestable(QUICK_GAP), &[
            "X|Take the desired net gain per share from Table 3.2 by purchase price p.76-77",
        ], QUICK_GAP),
        d("wdnbp_fast_movers", "Fast-moving stocks list", "Part III · Fast-moving stocks", "Long/Short", NeedsData(FAST_GAP), &[
            "F|Trade the listed high-volatility stocks, rotating by group p.78-81",
        ], FAST_GAP),
        d("wdnbp_thrust_stocks", "The Thrust Method in Stocks (weekly)", "Part III · The thrust method in stocks", "Long/Short", NeedsData(THRUST_STOCKS_GAP), &[
            "F|\"turn now to 'Gains in Grains' and as you read think of dollars instead of cents, and think of weekly data instead of daily data\" p.88",
            "X|\"On original Reversal Signal, place half of capital in Fund S and half in Fund L. In all commitments limit loss to amount listed in Column 4 of Work Sheet\"; Fund S quick profit Column 5, later Repeat trades Column 7; Fund L stop to break even after Column 6; \"Always close out both funds, and reverse your position, when Reversal Signal comes.\" p.91",
        ], THRUST_STOCKS_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "wdnbp_ndb" => Some(("wdnbp_ndb", "Dunnigan New Dow Barometer", "Rules 1+2 (book)")),
        "wdnbp_ndb_rule2" => Some(("wdnbp_ndb", "Dunnigan New Dow Barometer", "Rule 2 only")),
        _ if id.starts_with("wdnbp_") && catalogue().iter().any(|s| s.id == id) => Some(("wdnbp_other", "Dunnigan New Blueprints · other stock methods", "Not runnable here")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let base = vec![
        p("swing", "Swing size % (closes)", 4., 2., 8., 0.5),
        p("pen", "Rule 2 penetration % beyond last Top/Bottom", 2., 0., 5., 0.5),
        p("next_open", "Fill: 0 = signal close (book's study), 1 = next open (book's practical execution)", 0., 0., 1., 1.),
    ];
    match id {
        "wdnbp_ndb" => {
            let mut v = base;
            v.extend([
                p("test_in", "Rule 1 test zone: % on the near side of the old Bottom/Top", 2.5, 1., 5., 0.5),
                p("test_out", "Rule 1 test zone: % beyond the old Bottom/Top", 1.5, 0., 4., 0.5),
                p("rise", "Rule 1 rise/decline % from the test low/high", 5., 3., 8., 0.5),
            ]);
            Some(v)
        }
        "wdnbp_ndb_rule2" => Some(base),
        _ => None,
    }
}

/// New Dow Barometer on closes. `rule1` false = Rule 2 only.
pub fn ndb(c: &[f64], o: &[f64], from: usize, to: usize, sw: f64, pen: f64, rule1: Option<(f64, f64, f64)>, next_open: bool) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let (mut leg, mut ext) = (0i8, 0usize); // leg 1 = Upswing (last pivot a Bottom), -1 = Downswing
    let (mut lo0, mut hi0) = (0usize, 0usize); // before the first pivot
    let mut tops: Vec<f64> = vec![];
    let mut bots: Vec<f64> = vec![];
    let last = if next_open { to.saturating_sub(1) } else { to };
    for i in 0..=to {
        // swing update on today's close
        match leg {
            1 => {
                if c[i] > c[ext] { ext = i; } else if c[i] <= c[ext] * (1.0 - sw) { tops.push(c[ext]); leg = -1; ext = i; }
            }
            -1 => {
                if c[i] < c[ext] { ext = i; } else if c[i] >= c[ext] * (1.0 + sw) { bots.push(c[ext]); leg = 1; ext = i; }
            }
            _ => {
                if c[i] < c[lo0] { lo0 = i; }
                if c[i] > c[hi0] { hi0 = i; }
                // first pivot: whichever extreme the current close has moved `sw` away from
                let up = c[i] >= c[lo0] * (1.0 + sw);
                let dn = c[i] <= c[hi0] * (1.0 - sw);
                if up && (!dn || lo0 > hi0) {
                    bots.push(c[lo0]); leg = 1;
                    ext = (lo0..=i).fold(lo0, |a, k| if c[k] > c[a] { k } else { a });
                } else if dn {
                    tops.push(c[hi0]); leg = -1;
                    ext = (hi0..=i).fold(hi0, |a, k| if c[k] < c[a] { k } else { a });
                }
            }
        }
        // signals at today's close
        let mut sig = 0i8;
        let mut why = "";
        if let Some((tin, tout, rise)) = rule1 {
            // latest Downswing low (running, or the Bottom just set) vs the two Bottoms before it
            let low = match leg { -1 => Some((c[ext], bots.len())), 1 => bots.last().map(|&b| (b, bots.len() - 1)), _ => None };
            if let Some((lw, nb)) = low {
                let tested = bots[..nb].iter().rev().take(2).any(|&b| lw <= b * (1.0 + tin) && lw >= b * (1.0 - tout));
                if tested && c[i] >= lw * (1.0 + rise) && !tops.is_empty() { sig = 1; why = "Buy Rule 1 (test of Bottom, 5% rise)"; }
            }
            let high = match leg { 1 => Some((c[ext], tops.len())), -1 => tops.last().map(|&t| (t, tops.len() - 1)), _ => None };
            if sig == 0 { if let Some((hw, nt)) = high {
                let tested = tops[..nt].iter().rev().take(2).any(|&t| hw >= t * (1.0 - tin) && hw <= t * (1.0 + tout));
                if tested && c[i] <= hw * (1.0 - rise) && !bots.is_empty() { sig = -1; why = "Sell Rule 1 (test of Top, 5% decline)"; }
            } }
        }
        if sig == 0 {
            if tops.last().is_some_and(|&t| c[i] >= t * (1.0 + pen)) { sig = 1; why = "Buy Rule 2 (2% above last Top)"; }
            else if bots.last().is_some_and(|&b| c[i] <= b * (1.0 - pen)) { sig = -1; why = "Sell Rule 2 (2% below last Bottom)"; }
        }
        if sig == 0 || i < from || i > last || cur.as_ref().is_some_and(|t| t.dir == sig as f64) { continue; }
        let (fi, fp) = if next_open { (i + 1, o[i + 1]) } else { (i, c[i]) };
        if let Some(mut t) = cur.take() {
            t.exit_i = fi; t.exit_px = fp; t.signal_i = i; t.reason = format!("{} -> {}", t.reason, why); out.push(t);
        }
        cur = Some(Trade { dir: sig as f64, legs: vec![Leg { i: fi, px: fp, w: 1.0 }], exit_i: fi, exit_px: fp, reason: why.to_string(),
            entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = format!("{} -> open", t.reason); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(0.0));
    let rule1 = if id == "wdnbp_ndb" { Some((g("test_in") / 100.0, g("test_out") / 100.0, g("rise") / 100.0)) } else { None };
    Some(ndb(&x.c, &x.o, from, to, g("swing") / 100.0, g("pen") / 100.0, rule1, g("next_open") >= 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rule1_buys_on_5pct_rise_after_testing_a_bottom() {
        // top 100, bottom 90, top 99, test low 91 (within 2.5% above 90), rally to 95.55 = 91*1.05
        // (the 91 close is also a 5% decline from the 99 top that tested 100 -> Sell Rule 1 first)
        let c = [100., 95., 90., 94., 99., 95., 91., 94., 95.6, 96.];
        let t = ndb(&c, &c, 0, 9, 0.04, 0.02, Some((0.025, 0.015, 0.05)), false);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 6, 8));
        assert_eq!((t[1].dir, t[1].legs[0].i), (1.0, 8));
        // Rule 2 only: 2% above the last Top (99 -> 100.98) never reached
        assert!(ndb(&c, &c, 0, 9, 0.04, 0.02, None, false).is_empty());
    }
}
