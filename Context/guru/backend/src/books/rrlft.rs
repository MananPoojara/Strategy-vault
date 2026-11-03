//! RAR leftovers (rows listed as "remaining" in library/rrb3/README.md and library/rar77 / rar76 READMEs) that could NOT be run:
//! catalogue entries only, so the vault shows them with the exact reason. Sources (research drive, read in place, read-only) are
//! named per entry. Book id `rrlft`. Notes: library/rrlft/README.md. The runnable leftovers are modules mxswg, jlmms, bfm2c, tltdr.
//! Exception: rrlft_trendywendy runs (re-opened 2 Nov 2026, titles lane K20, lead decision 138): its MACD cross and ATR
//! parabolic trail stand alone; the price-unit stop and get-flat exit are held. Independent check authors/titles-1030/K20w/k20_ref.py.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, true_range, xavg, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const TW_NOTE: &str = "Source: USB HDD 1/TradeStation Systems & Indicators/TrendyWendy for TS2000i, TS7 (impressivetrading.com)/Open Code.rar -> Open Code.txt. \
No market, bar size or test is stated, so it runs on daily indices, ETFs and stocks, US and India, as a declared application. \
[Declared readings] MACD(Close,7,10) = XAverage(Close,7) - XAverage(Close,10); iAvg = XAverage(MACD,4); XAverage = TradeStation's EMA (2/(n+1)) seeded with the first value, all from the first bar of the data. \
'CROSS OVER 0' = iDiff above 0 at this close and at or below 0 at the previous one (mirror for CROSS UNDER). Entries and reversals fill at the next bar's open. \
The trail is the code's own variables: on the first bar the position is held at the close, stop = Low - 2.25 x AvgTrueRange(5) (5-bar average of the true range), AF = 0.0025, trade high = High; \
afterwards the trade high rises with each new High, stop = stop + AF x (trade high - stop), and AF grows by 0.0025 (to at most 0.6) on a bar that made a new trade high; the stop is never above that bar's Low; \
sell next bar on that stop (shorts mirror with Lows/Highs). UseLimitOrders(False) and ProfitTarget(0) are the printed inputs, so there are no limit entries or targets. \
HELD (lead decisions 19 and 138): 'SETSTOPLOSS(StopAmount * BIGPOINTVALUE)' with StopAmount 1 and the 'LX Flat'/'SX Flat' exits armed by MAXPOSITIONPROFIT >= GetFlatAmount x BIGPOINTVALUE \
(GetFlatAmount 1) are price-unit amounts of an unstated instrument and are not converted, so the card has neither; a position closes only at the trail or on a reversal.";

pub const BOOK: &str = "rrlft";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, "rrlft_trendywendy", "TrendyWendy (impressivetrading.com) MACD 7/10/4 + parabolic ATR trail", "TrendyWendy (impressivetrading.com, TradeStation code)", "Long/Short",
            Status::Live,
            &["E|\"If iMP <> 1 and iDiff CROSS OVER 0 and CLOSE > average(CLOSE, BuyAvg) then ... BUY (\\\"B\\\") NumContracts CONTRACT NEXT BAR MARKET\" (iMACD = MACD(CLOSE, FastLength 7, SlowLength 10); iAvg = XAverage(iMACD, MACDLength 4); iDiff = iMACD - iAvg; BuyAvg 5)",
              "E|\"If iMP <> - 1 and iDiff CROSS UNDER 0 and CLOSE < average(CLOSE, SellAvg) then ... SELL SHORT (\\\"S\\\") NumContracts CONTRACT NEXT BAR MARKET\" (SellAvg 5; an opposite entry reverses)",
              "X|\"If iMP[1] <> 1 then begin iStopPrice = LOW - AvgTrueRange(ATRLength) * NumATRs; iAF = afStep; iTradeHH = HIGH; end ELSE begin If HIGH > iTradeHH then iTradeHH = HIGH; iStopPrice = iStopPrice + iAF * (iTradeHH - iStopPrice); If iTradeHH > iTradeHH[1] and iAF < afLimit then iAF = iAF + MINLIST(afStep, afLimit - iAF); end; If iStopPrice > LOW then iStopPrice = LOW; SELL (\\\"LX\\\") NumContracts CONTRACT NEXT BAR iStopPrice STOP\" (ATRLength 5, NumATRs 2.25, afStep .0025, afLimit 0.6; shorts mirror with HIGH and BUY TO COVER)",
              "X|HELD: \"SETSTOPLOSS (StopAmount * BIGPOINTVALUE)\" (StopAmount 1) and the 'LX Flat' / 'SX Flat' exits armed by \"MAXPOSITIONPROFIT >= (GetFlatAmount * BIGPOINTVALUE)\" (GetFlatAmount 1): price-unit amounts, not converted (decisions 19, 138)"],
            TW_NOTE),
        sdef(BOOK, "rrlft_checkmate", "Checkmate (traderstech.net) ADX(18) 20-cross breakout", "RAR leftovers (not runnable)", "Long/Short",
            Status::NeedsData("EUR/USD forex intraday bars: the only published tests in the product folder are 'Report EURUSD 1 Min.mht', 'Report EURUSD 5 Min.mht', 'Report EURUSD 15 Min.mht'"),
            &["E|\"If ADX(18)2 BAR AGO < 20 and ADX(18)1 BAR AGO > 20 then condition5 = TRUE; If condition5 = TRUE and ADX(18) > ADX(18)1 BAR AGO + 1 and ADX(18)3 BAR AGO < 20 and DMIPLUS(18) > DMIMINUS(18) then BUY OPEN\" (SELL OPEN with DMIPLUS < DMIMINUS)",
              "X|\"EXITLONG LOWEST(LOW, 20) STOP\"; after HIGH >= CLOSE x BAR AGO + AVGTRUERANGE(3) x (y/4) (y 24, x = BARSSINCEENTRY): \"EXITLONG LOWEST(LOW, 2) STOP\"; VFILTER: \"EXITLONG OPEN x BAR AGO - ((AVGTRUERANGE(5) * 2)x BAR AGO) STOP\" (short mirror)"],
            "Source: USB HDD 1/TradeStation Systems & Indicators/Checkmate Trading System (traderstech.net)/_From other source/checkmate(opencode).rar -> checkmate_opencode_.txt. The line wraps only split whitespace; the rules are complete."),
        sdef(BOOK, "rrlft_statsignals_etf", "ETF Trading System Collection (statsignals.com): SPY / QQQQ / IWM / DIA / GLD cycle-band systems", "RAR leftovers (not runnable)", "Long",
            Status::NotTestable("every trade's exit is TradeStation's built-in SETPERCENTTRAILING on daily bars, armed by 'IF CLOSE > ENTRYPRICE+2 THEN Variable2 = TRUE' which also runs while flat (ENTRYPRICE of no position is not documented on the drive); Pruitt & Hill (p.74) state this built-in trail 'needs to know what occurred first—the high or the low of the day' and 'cannot be discerned from a daily bar'. DIA and GLD also need ETF data the vault lacks"),
            &["E|\"IF Variable8 CROSS OVER Variable24 THEN BUY NEXT BAR OPEN\" (Variable8 = detrended (3C+2C[1]+C[2]-C[4])/5 minus its average over half the Hilbert-measured cycle; Variable24 = BOLLINGERBAND(Variable8, len, -1))",
              "X|\"IF Variable8 >= Variable21 THEN SELL NEXT BAR OPEN\" (+2 band); \"SETSTOPCONTRACT; SETSTOPLOSS (6)\"; \"IF CLOSE > ENTRYPRICE+2 THEN Variable2 = TRUE; IF Variable2 = TRUE THEN SETSTOPCONTRACT; SETPERCENTTRAILING (0.75, 70)\" (SPY; DIA 6 / +1.6 / (1, 1); QQQQ 3 / +0.5 / (2, 70); IWM 5 / +1 / (1.25, 2); GLD 4 / +2 / (0.5, 50))",
              "F|Daily charts, TradeStation 8, an index as Data2 (SPY: $DJAIG; QQQQ: $IXT.X; IWM: $TICK; DIA: $TIKI; GLD: $DJAIGPR) used only as a licence check (ETF-HSYS.pdf pp.1-3)"],
            "Source: USB HDD 1/TradeStation Systems & Indicators/ETF Trading System Collection (statsignals.com)/ETF open code/ETF open code.rar -> ETF-*.txt; manual ETF-HSYS.pdf."),
        sdef(BOOK, "rrlft_schaff", "Schaff (Schaffer) Trend Cycle expert, MetaStock", "RAR leftovers (not runnable)", "Long/Short",
            Status::NotTestable("only MetaStock Expert Advisor highlights and symbols are given (no system test): no order type, fill price or delay is stated for the 'Enter Long' / 'Close Long' symbols"),
            &["F|\"MCD := Wilders( typical(),Sh) - Wilders( typical(),Lg); ST := ((MCD-LLV(MCD,Cycle)) / (HHV(MCD,Cycle)-LLV(MCD,Cycle)))*100; STC := Wilders(ST,Cycle/2)\" (Cycle 10, SH 23, LG 50)",
              "E|Expert symbol 'Enter Long': \"State=1 AND Ref(State,-1)<1\" with State from EL/ES/CL/CS on the trigger line tr = ValueWhen(1,Cross(stc,25),H) / ValueWhen(1,Cross(75,stc),L)"],
            "Source: USB HDD 1/MetaStock/@Systems & Indicators/Schaffer Cycle for MS (Formula, System and Expert)/Schaffer Cycle for MS (Formula, System and Expert).rar."),
        sdef(BOOK, "rrlft_commando2", "Commando II build 5.225 (commandotrader.com), daily branch", "RAR leftovers (not runnable)", "Long/Short",
            Status::NotTestable("the entry filter calls ADXClassic(Length) whose body is not in the archive (TradeStation built-in, formula not on the drive), and the whole signal block runs only 'IF LockFlag' where LockFlag = LTInitializeFromIni from CTS1LOCK32.DLL; the risk filters are dollars per contract (RANGE_TOL 3000 / BigPointValue, MMS_Exit 1000) of the E-mini S&P ('CME MINI S&P')"),
            &["E|\"IF Filter and pos[1]<>1 AND OK2TRADE_2 AND RangeOK AND MA_UP then ... fall_line=true; bp=(h+dif)\" ... \"Buy (\\\"LE\\\") Multiplier contracts next bar at Round2Fraction(bp) stop\" (datacompression>1 uses ma1 = xaverage(c,10), ma2 = xaverage(c,20))",
              "F|OK2TRADE_2 = _CommandoBUYfilterA(true,10,20,11): 'ADX=ADXClassic(Length)' ... 'If MRO(ADX[0]<ADX[1] and ... ADX[3]<ADX[4], 5, 1)[1] > -1 Then ADX_UP=false'"],
            "Source: USB HDD 1/TradeStation Systems & Indicators/Commando II Software Suite Build 50225 + Manual (commandotrader.com)/Code.rar -> C-II BUILD 5.225.txt (1,105 lines), CommandoBUYfilterA.txt, CommandoSELLfilterA.txt, Sl_Tck.txt."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == "rrlft_trendywendy" { return Some(("rrlft_trendywendy", "TrendyWendy (impressivetrading.com)", "MACD 7/10/4 + ATR parabolic trail")); }
    id.starts_with("rrlft_").then_some(("rrlft", "RAR leftovers (not runnable)", "catalogue only"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        pd("fast", "FastLength (MACD fast EMA)", 7., 2., 50., 1.), pd("slow", "SlowLength (MACD slow EMA)", 10., 3., 100., 1.), pd("sig", "MACDLength (signal EMA)", 4., 1., 30., 1.),
        pd("buyavg", "BuyAvg: close above its N-day average", 5., 1., 50., 1.), pd("sellavg", "SellAvg: close below its N-day average", 5., 1., 50., 1.),
        pd("atr_n", "ATRLength", 5., 1., 50., 1.), pd("atr_k", "NumATRs: first stop this many ATRs beyond the bar", 2.25, 0.25, 10., 0.25),
        pd("af_step", "afStep", 0.0025, 0.0005, 0.1, 0.0005), pd("af_lim", "afLimit", 0.6, 0.01, 1., 0.01),
    ])
}

pub fn reviewed(id: &str) -> bool { id == "rrlft_trendywendy" }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = |k: &str, d: f64| g(p, k, d).round().max(1.) as usize;
    let (fe, se) = (xavg(c, n("fast", 7.)), xavg(c, n("slow", 10.)));
    let macd: Vec<f64> = fe.iter().zip(&se).map(|(a, b)| a - b).collect();
    let sig = xavg(&macd, n("sig", 4.));
    let diff: Vec<f64> = macd.iter().zip(&sig).map(|(a, b)| a - b).collect();
    let (ba, sa) = (sma(c, n("buyavg", 5.)), sma(c, n("sellavg", 5.)));
    let atr = sma(&true_range(x), n("atr_n", 5.));
    let (k, step, lim) = (g(p, "atr_k", 2.25), g(p, "af_step", 0.0025), g(p, "af_lim", 0.6));
    let (mut prev, mut stop, mut af, mut hh, mut ll) = (0i8, 0., 0., 0., 0.);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        let mp = st.pos;
        if i >= 1 {
            if mp != 1 && diff[i] > 0. && diff[i - 1] <= 0. && c[i] > ba[i] { next.push(o(A::Buy, P::Mkt, "B")); }
            if mp != -1 && diff[i] < 0. && diff[i - 1] >= 0. && c[i] < sa[i] { next.push(o(A::Sell, P::Mkt, "S")); }
        }
        if mp == 1 {
            if prev != 1 { stop = l[i] - atr[i] * k; af = step; hh = h[i]; } else {
                let hh1 = hh;
                if h[i] > hh { hh = h[i]; }
                stop += af * (hh - stop);
                if hh > hh1 && af < lim { af += step.min(lim - af); }
            }
            if stop > l[i] { stop = l[i]; }
            next.push(o(A::Xl, P::Stop(stop), "LX"));
        }
        if mp == -1 {
            if prev != -1 { stop = h[i] + atr[i] * k; af = step; ll = l[i]; } else {
                let ll1 = ll;
                if l[i] < ll { ll = l[i]; }
                stop -= af * (stop - ll);
                if ll < ll1 && af < lim { af += step.min(lim - af); }
            }
            if stop < h[i] { stop = h[i]; }
            next.push(o(A::Xs, P::Stop(stop), "SX"));
        }
        prev = mp;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trendywendy_trails_and_reverses() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rrlft_trendywendy", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(t.iter().any(|z| z.reason == "LX" || z.reason == "SX"));
        assert!(run("rrlft_checkmate", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: TrendyWendy re-opened 2026-11-02 (titles lane K20, lead decision 138).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "impressivetrading.com TradeStation code with no market or test named (price-unit stop and get-flat exit held); runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "TrendyWendy for TS2000i, TS7 (impressivetrading.com)/Open Code.rar -> Open Code.txt (printed code); independent check authors/titles-1030/K20w/k20_ref.py",
};
