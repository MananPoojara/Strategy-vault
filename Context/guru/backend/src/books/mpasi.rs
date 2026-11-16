//! Martin J. Pring, "The All-Season Investor" (Wiley 1992; scan, 352 pdf pages; printed page = pdf page - 11 in Ch.1-12, - 13 in the appendices). Book id `mpasi`.
//! Source and checks: library/mpasi/. Business-cycle asset allocation: six stages from bonds, stocks and
//! commodities, identified by economic/financial barometers or 12-month MA crossovers, with suggested
//! stock/bond/cash weights per stage. Only the stock leg - the S&P 12-month MA crossover - runs on price data alone
//! (mpasi_sp_ma12, admitted 2026-10-23, authors/pring/admission/SOURCE-CONTRACT.md); the rest needs bond, CRB, yield or economic data.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "mpasi";

/// Lead decision 197 (K37, 4 Nov 2026): mpasi_sp_ma12 is held (MPASI_HELD).
pub const REVIEWED: &[&str] = &[];
const MPASI_HELD: &str = "the printed rule needs both legs: \"Favorable periods for equity ownership under this approach require the yield on three-month commercial paper to be below its 12-month average and for the S&P Composite to be above its 12-month moving average. All other conditions are regarded as unfavorable\" (p.101); the S&P leg alone is not a printed rule, and the vault holds no commercial-paper yield series (lead decision 197)";
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! d {
    ($id:expr, $name:expr, $group:expr, $st:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long", status: $st,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the scan (library/mpasi/README.md).", claim: None }
    };
}

fn catalogue_raw() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        StrategyDef { id: "mpasi_sp_ma12", name: "S&P 12-month moving-average crossover (stock leg of the stage model), long or cash", group: "Ch.5, 9 (p.101-105, 185-190)",
            session: 0, dir: "Long", status: NotTestable(MPASI_HELD), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"History tells us that 12-month moving average crossovers by the S&P Composite have been the most consistently reliable of any time span in identifying bull and bear trends in the stock market\" (p.101); bullish \"when the S&P Composite is at or above its moving average (MA)\" (Fig 5.16, p.105)",
                     "X|Bearish when the S&P \"crossed below its 12-month moving average\" (p.190): sell to cash",
                     "F|Monthly closes; simple 12-month average (Table 9.1, p.185)"],
            notes: "Month-end bar = last trading day of the month (the last bar of the data never counts as a month end). A close equal to the average counts as bullish (p.105). Long or cash; the book prints no results for this leg alone. Fill at the month-end close [Not from the book: no fill is stated]. Applied to US and Indian indices, ETFs and stocks as a declared application (the book: S&P Composite).",
            claim: None },
        d!("mpasi_six_stages_ma12", "Six-stage allocation from 12-month MA crossovers of bonds, stocks, commodities", "Ch.9-10, 12 (p.181-189, 205-206, 254-262)",
           NotTestable("needs monthly US long Treasury bond prices and the CRB Composite (not in the vault); 2 of the 8 above/below combinations are no stage; allocations are 'gradual', 'a range rather than a specific number' (p.182) and can depart from the targets ('The Outer Ranges', p.261-262)"),
           &["F|\"technicians use moving average crossovers rather than reversals\"; \"12 months is one of the most consistently reliable\" (p.185-186)",
             "F|Stage I bonds bullish, stocks and commodities bearish; II bonds and stocks bullish; III all bullish; IV bonds bearish; V commodities bullish only; VI all bearish (Fig 10.1, p.206)",
             "E|Stage weights stocks/bonds/cash: I 30/50/20 ('recommended 30 percent equity exposure', p.256), II 65/30/5, III 75/20/5, IV 75/15/10, V 30/15/55 (cash \"raised to 55 percent\", p.260), VI 20/30/50 (Figs 12.2-12.6, p.257-261)"]),
        d!("mpasi_barometers", "Bond, Stock and Inflation Barometers", "Ch.10, Appendices A-C (p.204-209, 281-307)",
           NotTestable("economic and money data (housing starts, vendor deliveries, LEI diffusion, M2, CRB spot raw materials, commercial paper and AAA yields, discount rate) are not in the vault; several weights are unspecified ('a special weighting', p.295)"),
           &["F|\"It starts off with 100 percent of its components in bullish mode. As a specific indicator goes negative its contribution is removed. Readings of 50 percent or lower signal periods of risk.\" (Stock Barometer, p.290)",
             "F|\"One point is subtracted when the S&P falls below its nine-month MA\" (p.299)"]),
        d!("mpasi_conservative_aggressive", "Conservative and aggressive allocation policies", "Ch.12 (p.263-273)",
           NotTestable("same stage model with different asset classes (intermediate bonds, utilities, convertibles, gold shares); the allocations are 'guidelines for an average investor' (p.264)"),
           &["E|e.g. conservative Stage I: stocks 10%, bonds 40%, intermediate bonds 20%, cash 30% (Fig 12.7, p.264)"]),
        d!("mpasi_gold", "Gold and the business cycle", "Ch.11 (p.231-245)",
           NotTestable("background factors and judgement; no trading rule"), &["E|Narrative only"]),
    ]
}

/// Lane D2 (16 Nov 2026, R4 #340, #339): the printed two-leg rule (p.101), with the 3-month commercial-paper yield replaced by the
/// Norgate 3-month Eurodollar rate, on the Norgate S&P 500 continuous contract (futures::CARDS_D2).
pub const CP_ID: &str = "mpasi_cp_sp_ma12";
const CP_NOTE: &str = "[Declared adaptation, R4 #340; UNVERIFIED, 2 declared substitutions and 4 unstated details, R4 #339/#343] The book's rule needs the 3-month commercial-paper yield and the S&P Composite. The vault holds neither series as such. Substitutions: (1) the commercial-paper yield is replaced by the 3-month Eurodollar rate = 100 minus the Norgate Eurodollar (&GE) continuous-contract close, the nearest real money-market rate we hold (book: commercial paper; 1982-02 onward only); 100 minus the futures close is a FORWARD 3-month rate (the rate for a quarter starting at the contract's expiry, not the spot rate), and it is taken from the back-adjusted continuous series, so its level is shifted at each roll and only its change against its own 12-month average is meaningful; (2) the S&P Composite is replaced by the Norgate S&P 500 (&SP) continuous contract, back-adjusted daily bars, roll method not stated by the vendor, ends 2020-09-11. Unstated card-specific choices (R4 #343, 4): (a) the month end is the last trading day of the month (the last bar of the data never counts); (b) the S&P 12-month average includes the signal month; (c) the rate's 12-month average includes the signal month; (d) both averages are simple averages of month-end values. Printed, so not choices: the S&P leg counts at-or-above (p.105) and the rate leg strictly below (p.101). Engine conventions, labelled not counted: the fill is the month-end close, one position, the 12-month warm-up. Long when both legs are favorable at a month end, otherwise cash. The book prints no results for this rule. Close-only rule: no same-day order question (#240: 0 trades), clamp_open false.";
fn catalogue_all() -> Vec<StrategyDef> {
    let mut v = catalogue_raw();
    v.push(StrategyDef { id: CP_ID, name: "S&P 12-month MA and 3-month rate below its 12-month average (Pring p.101), long or cash", group: "Ch.5 (p.101)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"Favorable periods for equity ownership under this approach require the yield on three-month commercial paper to be below its 12-month average and for the S&P Composite to be above its 12-month moving average.\" (p.101): buy at the month-end close when both hold",
                 "X|\"All other conditions are regarded as unfavorable\" (p.101): sell to cash at the month-end close",
                 "F|[Labelled reading, R4 #340] rate = 100 - Norgate Eurodollar close (&GE) in place of commercial paper; S&P = Norgate &SP continuous contract; monthly closes, simple 12-month averages"],
        notes: CP_NOTE, claim: None });
    v
}
pub fn catalogue() -> Vec<StrategyDef> { catalogue_all() }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == CP_ID { return Some(("mpasi_cp", "Pring All-Season Investor (S&P and rate)", "p.101 two-leg rule")); }
    if id == "mpasi_sp_ma12" { return Some(("mpasi_ma", "Pring All-Season Investor", "S&P 12-month MA")); }
    id.starts_with("mpasi_").then_some(("mpasi_nt", "Pring All-Season Investor (not testable)", "-"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mpasi_sp_ma12" || id == CP_ID).then(|| vec![ParamDef { key: "n", label: "MA length (months)", def: 12., min: 2., max: 36., step: 1. }])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if (id != "mpasi_sp_ma12" && id != CP_ID) || from > to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(12.).round().max(1.) as usize;
    let ends = crate::books::mpts::period_ends(x, true);
    let idx: Vec<usize> = (0..x.c.len()).filter(|&i| ends[i]).collect();
    let ma = sma(&idx.iter().map(|&i| x.c[i]).collect::<Vec<_>>(), n);
    // lane D2: the rate leg. rate = 100 - Eurodollar close on each month-end bar (the Eurodollar bar of that date, else its last earlier bar)
    let rate_ok: Vec<bool> = if id == CP_ID {
        let Some(ge) = crate::futures::rows("&GE") else { return Some(vec![]) }; // no Eurodollar loaded: futures::data2_issue refuses the card before any run is shown
        let rate = crate::second::align("&GE", x.r, ge).carried(crate::second::Field::C).into_iter().map(|c| 100. - c).collect::<Vec<f64>>();
        let rv: Vec<f64> = idx.iter().map(|&i| rate[i]).collect();
        let rma = sma(&rv, n);
        (0..idx.len()).map(|k| rv[k].is_finite() && rma[k].is_finite() && rv[k] < rma[k]).collect()
    } else { vec![true; idx.len()] };
    let (mut out, mut cur, mut last) = (vec![], None::<Trade>, None::<bool>);
    for (k, &i) in idx.iter().enumerate() {
        if i > to { break; }
        if !ma[k].is_finite() { continue; }
        let bull = x.c[i] >= ma[k] && rate_ok[k];
        let change = last.is_some_and(|b| b != bull);
        last = Some(bull);
        if i < from || !change { continue; }
        if bull { cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }); }
        else if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "S&P closed below its 12-month average".into(); out.push(t); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
