//! Larry Williams, "Million Dollar Challenge" seminar manual (copyright 2006, pages stamped "2005 ... Page N"). Book id `lwmdc`.
//! Source (read in place; COPY PENDING): research drive "Trading Knowledge III/Larry Williams - Million Dollar Challenge Seminar
//! (adest.com.au)/Million.pdf" (185 pp, OCR text with garbled tables). Transcription and checks: library/lwmdc/.
//! Page refs = the manual's own "Page N" stamps.
//!
//! The S&P studies are Genesis/System Writer tests of S&P 500 futures ($250 per point in the printed report headers, as lwlts).
//! Dollar stops are converted at p.pt_usd; other instruments use the same point distance (instrument swap only).
//! The author's Genesis exit code in this manual reads "Exit Long: Stop Loss ... Exit on Entry Bar: False" and the bailout
//! "IF Next Bar Open > Entry Price THEN SELL Next Bar at Next Bar Open" (p.52-53, p.99), so the dollar stop is armed from the
//! day after entry (p.stop_day0 = 0) and the bailout is the first profitable opening after the entry day.
use crate::books::lwicw::{getter, holiday, sim, tdm, td_rel, Cfg, Hol, Ord};
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwmdc";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = dollar stops kept as S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwmdc_down_smaller", "lwmdc_down_larger", "lwmdc_up_larger", "lwmdc_up_smaller", "lwmdc_up_sell_smaller", "lwmdc_up_sell_larger", "lwmdc_2down", "lwmdc_zone", "lwmdc_fdom_month", "lwmdc_thanksgiving", "lwmdc_od_up_oc", "lwmdc_od_down_oc", "lwmdc_nonod_up_oc", "lwmdc_nonod_down_oc", "lwmdc_od_oops_buy", "lwmdc_od_oops_sell", "lwmdc_od_high_buy", "lwmdc_od_prior_high_buy", "lwmdc_open_up_oc", "lwmdc_open_down_oc", "lwmdc_fdom_oc"];
pub const SPX_POINTS: &[&str] = &["lwmdc_down_smaller", "lwmdc_down_larger", "lwmdc_up_larger", "lwmdc_up_smaller", "lwmdc_up_sell_smaller", "lwmdc_up_sell_larger", "lwmdc_2down", "lwmdc_zone", "lwmdc_fdom_month", "lwmdc_thanksgiving"];

const BONDS: &str = "Needs daily T-Bond futures (opening and closing prices), and for some rules Gold or the CRB index; the vault has none of these.";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        sd!($id, $name, $group, $dir, $status, $rules, $notes, None)
    };
}

const EXIT45: &str = "X|\"in all instances here I am using the S&P 500, a $4,500 stop and my bailout exit\" (p.27): $4,500 = 18 points, from the day after entry; bailout = first profitable opening";
const NEXT_OPEN: &str = "E|Read: enter on the next day's opening (p.31: \"selling the next open\"; p.35: \"if I simply buy on the opening\")";
const TR: &str = "F|\"by true range I mean whichever is greater using today's high and low or if the market has gapped from the prior close using that close as the true high/low\" (p.27)";
const RANGE_N: &str = "S&P futures (Genesis data, window not printed; the manual's other S&P studies run from 1982). p.n = number of consecutive closes (book tests 1-5 for down closes, 1-2 for up closes); verify_lwmdc.py runs every printed row.";
const ZONES: &str = "F|Opening zones (p.34): zone 4 open > high; zone 3 open above the close and below the high; zone 2 open above the low and below the close; zone 1 open below the low";
const DT: &str = "X|\"I will be using an exit on the close of entry ... no hold over night\" (p.72); no stop is stated";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        // ---- "What does work" range studies (p.27-31)
        sd!("lwmdc_down_smaller", "Buy after N down closes, last day's true range smaller", "Range studies", "Long", Live,
            &["F|\"a day that closes lower than the prior close and that day's true range is less than the prior true range\" (p.27); N consecutive down closes (p.28-29)", TR, NEXT_OPEN, EXIT45],
            RANGE_N, claim("681", "", "$36 ($28,043)", "", "S&P (window not printed)")),
        sd!("lwmdc_down_larger", "Buy after N down closes, last day's true range larger", "Range studies", "Long", Live,
            &["F|\"CLOSE LESS THAN PRIOR DAY ON LARGER RANGE\" (p.27); N consecutive down closes, the last on a larger true range (p.28-29)", TR, NEXT_OPEN, EXIT45],
            RANGE_N, claim("695", "", "$121 ($106,695)", "", "S&P (window not printed)")),
        sd!("lwmdc_up_larger", "Buy after N up closes, last day's true range larger", "Range studies", "Long", Live,
            &["F|\"CLOSE GREATER THAN PRIOR DAY ON LARGER RANGE\" (p.30); 2 up closes (p.30)", TR, NEXT_OPEN, EXIT45],
            RANGE_N, claim("745", "", "-$157 (-$116,600)", "", "S&P (window not printed)")),
        sd!("lwmdc_up_smaller", "Buy after N up closes, last day's true range smaller", "Range studies", "Long", Live,
            &["F|Up close on a smaller range (p.30; the printed heading repeats 'CLOSE LESS THAN', the text says \"The best buy comes on an up day with a smaller range\")", TR, NEXT_OPEN, EXIT45],
            RANGE_N, claim("893", "", "$206 ($183,853)", "", "S&P (window not printed)")),
        sd!("lwmdc_up_sell_smaller", "Sell after 2 up closes, last range smaller", "Range studies", "Short", Live,
            &["F|\"in test of 2 up closes with the last coming on a smaller range and then selling the next open, some $20,515 was lost\" (p.31)", TR, "E|Sell the next open", EXIT45],
            "p.31, no trade count printed.", claim("", "", "-$20,515 total", "", "S&P (window not printed)")),
        sd!("lwmdc_up_sell_larger", "Sell after 2 up closes, last range larger", "Range studies", "Short", Live,
            &["F|\"Selling if the range had been a larger range made $5,108\" (p.31)", TR, "E|Sell the next open", EXIT45],
            "p.31, no trade count printed.", claim("", "", "$5,108 total", "", "S&P (window not printed)")),
        sd!("lwmdc_2down", "Buy after 2 down closes (gap and inside-day filters)", "Range studies", "Long", Live,
            &["F|\"we will buy after 2 down closes. That's all we are looking for\" (p.32)",
              "F|p.gap: 1 = h.1 < l.2 (67 trades, -$77,625); 2 = h.1 < c.2 (161 trades, -$60,220); 3 = h.1 > l.2 (598 trades, $80,791, $145) (p.32)",
              "F|p.inside: 1 = today inside (68 trades, $12,028); 2 = yesterday inside (161 trades, $54,548) (p.33); inside = high and low within the prior TRUE high/low",
              NEXT_OPEN, EXIT45],
            "Defaults = the plain 2-down-close test (607 trades, $75,098, $124). S&P, window not printed.",
            claim("607", "", "$124 ($75,098)", "", "S&P (window not printed)")),
        sd!("lwmdc_zone", "Day pattern x opening zone, buy the open", "Opening zones", "Long", Live,
            &["F|p.pattern: 1 up range up close; 2 up range down close; 3 down range up close; 4 down range down close; 5 = 4 with close > prior low; 6 = 4 with close < prior low (naked close) (p.34-36). Up range = higher high and higher low; down range = lower high and lower low",
              ZONES, "E|\"does it matter where it opens if I simply buy on the opening?\" (p.34)", "X|\"Here are the results from the same testing as above\" (p.35): Read: the p.27 exit, $4,500 stop and bailout"],
            "Tables p.34-36 (24 pattern/zone cells; verify_lwmdc.py runs all). Default pattern 1, zone 2 (578 trades, $123). The exact equality cases (open = high/close/low) belong to no zone. SPX opens before ~1993 equal the prior close, so use SPY or post-1993 SPX.",
            claim("578", "", "$123 ($71,190)", "", "S&P (window not printed)")),
        // ---- Outside days (p.72-74), day trades
        sd!("lwmdc_od_up_oc", "After an up-close outside day buy the open, exit the close", "Outside days", "Long", Live,
            &["F|Outside day: higher high and lower low than the prior day; up close = close above the prior close (p.72)", "E|\"bought the next day on the open\"", DT],
            "p.72: since 1982, 624 outside days; book results are after $50 commission.", claim("269", "", "-$195", "same day", "1982- (S&P)")),
        sd!("lwmdc_od_down_oc", "After a down-close outside day buy the open, exit the close", "Outside days", "Long", Live,
            &["F|Outside day with a down close (p.72)", "E|Buy the next open", DT], "p.72-73, after $50 commission.", claim("352", "", "-$72", "same day", "1982- (S&P)")),
        sd!("lwmdc_nonod_up_oc", "After an up day (not outside) buy the open, exit the close", "Outside days", "Long", Live,
            &["F|\"if you bought on the opening following all up days (that were not OD)\" (p.72)", "E|Buy the next open", DT], "p.72, after $50 commission.", claim("2,625", "", "$15", "same day", "1982- (S&P)")),
        sd!("lwmdc_nonod_down_oc", "After a down day (not outside) buy the open, exit the close", "Outside days", "Long", Live,
            &["F|\"Same scenario but with a down close\" (p.72)", "E|Buy the next open", DT], "p.72, after $50 commission.", claim("2,251", "", "-$62", "same day", "1982- (S&P)")),
        sd!("lwmdc_od_oops_buy", "Oops! buy at the low of a down-close outside day", "Outside days", "Long", Live,
            &["F|Down-close outside day; next day opens below its low (p.73)", "E|\"we will buy at the low of the OD if the market opens below the low the next day\" (stop)", DT],
            "p.73: \"loses a lot, $38,463 or $855 a trade\" (45 trades).", claim("45", "", "-$855 (-$38,463)", "same day", "1982- (S&P)")),
        sd!("lwmdc_od_oops_sell", "Sell the rally back to the low of a down-close outside day", "Outside days", "Short", Live,
            &["F|Down-close outside day; next day opens below its low (p.73)", "E|\"Instead of taking the OOPS! Buy if we sell should prices rally back to the OD low\" (p.73): sell at the OD low when the day trades back up to it", DT],
            "p.73: \"we make $34,413 on 45 trades with ... $765 per trade\".", claim("45", "", "$765 ($34,413)", "same day", "1982- (S&P)")),
        sd!("lwmdc_od_high_buy", "Down-close outside day, buy at its high", "Outside days", "Long", Live,
            &["F|\"an OD with a down close that opens less than the high of the OD\" (p.73)", "E|\"we then buy at the high of the OD\" (buy stop)", "X|\"keep in mind we buy on the stop and exit moc\" (p.73)"],
            "p.73: 63 trades, 65%, $332, drawdown $1,700.", claim("63", "65%", "$332", "same day", "1982- (S&P)")),
        sd!("lwmdc_od_prior_high_buy", "Down-close outside day, buy at the high of the day before it", "Outside days", "Long", Live,
            &["F|Down-close OD, next open below the OD high (p.73)", "E|\"we will buy at the high of the prior day\" = the day before the OD (buy stop)", "X|Exit on the close (moc)"],
            "p.73-74: 83 trades, $35,115, $423, over 67% winners.", claim("83", "67%", "$423 ($35,115)", "same day", "1982- (S&P)")),
        sd!("lwmdc_open_up_oc", "S&P opens above the prior close: buy the open, exit the close", "Opening direction", "Long", Live,
            &["F|\"829 times the S&P opened higher than the prior days close. Exit moc on these days would have lost $-88,330 or -107 a trade\" (p.70)", "E|Buy the open", DT],
            "p.70, \"the last 6 and 1/4 years\" (to 2004), after $50 per trade.", claim("829", "", "-$107 (-$88,330)", "same day", "1998-2004")),
        sd!("lwmdc_open_down_oc", "S&P opens below the prior close: buy the open, exit the close", "Opening direction", "Long", Live,
            &["F|\"When the S&P opened lower there were 770 trades losing $-28,275 or -37 a trade\" (p.70)", "E|Buy the open", DT],
            "p.70, after $50 per trade.", claim("770", "", "-$37 (-$28,275)", "same day", "1998-2004")),
        // ---- Month and holiday trades (p.64, 69, 176)
        sd!("lwmdc_fdom_oc", "Buy the open of the first trading day of every month, exit the close", "Trade day of month", "Long", Live,
            &["E|\"BUYING ON THE OPEN/EXIT ON THE CLOSE FIRST DAY OF EVERY MONTH\" (p.64)", "X|Exit on the close of the same day"],
            "p.64 summary: 270 trades, $97,138, $360 average, 12 trades a year (S&P, so about 1982-2004).", claim("270", "", "$360 ($97,138)", "same day", "about 1982-2004")),
        sd!("lwmdc_fdom_month", "Buy the first trading day of a given month, bailout", "Trade day of month", "Long", Live,
            &["E|\"buying on the open of the First Trading Day of the Month ... The best of the best are shown using a $3,500 stop and first profitable open for the exit\" (p.69)",
              "F|p.month: the table lists January (16 trades, 87%, $905), March (17, 94%, $547), June (15, 86%, $765), July (17, 100%, $652), August (17, 94%, $594), December (17, 100%, $961)",
              "X|$3,500 stop = 14 points from the day after entry; bailout"],
            "p.69; default January (the table's first row). Not the same as lwlts_month_day1 ($1,500 stop, all months).",
            claim("16", "87%", "$905 (January)", "", "about 1982-1998")),
        sd!("lwmdc_thanksgiving", "S&P Thanksgiving trade", "Holidays", "Long", Live,
            &["E|\"The best rule is to buy on the opening of the day before the holiday\" (p.176)",
              "X|\"we will use the same rules as used in the MDC system for trading this market\": \"BAIL OUT AND 3,500 STOP\" (p.75): bailout and a $3,500 stop (14 points) from the day after entry"],
            "p.176-177: 22 trades, $20,785, \"1987 was the only losing year\".", claim("22", "", "$945 ($20,785)", "", "about 1982-2004")),
        // ---- Needs data / not testable
        sd!("lwmdc_blast_off_wto", "Blast Off! indicator and Williams Trade Oscillator (\"Blue Line\") on tick bars", "Globex day trading", "Long/Short",
            NeedsData("Tick bars (400-900 ticks per bar, p.8) are not in the vault, and the WTO formula is withheld (\"The exact formula will not be given\", p.11)."),
            &["F|Blast Off! = MovingAvg(Abs(Close-Open),1) / MovingAvg(TrueHigh-TrueLow,1) * 100 (p.10)", "E|Index crosses zero: buy one tick above the bar's high / sell one tick below its low (p.10-11)", "X|Stop at the prior bar's high/low, then the 2-bar extreme (p.14); 10-tick targets 'up to you' (p.13)"], ""),
        sd!("lwmdc_bond_rules", "Bond short-term rules (TDOM by month with Gold, JAWS of death, bond trailer, best first days, CRB filters, MDC bond system)", "Bonds", "Long/Short", NeedsData(BONDS),
            &["E|e.g. TDOM 8 or 17, June/July/October/November, Gold below 6, 9 or 28 days ago, buy the next open (p.46)"], "Printouts p.46-61."),
        sd!("lwmdc_sp_bond_rules", "S&P rules using bonds: bond opening/trend studies, JAWS of death S&P, MDC 2004 S&P system, best month/TDOM with bonds, trade no one wants, lazy day trader", "S&P · Bond relationships", "Long/Short", NeedsData(BONDS),
            &["F|e.g. \"IF Close < Close.1 And Close.1 < Close.2 And MovingAvg(Bonds,6) > MovingAvg(Bonds,6).3 ...\" (p.39)", "X|MDC 2004 S&P system: bailout and $3,500 stop (p.75)"],
            "The combined systems also contain rules whose OCR text lost operators (p.75-92), so they could not be coded exactly even with bond data."),
        sd!("lwmdc_intraday", "Intraday S&P: first trade of the day (4-minute bars), three steps up, noon balloon, end-of-day trader, thirty something, triple trend", "Intraday", "Long/Short",
            NeedsData("Needs 4-, 15- and 30-minute S&P bars (and 30-minute bonds for 'thirty something'); the vault has daily data only."), &[], ""),
        sd!("lwmdc_best_tdom_months", "Best S&P buy/sell TDOMs by month (BEST SP DAYTRADER)", "Trade day of month", "Long/Short",
            NotTestable("The tables (p.63-66) list a TDOM per month with results, but the entry order and the exit are not stated (the printout is 'BEST SP DAYTRADER')."), &[], ""),
        sd!("lwmdc_rnr", "R 'n R system (21-day RSI trend, 4-day %R, Oops! entries)", "Systems", "Long/Short",
            NotTestable("The dollar stop amount is not given and the exits are a list ('$ STOP LOSS---FIRST PROFITABLE OPENING, %R GOING TO OVERBOUGHT/SOLD, TRAIL A STOP...', p.102) without an order of use; the %R buy/sell zones are not given numerically."), &[], ""),
        sd!("lwmdc_other", "DAX studies, soybean/gold Oops!, Sequential, specialist trap, full moon, Fibonacci and round-number studies", "Other", "Long/Short",
            NotTestable("DAX futures and soybean/gold data are not in the vault; Sequential (p.161) has no exit; the specialist trap is chart judgement ('a strong down trend'); the full-moon pages (p.178-182) are unreadable OCR; Fibonacci and stop-placement pages are studies, not trading rules."), &[], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwmdc_down_smaller" => f("lwmdc_range", "Range studies", "Down closes, smaller range"),
        "lwmdc_down_larger" => f("lwmdc_range", "Range studies", "Down closes, larger range"),
        "lwmdc_up_larger" => f("lwmdc_range", "Range studies", "Up closes, larger range"),
        "lwmdc_up_smaller" => f("lwmdc_range", "Range studies", "Up closes, smaller range"),
        "lwmdc_up_sell_smaller" => f("lwmdc_range", "Range studies", "Sell 2 up closes, smaller"),
        "lwmdc_up_sell_larger" => f("lwmdc_range", "Range studies", "Sell 2 up closes, larger"),
        "lwmdc_2down" => f("lwmdc_range", "Range studies", "2 down closes, gaps / inside days"),
        "lwmdc_zone" => f("lwmdc_zone", "Opening zones", "Pattern x zone"),
        "lwmdc_od_up_oc" | "lwmdc_od_down_oc" | "lwmdc_nonod_up_oc" | "lwmdc_nonod_down_oc" => f("lwmdc_od", "Outside days", "Open to close"),
        "lwmdc_od_oops_buy" | "lwmdc_od_oops_sell" => f("lwmdc_od", "Outside days", "Oops! at the OD low"),
        "lwmdc_od_high_buy" | "lwmdc_od_prior_high_buy" => f("lwmdc_od", "Outside days", "Buy stop above"),
        "lwmdc_open_up_oc" => f("lwmdc_open", "Opening direction", "Opens up"),
        "lwmdc_open_down_oc" => f("lwmdc_open", "Opening direction", "Opens down"),
        "lwmdc_fdom_oc" => f("lwmdc_fdom", "First trading day of the month", "Open to close"),
        "lwmdc_fdom_month" => f("lwmdc_fdom", "First trading day of the month", "Best months, bailout"),
        "lwmdc_best_tdom_months" => f("lwmdc_fdom", "First trading day of the month", "Best TDOMs (not testable)"),
        "lwmdc_thanksgiving" => f("lwmdc_holiday", "Holiday trades", "Thanksgiving"),
        "lwmdc_blast_off_wto" | "lwmdc_intraday" => f("lwmdc_intraday", "Day trading", "Needs intraday data"),
        "lwmdc_bond_rules" | "lwmdc_sp_bond_rules" => f("lwmdc_bonds", "Bond relationships", "Needs bond data"),
        "lwmdc_rnr" | "lwmdc_other" => f("lwmdc_other", "Other", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $250)", 250.0, 50.0, 2500.0, 50.0);
    let stop = |d: f64| p("stop_usd", "Dollar stop (book $)", d, 0.0, 9000.0, 250.0);
    let day0 = || p("stop_day0", "Stop armed on the entry day (1) or from the next day (0, the book's code)", 0.0, 0.0, 1.0, 1.0);
    let n = |d: f64| p("n", "Consecutive closes", d, 1.0, 6.0, 1.0);
    Some(match id {
        "lwmdc_down_smaller" | "lwmdc_down_larger" | "lwmdc_up_larger" | "lwmdc_up_smaller" => vec![n(1.0), stop(4500.0), day0(), pt()],
        "lwmdc_up_sell_smaller" | "lwmdc_up_sell_larger" => vec![n(2.0), stop(4500.0), day0(), pt()],
        "lwmdc_2down" => vec![p("gap", "Gap filter: 0 none, 1 h.1<l.2, 2 h.1<c.2, 3 h.1>l.2", 0.0, 0.0, 3.0, 1.0),
            p("inside", "Inside-day filter: 0 none, 1 today inside, 2 yesterday inside", 0.0, 0.0, 2.0, 1.0), stop(4500.0), day0(), pt()],
        "lwmdc_zone" => vec![p("pattern", "1 UR up, 2 UR down, 3 DR up, 4 DR down, 5 DR down above prior low, 6 DR down naked", 1.0, 1.0, 6.0, 1.0),
            p("zone", "Opening zone 1-4", 2.0, 1.0, 4.0, 1.0), stop(4500.0), day0(), pt()],
        "lwmdc_od_up_oc" | "lwmdc_od_down_oc" | "lwmdc_nonod_up_oc" | "lwmdc_nonod_down_oc" | "lwmdc_od_oops_buy" | "lwmdc_od_oops_sell"
        | "lwmdc_od_high_buy" | "lwmdc_od_prior_high_buy" | "lwmdc_open_up_oc" | "lwmdc_open_down_oc" | "lwmdc_fdom_oc" => vec![],
        "lwmdc_fdom_month" => vec![p("month", "Month (1-12; the table lists 1, 3, 6, 7, 8, 12)", 1.0, 1.0, 12.0, 1.0), stop(3500.0), day0(), pt()],
        "lwmdc_thanksgiving" => vec![p("td_before", "Trading days before Thanksgiving (book 1 = the day before)", 1.0, 1.0, 10.0, 1.0), stop(3500.0), day0(), pt()],
        _ => return None,
    })
}

fn tr(x: &Ctx, j: usize) -> f64 { x.h[j].max(x.c[j - 1]) - x.l[j].min(x.c[j - 1]) }

/// Inside day per p.33: high and low within the prior TRUE high and TRUE low.
fn inside(x: &Ctx, j: usize) -> bool { x.h[j] <= x.h[j - 1].max(x.c[j - 2]) && x.l[j] >= x.l[j - 1].min(x.c[j - 2]) }

fn outside(x: &Ctx, j: usize) -> bool { x.h[j] > x.h[j - 1] && x.l[j] < x.l[j - 1] }

/// Day trade with an optional limit sell at `lv` (fills when the day trades up to it, at the open if it opens above).
fn sell_limit_day(x: &Ctx, from: usize, to: usize, lv: impl Fn(usize) -> Option<f64>) -> Vec<Trade> {
    let mut out = vec![];
    for i in from.max(1)..=to {
        if let Some(v) = lv(i) {
            if x.h[i] >= v {
                let px = x.o[i].max(v);
                out.push(Trade { dir: -1.0, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: "time exit on the close".into(),
                    entry_kind: "limit", signal_i: i, open: false, exits: vec![] });
            }
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = getter(&schema, p);
    let mut cfg = Cfg::new(g("stop_usd") / g("pt_usd"));
    cfg.day0 = g("stop_day0") == 1.0;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let none = || vec![];
    let day = |sig: &dyn Fn(usize) -> Vec<(f64, Ord)>| {
        let mut d = Cfg::new(f64::NAN);
        d.hold = Some(0);
        sim(x, from, to, &d, sig)
    };
    let t = match id {
        "lwmdc_down_smaller" | "lwmdc_down_larger" | "lwmdc_up_larger" | "lwmdc_up_smaller" | "lwmdc_up_sell_smaller" | "lwmdc_up_sell_larger" => {
            let n = g("n").max(1.0) as usize;
            let up = id.starts_with("lwmdc_up");
            let larger = id.ends_with("larger");
            let dir = if id.contains("sell") { -1.0 } else { 1.0 };
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| {
                if i < n + 3 { return none(); }
                let s = i - 1;
                let closes = (0..n).all(|k| if up { c[s - k] > c[s - k - 1] } else { c[s - k] < c[s - k - 1] });
                let rng = if larger { tr(x, s) > tr(x, s - 1) } else { tr(x, s) < tr(x, s - 1) };
                if closes && rng { vec![(dir, Ord::Open)] } else { none() }
            })
        }
        "lwmdc_2down" => {
            let (gap, ins) = (g("gap") as i32, g("inside") as i32);
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| {
                if i < 5 { return none(); }
                let s = i - 1;
                let ok = c[s] < c[s - 1] && c[s - 1] < c[s - 2]
                    && match gap { 1 => h[s - 1] < l[s - 2], 2 => h[s - 1] < c[s - 2], 3 => h[s - 1] > l[s - 2], _ => true }
                    && match ins { 1 => inside(x, s), 2 => inside(x, s - 1), _ => true };
                if ok { vec![(1.0, Ord::Open)] } else { none() }
            })
        }
        "lwmdc_zone" => {
            let (pat, zone) = (g("pattern") as i32, g("zone") as i32);
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| {
                if i < 3 { return none(); }
                let s = i - 1;
                let ur = h[s] > h[s - 1] && l[s] > l[s - 1];
                let dr = h[s] < h[s - 1] && l[s] < l[s - 1];
                let upc = c[s] > c[s - 1];
                let dnc = c[s] < c[s - 1];
                let ok = match pat { 1 => ur && upc, 2 => ur && dnc, 3 => dr && upc, 4 => dr && dnc,
                    5 => dr && dnc && c[s] > l[s - 1], _ => dr && dnc && c[s] < l[s - 1] };
                let op = o[i];
                let z = if op > h[s] { 4 } else if op > c[s] && op < h[s] { 3 } else if op > l[s] && op < c[s] { 2 } else if op < l[s] { 1 } else { 0 };
                if ok && z == zone { vec![(1.0, Ord::Open)] } else { none() }
            })
        }
        "lwmdc_od_up_oc" | "lwmdc_od_down_oc" | "lwmdc_nonod_up_oc" | "lwmdc_nonod_down_oc" => {
            let want_od = id.starts_with("lwmdc_od");
            let upc = id.contains("_up_");
            day(&|i| {
                if i < 2 { return none(); }
                let s = i - 1;
                let cl = if upc { c[s] > c[s - 1] } else { c[s] < c[s - 1] };
                if outside(x, s) == want_od && cl { vec![(1.0, Ord::Open)] } else { none() }
            })
        }
        "lwmdc_od_oops_buy" => day(&|i| {
            if i < 2 { return none(); }
            let s = i - 1;
            if outside(x, s) && c[s] < c[s - 1] && o[i] < l[s] { vec![(1.0, Ord::Stop(l[s]))] } else { none() }
        }),
        "lwmdc_od_oops_sell" => sell_limit_day(x, from, to, |i| {
            if i < 2 { return None; }
            let s = i - 1;
            (outside(x, s) && c[s] < c[s - 1] && o[i] < l[s]).then_some(l[s])
        }),
        "lwmdc_od_high_buy" | "lwmdc_od_prior_high_buy" => {
            let prior = id.contains("prior");
            day(&|i| {
                if i < 3 { return none(); }
                let s = i - 1;
                if outside(x, s) && c[s] < c[s - 1] && o[i] < h[s] { vec![(1.0, Ord::Stop(if prior { h[s - 1] } else { h[s] }))] } else { none() }
            })
        }
        "lwmdc_open_up_oc" | "lwmdc_open_down_oc" => {
            let up = id.contains("_up_");
            day(&|i| if i >= 1 && (if up { o[i] > c[i - 1] } else { o[i] < c[i - 1] }) { vec![(1.0, Ord::Open)] } else { none() })
        }
        "lwmdc_fdom_oc" => {
            let td = tdm(x);
            day(&|i| if i >= 1 && td[i] == 1 { vec![(1.0, Ord::Open)] } else { none() })
        }
        "lwmdc_fdom_month" => {
            let td = tdm(x);
            let m = g("month") as u32;
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| {
                let mm: u32 = x.r[i].date[5..7].parse().unwrap_or(0);
                if i >= 1 && td[i] == 1 && mm == m { vec![(1.0, Ord::Open)] } else { none() }
            })
        }
        "lwmdc_thanksgiving" => {
            let k = g("td_before").max(1.0) as i32;
            cfg.bail = Some(0);
            let y0: i32 = x.r[0].date[..4].parse().ok()?;
            let y1: i32 = x.r[x.r.len() - 1].date[..4].parse().ok()?;
            let mut hit = vec![false; x.r.len()];
            for y in y0..=y1 {
                if let Some(j) = td_rel(x, &holiday(Hol::Thanksgiving, y), k) { hit[j] = true; }
            }
            sim(x, from, to, &cfg, |i| if hit[i] && i >= 1 { vec![(1.0, Ord::Open)] } else { none() })
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn od_prior_high_day_trade() {
        // day 2 is a down-close outside day; day 3 opens below its high and rallies through the high of day 1
        let r = vec![bar("2001-01-02", 100., 101., 99., 100.), bar("2001-01-03", 100., 102., 99., 101.),
            bar("2001-01-04", 101., 103., 98., 99.), bar("2001-01-05", 99.5, 104., 99., 103.5)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwmdc_od_prior_high_buy", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_px), (3, 102.0, 103.5));
        let t = run("lwmdc_od_high_buy", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].px, t[0].exit_px), (103.0, 103.5));
    }
}
