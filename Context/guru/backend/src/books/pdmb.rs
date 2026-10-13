//! Paul F. Desmond (Lowry's Reports Inc.), "Identifying Bear Market Bottoms and New Bull Markets" (26 February 2002,
//! 17 pp.). Book id `pdmb`. Source: research drive "USB HDD 2/tRADING KNO-2/Paul
//! Desmond - Indentifying Market Bottoms (Article)/", copied to Books/Paul Desmond/. Refs are the printed page numbers.
//! The paper defines 90% Downside / Upside Days and describes how they cluster around major bottoms, but states no
//! trading rule: no entry order, no holding period and no exit. A rule with no exit is not testable (no exit may be
//! invented), so the one entry is NotTestable. `run` returns None.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "pdmb";

const WHY: &str = "no trading rule: the paper defines 90% Downside/Upside Days and says the combination produces \"very powerful probabilities that a major trend reversal has begun\" (p.5), but gives no entry order, holding period or exit; it also says \"no prudent investment program should be based solely on a single indicator\" (p.12). Data: also needs NYSE daily Points Gained / Points Lost (sum of dollar price changes of advancing / declining NYSE stocks), which the vault does not have (NYSE up/down volume exists in extra_data/us)";

const NOTE: &str = "The Appendix (p.14-17) lists every 90% Down Day and 90% Up Day (or back-to-back 80% Up Days) from 1/1/1960 to 26 Feb 2002 with the DJIA close and a comment; it can serve as a check list for a 90%-day indicator once NYSE Points Gained/Lost data exist. The same idea appears in the MTA journal batch (mtats README: not testable, buy signal only). Not a duplicate of mbhfe_upvol77 (5-day up volume > 77%, Boucher).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "pdmb_90pct_days", name: "Lowry 90% Downside / Upside Days (major-bottom identification)",
        group: "p.3-7 · 90% Days", session: 0, dir: "Long", status: Status::NotTestable(WHY),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|90% Downside Day: \"Downside Volume equaled 90.0% or more of the total of Upside Volume plus Downside Volume, and Points Lost equaled 90.0% or more of the total of Points Gained plus Points Lost\" (p.4; worked example April 3, 2001: 90.8% / 90.7%)",
            "F|90% Upside Day: \"Points Gained equal 90.0% or more of the sum of Points Gained plus Points Lost, and on which Upside Volume equals 90.0% or more of the sum of Upside plus Downside Volume\"; \"or, on rare occasions, two back-to-back 80% Upside Days\" (p.5)",
            "E|\"These two events - panic selling (one or more 90% Downside Days) and panic buying (a 90% Upside Day, or on rare occasions, two back-to-back 80% Upside Days) - produce very powerful probabilities that a major trend reversal has begun\" (p.5) [no order or timing stated]",
            "X|None stated in the paper",
        ],
        notes: NOTE,
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "pdmb_90pct_days" => ("pdmb_90pct_days", "Desmond 90% Days", "Paper definition"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_not_testable_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("pdmb_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NotTestable(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
