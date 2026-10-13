//! Alexander Elder, "Entries & Exits: Visits to 16 Trading Rooms" (Wiley, 2006). Book id "elee". Catalogue only.
//! Source copy: Books/Alexander Elder/(Wiley Trading) Alexander Elder - Entries & Exits_ ... (2006).pdf (its text layer
//! is a +28 character shift; decoded and read in full, numbers checked on the page images). Page refs = printed pages.
//! The book is 16 traders' discretionary case studies; the only mechanical systems are listed below. README: library/elee/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "elee", claim: None };
    vec![
        d("elee_schutzman", "Fred Schutzman's 89-day SMA / Bollinger / 34-day channel breakout", "Ch.2 Fred Schutzman", "Long/Short",
          Status::NotTestable("one of its two exits is \"an MACD divergence (we prefer to identify them visually, even though we have written code to recognize them)\" (p.36); the divergence code is not given, so the system cannot be run as stated"),
          &["F|\"1. Define the major trend and trade only in its direction. Here we use an 89-day SMA\" (p.34)",
            "E|\"2. Place Bollinger bands one standard deviation above and one standard deviation below the MA; watch for a breakout in the direction of the MA\"",
            "E|\"3. Place lines extending forward from each 34-day high and a 34-day low; watch for a breakout in the direction of the MA\"",
            "E|\"To enter a trade, all three signals have to occur, but not necessarily at the same time: The moving average has to turn in the direction of the trade and prices have to penetrate both the Bollinger band and the 34-day line, in the same direction. For #2 and #3 the latest signal simply has to be in our favor and not against us.\" (p.34)",
            "X|\"This system has two exit rules - either an MACD divergence (we prefer to identify them visually ...) or when prices move against the trend and close on the opposite side of their 89-day MA.\" (p.36)"],
          "Book trade: 4/19/2002 buy Euro @ 89.12, 3/13/2003 sell @ 107.88 (p.36); cotton loss p.42. \"This system backtested well; it was profitable but the drawdowns were too high. We rejected it\" (p.36). Running only the MA-close exit would drop a book rule, so it is not done by default."),
        d("elee_testa_ema", "Ray Testa's EMA crossover (long and short)", "Ch.5 Ray Testa Jr.", "Long/Short",
          Status::NotTestable("\"I use a crossover of fairly short-term exponential moving averages\" (p.99): the EMA lengths are not given"), &[], ""),
        d("elee_fcsi", "Peter Tatarnikov's Forex Club Sentiment Index system", "Ch.13 Peter Tatarnikov", "Long/Short",
          Status::NeedsData("FCSI = 100 x long positions / all positions among Forex Club's clients, per currency, daily (proprietary, not published), plus the currencies' daily closes; the vault has neither"),
          &["F|\"FCSI = (L/T) x 100; L = the number of long positions in any currency; T = the number of all positions in any currency\" (p.227)",
            "E|\"we go short, entering the market on close if FCSI rises above 70. We go long, market on close, if FCSI falls below 30.\" (p.227-228)",
            "X|\"We cover shorts when it drops below 60 and sell longs when it rises above 40.\" (p.228)",
            "F|\"If there are fewer than 10 positions in a currency, that signal becomes suspect.\" (p.231)"],
          "\"We are up 12% for the first quarter of 2005\" (p.228). Book trade: short euro on close, covered when FCSI fell below 60 (p.229-230)."),
        d("elee_discretionary", "The other 13 trading rooms (Haskell, Perolo, Rabbani, McMahon, Appel, Brenke, Lovvorn, Buffalin, Weis, Doane, Makhmudov, Willain, Knapp)", "Ch.1-16", "Long/Short",
          Status::NotTestable("discretionary chart reading (divergences, trendlines, 'value zone', squeeze plays, Wyckoff, fundamentals) or intraday tick data (Willain's Effective Volume); no complete mechanical rule"),
          &["E|e.g. Lovvorn's squeeze play: \"a pullback to the fast EMA accompanied by shrinkage ... I look for an entry when the ranges are the smallest since the pullback began\" (p.162)",
            "X|e.g. Perolo: \"take profits on half of my position at a point between half and two-thirds of the distance to the stop loss\" (p.54)"], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
