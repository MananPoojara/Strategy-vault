//! Thomas Stridsman, "Trading Systems and Money Management" (McGraw-Hill, 2003).
//! Every Live strategy is the book's printed TradeStation 2000i code (Part 2 ch.8-15), the Part 3 ch.20
//! stop/exit versions (book's surface-chart exit code + the table captions' settings) or the ch.28
//! Counterpunch rules. Book tested 30 Dow + 30 NASDAQ-100 stocks and 5 US indexes with $100,000 per trade
//! (Normalize) - here one index/ETF at a time, full equity per trade (= percent returns). Page refs, oracle and
//! verification: library/strmm/.
use crate::connors::{sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "strmm";

const MULTI: &str = "Needs the closes of the 4 comparison markets aligned to the traded market: per-ticker series XMKT.C2..XMKT.C5 (file <TICKER>__XMKT.csv, columns Date,C2,C3,C4,C5; real index closes; a comparison market without a bar that day keeps its previous close, as TradeStation's DataN does). Book: index version = S&P 500, NASDAQ 100, DJIA, S&P 400 MidCap, Russell 2000. Vault: US SPX/NDX/DJI/MDY/IWM (MDY, IWM = ETF stand-ins for the two mid/small-cap indexes); India NIFTY 50/NIFTYBANK/NIFTYIT/NIFTYMIDCAP100/NIFTYMIDCAPSELECT (instrument swap; MIDCAPSELECT starts 2022, so p.markets=4 drops C5 as a labelled non-default).";
const DATA2: &str = "Needs Data2 = the market index the stock is compared to (series STR_DATA2, per ticker). Book compared each stock with its index. Vault files: NIFTYBANK/NIFTYIT/NIFTYMIDCAP100 vs NIFTY 50, NDX/DJI/MDY/IWM vs SPX (instrument swap). NIFTY 50 and SPX have no parent index -> data gap.";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $vol:expr, $series:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: $vol, needs_vix: false,
            needs_series: $series, book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const XS: &[&str] = &["XMKT.C2", "XMKT.C3", "XMKT.C4", "XMKT.C5"];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        sd!("strmm_hybrid", "Hybrid System No. 1 (final code)", "Ch 8 · Hybrid System No. 1", "Long", Live, false, &[], &[
            "F|RetraceDistance = (Highest(High, RetracePeriod) - Lowest(Low, RetracePeriod)) / Aggressiveness; RetracePeriod(6), Aggressiveness(3) (code p.102-103)",
            "E|LongEntryLevel = Lowest(Low, RetracePeriod) + RetraceDistance. If MarketPosition = 0 and Close < LongEntryLevel: Buy Next Bar on LongEntryLevel Stop (p.103)",
            "E|ShortEntryLevel = Highest(High, RetracePeriod) - RetraceDistance; Sell Next Bar on ShortEntryLevel Stop when Close > ShortEntryLevel - AllowShort(False) in the code (p.102-103)",
            "X|ExitLong Next Bar at Lowest(Low, RetracePeriod) Stop; ExitShort at Highest(High, RetracePeriod) Stop (p.103)",
            "X|Original rules (p.97-98, labelled non-default): nine-day low/high, retrace half (Aggressiveness 2), \"Exit with a loss if the market is below the entry price after nine days\" (p.slow_stop=9: at the close of the 9th bar after entry, if Close < entry price, exit at that close [reading])",
        ], "Final version of ch.8 (Table 8.4: six-day retrace, no slow-trade stop, aggressive level three, long only). Book tested 65 US stocks/indexes 1990-2002; Table 8.4 is a 65-market average, not a single-market result. Stops fill at the level or at the open when the bar gaps through it. Sizing (0.5% risk) not modelled: full equity per trade."),
        sd!("strmm_hybrid_sl", "Hybrid System No. 1 · ATR stop-loss version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_hybrid (RetracePeriod 6, Aggressiveness 3, long only); original stops disabled (\"Disable original stops in system\", p.103)",
            "X|Stop loss EntryPrice - AvgTrueRange(MaxBarsVar) x 1; ATR fixed on the entry bar (p.223-226)",
            "X|Profit target EntryPrice + ATR x 1.5 (Limit) (Table 20.1, p.233)",
            "X|SetExitOnClose when BarsSinceEntry + 1 >= MaxBarsVar (6) (p.223)",
        ], "Table 20.1 caption: \"Long only: 1 ATR stop loss, 1.5 ATR profit target, 6 bars max. trade length\". ATR = simple average of TrueRange over MaxBarsVar bars (TradeStation AvgTrueRange)."),
        sd!("strmm_hybrid_ts", "Hybrid System No. 1 · ATR trailing-stop version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_hybrid (long only), original stops disabled",
            "X|Trailing stop Highest(High, BarsSinceEntry + 1) - AvgTrueRange(MaxBarsVar) x 1.2, ATR recalculated each bar (p.223-224)",
            "X|Profit target EntryPrice + ATR x 3 (Limit); SetExitOnClose when BarsSinceEntry + 1 >= 6 (Table 20.2, p.236)",
        ], "Table 20.2 caption: \"Long only: 1.2 ATR trailing stop, 3 ATR profit target, 6 bars max. trade length\"."),
        sd!("strmm_rs1", "RS System No. 1 (final code)", "Ch 9 · RS System No. 1", "Long", Live, true, &["STR_DATA2"], &[
            "F|RelStrength = Close / Close Data2; LongPercRelStrength = RelStrength / Average(RelStrength, StrengthLookback 20) (p.111)",
            "F|VolStrength = OBV; LongPercVolStrength = OBV / Average(OBV, 20); LongPercStrength = LongPercRelStrength x LongPercVolStrength; ShortPercStrength = 1 / LongPercStrength (p.111)",
            "E|EntryStrength = 1 + StrengthModifier(-5) x 0.01 = 0.95. If MarketPosition = 0 and LongPercStrength crosses above EntryStrength: Buy Next Bar at Market (p.111)",
            "X|If EntryPrice > 0 and ShortPercStrength crosses above EntryStrength: ExitLong Next Bar at Market (p.111-112)",
            "X|Original rules (p.105-106, labelled non-default): five-day average, strength 1.0 (p.modifier=0, p.lookback=5), \"Exit with a loss if the market falls below the entry price minus 4 percent\" (p.stop_pct=4)",
        ], DATA2),
        sd!("strmm_meander", "Meander System V.1.0 (final code)", "Ch 10 · Meander System", "Long", Live, false, &[], &[
            "F|PriceDiff for each of the last MeanderLookback(10) bars: (Open, High, Low, Close of bar k - Close of bar k+1) / Close of bar k+1 -> 40 points; AverageDiff = mean; MeanderStDev = SquareRoot((N x SumSquareError - SumError^2) / (N x (N - 1))) (p.120-121)",
            "F|MeanderLow = Close x (1 + AverageDiff - MeanderStDev); MeanderLowest = Close x (1 + AverageDiff - 2 x StDev); MeanderHighest = Close x (1 + AverageDiff + 2 x StDev) (p.121)",
            "E|If MarketPosition = 0 and Close < Close[1] and Close < Open: Buy Next Bar at MeanderLow Limit (p.121)",
            "X|ExitLong Next Bar at MeanderLowest Stop; ExitLong Next Bar at MeanderHighest Limit (recomputed every bar) (p.121)",
        ], "Final version (Table 10.5: higher target, no slack period, one down day, 10-day lookback, long only). If a stop and the target both lie inside one bar, the stop is assumed first; a gap through either fills at the open."),
        sd!("strmm_meander_sl", "Meander V.1.0 · ATR stop-loss version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_meander; original stop/target disabled",
            "X|Stop loss EntryPrice - ATR x 1.8 (ATR fixed on the entry bar), profit target EntryPrice + ATR x 3, SetExitOnClose when BarsSinceEntry + 1 >= 8; ATR length = MaxBarsVar (p.223-224, Table 20.3 p.239)",
        ], "Table 20.3 caption: \"Long only: 1.8 ATR stop loss, 3 ATR profit target, 8 bars max. trade length\" (the prose on p.237 mentions nine days; the table and p.239 settle on eight)."),
        sd!("strmm_meander_ts", "Meander V.1.0 · ATR trailing-stop version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_meander; original stop/target disabled",
            "X|Trailing stop Highest(High since entry) - ATR x 2.7 (ATR recalculated each bar), target EntryPrice + ATR x 3.5, SetExitOnClose when BarsSinceEntry + 1 >= 9 (Table 20.4, p.240)",
        ], "Table 20.4 caption: \"Long only: 2.7 ATR trailing stop, 3.5 ATR profit target, 9 bars max. trade length\"."),
        sd!("strmm_rsbands", "Relative Strength Bands (final code, index version)", "Ch 11 · Relative Strength Bands", "Long/Short", Live, false, XS, &[
            "F|RelStrength.1n = Close / Close DataN for the 4 comparison markets; PercRelStre = RelStrength / Average(RelStrength, LB); TotRelStre = product of the four (p.134-135)",
            "E|Long: LongRelStrengthLB(100), LongStDevs(0): LongTriggerStrength = 1 + StdDev(LongTotRelStre, 100) x 0. If MarketPosition = 0 and LongTotRelStre > LongTriggerStrength: Buy Next Bar at Market (p.135)",
            "E|Short: ShortRelStrengthLB(21), ShortStDevs(3): ShortTriggerStrength = 1 - StdDev(ShortTotRelStre, 21) x 3. If MarketPosition = 0 and ShortTotRelStre < ShortTriggerStrength: Sell Next Bar at Market (p.135; the printed code says TotRelStrength, an undeclared name - read as ShortTotRelStre)",
            "X|ExitLong Next Bar at Market when LongTotRelStre < LongTriggerStrength; ExitShort when ShortTotRelStre > ShortTriggerStrength (p.135)",
        ], MULTI),
        sd!("strmm_rotation", "Rotation (final code, index version)", "Ch 12 · Rotation", "Long/Short", Live, false, XS, &[
            "F|For every market m and every other market k: RelStrength = Close m / Close k; AvgSlope = Average(RS, LB) / Average(RS, LB)[SlopeLookback 10]; TotStrength m = product over k. Long LB 80, short LB 60 (p.146-151)",
            "F|MinLongStrength = NthMaxList(NoLongMarkets 4, TotStrength of all 5); MaxShortStrength = NthMinList(NoShortMarkets 2, ...) (p.151-152)",
            "E|If MarketPosition = 0 and LongTotStrength.1 >= MinLongStrength and AvgTrueRange(10) > AvgTrueRange(5): Buy Next Bar at Market (p.152)",
            "E|If MarketPosition = 0 and ShortTotStrength.1 <= MaxShortStrength and AvgTrueRange(10) < AvgTrueRange(5): Sell Next Bar at Market (p.152)",
            "X|Long: stop EntryPrice - AvgTrueRange(5) x 2, target EntryPrice + ATR(5) x 6; short: stop + ATR(5) x 1, target - ATR(5) x 3; ATR(5) of the current bar (p.152)",
        ], "Code: NoLongMarkets(4), NoShortMarkets(2). The prose (p.139) says two of five indexes long and four of nine for stocks; the code value wins (p.n_long=2 = prose reading, labelled). The prose's 1%/3% short stop/target (Table 12.8) became ATR multiples in the code; the code is used. Book: 5 indexes + 20 stocks, March 1991-Feb 2001 original, retest through 2002. Needs the 4 comparison markets (see strmm_rsbands)."),
        sd!("strmm_vwa", "Volume-weighted Average (final code)", "Ch 13 · Volume-weighted Average", "Long/Short", Live, true, &[], &[
            "F|AverageVolume = Average(Volume, L); RelAvgVolume = (AverageVolume - Lowest(AverageVolume, L)) / (Highest(AverageVolume, L) - Lowest(AverageVolume, L)); VWA = Rel x AvgPrice + (1 - Rel) x VWA[1]; AvgPrice = (O+H+L+C)/4; long L = 21, short L = 10 (p.159-161)",
            "E|Long: MarketPosition = 0 and Close < VWA(21) and VWA(21) < VWA(21)[1]: Buy Next Bar at Highest(High, TriggerLength 2) Stop (p.160)",
            "E|Short: MarketPosition = 0 and Close > VWA(10) and VWA(10) > VWA(10)[1]: Sell Next Bar at Lowest(Low, 2) Stop (p.161)",
            "X|If MarketPosition = 1 and Close < VWA(21): ExitLong on Close; if MarketPosition = -1 and Close > VWA(10): ExitShort on Close (applies on the entry bar too) (p.161)",
        ], "Final code: AllowLong(True), AllowShort(True), no stop loss, no trend filter (Tables 13.5/13.6). VWA recursion starts from 0 at the first bar with a full volume window (TradeStation variable initial value); a bar where Highest = Lowest average volume leaves VWA unchanged (book silent on division by zero). If both entry stops are pending and both trade, the one nearer the open is assumed filled."),
        sd!("strmm_vwa_sl", "Volume-weighted Average · ATR stop-loss version", "Ch 20 · Adding exits", "Long", Live, true, &[], &[
            "E|Entry = strmm_vwa long side (21-day); exit on close below VWA kept (an exit, not a stop) - p.keep_vwa_exit=0 drops it [labelled]",
            "X|Stop loss EntryPrice - ATR x 1.4 (fixed at entry), target EntryPrice + ATR x 2, SetExitOnClose when BarsSinceEntry + 1 >= 6 (Table 20.5 caption, p.242)",
        ], "Table 20.5 caption: \"Long only: 1.4 ATR stop loss, 2 ATR profit target, 6 bars max\". The prose (p.241) also mentions a 0.4%/4.5%/7-day percentage version and a 1.4 ATR trailing stop; the printed tables are used and the contradiction is noted."),
        sd!("strmm_vwa_ts", "Volume-weighted Average · ATR trailing-stop version", "Ch 20 · Adding exits", "Long", Live, true, &[], &[
            "E|Entry = strmm_vwa long side; exit on close below VWA kept (p.keep_vwa_exit)",
            "X|Trailing stop Highest(High since entry) - ATR x 2.4 (recalculated), target EntryPrice + ATR x 2, SetExitOnClose when BarsSinceEntry + 1 >= 6 (Table 20.6, p.245)",
        ], "Table 20.6 caption: \"Long only: 2.4 ATR trailing stop, 2 ATR profit target, 6 bars max. trade length\"."),
        sd!("strmm_harris", "Harris 3L-R Pattern Variation (final code)", "Ch 14 · Harris 3L-R", "Long/Short", Live, false, &[], &[
            "E|If MarketPosition = 0 and Low < Low[2] and Low[1] < Low[2]: Buy Next Bar on High[2] Stop (p.171)",
            "E|If MarketPosition = 0 and High > High[2] and High[1] > High[2]: Sell Next Bar on Low[2] Stop (p.171)",
            "X|PercentTarget(3), PercentLoss(1): ExitLong at EntryPrice x 1.01... Stop: EntryPrice x (1 - 0.01); Limit: EntryPrice x (1 + 0.03); shorts mirrored (p.171-172)",
        ], "Final version (Tables 14.6/14.7: 3:1 risk-reward, open excluded, slightly altered pattern; AllowLong and AllowShort True). If both entry stops trade on one bar, the one nearer the open is assumed filled (book silent)."),
        sd!("strmm_harris_orig", "Harris 3L-R Pattern (original rules)", "Ch 14 · Harris 3L-R", "Long", Live, false, &[], &[
            "E|\"Go long on the next open if today's high is higher than the high three days ago, yesterday's low is lower than the low two days ago, and the low two days ago is lower than the low three days ago.\" (p.164)",
            "X|\"Exit all trades with a loss if the market moves against the position by 4 percent or more.\" (stop at entry x 0.96) (p.164)",
            "X|\"Exit all trades with a profit if the market moves in favor of the position by 12 percent or more.\" (limit at entry x 1.12) (p.164)",
        ], "Table 14.1 (65 markets, Jan 1990-Feb 2002, long only): 50.20 trades, 34.10% winners, avg profit $1,295 per $100,000 per market (averages over markets). Exits start the bar after entry (TradeStation next-bar orders)."),
        sd!("strmm_harris_sl", "Harris 3L-R · percentage stop-loss version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_harris long side; original stop/target disabled",
            "X|Stop EntryPrice x (1 - 0.4%), target EntryPrice x (1 + 3%), SetExitOnClose when BarsSinceEntry + 1 >= 4 (Table 20.7, p.247)",
        ], "Table 20.7 caption: \"Long only: 0.4% stop loss, 3% profit target, 4 bars max. trade length\"."),
        sd!("strmm_harris_ts", "Harris 3L-R · percentage trailing-stop version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_harris long side",
            "X|Trailing stop Highest(High, BarsSinceEntry + 1) x (1 - 2.7%), target EntryPrice x 1.045, SetExitOnClose when BarsSinceEntry + 1 >= 5 (Table 20.8, p.248)",
        ], "Table 20.8 caption: \"Long only: 2.7% trailing stop, 4.5% profit target, 5 bars max. trade length\"."),
        sd!("strmm_expert", "Expert Exits (code)", "Ch 15 · Expert Exits", "Long/Short", Live, false, &[], &[
            "E|If MarketPosition = 0 and High < High[2] and Close < Open and Open Next Bar < High: Buy Next Bar at High Stop (p.182) - the prose (p.174) says yesterday's high below the previous day's high (High[1]); p.hh_lag=1 is that labelled reading",
            "E|If MarketPosition = 0 and Low > Low[2] and Close > Open and Open Next Bar > Low: Sell Next Bar at Low Stop (p.182)",
            "X|LossExit(1): ExitLong at EntryPrice x 0.99 Stop; ProfitExit(4.5): ExitLong at EntryPrice x 1.045 Limit (shorts mirrored) (p.182-183)",
            "X|If BarsSinceEntry = BarsInTrade(8) + 1: Exit Next Bar at Market (p.183)",
            "X|TrailingStop(False) default; p.trailing=1: stop at Top x (1 - LossExit) where the code sets Top = High each bar before Top = MaxList(Top, High), i.e. the current bar's high (code literal; Table 15.5 used LossExit 1.6, ProfitExit 4)",
        ], "Original system both sides (Table 15.1, 65 markets: 297 trades, 32.52% winners, avg $348.56). RandomEntry(False). Sizing (4 x ATR(10), 2.5% risk) not modelled."),
        sd!("strmm_expert_sl", "Expert Exits · percentage stop-loss version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_expert long side; original exits disabled",
            "X|Stop EntryPrice x (1 - 0.6%), target EntryPrice x 1.035, SetExitOnClose when BarsSinceEntry + 1 >= 3 (Table 20.9, p.249)",
        ], "Table 20.9 caption: \"Long only: 0.6% stop loss, 3.5% profit target, 3 bars max. trade length\"."),
        sd!("strmm_expert_ts", "Expert Exits · percentage trailing-stop version", "Ch 20 · Adding exits", "Long", Live, false, &[], &[
            "E|Entry = strmm_expert long side",
            "X|Trailing stop Highest(High, BarsSinceEntry + 1) x (1 - 2.7%), target EntryPrice x 1.045, SetExitOnClose when BarsSinceEntry + 1 >= 4 (Table 20.10, p.250)",
        ], "Table 20.10 caption: \"Long only: 2.7% trailing stop, 4.5% profit target, 4 bars max. trade length\"."),
        sd!("strmm_counterpunch", "Counterpunch Stock System", "Ch 28 · Counterpunch", "Long/Short", Live, false, &[], &[
            "E|\"Go long tomorrow on the open if a) today's close is below both yesterday's close and the close of the previous week, b) yesterday's close is below the previous day's close and, c) the close of the previous week is below the close of the week before that.\" (p.376) Week = calendar week (Mon-Sun); close of a week = its last daily close",
            "X|\"Exit first half of position with a loss if the trade goes against you by 1 percent. Exit first half of position with a profit if the trade goes your way by 4 percent.\" (p.376)",
            "X|\"Exit second half with a profit or loss if the trade moves 1.6 percent away from your maximum open profit (i.e., trail a stop 1.6 percent below the high of the trade). Exit second half with a profit if the trade goes your way by 4.5 percent.\" (p.376)",
            "X|\"Exit the entire position after eight days in the trade.\" Read with the book's Expert Exits code, which these exits come from (p.374, 183): when BarsSinceEntry = 8 + 1, exit next bar at market [reading]",
            "X|\"Reverse the rules for short trades.\" (p.376)",
        ], "Book: 30 Dow stocks Jan 1993-Jul 2002, $10 costs, 2.5% risk with 4 x prior-day true range sizing: 18,893 trades, 38.4% winners, avg $205, avg 2.3 days (Table 28.19). Portfolio money-managed result; not comparable to one index. Stops/targets work from the bar after entry; trade high includes the entry bar."),
        sd!("strmm_rsbands_orig", "Relative Strength Bands (original rules)", "Ch 11 · Relative Strength Bands", "Long/Short",
            NotTestable("original code not printed; the prose places the entry trigger at a 21-day one-SD 'Bollinger band' of PRSV while the printed (revised) code centres the band on 1.0 - band centre not determinable from the book"), false, &[], &[
            "E|Look for long signals only when the PRSV is above its 21-day, one-SD Bollinger band; enter at the close when PRSV > its value two days ago (p.124)",
            "X|Trailing stop at lowest low of last two bars or PRSV < its value four days ago; exit unprofitable trades after eight bars (p.124)",
        ], "Revised version is strmm_rsbands."),
        sd!("strmm_rotation_orig", "Rotation (original rules)", "Ch 12 · Rotation", "Long/Short",
            NotTestable("the original MAS indicator is only referenced to Active Trader May 2001 (p.137); its exact formula is not in this book"), false, &[], &[
            "E|Long the two strongest of DJIA/NASDAQ 100/S&P 500 by the 80-day MAS 20-day slope; short the weakest; enter at the open if the 3-day average range < 30-day average range (p.138)",
        ], "Revised version is strmm_rotation."),
        sd!("strmm_filters", "Short-term systems filtered by RS1 / RS Bands / Rotation", "Ch 21 · Systems as filters", "Long",
            NotTestable("the book prints no code or rule for how a filter system gates the short-term system (in a position? signal day? same side?) - Tables 21.3-21.12 cannot be reproduced without inventing it"), false, &[], &[
            "F|Meander / VWA / Harris / Expert Exits / Hybrid exit versions traded only when RS System No. 1, Relative Strength Bands or Rotation agree (p.255-270)",
        ], "Unfiltered versions are Live."),
        sd!("strmm_random", "Random-entry tests (Expert Exits RNG, trend-filter study)", "Ch 15, 21", "Long/Short",
            NotTestable("random-number entries (IntPortion(Random(2))); the book reports averages over 10 random runs per market, not a reproducible rule"), false, &[], &[
            "E|RandomTrigger = IntPortion(Random(2)); enter only when RandomTrigger = 1 (p.182, 225); random 5-day trades with/without a 200-day MA (p.255-257)",
        ], "Statistics about entries, not strategies."),
        sd!("strmm_money_mgmt", "Kelly / fixed fractional / dynamic ratio money management (Strategies 1-12)", "Part 4 · Ch 24-28", "Long",
            NotTestable("portfolio money management across 65 markets and several systems traded together (dynamic ratio, fixed fractional f); the vault runs one market at a time"), false, &[], &[
            "A|Fixed fractional and dynamic ratio money management across the system/market portfolio (p.289-372)",
        ], "Position sizing studies, not entry/exit rules."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "strmm_hybrid" => ("strmm_hybrid", "Hybrid System No. 1", "Final code (ch.8)"),
        "strmm_hybrid_sl" => ("strmm_hybrid", "Hybrid System No. 1", "ATR stop-loss (ch.20)"),
        "strmm_hybrid_ts" => ("strmm_hybrid", "Hybrid System No. 1", "ATR trailing stop (ch.20)"),
        "strmm_rs1" => ("strmm_rs", "Relative strength systems", "RS System No. 1"),
        "strmm_rsbands" | "strmm_rsbands_orig" => ("strmm_rs", "Relative strength systems", "Relative Strength Bands"),
        "strmm_rotation" | "strmm_rotation_orig" => ("strmm_rs", "Relative strength systems", "Rotation"),
        "strmm_meander" => ("strmm_meander", "Meander V.1.0", "Final code (ch.10)"),
        "strmm_meander_sl" => ("strmm_meander", "Meander V.1.0", "ATR stop-loss (ch.20)"),
        "strmm_meander_ts" => ("strmm_meander", "Meander V.1.0", "ATR trailing stop (ch.20)"),
        "strmm_vwa" => ("strmm_vwa", "Volume-weighted Average", "Final code (ch.13)"),
        "strmm_vwa_sl" => ("strmm_vwa", "Volume-weighted Average", "ATR stop-loss (ch.20)"),
        "strmm_vwa_ts" => ("strmm_vwa", "Volume-weighted Average", "ATR trailing stop (ch.20)"),
        "strmm_harris" => ("strmm_harris", "Harris 3L-R", "Final code (ch.14)"),
        "strmm_harris_orig" => ("strmm_harris", "Harris 3L-R", "Original rules 4%/12%"),
        "strmm_harris_sl" => ("strmm_harris", "Harris 3L-R", "% stop-loss (ch.20)"),
        "strmm_harris_ts" => ("strmm_harris", "Harris 3L-R", "% trailing stop (ch.20)"),
        "strmm_expert" => ("strmm_expert", "Expert Exits", "Code (ch.15)"),
        "strmm_expert_sl" => ("strmm_expert", "Expert Exits", "% stop-loss (ch.20)"),
        "strmm_expert_ts" => ("strmm_expert", "Expert Exits", "% trailing stop (ch.20)"),
        "strmm_counterpunch" => ("strmm_counterpunch", "Counterpunch", ""),
        "strmm_filters" | "strmm_random" | "strmm_money_mgmt" => ("strmm_other", "Filters, random entries, money management", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hyb = || vec![p("retrace", "RetracePeriod (bars)", 6., 2., 20., 1.), p("aggr", "Aggressiveness (range divisor)", 3., 1., 5., 0.5)];
    let sct = |sl: f64, tr: f64, pt: f64, bars: f64, atr: bool| {
        let u = if atr { "ATR" } else { "%" };
        vec![
            p("sl", if atr { "Stop loss (ATR, 0 = off)" } else { "Stop loss (%, 0 = off)" }, sl, 0., 5., if atr { 0.2 } else { 0.1 }),
            p("trail", if atr { "Trailing stop (ATR, 0 = off)" } else { "Trailing stop (%, 0 = off)" }, tr, 0., 5., if atr { 0.1 } else { 0.1 }),
            p("pt", if u == "ATR" { "Profit target (ATR)" } else { "Profit target (%)" }, pt, 0.5, 15., 0.5),
            p("max_bars", "Max trade length (bars; also the ATR length)", bars, 1., 20., 1.),
        ]
    };
    let cat = |mut a: Vec<ParamDef>, b: Vec<ParamDef>| { a.extend(b); a };
    Some(match id {
        "strmm_hybrid" => cat(hyb(), vec![p("allow_short", "AllowShort (0 = book code)", 0., 0., 1., 1.), p("slow_stop", "Slow-trade stop bars (0 = final code; 9 = original rules)", 0., 0., 20., 1.)]),
        "strmm_hybrid_sl" => cat(hyb(), sct(1., 0., 1.5, 6., true)),
        "strmm_hybrid_ts" => cat(hyb(), sct(0., 1.2, 3., 6., true)),
        "strmm_rs1" => vec![p("lookback", "StrengthLookback (bars)", 20., 3., 60., 1.), p("modifier", "StrengthModifier (%)", -5., -10., 5., 1.),
            p("stop_pct", "Stop loss % (0 = final code; 4 = original rules)", 0., 0., 10., 0.5)],
        "strmm_meander" => vec![p("lookback", "MeanderLookback (max 25)", 10., 2., 25., 1.)],
        "strmm_meander_sl" => cat(vec![p("lookback", "MeanderLookback (max 25)", 10., 2., 25., 1.)], sct(1.8, 0., 3., 8., true)),
        "strmm_meander_ts" => cat(vec![p("lookback", "MeanderLookback (max 25)", 10., 2., 25., 1.)], sct(0., 2.7, 3.5, 9., true)),
        "strmm_rsbands" => vec![p("long_lb", "LongRelStrengthLB", 100., 10., 250., 5.), p("short_lb", "ShortRelStrengthLB", 21., 5., 100., 1.),
            p("long_sd", "LongStDevs", 0., 0., 3., 0.5), p("short_sd", "ShortStDevs", 3., 0., 4., 0.5),
            p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.),
            p("markets", "Markets incl. traded (5 = book; 4 = drop C5, labelled)", 5., 4., 5., 1.)],
        "strmm_rotation" => vec![p("long_lb", "LongRelStrenLookb", 80., 20., 160., 5.), p("short_lb", "ShortRelStrenLookb", 60., 20., 160., 5.),
            p("slope", "SlopeLookback", 10., 2., 40., 1.), p("n_long", "NoLongMarkets (4 = code; 2 = prose)", 4., 1., 5., 1.),
            p("n_short", "NoShortMarkets", 2., 1., 5., 1.), p("ltr", "LongTermTRLookback", 10., 3., 60., 1.), p("str", "ShortTermTRLookback", 5., 1., 30., 1.),
            p("long_stop", "LongStopMulti (ATR)", 2., 0.5, 6., 0.5), p("short_stop", "ShortStopMulti (ATR)", 1., 0.5, 6., 0.5),
            p("long_tgt", "LongTargetMulti (ATR)", 6., 1., 12., 0.5), p("short_tgt", "ShortTargetMulti (ATR)", 3., 1., 12., 0.5),
            p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.),
            p("markets", "Markets incl. traded (5 = book; 4 = drop C5, labelled)", 5., 4., 5., 1.)],
        "strmm_vwa" => vec![p("up_len", "AverageUpLength", 21., 3., 60., 1.), p("down_len", "AverageDownLength", 10., 3., 60., 1.),
            p("trigger", "TriggerLength", 2., 1., 10., 1.), p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.)],
        "strmm_vwa_sl" => cat(vec![p("up_len", "AverageUpLength", 21., 3., 60., 1.), p("trigger", "TriggerLength", 2., 1., 10., 1.),
            p("keep_vwa_exit", "Keep exit on close below VWA (1 = kept)", 1., 0., 1., 1.)], sct(1.4, 0., 2., 6., true)),
        "strmm_vwa_ts" => cat(vec![p("up_len", "AverageUpLength", 21., 3., 60., 1.), p("trigger", "TriggerLength", 2., 1., 10., 1.),
            p("keep_vwa_exit", "Keep exit on close below VWA (1 = kept)", 1., 0., 1., 1.)], sct(0., 2.4, 2., 6., true)),
        "strmm_harris" => vec![p("pt", "PercentTarget", 3., 0.5, 15., 0.5), p("sl", "PercentLoss", 1., 0.2, 10., 0.1),
            p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.)],
        "strmm_harris_orig" => vec![p("pt", "Profit target %", 12., 1., 20., 0.5), p("sl", "Stop loss %", 4., 0.5, 10., 0.5)],
        "strmm_harris_sl" => sct(0.4, 0., 3., 4., false),
        "strmm_harris_ts" => sct(0., 2.7, 4.5, 5., false),
        "strmm_expert" => vec![p("bars", "BarsInTrade", 8., 1., 20., 1.), p("pt", "ProfitExit %", 4.5, 0.5, 10., 0.5), p("sl", "LossExit %", 1., 0.2, 5., 0.1),
            p("trailing", "TrailingStop (0 = code default)", 0., 0., 1., 1.), p("hh_lag", "High < High[n]: 2 = code, 1 = prose", 2., 1., 2., 1.),
            p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.)],
        "strmm_expert_sl" => cat(vec![p("hh_lag", "High < High[n]: 2 = code, 1 = prose", 2., 1., 2., 1.)], sct(0.6, 0., 3.5, 3., false)),
        "strmm_expert_ts" => cat(vec![p("hh_lag", "High < High[n]: 2 = code, 1 = prose", 2., 1., 2., 1.)], sct(0., 2.7, 4.5, 4., false)),
        "strmm_counterpunch" => vec![p("sl1", "First half stop %", 1., 0.2, 5., 0.1), p("pt1", "First half target %", 4., 0.5, 10., 0.5),
            p("trail2", "Second half trailing stop %", 1.6, 0.2, 5., 0.1), p("pt2", "Second half target %", 4.5, 0.5, 10., 0.5),
            p("days", "Days in trade", 8., 1., 20., 1.), p("allow_long", "AllowLong", 1., 0., 1., 1.), p("allow_short", "AllowShort", 1., 0., 1., 1.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

/// TradeStation StdDev (population, divides by n); NaN while the window holds a NaN.
fn stdev_pop(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| {
        if i + 1 < n || !m[i].is_finite() { return f64::NAN; }
        (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}
fn highest(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
fn lowest(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min)
}
/// TradeStation AvgTrueRange(n) = Average(TrueRange, n).
fn avg_tr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n.max(1)) }

/// A comparison close series, carried forward over days the comparison market did not trade.
fn ffill(v: &[f64]) -> Vec<f64> {
    let mut last = f64::NAN;
    v.iter().map(|&x| { if x.is_finite() && x > 0. { last = x; } last }).collect()
}
/// Own close + the comparison closes XMKT.C2.. (markets - 1 of them).
fn markets(x: &Ctx, k: usize) -> Option<Vec<Vec<f64>>> {
    let mut out = vec![x.c.clone()];
    for name in XS.iter().take(k.saturating_sub(1)) { out.push(ffill(x.series(name)?)); }
    Some(out)
}

#[derive(Clone, Copy)]
enum Ord { Mkt, Stop(f64), Limit(f64) }

struct Pos { t: Trade, bse: usize, fix: f64, top: f64, bot: f64 }

#[derive(Default)]
struct Ex { stop: Option<f64>, limit: Option<f64>, mkt: Option<&'static str> }

fn open_trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_trade(mut t: Trade, i: usize, px: f64, why: &str, sig: usize) -> Trade {
    t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = sig; t
}
fn fill_entry(x: &Ctx, i: usize, dir: f64, o: Ord) -> Option<f64> {
    match o {
        Ord::Mkt => Some(x.o[i]),
        Ord::Stop(s) if !s.is_finite() => None,
        Ord::Limit(s) if !s.is_finite() => None,
        Ord::Stop(s) => if dir > 0. { (x.h[i] >= s).then(|| x.o[i].max(s)) } else { (x.l[i] <= s).then(|| x.o[i].min(s)) },
        Ord::Limit(s) => if dir > 0. { (x.l[i] <= s).then(|| x.o[i].min(s)) } else { (x.h[i] >= s).then(|| x.o[i].max(s)) },
    }
}
/// Exit fill on bar i: market at the open; a gap through the stop or target fills at the open; otherwise the
/// stop is assumed before the target when both lie inside the bar.
fn fill_exit(x: &Ctx, i: usize, dir: f64, e: &Ex) -> Option<(f64, &'static str)> {
    if let Some(why) = e.mkt { return Some((x.o[i], why)); }
    let (o, adverse, favour) = if dir > 0. { (x.o[i], x.l[i], x.h[i]) } else { (x.o[i], x.h[i], x.l[i]) };
    let worse = |a: f64, b: f64| if dir > 0. { a <= b } else { a >= b }; // a at or beyond b against the trade
    if let Some(s) = e.stop.filter(|s| s.is_finite()) { if worse(o, s) { return Some((o, "stop")); } }
    if let Some(t) = e.limit.filter(|t| t.is_finite()) { if worse(t, o) { return Some((o, "target")); } }
    if let Some(s) = e.stop.filter(|s| s.is_finite()) { if worse(adverse, s) { return Some((s, "stop")); } }
    if let Some(t) = e.limit.filter(|t| t.is_finite()) { if worse(t, favour) { return Some((t, "target")); } }
    None
}

/// Bar loop with TradeStation next-bar order semantics: at the close of bar i the system places entry orders
/// (when flat) or exit orders (when in a position) for bar i+1; "on close" exits act on bar i itself.
/// No exit orders exist on the entry bar (they are placed at its close); a stop exit frees the next close for a new entry.
fn sim(x: &Ctx, from: usize, to: usize,
       mut entry: impl FnMut(usize) -> Vec<(f64, Ord, &'static str)>,
       mut on_close: impl FnMut(usize, &mut Pos) -> Option<&'static str>,
       mut exits: impl FnMut(usize, &Pos) -> Ex) -> Vec<Trade> {
    let mut out = vec![];
    let (mut pend_e, mut pend_x): (Vec<(f64, Ord, &'static str)>, Ex) = (vec![], Ex::default());
    let mut pos: Option<Pos> = None;
    for i in from..=to {
        if let Some(p) = pos.as_mut() {
            p.bse += 1;
            if let Some((px, why)) = fill_exit(x, i, p.t.dir, &pend_x) {
                let t = pos.take().unwrap().t;
                out.push(close_trade(t, i, px, why, if pend_x.mkt.is_some() { i - 1 } else { i }));
            }
        } else if !pend_e.is_empty() {
            // several pending entry orders: markets first, then the stop/limit nearest the open
            let mut best: Option<(f64, f64, &'static str, f64)> = None;
            for &(dir, o, kind) in &pend_e {
                if let Some(px) = fill_entry(x, i, dir, o) {
                    let d = match o { Ord::Mkt => -1., _ => (px - x.o[i]).abs() };
                    if best.map_or(true, |b| d < b.3) { best = Some((dir, px, kind, d)); }
                }
            }
            if let Some((dir, px, kind, _)) = best {
                pos = Some(Pos { t: open_trade(dir, i, px, kind, i - 1), bse: 0, fix: f64::NAN, top: x.h[i], bot: x.l[i] });
            }
        }
        pend_e.clear();
        pend_x = Ex::default();
        if let Some(p) = pos.as_mut() {
            p.top = p.top.max(x.h[i]);
            p.bot = p.bot.min(x.l[i]);
            if let Some(why) = on_close(i, p) {
                let t = pos.take().unwrap().t;
                out.push(close_trade(t, i, x.c[i], why, i));
                continue;
            }
            if i < to { pend_x = exits(i, p); }
        } else if i < to {
            pend_e = entry(i);
        }
    }
    if let Some(p) = pos { let mut t = close_trade(p.t, to, x.c[to], "open", to); t.open = true; out.push(t); }
    out
}

/// Stridsman's surface-chart exit code (ch.20 p.222-224): percentage or true-range stop loss, trailing stop,
/// profit target and SetExitOnClose after MaxBarsVar bars; ATR length = MaxBarsVar; stop-loss ATRs fixed on the
/// entry bar, trailing-stop ATRs recalculated every bar.
struct Sct { atr: bool, sl: f64, tr: f64, pt: f64, bars: usize, a: Vec<f64> }
impl Sct {
    fn new(x: &Ctx, g: &dyn Fn(&str) -> f64, atr: bool) -> Self {
        let bars = g("max_bars").round().max(1.) as usize;
        Sct { atr, sl: g("sl"), tr: g("trail"), pt: g("pt"), bars, a: if atr { avg_tr(x, bars) } else { vec![] } }
    }
    fn on_close(&self, i: usize, p: &mut Pos) -> Option<&'static str> {
        if p.bse == 0 { p.fix = if self.atr { self.a[i] } else { 0. }; }
        (p.bse + 1 >= self.bars).then_some("max bars (close)")
    }
    fn exits(&self, i: usize, p: &Pos) -> Ex {
        let (d, ep) = (p.t.dir, p.t.legs[0].px);
        let mut e = Ex::default();
        let tighter = |a: Option<f64>, b: f64| Some(a.map_or(b, |a| if d > 0. { a.max(b) } else { a.min(b) }));
        if self.atr {
            let (unit_sl, unit) = if self.tr > 0. { (self.a[i], self.a[i]) } else { (p.fix, p.fix) };
            if self.sl > 0. { e.stop = tighter(e.stop, ep - d * unit_sl * self.sl); }
            if self.tr > 0. { e.stop = tighter(e.stop, if d > 0. { p.top - self.a[i] * self.tr } else { p.bot + self.a[i] * self.tr }); }
            if self.pt > 0. { e.limit = Some(ep + d * unit * self.pt); }
        } else {
            if self.sl > 0. { e.stop = tighter(e.stop, ep * (1. - d * self.sl / 100.)); }
            if self.tr > 0. { e.stop = tighter(e.stop, if d > 0. { p.top * (1. - self.tr / 100.) } else { p.bot * (1. + self.tr / 100.) }); }
            if self.pt > 0. { e.limit = Some(ep * (1. + d * self.pt / 100.)); }
        }
        e
    }
}

/// Meander bands for bar i (ch.10 code): (MeanderLow, MeanderLowest, MeanderHigh, MeanderHighest).
fn meander(x: &Ctx, i: usize, lb: usize) -> Option<(f64, f64, f64, f64)> {
    if i < lb { return None; }
    let mut d = Vec::with_capacity(lb * 4);
    for k in 0..lb {
        let (b, pc) = (i - k, x.c[i - k - 1]);
        for v in [x.o[b], x.h[b], x.l[b], x.c[b]] { d.push((v - pc) / pc); }
    }
    let n = d.len() as f64;
    let avg = d.iter().sum::<f64>() / n;
    let sse: f64 = d.iter().map(|v| (v - avg).powi(2)).sum();
    let se: f64 = d.iter().map(|v| v - avg).sum();
    let sd = ((n * sse - se * se) / (n * (n - 1.))).sqrt();
    let c = x.c[i];
    Some((c * (1. + avg - sd), c * (1. + avg - 2. * sd), c * (1. + avg + sd), c * (1. + avg + 2. * sd)))
}

/// Volume-weighted average (ch.13 code), recursion from 0 at the first full window.
fn vwa(x: &Ctx, n: usize) -> Vec<f64> {
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(0.)).collect();
    let av = sma(&vol, n);
    let mut out = vec![f64::NAN; x.c.len()];
    let mut prev = 0.;
    for i in 0..x.c.len() {
        let (hi, lo) = (highest(&av, i, n), lowest(&av, i, n));
        if !(hi.is_finite() && lo.is_finite() && av[i].is_finite()) { continue; }
        if hi > lo {
            let rv = (av[i] - lo) / (hi - lo);
            let ap = (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.;
            prev = rv * ap + (1. - rv) * prev;
        }
        out[i] = prev;
    }
    out
}

fn hybrid_entry<'a>(x: &'a Ctx<'a>, rp: usize, aggr: f64, long: bool, short: bool) -> impl FnMut(usize) -> Vec<(f64, Ord, &'static str)> + 'a {
    move |i| {
        let (hh, ll) = (highest(&x.h, i, rp), lowest(&x.l, i, rp));
        if !(hh.is_finite() && ll.is_finite()) { return vec![]; }
        let dist = (hh - ll) / aggr;
        let mut v = vec![];
        if long && x.c[i] < ll + dist { v.push((1., Ord::Stop(ll + dist), "stop")); }
        if short && x.c[i] > hh - dist { v.push((-1., Ord::Stop(hh - dist), "stop")); }
        v
    }
}
fn meander_entry<'a>(x: &'a Ctx<'a>, lb: usize) -> impl FnMut(usize) -> Vec<(f64, Ord, &'static str)> + 'a {
    move |i| {
        if i < 1 || !(x.c[i] < x.c[i - 1] && x.c[i] < x.o[i]) { return vec![]; }
        meander(x, i, lb).map_or(vec![], |m| vec![(1., Ord::Limit(m.0), "limit")])
    }
}
fn harris_entry<'a>(x: &'a Ctx<'a>, long: bool, short: bool) -> impl FnMut(usize) -> Vec<(f64, Ord, &'static str)> + 'a {
    move |i| {
        let mut v = vec![];
        if i < 2 { return v; }
        if long && x.l[i] < x.l[i - 2] && x.l[i - 1] < x.l[i - 2] { v.push((1., Ord::Stop(x.h[i - 2]), "stop")); }
        if short && x.h[i] > x.h[i - 2] && x.h[i - 1] > x.h[i - 2] { v.push((-1., Ord::Stop(x.l[i - 2]), "stop")); }
        v
    }
}
fn expert_entry<'a>(x: &'a Ctx<'a>, lag: usize, long: bool, short: bool) -> impl FnMut(usize) -> Vec<(f64, Ord, &'static str)> + 'a {
    move |i| {
        let mut v = vec![];
        if i < lag || i + 1 >= x.c.len() { return v; }
        let nx = x.o[i + 1]; // "Open Next Bar" (TradeStation peeks at the next open for this order)
        if long && x.h[i] < x.h[i - lag] && x.c[i] < x.o[i] && nx < x.h[i] { v.push((1., Ord::Stop(x.h[i]), "stop")); }
        if short && x.l[i] > x.l[i - lag] && x.c[i] > x.o[i] && nx > x.l[i] { v.push((-1., Ord::Stop(x.l[i]), "stop")); }
        v
    }
}
fn vwa_entry<'a>(x: &'a Ctx<'a>, up: &'a [f64], dn: Option<&'a [f64]>, trig: usize) -> impl FnMut(usize) -> Vec<(f64, Ord, &'static str)> + 'a {
    move |i| {
        let mut v = vec![];
        if i < 1 { return v; }
        if x.c[i] < up[i] && up[i] < up[i - 1] { v.push((1., Ord::Stop(highest(&x.h, i, trig)), "stop")); }
        if let Some(dn) = dn { if x.c[i] > dn[i] && dn[i] > dn[i - 1] { v.push((-1., Ord::Stop(lowest(&x.l, i, trig)), "stop")); } }
        v
    }
}

/// Monday-based week number of a YYYY-MM-DD date (days since 1970-01-01 shifted to Monday).
fn week_of(date: &str) -> i64 {
    let f = |a: usize, b: usize| date.get(a..b).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let (y, m, d) = (f(0, 4), f(5, 7), f(8, 10));
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    (days + 3).div_euclid(7)
}

fn counterpunch(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let n = x.c.len();
    let wk: Vec<i64> = x.r.iter().map(|r| week_of(&r.date)).collect();
    // (close of previous week, close of the week before) for bar i, from completed weeks only
    let mut wc: Vec<(f64, f64)> = vec![(f64::NAN, f64::NAN); n];
    let (mut last_w, mut prev1, mut prev2) = (i64::MIN, f64::NAN, f64::NAN);
    for i in 0..n {
        if i > 0 && wk[i] != last_w { prev2 = prev1; prev1 = x.c[i - 1]; }
        last_w = wk[i];
        wc[i] = (prev1, prev2);
    }
    let (sl1, pt1, tr2, pt2) = (g("sl1") / 100., g("pt1") / 100., g("trail2") / 100., g("pt2") / 100.);
    let days = g("days").round().max(1.) as usize;
    let (al, ashort) = (g("allow_long") > 0., g("allow_short") > 0.);
    let mut out = vec![];
    // position: trade, bars since entry, open halves [first, second], trade extreme
    let mut pos: Option<(Trade, usize, [bool; 2], f64)> = None;
    let mut pend: Option<f64> = None;
    let mut time_exit = false;
    for i in from..=to {
        if let Some((t, bse, open, ext)) = pos.as_mut() {
            *bse += 1;
            let (d, ep) = (t.dir, t.legs[0].px);
            if time_exit {
                for h in 0..2 { if open[h] { t.exits.push((i, x.o[i], 0.5)); open[h] = false; } }
                t.reason = "time (next open)".into();
            } else {
                let worse = |a: f64, b: f64| if d > 0. { a <= b } else { a >= b };
                let (o, adv, fav) = if d > 0. { (x.o[i], x.l[i], x.h[i]) } else { (x.o[i], x.h[i], x.l[i]) };
                let stops = [ep * (1. - d * sl1), *ext * (1. - d * tr2)];
                let tgts = [ep * (1. + d * pt1), ep * (1. + d * pt2)];
                for h in 0..2 {
                    if !open[h] { continue; }
                    let (s, tg) = (stops[h], tgts[h]);
                    let fill = if worse(o, s) { Some((o, "stop")) } else if worse(tg, o) { Some((o, "target")) }
                        else if worse(adv, s) { Some((s, "stop")) } else if worse(tg, fav) { Some((tg, "target")) } else { None };
                    if let Some((px, why)) = fill { t.exits.push((i, px, 0.5)); open[h] = false; t.reason = format!("{} half {}", if h == 0 { "first" } else { "second" }, why); }
                }
            }
            if !open[0] && !open[1] {
                let (mut t, _, _, _) = pos.take().unwrap();
                t.exit_i = i; t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>(); t.signal_i = if time_exit { i - 1 } else { i };
                out.push(t);
            }
        } else if let Some(d) = pend.take() {
            pos = Some((open_trade(d, i, x.o[i], "open", i - 1), 0, [true, true], if d > 0. { x.h[i] } else { x.l[i] }));
        }
        pend = None;
        time_exit = false;
        if let Some(e) = pos.as_mut() {
            let d = e.0.dir;
            e.3 = if d > 0. { e.3.max(x.h[i]) } else { e.3.min(x.l[i]) };
            time_exit = i < to && e.1 == days + 1;
        } else if i < to && i >= 2 {
            let (w1, w2) = wc[i];
            if al && x.c[i] < x.c[i - 1] && x.c[i] < w1 && x.c[i - 1] < x.c[i - 2] && w1 < w2 { pend = Some(1.); }
            else if ashort && x.c[i] > x.c[i - 1] && x.c[i] > w1 && x.c[i - 1] > x.c[i - 2] && w1 > w2 { pend = Some(-1.); }
        }
    }
    if let Some((mut t, _, open, _)) = pos {
        for h in 0..2 { if open[h] { t.exits.push((to, x.c[to], 0.5)); } }
        t.exit_i = to; t.exit_px = t.exits.iter().map(|e| e.1 * e.2).sum::<f64>(); t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let on = |k: &str| g(k) > 0.5;
    let no_close = |_: usize, _: &mut Pos| None;
    Some(match id {
        "strmm_hybrid" => {
            let (rp, slow) = (u("retrace"), g("slow_stop").round().max(0.) as usize);
            sim(x, from, to, hybrid_entry(x, rp, g("aggr"), true, on("allow_short")),
                |i, p| (slow > 0 && p.bse == slow && p.t.dir * (x.c[i] - p.t.legs[0].px) < 0.).then_some("slow-trade stop (close)"),
                |i, p| Ex { stop: Some(if p.t.dir > 0. { lowest(&x.l, i, rp) } else { highest(&x.h, i, rp) }), ..Ex::default() })
        }
        "strmm_hybrid_sl" | "strmm_hybrid_ts" => {
            let s = Sct::new(x, &g, true);
            sim(x, from, to, hybrid_entry(x, u("retrace"), g("aggr"), true, false), |i, p| s.on_close(i, p), |i, p| s.exits(i, p))
        }
        "strmm_rs1" => {
            let Some(d2) = x.series("STR_DATA2").map(ffill) else { return Some(vec![]) }; // data gap: no trades
            let (lb, e) = (u("lookback"), 1. + g("modifier") * 0.01);
            let stop = g("stop_pct") / 100.;
            let n = x.c.len();
            let mut rel = vec![1.; n];
            for i in 0..n { rel[i] = if d2[i].is_finite() && d2[i] != 0. { x.c[i] / d2[i] } else if i > 0 { rel[i - 1] } else { f64::NAN }; }
            let mut obv = vec![0.; n];
            for i in 1..n {
                let v = x.r[i].volume.unwrap_or(0.);
                obv[i] = obv[i - 1] + if x.c[i] > x.c[i - 1] { v } else if x.c[i] < x.c[i - 1] { -v } else { 0. };
            }
            let (ar, ao) = (sma(&rel, lb), sma(&obv, lb));
            let mut lps = vec![f64::NAN; n];
            let (mut pr, mut pv) = (1., 1.);
            for i in 0..n {
                if !(ar[i].is_finite() && ao[i].is_finite()) { continue; }
                if ar[i] != 0. { pr = rel[i] / ar[i]; }
                if ao[i] != 0. { pv = obv[i] / ao[i]; }
                lps[i] = pr * pv;
            }
            let sps: Vec<f64> = lps.iter().map(|v| if *v != 0. { 1. / v } else { f64::NAN }).collect();
            let cross = |s: &[f64], i: usize| i > 0 && s[i] > e && s[i - 1] <= e;
            sim(x, from, to, |i| if cross(&lps, i) { vec![(1., Ord::Mkt, "open")] } else { vec![] },
                no_close,
                |i, p| Ex { mkt: cross(&sps, i).then_some("strength exit (next open)"), stop: (stop > 0.).then(|| p.t.legs[0].px * (1. - stop)), ..Ex::default() })
        }
        "strmm_meander" => {
            let lb = u("lookback").min(25);
            sim(x, from, to, meander_entry(x, lb), no_close,
                |i, _| meander(x, i, lb).map_or(Ex::default(), |m| Ex { stop: Some(m.1), limit: Some(m.3), mkt: None }))
        }
        "strmm_meander_sl" | "strmm_meander_ts" => {
            let s = Sct::new(x, &g, true);
            sim(x, from, to, meander_entry(x, u("lookback").min(25)), |i, p| s.on_close(i, p), |i, p| s.exits(i, p))
        }
        "strmm_rsbands" => {
            let Some(m) = markets(x, u("markets").clamp(4, 5)) else { return Some(vec![]) }; // data gap
            let tot = |lb: usize| -> Vec<f64> {
                let mut t = vec![1.; x.c.len()];
                for k in 1..m.len() {
                    let rs: Vec<f64> = (0..x.c.len()).map(|i| m[0][i] / m[k][i]).collect();
                    let a = sma(&rs, lb);
                    for i in 0..t.len() { t[i] *= rs[i] / a[i]; }
                }
                t
            };
            let (lt, st) = (tot(u("long_lb")), tot(u("short_lb")));
            let (lsd, ssd) = (stdev_pop(&lt, u("long_lb")), stdev_pop(&st, u("short_lb")));
            let ltrig: Vec<f64> = lsd.iter().map(|s| 1. + s * g("long_sd")).collect();
            let strig: Vec<f64> = ssd.iter().map(|s| 1. - s * g("short_sd")).collect();
            let (al, ash) = (on("allow_long"), on("allow_short"));
            sim(x, from, to, |i| {
                    let mut v = vec![];
                    if al && lt[i] > ltrig[i] { v.push((1., Ord::Mkt, "open")); }
                    if ash && st[i] < strig[i] { v.push((-1., Ord::Mkt, "open")); }
                    v.truncate(1); // both at once: the Buy is first in the code
                    v
                }, no_close,
                |i, p| Ex { mkt: if p.t.dir > 0. { (lt[i] < ltrig[i]).then_some("strength exit (next open)") } else { (st[i] > strig[i]).then_some("strength exit (next open)") }, ..Ex::default() })
        }
        "strmm_rotation" => {
            let Some(m) = markets(x, u("markets").clamp(4, 5)) else { return Some(vec![]) }; // data gap
            let k = m.len();
            let slope = u("slope");
            let tot = |lb: usize| -> Vec<Vec<f64>> {
                (0..k).map(|a| {
                    let mut t = vec![1.; x.c.len()];
                    for b in (0..k).filter(|&b| b != a) {
                        let rs: Vec<f64> = (0..x.c.len()).map(|i| m[a][i] / m[b][i]).collect();
                        let av = sma(&rs, lb);
                        for i in 0..t.len() { t[i] *= if i >= slope { av[i] / av[i - slope] } else { f64::NAN }; }
                    }
                    t
                }).collect()
            };
            let (lt, st) = (tot(u("long_lb")), tot(u("short_lb")));
            let (ltr, str_) = (avg_tr(x, u("ltr")), avg_tr(x, u("str")));
            let (nl, ns) = (u("n_long").min(k), u("n_short").min(k));
            let (al, ash) = (on("allow_long"), on("allow_short"));
            let nth = |v: &Vec<Vec<f64>>, i: usize, n: usize, max: bool| -> f64 {
                let mut s: Vec<f64> = v.iter().map(|t| t[i]).collect();
                if s.iter().any(|x| !x.is_finite()) { return f64::NAN; }
                s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                if max { s[k - n] } else { s[n - 1] }
            };
            let (ls, ss, lt_, tt) = (g("long_stop"), g("short_stop"), g("long_tgt"), g("short_tgt"));
            sim(x, from, to, |i| {
                    let mut v = vec![];
                    let (lmin, smax) = (nth(&lt, i, nl, true), nth(&st, i, ns, false));
                    if !(lmin.is_finite() && smax.is_finite()) { return v; } // TradeStation MaxBarsBack: all lookbacks filled
                    if al && lt[0][i] >= lmin && ltr[i] > str_[i] { v.push((1., Ord::Mkt, "open")); }
                    if ash && st[0][i] <= smax && ltr[i] < str_[i] { v.push((-1., Ord::Mkt, "open")); }
                    v
                }, no_close,
                |i, p| {
                    let ep = p.t.legs[0].px;
                    if p.t.dir > 0. { Ex { stop: Some(ep - str_[i] * ls), limit: Some(ep + str_[i] * lt_), mkt: None } }
                    else { Ex { stop: Some(ep + str_[i] * ss), limit: Some(ep - str_[i] * tt), mkt: None } }
                })
        }
        "strmm_vwa" => {
            let (up, dn) = (vwa(x, u("up_len")), vwa(x, u("down_len")));
            let (al, ash) = (on("allow_long"), on("allow_short"));
            let mut ent = vwa_entry(x, &up, Some(&dn), u("trigger"));
            sim(x, from, to, |i| ent(i).into_iter().filter(|o| if o.0 > 0. { al } else { ash }).collect(),
                |i, p| if p.t.dir > 0. { (x.c[i] < up[i]).then_some("close below VWA") } else { (x.c[i] > dn[i]).then_some("close above VWA") },
                |_, _| Ex::default())
        }
        "strmm_vwa_sl" | "strmm_vwa_ts" => {
            let up = vwa(x, u("up_len"));
            let s = Sct::new(x, &g, true);
            let keep = on("keep_vwa_exit");
            let mut ent = vwa_entry(x, &up, None, u("trigger"));
            sim(x, from, to, |i| ent(i), |i, p| {
                    if p.bse == 0 { p.fix = s.a[i]; }
                    if keep && x.c[i] < up[i] { return Some("close below VWA"); }
                    s.on_close(i, p)
                }, |i, p| s.exits(i, p))
        }
        "strmm_harris" => {
            let (pt, sl) = (g("pt") / 100., g("sl") / 100.);
            sim(x, from, to, harris_entry(x, on("allow_long"), on("allow_short")), no_close,
                |_, p| { let (d, ep) = (p.t.dir, p.t.legs[0].px); Ex { stop: Some(ep * (1. - d * sl)), limit: Some(ep * (1. + d * pt)), mkt: None } })
        }
        "strmm_harris_orig" => {
            let (pt, sl) = (g("pt") / 100., g("sl") / 100.);
            sim(x, from, to, |i| {
                    if i >= 3 && x.h[i] > x.h[i - 3] && x.l[i - 1] < x.l[i - 2] && x.l[i - 2] < x.l[i - 3] { vec![(1., Ord::Mkt, "open")] } else { vec![] }
                }, no_close,
                |_, p| { let ep = p.t.legs[0].px; Ex { stop: Some(ep * (1. - sl)), limit: Some(ep * (1. + pt)), mkt: None } })
        }
        "strmm_harris_sl" | "strmm_harris_ts" => {
            let s = Sct::new(x, &g, false);
            sim(x, from, to, harris_entry(x, true, false), |i, p| s.on_close(i, p), |i, p| s.exits(i, p))
        }
        "strmm_expert" => {
            let (bars, pt, sl, trailing) = (u("bars"), g("pt") / 100., g("sl") / 100., on("trailing"));
            sim(x, from, to, expert_entry(x, u("hh_lag").min(2), on("allow_long"), on("allow_short")), no_close,
                |i, p| {
                    let (d, ep) = (p.t.dir, p.t.legs[0].px);
                    // code: Top = High; Top = MaxList(Top, High) -> today's high (literal)
                    let stop = if trailing { if d > 0. { x.h[i] * (1. - sl) } else { x.l[i] * (1. + sl) } } else { ep * (1. - d * sl) };
                    Ex { stop: Some(stop), limit: Some(ep * (1. + d * pt)), mkt: (p.bse == bars + 1).then_some("time (next open)") }
                })
        }
        "strmm_expert_sl" | "strmm_expert_ts" => {
            let s = Sct::new(x, &g, false);
            sim(x, from, to, expert_entry(x, u("hh_lag").min(2), true, false), |i, p| s.on_close(i, p), |i, p| s.exits(i, p))
        }
        "strmm_counterpunch" => counterpunch(x, from, to, &g),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("2001-01-{:02}", 1 + i % 28), open: 100., high: 101., low: 99., close: 100., volume: Some(1000.) }).collect()
    }
    #[test]
    fn hybrid_buys_on_the_stop_and_exits_at_the_six_day_low() {
        let mut r = rows(40);
        // bars 10..15: range 99..101 -> level = 99 + 2/3 = 99.667; close 99.5 below it on bar 15
        r[15].close = 99.5;
        r[16].open = 99.6; r[16].high = 100.5; // stop 99.667 hit -> fill at 99.667
        r[20].open = 99.2; r[20].low = 98.; // 6-day low (99) taken out with a gap below -> open 99.2? no: open 99.2 > 99 -> fill 99
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("strmm_hybrid", &x, 10, 39, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 16);
        assert!((ts[0].legs[0].px - (99. + 2. / 3.)).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 17); // bar 17 low 99 <= stop 99 (lowest low of 6 bars incl. 16)
        assert_eq!(ts[0].exit_px, 99.);
    }
    #[test]
    fn harris_original_stop_and_target() {
        let mut r = rows(30);
        r[8].low = 98.5; r[9].low = 98.; r[10].high = 101.5; // H > H[3], L[1] < L[2] < L[3]
        r[11].open = 100.; r[13].high = 113.; // target 112
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("strmm_harris_orig", &x, 5, 29, &Params::new()).unwrap();
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px), (11, 100.));
        assert_eq!(ts[0].exit_i, 13); assert!((ts[0].exit_px - 112.).abs() < 1e-9);
    }
    #[test]
    fn week_numbers_are_monday_based() {
        assert_eq!(week_of("2024-01-01"), week_of("2024-01-07")); // Mon..Sun
        assert_eq!(week_of("2024-01-08"), week_of("2024-01-07") + 1);
    }
}
