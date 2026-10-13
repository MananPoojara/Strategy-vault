//! Joe Krutsinger, "$36,000 Worth of Trading Systems?" (1997 system manual, joekrut.com). Book id `jk36k`.
//! Source copies: Books/Joe Krutsinger/36000 Worth of Trading Systems/MANUAL.DOC (text) and the same pages reprinted
//! in Building Winning Trading Systems Manual.pdf printed pp.43-72 (with the performance printouts and the JK1 Quick
//! System sheet, p.47). The JK20-JK31 code exists only in KRUTSI.ELA / TSSTUFF.ELA / the .ELS: their outer PKWARE-DCL
//! layer decodes, but each code record is a second, unidentified compressed/encrypted stream, so the rules are unread.
//! Library: library/jk36k/. Simulator conventions: see `jkbwts`.
use super::jkbwts::buy_monday;
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jk36k";
const NOCODE: &str = "rules not printed: the manual gives only the market, stop and 'ideas' (e.g. 'Volatility Breakout, Trend Switching'); the code is only in the compressed ELA archive, which could not be decoded";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let nt = |id, name, dir, rules, notes, cl| d(id, name, "Undisclosed code (ELA only)", dir, NotTestable(NOCODE), rules, notes, cl);
    vec![
        d("jk36k_jk1_buy_monday", "JK1BuyMonday", "S&P 500 day trade", "Long", Live,
          &["E|If DayOfWeek(Date) = 5 (Friday) then Buy at Market (next bar's open) (Quick System sheet, BWTS p.47)",
            "X|Money Management stop $1,250 = 2.5 S&P points at $500/point, from the entry",
            "X|Close all trades at end of day session (exit at the entry day's close)",
            "F|Prose (MANUAL.DOC / BWTS p.46): 'buy one contract at the market every Monday on the open, a $1,250 money management stop, and an order to exit all trades market on close'"],
          "BWTS p.46: S&P 500 Index - CME daily 07/25/90-07/24/97, $55 commission: 351 trades, 53% profitable, net $102,795, avg $292.86. Run on SPX/SPY/NIFTY (instrument swap); same rule as jkbwts_buymon1400 with a different stop.",
          claim("351", "53%", "$292.86", "same day", "S&P 500 futures 7/1990-7/1997 (BWTS p.46)")),
        nt("jk36k_jk20_us", "JK20-SC-US (US Bonds)", "Long/Short", &["F|$750 MM stop, input value 5; ideas: Volatility Breakout, Trend Switching (p.48)"], "US T-bond futures; also no bond data.", claim("185", "61%", "$208.39", "", "T-Bonds 7/1990-7/1997 (p.48)")),
        nt("jk36k_jk21_sp_dt", "JK21-SC-SP-DT (S&P day trade)", "Long/Short", &["F|$1,000 MM stop, exit market on close; ideas: Breakout, Trend Switching (p.49)"], "", claim("167", "43%", "$500.09", "same day", "S&P 500 7/1995-7/1997 (p.49)")),
        nt("jk36k_jk22_sp_dt", "JK22-SC-SP-DT (S&P Friday-qualified day trade)", "Long", &["F|$1,250 MM stop, exit market on close; ideas: DayOfWeek filtering, Trend Switching; buys only (p.50)"], "", claim("164", "58%", "$416.49", "same day", "S&P 500 7/1990-7/1997 (p.50)")),
        nt("jk36k_jk22_index_dt", "JK22-SC-INDEX-DT (NYSE day trade)", "Long", &["F|$750 MM stop, exit market on close; DayOfWeek filtering, Trend Switching (p.51)"], "", claim("161", "59%", "$134.31", "same day", "NYSE Composite 7/1990-7/1997 (p.51)")),
        nt("jk36k_jk23_26_currency_lt", "JK23-JK26-SC-LT (JY/SF/DM/BP long term)", "Long/Short", &["F|$1,000 MM stop; Gap Breakout + Channel Breakout combo; inputs JK23 65,50 / JK24 45,45 / JK25 45,60 / JK26 45,45 (p.52)"], "Currency futures; also no currency data.", claim("130", "38%", "$1,394.33", "", "Japanese Yen 8/1976-7/1997, JK23 (p.53)")),
        nt("jk36k_jk27_nyse_dt", "JK27-SC-NYSE-DT", "Long", &["F|$750 MM stop, exit market on close, buy stop only (p.55)"], "", claim("147", "50%", "$221.19", "same day", "NYSE Composite 7/1995-7/1997 (p.55)")),
        nt("jk36k_jk28_dual", "JK28-SC-Dual (Bonds vs weekly Gold)", "Long/Short", &["F|data1 daily US Bonds, data2 weekly Comex Gold, input 9; $750 stop (v1) or no stop (v2); 'Trend Fading' (p.56-57)"], "Bond and gold futures; also no data.", claim("133", "33%", "$418.21", "", "T-Bonds 7/1990-7/1997, $750 stop (p.56)")),
        nt("jk36k_jk29_bp_lw19", "JK29-SC-BP-LW-19 (5 currencies)", "Long/Short", &["F|No MM stop; 'Complex Numbers, Channel Breakout Combo', based on the Larry Williams chapter of Secrets of the Masters (p.58)"], "Currency futures; the Secrets of the Masters book is not on the drive.", claim("100", "50%", "$1,362.63", "", "Swiss Franc 2/1975-7/1997 (p.58)")),
        nt("jk36k_jk30_dual", "JK30-SC-Dual (S&P vs weekly Bonds)", "Long", &["F|data1 daily S&P 500, data2 weekly US Bonds, input 9; $2,000 stop (v1) or none (v2); 'Trend Focus and Dual Data Qualifier/Exit' (p.62)"], "", claim("38", "37%", "$5,256.18", "", "S&P 500 4/1982-7/1997, $2,000 stop (p.63)")),
        nt("jk36k_jk31_pat", "JK31-SC-Pat (weekly Dow 30 stocks)", "Long/Short", &["F|Weekly bars, $5 per share MM stop; 'Pattern filtering, Breakout trading' (p.64)"], "", claim("70", "54%", "$0.91", "", "General Motors weekly 3/1970-1/1997 (p.65)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == "jk36k_jk1_buy_monday" { return Some(("jk_buy_monday", "Buy Monday (Krutsinger)", "$36K manual 1997, $1,250 stop")); }
    catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, ""))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "jk36k_jk1_buy_monday").then(|| vec![p("stop_usd", "Money management stop $ (0 = none)", 1250., 0., 5000., 50.),
        p("pt_usd", "$ per index point of the book's futures contract", 500., 1., 1000., 50.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let stop = if g("stop_usd") > 0.0 { g("stop_usd") / g("pt_usd") } else { 0.0 };
    Some(buy_monday(x, from, to, stop))
}
