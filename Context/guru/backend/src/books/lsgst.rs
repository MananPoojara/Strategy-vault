//! Larry Swing, "A Practical Guide to Swing Trading" (mrswing.com e-book). The SwingLab FORCESWING scans and the
//! Master Plan entry/exit rules. Every entry and every daily stop update depends on whether the stock opened 50 cents
//! or more away from the prior close; on such gap days the price levels come from the first 5 or 30 minutes of the
//! session, so the rules need intraday data the vault does not have. Page numbers are pdf pages. Sources: library/lsgst/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "lsgst";
const GAP: &str = "intraday bars: on any day the stock opens $0.50 or more from the prior close the Master Plan waits 30 minutes (gap with the trade) or 5 minutes (gap against it) and then sets the buy stop 6 cents beyond that day's high/low so far, and the trailing stop 6 cents beyond that day's low/high so far (pdf p.29-33); kept literally ($0.50 = 0.50 price points), so on index points or NSE rupee prices nearly every day is a gap day. Needs per-ticker series of the high and low of the first 5 and 30 minutes (e.g. OPEN5_HIGH/LOW, OPEN30_HIGH/LOW).";
const P0: &str = "E|No gap (open within $0.50 of the prior close): \"buy the stock the moment it trades 6 cents (1/16) above the previous day's high ... using a buy stop order\" (pdf p.30)";
const P1: &str = "E|Gap of $0.50 or more: \"buy the stock the moment it trades 6 cents above the high of the new day. This would be 30 minutes after the market opens for a gap up or 5 minutes after the market opens for a gap down.\" (pdf p.30)";
const P2: &str = "E|Not executed: \"You can repeat the process for up to 5 trading days.\" (pdf p.31)";
const P3: &str = "X|\"a sell limit order is placed at a price that is 7% above the entry price\" for half the shares (pdf p.30, chart pdf p.32)";
const P4: &str = "X|\"a sell stop ... at 4% below the entry price OR 6 cents below the low of the day that was used for the trade (whichever is higher)\" (pdf p.30)";
const P5: &str = "X|Next days: \"if 6 cents below the previous day's low is higher than yesterday's stop loss, raise the stop loss to this new price\"; on gap days use today's low after waiting 30 min (gap down) / 5 min (gap up) (pdf p.31); the second half is closed only by this trailing stop (pdf p.33)";


pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef {
        id, name, group: "Ch. 6 · The Master Plan + SwingLab scans", session: 0, dir, status: Status::NeedsData(GAP),
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("lsgst_forceswing_long", "FORCESWING long + Master Plan", "Long",
            LONG_RULES, "No published statistics. Case studies p.61-63 (DFXI, PFG, PTEN 2002) are single US-stock trades."),
        d("lsgst_forceswing_short", "FORCESWING short + Master Plan (mirror)", "Short", SHORT_RULES, "No published statistics."),
        d("lsgst_casestudy_long", "Case-study long setup + Master Plan", "Long", CASE_RULES,
            "Book trades: DFXI 4/26/2002 +4.63%, PFG 4/12/2002 +2.49%, PTEN 3/26/2002 +9.37% (US stocks, not in the vault)."),
    ]
}

const LONG_RULES: &[&str] = &["F|SwingLab code: \"MAV20 >=500000 AND CLOSE>12 AND CLOSE > SMAC10 and CLOSE > SMAC20 AND HIGH < HIGH1 and HIGH1 < HIGH2 AND FORCE3<=0 AND FORCE13>=0 AND ADX20>30\" (p.56)",
    "F|Force Index = Volume(today) * (Close(today) - Close(yesterday)); FORCE3 / FORCE13 = 3- and 13-day moving averages of it (p.43; the book's only MA formula is the simple one, p.41)",
    "F|ADX20 = Wilder ADX over 20 days (p.26, p.45)",
    P0, P1, P2, P3, P4, P5];
const SHORT_RULES: &[&str] = &["F|SwingLab code: \"MAV20 >=500000 AND CLOSE>12 AND CLOSE < SMAC10 and CLOSE < SMAC20 AND LOW > LOW1 and LOW1 > LOW2 AND FORCE3<=0 AND FORCE13>=0 AND ADX20>30\" (p.57). The force line repeats the long scan; the comment beside it ('won by the bulls, but still the bears are in control') and p.44 'SELL opportunity: FI-13MA =<0, FI-3MA >= 0' give the mirror FORCE3>=0 AND FORCE13<=0",
    "E|\"A short swing is a mirror image of a long swing\" (pdf p.33): sell stop 6 cents below the previous day's low; buy stop 4% above entry or 6 cents above the trigger day's high, whichever is lower; buy limit for half at 7% below (pdf p.34-35)"];
const CASE_RULES: &[&str] = &["F|\"CLOSE > SMAC50 and CLOSE > SMAC20 ... and HIGH < HIGH1 and HIGH1 < HIGH2 AND ... FORCE3<=0 AND FORCE13>=0\" (case studies p.61-63)", P0, P1, P3, P4, P5];

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lsgst_forceswing_long" => ("lsgst_master", "Larry Swing Master Plan", "FORCESWING long"),
        "lsgst_forceswing_short" => ("lsgst_master", "Larry Swing Master Plan", "FORCESWING short"),
        "lsgst_casestudy_long" => ("lsgst_master", "Larry Swing Master Plan", "case-study long"),
        _ => return None,
    })
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_runs_without_intraday_data() {
        assert!(super::catalogue().iter().all(|d| matches!(d.status, crate::connors::Status::NeedsData(_))));
    }
}
