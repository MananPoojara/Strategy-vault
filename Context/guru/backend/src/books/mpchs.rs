//! Lane K80 (13 Nov 2026; authors/titles-1030/K80.tsv): Michael J. Parsons, "Channel Surfing: Riding the Waves of Channels to
//! Profitable Trading" (AuthorHouse, 2005). Book id "mpchs". Catalogue only: every entry and exit is defined by channel lines
//! the trader draws by hand ("you draw a trend line following the edges of the price bars ... it is not a matter of just
//! finding the most bars, but rather the bars that outline the extreme of price activity", p.2-3), redrawn and fanned by
//! judgement as the market accelerates (p.10). Chapter 6 only extends the drawn lines arithmetically (p.89-90).
//! Source copy: research share 'Trading Courses,Seminars,Videos/Michael Parsons - Channel Surfing. Riding the Waves of
//! Channels to Profitable Trading (Book, 214 pgs)/...pdf' (216 PDF pages, ClearScan text layer; printed page = PDF page - 10).
//! Read: contents, ch.1 (p.1-30), ch.2 entries (p.31-42), ch.3, ch.6 (p.85-104), ch.8 (p.121-130), ch.10 headings.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SRC: &str = "Parsons, Channel Surfing (AuthorHouse 2005); printed pages cited (PDF page = printed + 10).";
const DRAWN: &str = "Held: the channel lines are drawn by hand from 'the bars that outline the extreme of price activity' and redrawn or fanned by judgement (p.2-3, 8-10); no objective rule fixes which highs and lows make a line";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), rules,
        notes: SRC, needs_volume: false, needs_vix: false, needs_series: &[], book: "mpchs", claim: None };
    vec![
        d("mpchs_conservative", "Conservative entry: close beyond the inside channel line", "Ch.1 · Entries using channels",
          "Held: the channel lines are drawn by hand from 'the bars that outline the extreme of price activity' and redrawn or fanned by judgement (p.2-3, 8-10); the temporary stop is 'a prior low or a certain limit based on bar movement, such as a maximum of three bars' (p.14-15).",
          &["E|\"Enter when an inside channel line is broken and price bars close beyond that line\" (p.13)",
            "X|\"Exit whenever a channel line is broken, plain and simple\" (p.8); \"The previous inside channel line now becomes your initial stop\" (p.14)"]),
        d("mpchs_aggressive", "Aggressive entry: outside line broken, then the secondary channel breaks", "Ch.1 · Entries using channels", DRAWN,
          &["E|\"Enter when the outside channel line is broken and the developing secondary channel breaks\" (p.15)"]),
        d("mpchs_inside", "Inside entry: as soon as the inside channel line is broken", "Ch.1 · Entries using channels", DRAWN,
          &["E|\"Enter as soon as an inside channel line is broken\" (p.17)"]),
        d("mpchs_rebound", "Rebound (kiss) entry: break of the secondary channel back toward the old inside line", "Ch.1 · Entries using channels", DRAWN,
          &["E|\"Enter at the break of the secondary channel that rebounds toward the previous inside channel line, as long as it does not exceed the prior high or low\" (p.19)"]),
        d("mpchs_trend_entry", "Trend entry: within the third of the channel nearest the inside line", "Ch.1 · Entries using channels", DRAWN,
          &["E|\"Enter an established trend within one-third nearest to an inside channel line\" (p.22)"]),
        d("mpchs_breakout", "Breaking Waves: breakout and mini-channel entries", "Ch.2 · Breaking Waves",
          "Held: breakouts of hand-drawn channels and 'mini-channel' zones, with flags and 'a defined rule such as a set period of time' left to the trader (p.31-42).", &[]),
        d("mpchs_kiss", "Kiss of the channel line: price returning to touch a broken line", "Ch.3 · Kiss of the Channel Line",
          "Held: the same hand-drawn channel lines (p.2-3, 8-10); the chapter adds crowd-psychology reading, no new order or exit (p.43-58).", &[]),
        d("mpchs_repeating", "Repeating channel and trend angle", "Ch.8 · The Repeating Channel and Trend Angle",
          "Held: a grid of hand-drawn lines repeated 'at the same angle, axis and spacing' that the trader adjusts ('By adjusting the angles of this grid', p.121-125).", &[]),
        d("mpchs_options", "Trading options on channel signals", "Ch.10 · Trading Options",
          "Held: the option trades follow the hand-drawn channel signals above, with strikes and expiries chosen by the trader (p.155-180).", &[]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
