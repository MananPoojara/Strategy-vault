// BTST match badges, from the checks of 2026-10-29 (BTST/Reference_Match_Summary_2026-10-26.md, BTST/Sibling_Verification_2026-10-29.md):
// id -> [own|near, percent, reference, {ticker: [percent, reference]}?]
// percent = share of the reference trades the software takes on the same stock and day with the same prices
// (stocks: or the same % return after a later price adjustment); own = its own trade sheet, near = the nearest version's.
export const BTST_VERIFY = {
  ytbtst_core: ['own', 100.0, "Nifty_EN 1450 Ex 0925.xlsx (the team's AmiBroker run on the current database): 454 of 454 trades",
    {NIFTYBANK: [98.7, 'NIFTY & BANKNIFTY.xlsx [Bktst_BN_EN 1510_Ex 925_2day]: 467 of 473 (sheet made on the 2021 database)']}],
  ytbtst_old_fixed: ['own', 85.2, 'btst trad in intraday ver2_updated-Intraday-0.03-BT-NIFTY-BN.xlsb [NIFTY-TL]: 69 of 81 (sheet made with ver2 on the 2021 database)',
    {NIFTYBANK: [82.8, 'btst trad in intraday ver2_updated-Intraday-0.03-BT-NIFTY-BN.xlsb [BN-TL]: 72 of 87 (sheet made with ver2 on the 2021 database)']}],
  ytbtst_candles: ['own', 70.0, 'NIFTY_Candles based entry-exit_5 minute_PT 0_Optim.xlsx [En=3 Ex=3_PT=0_WOF]: 2,568 of 3,670 (every date matches; the 2021 database lacked some minutes)',
    {NIFTYBANK: [39.5, 'N & BN_Intraday FO_15 min_Candles based entry exit (BST) N&BN_Comm 0.04.xlsx [BN_Weekly 20,3_En 2_Ex 1]: 922 of 2,336 (every date matches; the 2021 database lacked some minutes)']}],
  ytbtst_rank_price: ['own', 96.6, 'BTST_Increamental upclose_HIGH to LOW ranking.xlsx [10 open_Wo flt_2 days]: 28,700 of 29,709 (Low to High, 20 open: 97.5%)'],
  ytbtst_rank_ibs: ['own', 97.6, 'FOC&P_BTST with increamental upclose+Ranking as per IBS_H To L.xlsx [10 open_Lookback days=2]: 28,981 of 29,703 (Low to High: 98.3%)'],
  ytbtst_rank_lookback: ['own', 81.8, 'FO stocks c&p_BTST with increamental diff & up close with RANKING (Low to HIgh).xlsx [20 open_Lookback days=2]: 39,914 of 48,806 (AmiBroker\'s order between equal scores is unknown)'],
  ytbtst_rank_roc: ['own', 98.7, 'FO C&P_BTST with upclose_ROC based ranking.xlsx [20 open_Lookback=2_ROC=3 days]: 47,362 of 47,980'],
  ytbtst_nifty_upclose: ['own', 97.8, 'FO stocks_BTST with increamental UPCLOSE + 1 UPCLOSE in NIFTY.xlsx [Tradelist]: 31,726 of 32,441 (20 open, pad and align)'],
  ytbtst_nifty_incremental: ['own', 96.8, 'FOC&P_BTST with upclose & increamental diff in stock & NIFTY both.xlsx [10 open_Lookback days=2_Wo filt]: 4,390 of 4,533'],
  ytbtst_ath: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: all 99 trades identical"],
  ytbtst_ath_only: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: all 99 list days that meet the ATH rule identical"],
  ytbtst_high_n: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: every trade identical (N = 3, 9, 15)"],
  ytbtst_green: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: every trade identical (2, 3, 5 days; ANY = 454 of 454)"],
  ytbtst_volume: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: every trade identical (3 settings)"],
  ytbtst_athc: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: every trade identical (ATHC 10% and 20%)"],
  ytbtst_ibs_roc: ['near', 100.0, "Up-close core · the team's AmiBroker list Nifty_EN 1450 Ex 0925.xlsx: every trade identical on NIFTY (filters off: 454 of 454)"],
  ytbtst_rank_ibs_volume: ['near', 100.0, 'IBS ranking High to Low · sheet 10 open_Lookback days=2: 13,788 of 13,788 sheet trades that pass the volume rule'],
};

/** [own|near, percent, reference] of a BTST sub-variant on an instrument (its own sheet for that index when there is one). */
export function btstVerify(id, tk) {
  const a = BTST_VERIFY[id];
  if (!a) return null;
  const t = a[3]?.[tk];
  return t ? [a[0], t[0], t[1]] : [a[0], a[1], a[2]];
}
