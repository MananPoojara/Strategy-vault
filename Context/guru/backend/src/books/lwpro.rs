//! Larry Williams, "Protege Program" workbook (Genesis printouts dated Jan 2003). Book id `lwpro`.
//! Source (read in place; COPY PENDING): research drive "USB HDD 2/tRADING KNO-2/Larry Williams - Protégé Program (Video & WorkBook
//! 4.34 GB) (ctiming.com)_ ON DVD34_/LarryWilliams-Protege2003.pdf" (61 pp, text). The other copy, "Trading Knowledge III/Larry
//! Williams - Protege Course (adest.com.au)/Larry Williams Protege Course.pdf" (62 pp), is an image scan of the same course.
//! Page refs = printed page numbers. Transcription and checks: library/lwpro/.
//!
//! Most of the workbook repeats the Inner Circle Workshop notes (set-ups, entries, exits, money management, bond and S&P
//! TDW/TDOM tables, holiday trades, Oops! by TDOM, best days of the year): those are coded once in `lwicw` and are not
//! duplicated here. New and testable without bond data are the S&P close/open studies on p.57.
//! Dollar stops at $250 per S&P point (p.pt_usd); the author's Genesis stop is armed from the day after entry (p.stop_day0 = 0,
//! "Exit on Entry Bar: False", MDC manual p.52-53).
use crate::books::lwicw::{getter, sim, Cfg, Ord};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwpro";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}

const X: &str = "X|\"exiting on the first profitable opening or a $4,500 stop (this is the exit for all trades discussed here)\" (p.57): $4,500 = 18 points from the day after entry";
const N: &str = "p.57, S&P futures \"since 1982\" (the workbook's printouts end 12/09/2002). Book figures are totals and averages per trade in dollars.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        sd!("lwpro_after_up", "Buy the open after an up close", "S&P short-term set-ups", "Long", Live,
            &["F|\"If today closes higher in the SP\" (p.57)", "E|\"we buy on the next bar opening\"", X], N, claim("1,049", "77.5%", "$42.65 ($44,735)", "", "1982-2002")),
        sd!("lwpro_after_down", "Buy the open after a down close", "S&P short-term set-ups", "Long", Live,
            &["F|\"If we bought following down closes in the SP\" (p.57)", "E|Buy the next open", X], N, claim("1,080", "77%", "$44 ($48,023)", "", "1982-2002")),
        sd!("lwpro_open_up", "Buy the open when it is above the prior close", "S&P short-term set-ups", "Long", Live,
            &["F|\"If we buy only on days that opened higher than the prior day\" (p.57); Read: open above the prior close", "E|Buy that open", X], N,
            claim("1,016", "77%", "-$9.43 (-$9,585)", "", "1982-2002")),
        sd!("lwpro_open_down", "Buy the open when it is below the prior close", "S&P short-term set-ups", "Long", Live,
            &["F|\"If we limit our buying to just days the SP opens lower\" (p.57)", "E|Buy that open", X], N,
            claim("1,073", "77%", "$104.53 ($112,165)", "", "1982-2002")),
        sd!("lwpro_bond_opens", "S&P buys by S&P and bond opening direction; Protege S&P patterns; Protege program", "S&P · Bond relationships", "Long/Short",
            NeedsData("Needs daily T-Bond futures opens and closes (and CRB for one rule); the vault has neither."),
            &["F|\"if you buy when bonds have opened lower and the Sp higher you now fumble away -$105,695 on 591 trades\"; SP down, bonds up: 617 trades, $150,610 (p.57-58)",
              "E|Protege SP patterns 'two ups', 'RS %R hidden oohs!', 'x days ago sell', 'MDC Day Trader Sell O>H', 'goofy gapper-1', 'MDC SP500 21-IB2' (p.58-59)"],
            "Protege program printout p.60-61: 351 trades, 93.2%, $1,401.83, 08/18/1982-12/09/2002 (needs bonds).", None),
        sd!("lwpro_pct_r", "%R with the 50-day average trend; R 'n R system", "Systems", "Long/Short",
            NotTestable("%R zones are only 'usually' (\"greater than 75% tops are usually near and when less than 20% bottoms usually form\", p.53), the exit is 'use the sell signals to exit' without numbers (p.54), and the R 'n R system lists a dollar stop of no stated size and four exits (p.55)."),
            &[], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwpro_after_up" => f("lwpro_close_open", "Close and open direction", "After an up close"),
        "lwpro_after_down" => f("lwpro_close_open", "Close and open direction", "After a down close"),
        "lwpro_open_up" => f("lwpro_close_open", "Close and open direction", "Opens higher"),
        "lwpro_open_down" => f("lwpro_close_open", "Close and open direction", "Opens lower"),
        "lwpro_bond_opens" => f("lwpro_close_open", "Close and open direction", "With bonds (needs data)"),
        "lwpro_pct_r" => f("lwpro_other", "Other", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lwpro_after_up" | "lwpro_after_down" | "lwpro_open_up" | "lwpro_open_down" => Some(vec![
            p("stop_usd", "Dollar stop (book $)", 4500.0, 0.0, 9000.0, 250.0),
            p("stop_day0", "Stop armed on the entry day (1) or from the next day (0, the book's code)", 0.0, 0.0, 1.0, 1.0),
            p("pt_usd", "Dollars per index point (book: S&P $250)", 250.0, 50.0, 2500.0, 50.0)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = getter(&schema, p);
    let mut cfg = Cfg::new(g("stop_usd") / g("pt_usd"));
    cfg.day0 = g("stop_day0") == 1.0;
    cfg.bail = Some(0);
    let (o, c) = (&x.o, &x.c);
    Some(sim(x, from, to, &cfg, |i| {
        let ok = i >= 2 && match id {
            "lwpro_after_up" => c[i - 1] > c[i - 2],
            "lwpro_after_down" => c[i - 1] < c[i - 2],
            "lwpro_open_up" => o[i] > c[i - 1],
            _ => o[i] < c[i - 1],
        };
        if ok { vec![(1.0, Ord::Open)] } else { vec![] }
    }))
}
