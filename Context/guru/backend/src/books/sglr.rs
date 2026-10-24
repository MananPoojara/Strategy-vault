//! Jeremy J. Siegel, Stocks for the Long Run (3rd ed., McGraw-Hill 2002; "4th ed." in the queue).
//! Ch 17 (200-day moving average with a 1% band) is Live; Ch 18 calendar anomalies are statistics or
//! already in the vault. Page refs are printed pages (PDF page = printed + 34). Sources: library/sglr/.
use crate::books::apta::run_long;
use crate::connors::{claim, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sglr";
/// Admitted ids (authors/siegel/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["sglr_dow_ma200_band", "sglr_nasdaq_ma200_band"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BAND: &[&str] = &[
    "E|\"Whenever the Dow closed at least 1 percent above its 200-day moving average, stocks were purchased at these closing prices.\" (p.290)",
    "X|\"Whenever the Dow closed by at least 1 percent below its 200-day moving average, stocks were sold. The portfolio was then invested in Treasury bills and earned interest income.\" (p.290)",
    "F|\"a 200-day moving average is the average of the past 200 days' closing prices\" (p.289, simple average incl. today). \"an investor buys or sells stocks at the closing price\" (p.290).",
    "F|Book returns include dividends and T-bill interest (p.290); the vault runs price-only with cash at 0%.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("sglr_dow_ma200_band", "Dow 200-day moving average with 1% band", "Ch 17 · Testing the Dow Jones moving-average strategy", Live, BAND,
            "Book: DJIA 1885-2001 (vault DJI from 1970). Dated signals in the text: sold Friday Oct 16, 1987 (p.291); exited June 25, 2001, re-entered Jan 3, 2002 (p.294). Table 17-1 1990-2001: 49 switches, 77.4% in the market.",
            claim("325 switches", "", "10.43%/yr (8.89% after 0.5% costs)", "62.8% in market", "DJIA 1886-2001 incl. dividends (1990-2001: 49 switches, 77.4% in, 7.39%/yr)")),
        d("sglr_nasdaq_ma200_band", "Nasdaq 200-day moving average with 1% band", "Ch 17 · The Nasdaq moving-average strategy", Live, BAND,
            "Same rule on the Nasdaq Composite (p.294; not in the vault -> NDX swap). Dated: entered Nov 2, 1998 at 1,801; exited Sep 11, 2000 at 3,896; re-entered Dec 5, 2001 at 2,046 (p.294).",
            claim("77 switches", "", "16.67%/yr (15.20% net)", "68.4% in market", "Nasdaq Composite 1972-2001 (1990-2001: 34 switches, 72.4%, 17.52%/yr)")),
        d("sglr_no_september", "Every month except September", "Ch 18 · The September effect", NotTestable("duplicate: masmt_sept_avoid (out from the last August close to the first October day)"),
            &["E|\"if you put your money in the stock market every month except September, it would have been worth $1,548 at the end of 2001\" (p.309)"],
            "Fig 18-4: DJIA 1885-2001 ex dividends, $1 -> $1,548 without September vs $394 always invested, $0.25 September only.", None),
        d("sglr_jan_small", "January small-stock switch", "Ch 18 · The January effect", NeedsData("a small-stock index (Ibbotson/CRSP small stocks) next to the S&P 500; the vault has no small-cap total-return series and the engine runs one instrument per strategy"),
            &["E|\"By buying small stocks at the end of December and transferring them back to the S&P 500 Index at the end of January, your accumulation would grow to $58,173, or a 15.5 percent annual rate of return\" (p.300)"],
            "Fig 18-1, 1926-2001.", claim("", "", "15.5%/yr", "1 month small stocks, 11 months S&P", "1926-2001")),
        d("sglr_calendar_stats", "Calendar anomaly statistics", "Ch 18 · Monthly, intramonth, day-of-week and holiday returns", NotTestable("statistics with no entry/exit rule (Table 18-2, Fig 18-2, 18-5); holiday and weekday rules are in hsta_holidays / hsta_weekdays"),
            &["F|First half of month, last day of month, days of the week, day before holiday averages (Table 18-2, p.311)"], "", None),
        d("sglr_gordon", "Gordon: Dow above its 200-day average", "Ch 17 · Moving averages", NotTestable("duplicate: plain 200-day crossing = apta_ma200_dji"),
            &["F|\"buying stocks when the Dow broke above the moving average produced nearly seven times the returns as buying when the Dow broke below the average\" (p.289)"], "Cited study, 1897-1967.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "sglr_dow_ma200_band" => ("sglr_ma", "Siegel 200-day MA with 1% band", "Dow"),
        "sglr_nasdaq_ma200_band" => ("sglr_ma", "Siegel 200-day MA with 1% band", "Nasdaq"),
        s if s.starts_with("sglr_") => ("sglr_cal", "Siegel calendar anomalies / notes", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "sglr_dow_ma200_band" | "sglr_nasdaq_ma200_band" => vec![p("n", "Moving average days", 200.0, 20.0, 300.0, 10.0), p("band", "Band around the average, %", 1.0, 0.0, 5.0, 0.25)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let ma = sma(&x.c, g("n").round().max(1.0) as usize);
    let b = g("band") / 100.0;
    let c = &x.c;
    let idx: Vec<usize> = (0..n).collect();
    Some(run_long(x, &idx, from, to, |_, i, holding| {
        if !ma[i].is_finite() {
            None
        } else if !holding && c[i] >= ma[i] * (1.0 + b) {
            Some("close >= 1% above the 200-day MA".into())
        } else if holding && c[i] <= ma[i] * (1.0 - b) {
            Some("close <= 1% below the 200-day MA".into())
        } else {
            None
        }
    }))
}
