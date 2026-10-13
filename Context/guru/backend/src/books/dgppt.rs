//! Daryl Guppy, "Precision Pattern Trading" (guppytraders.com): manual pattern.pdf (27 slides) + workbook patternwb.pdf (30 slides).
//! Book id "dgppt". Catalogue only: every pattern rests on hand-drawn trend lines and support/resistance ("Good pattern
//! trading is based on chart reading", slide 5). Source copies: Books/Daryl Guppy/pattern.pdf, patternwb.pdf (all slides read).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, why, notes| StrategyDef { id, name, group, session: 0, dir, status: Status::NotTestable(why), rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgppt", claim: None };
    vec![
        d("dgppt_asc_triangle", "Ascending triangle breakout", "Triangles", "Long",
          &["F|\"Find the resistance level / Find the significant lows / Project trend and resistance lines forward / Project trend line back / Measure base - Must be continuous price action in a single direction / Project target\" (pattern slide 17)",
            "E|\"Select entry point / Aggressive - before breakout / Conservative - after breakout\"; \"Define downside risk\" (slide 18)",
            "X|Target = base measurement projected from the breakout (\"Usually minimum targets\", slide 18)"],
          "trend line needs 'a minimum of three price reaction points' chosen by eye; resistance = 'areas where price has the most rebounds' (slides 9, 14)",
          "pattern slides 9-18. Claim (no test details): \"High probability pattern / 80% success in bull markets / 70% success in bear markets\" (slide 18)."),
        d("dgppt_desc_triangle", "Descending triangle (short)", "Triangles", "Short",
          &["E|Down-sloping triangle short; \"Double downside risk\" (pattern slide 19)"],
          "hand-drawn trend line and support", "pattern slide 19. Claim: \"85% in bear markets / 70% in bull markets\"."),
        d("dgppt_sym_triangle", "Symmetrical triangle", "Triangles", "Long/Short",
          &["E|\"low probability pattern ... Probability of success 50% all markets\"; \"Divide developing pattern into thirds / Pattern has a time limit\" (pattern slides 20-21)"],
          "hand-drawn converging trend lines; entry area A/B/C chosen on the chart", "pattern slides 20-21."),
        d("dgppt_bull_flag", "Bullish flag", "Flags", "Long",
          &["F|\"Starts with 1 to 5 large range days in a single direction / Flag is a retracement with parallel sides / Top edge is set first / Bottom edge is inferred\" (pattern slide 22)",
            "X|\"Price target is based on flagpole height / Breakout is powerful / Slow breakouts show pattern failure\" (slide 22)"],
          "'large range days' has no size; the flag edges are drawn and 'inferred' by eye; no stop is given",
          "pattern slide 22, workbook answers (Eli Lilly 10%, Blue Coat Systems 43%, MMI Holdings 'Wrong. Broken flagpole'). Claim: \"85% in bull markets. 75% in bear markets\"."),
        d("dgppt_stirrup", "Stirrup pattern", "Stirrup", "Long",
          &["F|\"continuation pattern with a set target / Starts with a sudden price drop not dividend related / Key feature is upward sloping triangle / Once breakout is confirmed, pattern measurement and price projection is applied / Pattern projection is not based on triangle\" (pattern slides 23-25)"],
          "'sudden price drop' and the triangle are not defined numerically; needs dividend dates", "pattern slides 23-25."),
        d("dgppt_workbook", "Pattern recognition exercises", "Workbook", "Long",
          &["E|\"Identify the pattern / Define the pattern / Establish pattern base measurement / Complete price projection / Set stop / Calculate return. Assume entry tomorrow at today's closing price\" (patternwb slides 7-28)"],
          "visual identification exercises on hand-picked stocks", "patternwb slides 7-28: answers on single US/Singapore/HK stocks (Eli Lilly, Blue Coat, Akamai, China Aviation Oil, Neurobiological, Lantrovision, MMI, Elec & Eltek, Nera Telecom, Tan Chong)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
