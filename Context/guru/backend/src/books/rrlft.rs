//! RAR leftovers (rows listed as "remaining" in library/rrb3/README.md and library/rar77 / rar76 READMEs) that could NOT be run:
//! catalogue entries only, so the vault shows them with the exact reason. Sources (research drive, read in place, read-only) are
//! named per entry. Book id `rrlft`. Notes: library/rrlft/README.md. The runnable leftovers are modules mxswg, jlmms, bfm2c, tltdr.
use crate::books::rrsim::sdef;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rrlft";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, "rrlft_trendywendy", "TrendyWendy (impressivetrading.com) MACD 7/10/4 + parabolic ATR trail", "RAR leftovers (not runnable)", "Long/Short",
            Status::NotTestable("the stop and the get-flat rule are absolute price units of an unstated instrument: 'SETSTOPLOSS (StopAmount * BIGPOINTVALUE)' with StopAmount(1) and 'MAXPOSITIONPROFIT >= (GetFlatAmount * BIGPOINTVALUE)' with GetFlatAmount(1); no market or bar size is stated anywhere in the archive"),
            &["E|\"If iMP <> 1 and iDiff CROSS OVER 0 and CLOSE > average(CLOSE, BuyAvg) then BUY NEXT BAR MARKET; SETSTOPLOSS (StopAmount * BIGPOINTVALUE)\" (iDiff = MACD(C,7,10) - XAverage(MACD,4), BuyAvg 5; short mirror)",
              "X|Parabolic trail from LOW - AvgTrueRange(5) x 2.25 (afStep .0025, afLimit 0.6); 'LX Flat'/'SX Flat' on the opposite MACD cross once MAXPOSITIONPROFIT >= GetFlatAmount x BIGPOINTVALUE"],
            "Source: USB HDD 1/TradeStation Systems & Indicators/TrendyWendy for TS2000i, TS7 (impressivetrading.com)/Open Code.rar -> Open Code.txt."),
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
    id.starts_with("rrlft_").then_some(("rrlft", "RAR leftovers (not runnable)", "catalogue only"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
