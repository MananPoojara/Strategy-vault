//! Nelson Freeburg, "Building a Mechanical Trading System from the Ground Up" (TAG XVIII, 1996,
//! INO workbook, 49 pp. scan). Source and checks: library/nfgu/.
//! Every system here needs futures contracts, rates or Value Line data the vault lacks; nothing is Live.
//! The 2000 version of the Quadram system (no PAL filter) is Live in nfhp.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "nfgu";

const FX_TB: &str = "daily D-mark, Swiss franc, pound, yen futures AND US T-Bond futures (setup filter), with contract point values for the $2,000 stop; none in the vault";
const FX: &str = "daily D-mark, Swiss franc, pound and yen futures with contract point values ($2,000 stops); none in the vault";
const METALS: &str = "silver/gold futures, Swiss franc futures, US T-bill yield and CPI inflation; none in the vault";
const PAL: &str = "weekly US T-Bill yield and T-Bond yield (PAL model filter: S&P < 42-week MA and T-Bill yield > 42-week MA or T-Bond yield > 10-week MA); not in the vault";
const HEINE: &str = "Lehman bond index, T-Bond / T-Bill yields, DJ Utilities, CRB (same model as nftm_heine_bond); not in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("nfgu_pathfinder", "Pathfinder Currency Trading System", "Formulas p.1 · Pathfinder", "Long/Short", NeedsData(FX_TB),
          &["F|Setup currency: long 9-day MA > 18-day MA, short 9 < 18; setup US T-Bonds: long 3-day MA > 25-day MA, short 3 < 25 (formulas p.1)",
            "E|\"Long: If 6-Day Ma > 6-Day Ma[1] Then Buy On Next Day Open\"; short mirror (p.1)",
            "X|Protective stop $2,000 from entry; price reversal exit: long when currency 9-day MA < 18-day MA, short when > (p.1)"],
          "Book 1/78-10/96 (Figs 9-14): DM 181 trades 46%, SF 185 49%, BP 192 40%, JY 167 48%; composite 725 trades 46%, avg $576."),
        d("nfgu_r2_metals", "R-Squared Silver/Gold System", "Formulas p.2 · R-Squared", "Long/Short", NeedsData(METALS),
          &["F|Real interest rate = US T-Bill yield minus CPI inflation rate (p.2)",
            "E|Long: silver > 12-day SMA, real T-Bill rate < 1.0%, Swiss franc > 4-day SMA; short: silver < 12-day SMA, real rate >= 1.0%, SF < 4-day SMA (p.2)",
            "X|Protective stop $2,000; trailing stop $2,500 (p.2)"],
          "Book: silver 4/75-10/96 125 trades 54% (Fig. 15-17)."),
        d("nfgu_channel25", "Channel 25 Currency System (TradeStation code)", "Formulas p.3 · Channel 25", "Long/Short", NeedsData(FX),
          &["E|\"Buy tomorrow at highest(H, Length) stop; Sell tomorrow at lowest(L, Length) stop;\" Length 25 (TradeStation code, Fig. 27, p.20)",
            "X|\"ExitLong tomorrow at EntryPrice - InitStop/BigPointValue stop\" InitStop 2000 (Fig. 27)",
            "X|\"If (Highest(c, Value1) - EntryPrice) * BigPointValue >= TrlStop then ExitLong tomorrow at Highest(c, Value1) - TrlStop/BigPointValue stop\" TrlStop 2000, Value1 = BarsSinceEntry (Fig. 27)"],
          "Book 1/78-9/96 (Figs 21-25): DM 128 trades 49%, SF 152 56%, BP 163 53%, JY 133 52%; composite 576 trades 53%. Stop-and-reverse breakout; $ stops need the currency contracts."),
        d("nfgu_cqiii", "CQIII Currency System with adaptive trailing stop", "Formulas pp.4-5 · CQIII", "Long/Short", NeedsData(FX),
          &["E|Enter long next day open if Close > 10-day MA, 10-day MA > 10-day MA of ten days ago, Close > close of 40 days ago; short the reverse (p.4)",
            "X|Reversing signal; protective stop $2,000 from entry (p.4)",
            "X|Adaptive trailing stop: default 10 x 49-day average true range when all 3 indicators support the trade; x0.60 when 2 of 3; x0.40 when 1 (p.5)"],
          "Book 1/78-9/96 (Figs 33-36): DM 82 trades 46%, SF 74 47%, BP 93 32%, JY 75 54%; composite 323 trades 44%."),
        d("nfgu_quadram_pal", "Quadram/PAL Stock Index System", "Formulas p.6 · Quadram/PAL", "Long/Short", NeedsData(PAL),
          &["F|PAL bullish except when S&P 500 < 42-week MA and either T-Bill yield > 42-week MA or T-Bond yield > 10-week MA; bullish PAL -> longs only, bearish -> shorts only (p.6)",
            "F|CHAD = 5-day total NYSE advances minus 5-day total declines; countertrend bands = 10-day MA of CHAD +/- 1.5 SD; trend-following bands = 25-day MA +/- 1.0 SD (p.6)",
            "E|Next day open. Countertrend: long PAL bullish and CHAD crosses above lower band; short PAL bearish and CHAD crosses below upper band. Trend-following: long crosses above upper band; short crosses below lower band (p.6)",
            "X|Protective stop $1,000; trailing stop $2,000 (p.6)"],
          "Book 5/82-10/96 (Figs 68-70): 789 trades 43%, net $214,200, avg $266.25; long 682 41%, short 107 53%. The PAL filter needs T-Bill / T-Bond yields. The 2000 version without PAL is Live as nfhp_quadram_*."),
        d("nfgu_pal_model", "PAL stock market timing model", "PAL Timing Model", "Long", NeedsData(PAL),
          &["E|PAL bullish (S&P not below its 42-week MA with rising rates) = in stocks; bearish = out (p.6, Figs 59-62)"],
          "Book: S&P 1/8/54-10/4/96 CAR 15.6% vs 12.2%, DD 18.2%, exposure 75.4%; NASDAQ 1/3/64-10/4/96 15.8% vs 11.4% (pp.37-38). The workbook never states the PAL exit as a separate rule - read from the filter."),
        d("nfgu_heine", "Heine bond model (cash & futures)", "Heine Bond Model", "Long", NeedsData(HEINE),
          &["E|Same five-indicator point model as nftm_heine_bond (buy >= 3 points, sell <= 2)"],
          "Book: Heine cash 3/1/57-10/3/96 CAR 12.5% vs DJ20 bonds 8.0%, DD 3.2%, exposure 50% (p.43)."),
        d("nfgu_vl_swing", "Value Line Percent Swing System", "Value Line Percent Swing", "Long", NotTestable("Only the table labels 'Buy on N-Percent Rally / Sell on N-Percent Pullback' (Figs 2-3): no reference point, frequency or price basis; needs the Value Line Geometric index."),
          &["E|Buy on an N-percent rally, sell on an N-percent pullback (Fig. 2)"], "Book: 2% swing model 8/9/63-9/30/96 CAR 18.5% vs NASDAQ B&H 11.5%, DD 19.6%, exposure 63% (Fig. 4)."),
        d("nfgu_results_only", "Momentum, Kestner Hi/Lo MA, dual WMA T-Bond and range-breakout bond systems", "Bond systems filtered by Heine", "Long/Short", NotTestable("Result tables only (pp.44-46); rules not printed; also need T-Bond futures."),
          &["E|No rules printed"], "Dual WMA T-Bond 9/77-10/96: 122 trades 41%."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "nfgu_pathfinder" => ("nf_pathfinder", "Freeburg Pathfinder currency", "1996 · Data missing"),
        "nfgu_r2_metals" => ("nf_r2", "Freeburg R-Squared metals", "1996 · Data missing"),
        "nfgu_channel25" => ("nf_channel25", "Freeburg Channel 25", "1996 code · Data missing"),
        "nfgu_cqiii" => ("nf_cqiii", "Freeburg CQIII", "1996 adaptive stop · Data missing"),
        "nfgu_quadram_pal" => ("nf_quadram", "Freeburg Quadram S&P system", "PAL-filtered (1996) · Data missing"),
        "nfgu_pal_model" => ("nf_pal", "Freeburg PAL timing model", "Data missing"),
        "nfgu_heine" => ("nf_heine", "Heine bond timing model", "1996 · Data missing"),
        "nfgu_vl_swing" => ("nf_vlswing", "Value Line percent swing", "1996 · not testable"),
        "nfgu_results_only" => ("nf_results", "Freeburg results-only examples", "1996"),
        _ => return None,
    })
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
