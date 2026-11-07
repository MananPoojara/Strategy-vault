// Advance Decline Ratio match badges, from ADR/ADR_Coding_and_Verification_2026-10-31.md (Strategy Vault folder):
// id -> [own|near, percent, reference]. percent = share of the team's AmiBroker trades (or optimisation rows: trade count
// and net profit) the software reproduces exactly; own = the AFL's own sheets, near = a sibling AFL with the same rules.
export const ADR_VERIFY = {
  ytadr_versions: ['own', 100.0, 'Trade lists: Best Setting Tradelist A-I, Summary N Candle Exit A-E, Summary _ADR Exit AA-AD, ADR_Final Conclusion C/G/H: 1,489 of 1,489 trades. Optimisation sheets (52): 5,303,797 of 5,964,169 rows (88.9 %)'],
  ytadr_without_rsi: ['own', 100.0, 'Trade lists: Nifty_Advance_Decline_Ratio-Stats Upclose v1 / v3 / green: 1,521 of 1,521 trades. Optimisation sheets (50, Green Candle + Up Close): 2,615,058 of 2,655,360 rows (98.5 %)'],
  ytadr_without_rsi_nday: ['own', 80.6, 'Optimisation sheets HSL / PB (10): 2,139,452 of 2,653,632 rows; same trades as Versions on its 21 tagged trade lists (1,489 of 1,489)'],
  ytadr_two_st: ['own', 98.9, 'Two STR Filter optimisation sheets (16): 542,961 of 549,000 rows'],
  ytadr_longshort: ['own', 97.0, 'Basic LongShort optimisation sheet: 194 of 200 rows'],
  ytadr_5dp: ['own', 100.0, '5DP + ADR/Test.xlsx, 14 sheets (with / without ADR, candle colour, RSI, ADX, ROC): 1,388 of 1,388 trades'],
  ytadr_long_2_11_5dp_roc: ['own', 100.0, 'checking for ADR in 5dp.xlsx 0.1 / 0.11 / 0.12 (5DP patch, from 2011): 136 of 136 trades'],
};
