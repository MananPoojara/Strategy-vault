/**
 * Where the team's own reference files for each BTST sub-variant are, so a researcher can open them.
 * Paths are inside the team's BTST folder. A sub-variant without a trade list says so plainly and points to the
 * team's results workbook (optimisation summaries) where one exists.
 */
/**
 * `ticker`: the index this sheet belongs to; the box shows one sheet, the one for the selected index (else the first).
 * `params`: the settings the team ran that sheet with (engine keys; a page opens with them). `select`: settings that
 * choose between sheets of one sub-variant (e.g. rank_dir 2 = the Low-to-High workbook).
 */
export interface ReferenceFile {file: string; sheet: string; detail: string; ticker?: string; params?: Record<string, number>; select?: Record<string, number>; /** brokerage the sheet was run with, in bps a side (ATH: the AFL's own default when absent) */ cost?: number}
/**
 * A sub-variant without a trade sheet is compared with ONE row of the team's results workbook: the highest Net % Profit
 * among the rows the Vault can run (BTST/Team_Results_Best_Settings.md). `row` / `cells` say where that row is; `scope`
 * and `ticker` the instrument it ran on (`universe` = the F&O stocks); `params` its settings (engine keys), `cost` its
 * brokerage in bps a side, `from` / `to` the team's database period (no `from` = the first data day); `team` its numbers
 * (Net %, CAR %, max system drawdown %, trades, % winners, average % per trade).
 */
export interface BestSetting {
  file: string; sheet: string; table?: string; row: string; cells: string;
  scope: 'index' | 'universe'; ticker?: string;
  params: Record<string, number>; cost: number; from?: string; to: string;
  settings: string;
  team: {net: number; car: number; mdd: number; trades: number; win: number; avg: number};
}
export interface BtstReference {
  afl: string;
  aflNote: string;
  tradesheets: ReferenceFile[];
  summary?: ReferenceFile;
  best?: BestSetting;
  /** An optimization workbook exists for this variant, even when no reference trade list is available. */
  hasOptimization?: boolean;
}

const D = 'BTST_Intra day/';
const CORE = `${D}BTST after 3 consecutive upclose_with profit booking/`;
const OLD = `${D}BTST_Intraday after 3 consecutive upclose_Without profit booking & SL/`;
const SLC = `${D}BTST Intraday (Buy at 2nd last candle_Sell at next day opening)/`;
const SLC_RESULTS = `${D}BTST Intraday_FO_All quotes_Comm 0.04_Wo flt,BL,BR_Optim.xlsx`;
const CANDLES = `${D}BTST on the basis of no. of candles_Intraday FO/`;
const STOCKS = 'F&O stock universe (FO STOCKS C&P)';

export const NO_TRADESHEET = 'No Reference Tradesheet for this Variant';
export const BEST_SETTING = "Comparing with the team's highest-return setting";

// the team's database ends 2021-10-28 (stock and index workbooks); the Second-last candle workbook used another copy
// (NIFTY from 2008-07-10, BANKNIFTY from 2010-01-12, both to 2021-09-02)
const TEAM_END = '2021-10-28';
// the team's stock runs: Penny Stock Price 10 (settings screenshots in the workbooks; the ATH-only workbook has none: its sibling ATH file)
const STOCKS10 = {pb_pct: 0, hardstop: 0, penny: 10, max_positions: 10, pct: 10};
const STOCK_RUN = 'F&O stocks, 10 open positions at 10% of equity each, penny stock price 10, profit booking 0, stop 0, trend filter OFF';

const REFERENCES: Record<string, BtstReference> = {
  ytbtst_core: {
    afl: `${CORE}btst trad in intraday with flt and profit booking.afl`,
    aflNote: 'Same rules in "btst trad in intraday with flt and profit booking & Hard SL.afl" (explicit stop price). The lists below come from the team IEOD_FO database, which the Vault uses by default.',
    tradesheets: [
      {file: `${CORE}NIFTY & BANKNIFTY.xlsx`, sheet: 'Bktst_NIFTY_EN 1450_Ex 925_2day', detail: '456 NIFTY trades, 2007-2021 (preset "Team Nifty list")', ticker: 'NIFTY 50', params: {}},
      {file: `${CORE}NIFTY & BANKNIFTY.xlsx`, sheet: 'Bktst_BN_EN 1510_Ex 925_2day', detail: '473 BANKNIFTY trades, 2007-2021 (preset "Team BankNifty list")', ticker: 'NIFTYBANK', params: {entry_time: 1510}},
    ],
    summary: {file: `${CORE}NIFTY & BANKNIFTY.xlsx`, sheet: 'NIFTY_Without filter, NIFTY_With ST 20,3 weekly', detail: 'trade counts for lookback 2/3/4 and the weekly Supertrend presets'},
  },
  ytbtst_old_fixed: {
    afl: `${OLD}btst trad in intraday ver3_bar-count.afl`,
    aflNote: 'The Vault runs ver3_bar-count (entry / exit candles counted, the entry candle\'s own close, no look-ahead), sent to the research team for an AmiBroker check. The reference sheet was made with the older ver2_updated (named in its Info sheet), so a few trades differ.',
    tradesheets: [
      {file: `${OLD}btst trad in intraday ver2_updated-Intraday-0.03-BT-NIFTY-BN.xlsb`, sheet: 'NIFTY-TL', detail: '81 NIFTY trades, 2007-2021 (binary .xlsb, open in Excel or LibreOffice)', ticker: 'NIFTY 50', params: {}},
      {file: `${OLD}btst trad in intraday ver2_updated-Intraday-0.03-BT-NIFTY-BN.xlsb`, sheet: 'BN-TL', detail: '87 BANKNIFTY trades, 2007-2021 (binary .xlsb)', ticker: 'NIFTYBANK', params: {}},
    ],
  },
  ytbtst_ibs_roc: {
    afl: 'btst trad in intraday with flt and profit booking with IBS flt and ROC ranking .afl',
    aflNote: 'The final AFL (2022-09-08), in the root of the BTST folder. The earlier version that tests IBS the other way: BTST_with IBS + Threshold and ROC Ranking with Threshold.',
    tradesheets: [],
    summary: {file: 'Optim_BTST trad in intraday without flt_Index_NiftyFut-BNF.xlsx', sheet: 'Nifty Futures BTST, BNF BTST', detail: 'optimisation results only (no trade list)'},
    best: {file: 'Optim_BTST trad in intraday without flt_Index_NiftyFut-BNF.xlsx', sheet: 'BNF BTST', row: 'Excel row 5 (No. 10)', cells: 'Net % Profit C5, ROC DAYS AL5, ROC Threshold level AM5',
      scope: 'index', ticker: 'NIFTYBANK', params: {days: 2, entry_time: 1440, exit_time: 915, pb_pct: 0, hardstop: 0, flt_type: 3, ibs_dir: 1, ibs_thr: 1, roc_days: 3, roc_thr: 1.5}, cost: 2, to: TEAM_END,
      settings: 'BANKNIFTY_F1, 100% of equity, lookback 2, entry 14:40, exit 09:15, profit booking 0, stop 0, trend filter OFF, IBS threshold 1, ROC days 3, ROC threshold 1.5%, brokerage 0.02 (settings screenshot: sheet "info sheet 2")',
      team: {net: 74.47, car: 3.93, mdd: -10.26, trades: 286, win: 60.84, avg: 0.2}},
  },
  ytbtst_ath: {
    afl: `${D}BTST on ATH stocks with increamental close/btst trad in intraday with flt and profit booking of  incremental close and ath.afl`,
    aflNote: 'The team ran this add-on on F&O stocks only.', tradesheets: [],
    summary: {file: `${D}BTST on ATH stocks with increamental close/FO Stocks_Without flt_BTST on ATH stocks with increamental up close_0.03_OPTIM.xlsx`, sheet: 'all sheets', detail: `optimisation results, ${STOCKS}`},
    best: {file: `${D}BTST on ATH stocks with increamental close/FO Stocks_Without flt_BTST on ATH stocks with increamental up close_0.03_OPTIM.xlsx`, sheet: '10 open_2 days_Wo filter', row: 'Excel row 14 (No. 13)', cells: 'Net % Profit C14, ENTRY TIME AL14, EXIT TIME AM14',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 925, flt_type: 3, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, lookback 2, entry 14:50, exit 09:25, brokerage 0.03`,
      team: {net: 409.52, car: 11.6, mdd: -14.73, trades: 6241, win: 56.47, avg: 0.27}},
  },
  ytbtst_ath_only: {
    afl: `${D}BTST on ATH stocks without increamental close/btst trad in intraday with flt and profit booking on ath stock.afl`,
    aflNote: 'The team ran this add-on on F&O stocks only.', tradesheets: [],
    summary: {file: `${D}BTST on ATH stocks without increamental close/FO Stocks_With & without flt_BTST on ATH stocks_INTRADAY_PB_0.03_OPTIM.xlsx`, sheet: 'all sheets', detail: `optimisation results, ${STOCKS}`},
    best: {file: `${D}BTST on ATH stocks without increamental close/FO Stocks_With & without flt_BTST on ATH stocks_INTRADAY_PB_0.03_OPTIM.xlsx`, sheet: '10 open_Wo filter_PB=0', row: 'Excel row 44 (No. 43)', cells: 'Net % Profit C44, ENTRY TIME AL44, EXIT TIME AM44',
      scope: 'universe', params: {entry_time: 1450, exit_time: 955, flt_type: 3, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, entry 14:50, exit 09:55, brokerage 0.03`,
      team: {net: 1398.73, car: 20.02, mdd: -16.46, trades: 13863, win: 53.25, avg: 0.2}},
  },
  ytbtst_high_n: {
    afl: `${D}BTST with increamental up close & HIGHEST close of 'n' days/btst trad in intraday with flt and profit booking with highest clsoing of n days.afl`,
    aflNote: 'The team ran this add-on on F&O stocks only.', tradesheets: [],
    summary: {file: `${D}BTST with increamental up close & HIGHEST close of 'n' days/FO C&P_BTST with increamental close & highest close of n days_Optim.xlsx`, sheet: 'all sheets', detail: `optimisation results, ${STOCKS}`},
    best: {file: `${D}BTST with increamental up close & HIGHEST close of 'n' days/FO C&P_BTST with increamental close & highest close of n days_Optim.xlsx`, sheet: '10 & 20 open_Wo filter', table: '10 open_Wo filter_Days=2_Entry 14:50_Exit 09:15 (left table, header row 2)', row: 'Excel row 3 (No. 1)', cells: 'Net % Profit C3, highest close days AL3',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 915, flt_type: 3, hdays: 3, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, lookback 2, entry 14:50, exit 09:15, highest close of 3 days, brokerage 0.03`,
      team: {net: 93465.74, car: 58.6, mdd: -14.48, trades: 28293, win: 58.08, avg: 0.25}},
  },
  ytbtst_green: {
    afl: `${D}BTST_with increamental up close + GREEN candle of prev 2 days/btst trad in intraday with flt and profit booking with green candle option.afl`,
    aflNote: 'The team ran this add-on on F&O stocks only.', tradesheets: [],
    summary: {file: `${D}BTST_with increamental up close + GREEN candle of prev 2 days/FOC&P_BTST with increramental up close + GREEN candle of 2 days.xlsx`, sheet: 'all sheets', detail: `results, ${STOCKS}`},
    best: {file: `${D}BTST_with increamental up close + GREEN candle of prev 2 days/FOC&P_BTST with increramental up close + GREEN candle of 2 days.xlsx`, sheet: '10 open_Wo flt_Lookback=2 days', row: 'Excel row 2 (No. 1)', cells: 'Net % Profit C2, Green Candle days AL2',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 915, flt_type: 3, candle: 1, green_days: 2, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, lookback 2, entry 14:50, exit 09:15, candle colour GREEN, green candle days 2, brokerage 0.03`,
      team: {net: 59017.89, car: 53.77, mdd: -15.17, trades: 26603, win: 57.7, avg: 0.25}},
  },
  ytbtst_volume: {
    afl: `${D}BTST_with increamental up close + Today's volume greater than 5 days avg/btst trad in intraday with flt and profit booking with volume condition_WITH VOL. MULTI..afl`,
    aflNote: 'Multiplier 1 reproduces the plain AFL in the same folder. The team ran it on F&O stocks only.', tradesheets: [],
    summary: {file: `${D}BTST_with increamental up close + Today's volume greater than 5 days avg/FO stocks C&P_BTST with increamental up close + Today's volume greater than x times of last 5 days avg_Wo flt.xlsx`, sheet: 'all sheets', detail: `results, ${STOCKS}`},
    best: {file: `${D}BTST_with increamental up close + Today's volume greater than 5 days avg/FO stocks C&P_BTST with increamental up close + Today's volume greater than x times of last 5 days avg_Wo flt.xlsx`, sheet: '10 OPEN_2 DAYS_Volume of 5 days', table: '10 open_Lookback days=2_Volume of 3 days (right-hand table from column AN, header row 2)', row: 'Excel row 3 (No. 1)', cells: 'Net % Profit AP3, VOLUME MULTIPLE BY3',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 915, flt_type: 3, vol_days: 3, vol_mult: 1, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, lookback 2, entry 14:50, exit 09:15, volume check days 3, volume multiple 1, brokerage 0.03`,
      team: {net: 105679.76, car: 59.92, mdd: -13.72, trades: 23486, win: 58.98, avg: 0.3}},
  },
  ytbtst_athc: {
    afl: `${D}BTST_with increamental up close_ATH-%swing as filter/btst trad in intraday with Profit booking AND ATHC FLT.afl`,
    aflNote: 'The AFL computes a sell signal from the swing but never uses it, so trades still end next morning.', tradesheets: [],
    summary: {file: `${D}BTST_with increamental up close_ATH-%swing as filter/Book1.xlsx`, sheet: 'all sheets', detail: `results, ${STOCKS}`},
    best: {file: `${D}BTST_with increamental up close_ATH-%swing as filter/Book1.xlsx`, sheet: '10 open_Lookback days=2', row: 'Excel row 7 (No. 6)', cells: 'Net % Profit C7, ATHC EXIT % AL7',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 915, athc_pct: 20, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: 'F&O stocks, 10 open positions at 10% of equity each, penny stock price 10, profit booking 0, stop 0, lookback 2, entry 14:50, exit 09:15, ATHC exit 20%, brokerage 0.03',
      team: {net: 1409.21, car: 20.08, mdd: -13.98, trades: 14838, win: 57.45, avg: 0.19}},
  },
  ytbtst_rank_price: {
    afl: `${D}BTST with increamental upclose_HIGH to LOW based ranking/btst trad in intraday with flt and profit booking_PRICE RANKING_HIGH TO LOW.afl`,
    aflNote: `Low-to-high version in the folder "BTST with increamental upclose_LOW to HIGH based ranking". ${STOCKS}.`,
    tradesheets: [
      {file: `${D}BTST with increamental upclose_HIGH to LOW based ranking/BTST_Increamental upclose_HIGH to LOW ranking.xlsx`, sheet: '10 open_Wo flt_2 days (and 3 more)', detail: '29,709 F&O stock trades in the first sheet, 2007-2021', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10, rank_dir: 1}, select: {rank_dir: 1}},
      {file: `${D}BTST with increamental upclose_LOW to HIGH based ranking/BTST_Increamental upclose_LOW to HIGH ranking.xlsx`, sheet: '20 open_Wo flt_2 days (and 3 more)', detail: '47,979 F&O stock trades in the first sheet', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 20, pct: 5, rank_dir: 2}, select: {rank_dir: 2}},
    ],
  },
  ytbtst_rank_ibs: {
    afl: `${D}BTST_With increamental up close_Ranking as per IBS value (High to Low)/btst trad in intraday with flt and profit booking with IBS RANKING (HIGH TO LOW).afl`,
    aflNote: `Low-to-high version in "BTST_With increamental up close_Ranking as per IBS value (Low to high)". ${STOCKS}.`,
    tradesheets: [
      {file: `${D}BTST_With increamental up close_Ranking as per IBS value (High to Low)/FOC&P_BTST with increamental upclose+Ranking as per IBS_H To L.xlsx`, sheet: '10 open_Lookback days=2 (and 3 more)', detail: '29,703 F&O stock trades in the first sheet', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10, rank_dir: 1}, select: {rank_dir: 1}},
      {file: `${D}BTST_With increamental up close_Ranking as per IBS value (Low to high)/FOC&P_BTST with increamental upclose+Ranking as per IBS.xlsx`, sheet: '10 open_Lookback days=2_Wo flt (and 3 more)', detail: '29,702 F&O stock trades in the first sheet', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10, rank_dir: 2}, select: {rank_dir: 2}},
    ],
  },
  ytbtst_rank_lookback: {
    afl: `${D}BTST with increamental up close with ranking VER 11 (Higher to lower lookback days)/btst trad in intraday with flt and profit booking with ranking ver 11.afl`,
    aflNote: `Ver 2 (lower to higher) in the sibling folder. ${STOCKS}.`,
    tradesheets: [
      {file: `${D}BTST with increamental up close with ranking VER 2 (Lower to higher lookback days)/FO stocks c&p_BTST with increamental diff & up close with RANKING (Low to HIgh).xlsx`, sheet: '20 open_Lookback days=2 (and 10 open)', detail: '48,806 and 30,009 F&O stock trades (ver 2)', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 20, pct: 5, lb_version: 2, pad: 1}, select: {lb_version: 2}},
      {file: `${D}BTST with increamental up close with ranking VER 11 (Higher to lower lookback days)/FO stocks c&p_BTST with increamental diff & up close with RANKING (High to low).xlsx`, sheet: 'Tradelist', detail: '30,009 F&O stock trades (ver 11)', select: {lb_version: 11}},
    ],
  },
  ytbtst_rank_roc: {
    afl: `${D}BTST with increamental up close_ROC ranking/btst trad in intraday with flt and profit booking with ranking ON ROC.afl`,
    aflNote: STOCKS + '.',
    tradesheets: [{file: `${D}BTST with increamental up close_ROC ranking/FO C&P_BTST with upclose_ROC based ranking.xlsx`, sheet: '20 open_Lookback=2_ROC=3 days (and lookback 3, ROC 4)', detail: '47,980 and 11,426 F&O stock trades', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 20, pct: 5, roc_days: 3}}],
  },
  ytbtst_rank_ibs_volume: {
    afl: `${D}BTST_With increamental upclose+IBS ranking (High to Low)+Volume multiplier/btst trad in intraday with flt and profit booking with IBS RANKING (HIGH TO LOW) AND VOL CONF..afl`,
    aflNote: STOCKS + '.', tradesheets: [],
    summary: {file: `${D}BTST_With increamental upclose+IBS ranking (High to Low)+Volume multiplier/FOC&P_BTST with increamental upclose+IBS ranking (High to Low)+Vol multiplier_Wo flt.xlsx`, sheet: 'all sheets', detail: 'results'},
    best: {file: `${D}BTST_With increamental upclose+IBS ranking (High to Low)+Volume multiplier/FOC&P_BTST with increamental upclose+IBS ranking (High to Low)+Vol multiplier_Wo flt.xlsx`, sheet: '10 open_Lookback days=2 & 3', table: 'Lookback days=2_Volume lookback days=5 (upper table, header row 2)', row: 'Excel row 3 (No. 1)', cells: 'Net % Profit C3, VOLUME MULTIPLE AL3',
      scope: 'universe', params: {days: 2, entry_time: 1450, exit_time: 915, flt_type: 3, vol_days: 5, vol_mult: 1, ...STOCKS10}, cost: 3, to: TEAM_END,
      settings: `${STOCK_RUN}, lookback 2, entry 14:50, exit 09:15, volume check days 5, volume multiple 1, brokerage 0.03`,
      team: {net: 58424.14, car: 53.66, mdd: -13.69, trades: 23071, win: 59.07, avg: 0.28}},
  },
  ytbtst_nifty_upclose: {
    afl: `${D}BTST with increamental upclose + 1 UPCLOSE of NIFTY/btst trad in intraday with flt and profit booking with nifty one day higher close.afl`,
    aflNote: STOCKS + '.',
    tradesheets: [{file: `${D}BTST with increamental upclose + 1 UPCLOSE of NIFTY/FO stocks_BTST with increamental UPCLOSE + 1 UPCLOSE in NIFTY.xlsx`, sheet: 'Tradelist', detail: '32,441 F&O stock trades'}],
  },
  ytbtst_nifty_incremental: {
    afl: `${D}BTST_With increamental upclose + Upclose & increamental difference in NIFTY/btst trad in intraday with flt and profit booking with nifty with incremental close ver2.afl`,
    aflNote: STOCKS + '. Its "Lookback days for nifty" parameter is declared but never used.',
    tradesheets: [{file: `${D}BTST_With increamental upclose + Upclose & increamental difference in NIFTY/FOC&P_BTST with upclose & increamental diff in stock & NIFTY both.xlsx`, sheet: '10 open_Lookback days=2_Wo filt (and 3 more)', detail: '4,533 / 585 / 8,793 / 790 F&O stock trades', params: {days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10, pad: 1}}],
  },
  ytbtst_slc: {
    afl: `${SLC}BTST Intraday (Buy at 2nd last candle_Sell at next day opening).afl`,
    aflNote: 'The title says "sell at next day opening"; the code sells at the close of the next day\'s first candle, and the Vault follows the code.', tradesheets: [],
    summary: {file: SLC_RESULTS, sheet: ' Nifty Backtest, BN Backtest', detail: 'trade counts only: 3,253 NIFTY and 2,888 BANKNIFTY trades (5-minute, all quotes)'},
    best: {file: SLC_RESULTS, sheet: ' BN Backtest', table: '%Equity_Brokrage 0.04 (column B; the team ran no optimisation of this AFL)', row: 'column B, rows 7-44', cells: 'Net Profit % B7, Annual Return % B10, All trades B15, Winners B21, Max. system % drawdown B44',
      scope: 'index', ticker: 'NIFTYBANK', params: {tf: 1}, cost: 4, from: '2010-01-12', to: '2021-09-02',
      settings: 'BANKNIFTY_F1, 5-minute candles, 100% of equity, brokerage 0.04, the team\'s database period 2010-01-12 to 2021-09-02',
      team: {net: -69.67, car: -9.71, mdd: -78.71, trades: 2888, win: 47.26, avg: -0.04}},
  },
  ytbtst_slc_bull: {
    afl: `${SLC}BTST Intraday with monthly filter for Bull Market.afl`,
    aflNote: 'Same as the plain AFL plus a monthly Supertrend that must be up.', tradesheets: [],
    summary: {file: SLC_RESULTS, sheet: 'Nifty Bull Optimization, BN Bull Optimization', detail: 'trade counts per Supertrend period and factor'},
    best: {file: SLC_RESULTS, sheet: 'Nifty Bull Optimization', table: '%Equity_Stretch 1 to 6_ST Period 5 to 20 (left table, header row 2)', row: 'Excel row 3 (No. 1)', cells: 'Net % Profit C3, ST FILTER PERIOD AL3, ST FILTER FACTOR AM3',
      scope: 'index', ticker: 'NIFTY 50', params: {tf: 1, flt_tf: 3, st_point: 1, st_period: 5, st_factor: 1}, cost: 4, from: '2008-07-10', to: '2021-09-02',
      settings: 'NIFTY_F1, 5-minute candles, monthly Supertrend 5, 1 on the mid point, 100% of equity, brokerage 0.04, the team\'s database period 2008-07-10 to 2021-09-02',
      team: {net: -28.03, car: -2.45, mdd: -43.7, trades: 1870, win: 46.42, avg: -0.01}},
  },
  ytbtst_slc_bear: {
    afl: `${SLC}BTST Intraday with monthly filter for Bear Market.afl`,
    aflNote: 'Same as the plain AFL plus a monthly Supertrend that must be down.', tradesheets: [],
    summary: {file: SLC_RESULTS, sheet: 'Nifty BEAR Optimization, BN BEAR Optimization', detail: 'trade counts per Supertrend period and factor'},
    best: {file: SLC_RESULTS, sheet: 'BN BEAR Optimization', table: '%Equity_Stretch 1 to 6_ST Period 5 to 20 (left table, header row 2)', row: 'Excel row 22 (No. 20)', cells: 'Net % Profit C22, ST FILTER PERIOD AL22, ST FILTER FACTOR AM22',
      scope: 'index', ticker: 'NIFTYBANK', params: {tf: 1, flt_tf: 3, st_point: 1, st_period: 20, st_factor: 3}, cost: 4, from: '2010-01-12', to: '2021-09-02',
      settings: 'BANKNIFTY_F1, 5-minute candles, monthly Supertrend 20, 3 on the mid point, 100% of equity, brokerage 0.04, the team\'s database period 2010-01-12 to 2021-09-02',
      team: {net: 34.14, car: 2.55, mdd: -12.77, trades: 723, win: 48.82, avg: 0.05}},
  },
  ytbtst_candles: {
    afl: `${CANDLES}Candles based Entry Exit(BTST).afl`,
    aflNote: 'Team note: for the buy the 15:15 candle is counted as 0; for the sell the 09:15 candle is counted as 1.',
    tradesheets: [
      {file: `${CANDLES}NIFTY_Candles based entry-exit_5 minute_PT 0_Optim.xlsx`, sheet: 'En=3 Ex=3_PT=0_WOF (no filter, 3,670 trades); En=3_Ex=3_PT=0_with daily 5,2fl (daily Supertrend 5,2, 2,108 trades)', detail: 'NIFTY, 5-minute, entry candle 3, exit candle 3, no profit target, no filter', ticker: 'NIFTY 50', params: {entry_candle: 3, exit_candle: 3, pt_pct: 0, flt_type: 3}},
      {file: `${CANDLES}N & BN_Intraday FO_15 min_Candles based entry exit (BST) N&BN_Comm 0.04.xlsx`, sheet: 'BN_Weekly 20,3_En 2_Ex 1', detail: '2,336 BANKNIFTY trades, 15-minute, weekly Supertrend 20,3, entry candle 2, exit candle 1, profit target 7%', ticker: 'NIFTYBANK', params: {tf: 2, entry_candle: 2, exit_candle: 1, pt_pct: 7, flt_type: 1, flt_tf: 2, st_period: 20, st_factor: 3}},
    ],
  },
  ytbtst_index_filter: {
    afl: `${D}BTST with INDEX filter/btst trad in intraday with flt and profit booking with index filter.afl`,
    aflNote: STOCKS + '.', tradesheets: [],
    summary: {file: `${D}BTST with INDEX filter/FO C&P_BTST without stock flt & with INDEX flt_Optim.xlsx`, sheet: 'all sheets', detail: 'optimisation results'},
  },
};

/**
 * Reference settings run on the main database only (IEOD_FO on the share, as stored) — user decision 2026-10-26:
 * the reconstructed "team's 2021 copy" and the futures feed are not offered.
 */
/** The team's brokerage in every BTST reference sheet: 0.03% a side (= 3 bps, the Vault's fee setting). */
export const BTST_BROKERAGE_BPS = 3;

const matches = (sel: Record<string, number> | undefined, values: Record<string, number>) => !sel || Object.entries(sel).every(([k, v]) => (values[k] ?? v) === v);

/**
 * The ONE reference trade sheet to show: among the sheets whose `select` settings match the current values, the one of
 * the selected index, else the main (first) one; null when there is none.
 */
export function mainTradesheet(ref: BtstReference, ticker?: string, values: Record<string, number> = {}): ReferenceFile | null {
  const ok = ref.tradesheets.filter(t => matches(t.select, values));
  return ok.find(t => t.ticker === ticker) || ok.find(t => !t.ticker) || ok[0] || null;
}

/** The settings a BTST page opens with: the team's settings of its reference sheet (none = the engine defaults), brokerage 0.03%. */
export function referenceDefaults(id: string, ticker?: string): {params: Record<string, number>; cost: number; sheet: ReferenceFile | null} | null {
  const ref = btstReference(id); if (!ref) return null;
  const sheet = mainTradesheet(ref, ticker);
  // no trade sheet: the team's highest-return setting when it ran on an index (on any index button; the reference box names its own)
  const best = !sheet && ref.best?.scope === 'index' ? ref.best : null;
  return {params: {...(sheet?.params || best?.params || {})}, cost: best ? best.cost : BTST_BROKERAGE_BPS, sheet};
}

/**
 * The settings a universe run (F&O stocks) opens with: the sub-variant's stock trade sheet (sheets without an index
 * `ticker`), else none (the engine defaults); brokerage 0.03%.
 */
export function universeReferenceDefaults(id: string): {params: Record<string, number>; cost: number; sheet: ReferenceFile | null} | null {
  const ref = btstReference(id); if (!ref) return null;
  const sheet = ref.tradesheets.find(t => !t.ticker && t.params) || null;
  const best = !sheet && ref.best?.scope === 'universe' ? ref.best : null; // no stock trade sheet: the team's highest-return stock setting
  return {params: {...(sheet?.params || best?.params || {})}, cost: best ? best.cost : BTST_BROKERAGE_BPS, sheet};
}

/**
 * True when a run is exactly the team's highest-return setting: same instrument (F&O stocks, or the same index), every
 * setting of the best row, the same brokerage (bps a side) and the team's period (`from` '' = the first data day).
 * `params` are the run's full values (defaults included).
 */
export function bestActive(best: BestSetting, run: {universe: boolean; ticker?: string; params: Record<string, number>; cost: number; from: string; to: string}): boolean {
  if ((best.scope === 'universe') !== run.universe || (best.scope === 'index' && run.ticker !== best.ticker)) return false;
  return Object.entries(best.params).every(([k, v]) => run.params[k] === v) && run.cost === best.cost && run.from === (best.from || '') && run.to === best.to;
}

/** The stock trade sheet to show for a universe run (index sheets never apply), chosen by the current settings. */
export function universeTradesheet(ref: BtstReference, values: Record<string, number> = {}): ReferenceFile | null {
  const ok = ref.tradesheets.filter(t => !t.ticker && matches(t.select, values));
  return ok[0] || null;
}

/** The reference files of a BTST sub-variant, or null for any other strategy. */
export function btstReference(id: string | undefined): BtstReference | null {
  return id && Object.prototype.hasOwnProperty.call(REFERENCES, id) ? REFERENCES[id] : null;
}

export const BTST_VARIANT_IDS = Object.keys(REFERENCES);

/** Every file path the references point to, relative to the BTST folder (used to check they exist). */
export function referencedPaths(): string[] {
  const out = new Set<string>();
  for (const r of Object.values(REFERENCES)) {
    out.add(r.afl);
    for (const f of r.tradesheets) out.add(f.file);
    if (r.summary) out.add(r.summary.file);
    if (r.best) out.add(r.best.file);
  }
  return [...out];
}
