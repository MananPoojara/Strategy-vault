/**
 * Where the team's own reference files for each Advance Decline Ratio (ADR) sub-variant are, so a researcher can open them.
 * Paths are inside the team's folder "Advance Decline Ratio" (share Jay/Old Topics). GENERATED from ADR/verify/tradelists_result.json
 * and ADR/verify/results/opt_summary.json (the checks in ADR/ADR_Coding_and_Verification_2026-10-31.md): do not edit by hand.
 * A sheet listed here is one the software reproduces trade by trade; its `params` are the settings found for it (engine keys).
 * A sub-variant without a trade list points to its largest optimisation workbook (`summary`) and says how many rows match.
 */
import type {BtstReference} from './btst-reference';

export const ADR_REFERENCES: Record<string, BtstReference> = {
  ytadr_longshort: {
    afl: "AFLs/Advance Decline LongShort.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
    ],
    summary: {file: "Exit by N days/ADR_Below Threshold_Long only_Optim.xlsx", sheet: "optimisation rows", detail: "No trade list for this sub-variant. Its optimisation workbook: 194 of 200 rows reproduced by the software (trade count and net profit)"},
  },
  ytadr_versions: {
    afl: "AFLs/Advance Decline LongShort_Versions.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "A", detail: "122 NIFTY trades, 2000-01-01 to 2024-06-21: all 122 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 1.0, "st_fact": 2.5, "thr": 0.2, "consec": 1.0, "n_entry": 1.0, "exit_n": 16.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "B", detail: "135 NIFTY trades, 2000-01-01 to 2024-06-21: all 135 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "thr": 0.3, "consec": 1.0, "n_entry": 1.0, "exit_n": 20.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "C", detail: "42 NIFTY trades, 2000-01-01 to 2024-06-21: all 42 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "thr": 0.1, "consec": 1.0, "n_entry": 1.0, "exit_n": 17.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "D", detail: "55 NIFTY trades, 2000-01-01 to 2024-06-21: all 55 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 2.0, "st_fact": 1.5, "consec": 1.0, "n_entry": 4.0, "thr": 0.7, "exit_n": 20.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "E", detail: "119 NIFTY trades, 2000-01-01 to 2024-06-21: all 119 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 1.0, "st_fact": 2.5, "consec": 1.0, "n_entry": 2.0, "thr": 0.6, "exit_n": 20.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "F", detail: "98 NIFTY trades, 2000-01-01 to 2024-06-21: all 98 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "consec": 1.0, "n_entry": 2.0, "thr": 0.6, "exit_n": 20.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "G", detail: "67 NIFTY trades, 2000-01-01 to 2024-06-21: all 67 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 7.0, "st_tf": 2.0, "st_fact": 1.5, "consec": 0.0, "n_entry": 5.0, "thr": 0.6, "exit_n": 8.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "H", detail: "49 NIFTY trades, 2000-01-01 to 2024-06-21: all 49 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 7.0, "st_tf": 1.0, "st_fact": 4.0, "consec": 0.0, "n_entry": 5.0, "thr": 0.6, "exit_n": 8.0}},
      {file: "All Conclusion and Summary Sheets/Best Setting Tradelist.xlsx", sheet: "I", detail: "57 NIFTY trades, 2000-01-01 to 2024-06-21: all 57 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 10.0, "st_tf": 0.0, "st_fact": 3.0, "consec": 0.0, "n_entry": 3.0, "thr": 0.3, "exit_n": 7.0}},
      {file: "Exit by N days/Summary N Candle Exit.xlsx", sheet: "A", detail: "122 NIFTY trades, 2000-01-01 to 2024-06-21: all 122 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 1.0, "st_fact": 2.5, "thr": 0.2, "consec": 1.0, "n_entry": 1.0, "exit_n": 16.0}},
      {file: "Exit by N days/Summary N Candle Exit.xlsx", sheet: "B", detail: "135 NIFTY trades, 2000-01-01 to 2024-06-21: all 135 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "thr": 0.3, "consec": 1.0, "n_entry": 1.0, "exit_n": 20.0}},
      {file: "Exit by N days/Summary N Candle Exit.xlsx", sheet: "C", detail: "42 NIFTY trades, 2000-01-01 to 2024-06-21: all 42 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "thr": 0.1, "consec": 1.0, "n_entry": 1.0, "exit_n": 17.0}},
      {file: "Exit by N days/Summary N Candle Exit.xlsx", sheet: "D", detail: "55 NIFTY trades, 2000-01-01 to 2024-06-21: all 55 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 2.0, "st_fact": 1.5, "consec": 1.0, "n_entry": 4.0, "thr": 0.7, "exit_n": 20.0}},
      {file: "Exit by N days/Summary N Candle Exit.xlsx", sheet: "E", detail: "119 NIFTY trades, 2000-01-01 to 2024-06-21: all 119 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 1.0, "st_fact": 2.5, "consec": 1.0, "n_entry": 2.0, "thr": 0.6, "exit_n": 20.0}},
      {file: "Exit by ADR/Summary _ADR Exit.xlsx", sheet: "AA", detail: "35 NIFTY trades, 2000-01-01 to 2024-06-21: all 35 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 2.0, "st_fact": 2.0, "consec": 1.0, "n_entry": 3.0, "thr": 0.4, "exit_type": 1.0, "exit_ad": 1.5}},
      {file: "Exit by ADR/Summary _ADR Exit.xlsx", sheet: "AB", detail: "21 NIFTY trades, 2000-01-01 to 2024-06-21: all 21 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 4.0, "st_tf": 2.0, "st_fact": 2.0, "consec": 0.0, "n_entry": 3.0, "thr": 0.2, "exit_type": 1.0, "exit_ad": 1.0}},
      {file: "Exit by ADR/Summary _ADR Exit.xlsx", sheet: "AC", detail: "21 NIFTY trades, 2000-01-01 to 2024-06-21: all 21 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 4.0, "st_tf": 2.0, "st_fact": 2.0, "consec": 0.0, "n_entry": 3.0, "thr": 0.2, "exit_type": 1.0, "exit_ad": 2.0}},
      {file: "Exit by ADR/Summary _ADR Exit.xlsx", sheet: "AD", detail: "37 NIFTY trades, 2000-01-01 to 2024-06-21: all 37 reproduced exactly by the software (entry, exit, prices); settings from the tagged row of the team workbook", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 8.0, "st_tf": 2.0, "st_fact": 2.0, "consec": 0.0, "n_entry": 6.0, "thr": 0.6, "exit_type": 1.0, "exit_ad": 5.0}},
      {file: "All Conclusion and Summary Sheets/ADR_Final Conclusion_1st Stage.xlsx", sheet: "C", detail: "42 NIFTY trades, 2000-01-01 to 2024-06-21: all 42 reproduced exactly by the software (entry, exit, prices); settings same as Best Setting C", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 1.0, "st_tf": 0.0, "st_fact": 3.0, "thr": 0.1, "consec": 1.0, "n_entry": 1.0, "exit_n": 17.0}},
      {file: "All Conclusion and Summary Sheets/ADR_Final Conclusion_1st Stage.xlsx", sheet: "G", detail: "67 NIFTY trades, 2000-01-01 to 2024-06-21: all 67 reproduced exactly by the software (entry, exit, prices); settings same as Best Setting G", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 7.0, "st_tf": 2.0, "st_fact": 1.5, "consec": 0.0, "n_entry": 5.0, "thr": 0.6, "exit_n": 8.0}},
      {file: "All Conclusion and Summary Sheets/ADR_Final Conclusion_1st Stage.xlsx", sheet: "H", detail: "49 NIFTY trades, 2000-01-01 to 2024-06-21: all 49 reproduced exactly by the software (entry, exit, prices); settings same as Best Setting H", ticker: "NIFTY 50", params: {"st_on": 1.0, "lookback": 7.0, "st_tf": 1.0, "st_fact": 4.0, "consec": 0.0, "n_entry": 5.0, "thr": 0.6, "exit_n": 8.0}},
    ],
  },
  ytadr_versions_5dp_exit: {
    afl: "AFLs/Advance Decline LongShort_with5DayExit.afl",
    aflNote: "The AFL the sub-variant is coded from. No team setting file exists for it: the grid card runs on a BORROWED setting (Versions Best setting A with the AFL's 5DP Entry exit). Every parameter on this page can be changed.",
    tradesheets: [
    ],
  },
  ytadr_without_rsi: {
    afl: "AFLs/Advance Decline LongShort_Versions_Without RSI.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
      {file: "Nifty_Advance_Decline_Ratio-Stats.xlsx", sheet: "Upclose v1", detail: "575 NIFTY trades, 2000-01-01 to 2024-06-21: all 575 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"st_on": 0.0, "exit_type": 3.0, "consec": 1.0, "lookback": 1.0, "thr": 0.7, "n_entry": 1.0, "exit_n": 4.0}},
      {file: "Nifty_Advance_Decline_Ratio-Stats.xlsx", sheet: "Upclose v3", detail: "483 NIFTY trades, 2000-01-01 to 2024-06-21: all 483 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"st_on": 0.0, "exit_type": 3.0, "consec": 0.0, "thr": 0.9, "n_entry": 3.0, "exit_n": 4.0, "lookback": 6.0}},
      {file: "Nifty_Advance_Decline_Ratio-Stats.xlsx", sheet: "green", detail: "463 NIFTY trades, 2000-01-01 to 2024-06-21: all 463 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"st_on": 0.0, "exit_type": 2.0, "consec": 0.0, "thr": 0.9, "n_entry": 3.0, "exit_n": 4.0, "lookback": 6.0}},
    ],
  },
  ytadr_without_rsi_nday: {
    afl: "Exit by N days/HSL- PB Optim/V1+V2/ (older copy, 3 copies differ only in defaults)",
    aflNote: "Older copy of \"Without RSI\" with the N-candle / HSL-PB exits (3 copies differ only in defaults).",
    tradesheets: [
    ],
    summary: {file: "Exit by N days/HSL- PB Optim/V3/SL+PB/ADR_Exit By N Candle HSL + PB Optim Weekly  OF V3.xlsx", sheet: "optimisation rows", detail: "No trade list for this sub-variant. Its optimisation workbook: 300,583 of 555,984 rows reproduced by the software (trade count and net profit)"},
  },
  ytadr_two_st: {
    afl: "AFLs/Advance Decline LongShort_Versions_Two_SuperTrend.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
    ],
    summary: {file: "Two STR Filter/Exit By ADR Ratio/Entry Using Average ADR/V3/Entry Uisng AVG ADR_ Monthly Bear & Daily - Bull.xlsx", sheet: "optimisation rows", detail: "No trade list for this sub-variant. Its optimisation workbook: 291,600 of 291,600 rows reproduced by the software (trade count and net profit)"},
  },
  ytadr_long_2_11_5dp: {
    afl: "AFLs/ADR Long 2-11 and 5DP.afl",
    aflNote: "The AFL the sub-variant is coded from. No team setting file exists for it: the grid card runs on a BORROWED setting (the 2-11 / 5DP ROC team setting). Every parameter on this page can be changed.",
    tradesheets: [
    ],
  },
  ytadr_long_2_11_5dp_roc: {
    afl: "AFLs/ADR Long 2-11 and 5DP ROC.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
      {file: "checking for ADR in 5dp.xlsx", sheet: "0.1", detail: "41 NIFTY trades, 2011-01-01 to 2024-06-21: all 41 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"thr": 0.1, "adr_on": 1.0, "patch": 1.0, "entry_day": 8.0, "exit_day": 3.0, "exit_type": 2.0}},
      {file: "checking for ADR in 5dp.xlsx", sheet: "0.11", detail: "42 NIFTY trades, 2011-01-01 to 2024-06-21: all 42 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"thr": 0.11, "adr_on": 1.0, "patch": 1.0, "entry_day": 8.0, "exit_day": 3.0, "exit_type": 2.0}},
      {file: "checking for ADR in 5dp.xlsx", sheet: "0.12", detail: "53 NIFTY trades, 2011-01-01 to 2024-06-21: all 53 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"thr": 0.12, "adr_on": 1.0, "patch": 1.0, "entry_day": 8.0, "exit_day": 3.0, "exit_type": 2.0}},
    ],
  },
  ytadr_5dp: {
    afl: "5DP + ADR/FIVE Day Phenomena with ADR.afl",
    aflNote: "The AFL the sub-variant is coded from.",
    tradesheets: [
      {file: "5DP + ADR/Test.xlsx", sheet: "Without ADR", detail: "293 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 293 reproduced exactly by the software (entry, exit, prices); settings grid (AFL defaults, no ADR)", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "With ADR", detail: "88 NIFTY trades, 2000-01-01 to 2024-06-21: all 88 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 0.5, "n_entry": 1.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Without ADR + ADR Values", detail: "89 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 89 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 0.5, "n_entry": 1.0, "entry_mode": 1.0, "lookback": 8.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Green Candle - WO ADR", detail: "67 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 67 reproduced exactly by the software (entry, exit, prices); settings grid (AFL defaults)", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "candle": 0.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Red Candle - WO ADR", detail: "87 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 87 reproduced exactly by the software (entry, exit, prices); settings grid (AFL defaults)", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "candle": 1.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Green Candle - With ADR", detail: "1 NIFTY trades, 2000-01-01 to 2024-06-21: all 1 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 0.3, "candle": 0.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "RSI - Without ADR", detail: "165 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 165 reproduced exactly by the software (entry, exit, prices); settings grid (AFL RSI 14 / 50)", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "rsi_on": 1.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "RSI - With ADR", detail: "30 NIFTY trades, 2000-01-01 to 2024-06-21: all 30 reproduced exactly by the software (entry, exit, prices); settings grid", ticker: "NIFTY 50", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 0.5, "rsi_on": 1.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "RSI Green - Without ADR", detail: "50 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 50 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "rsi_on": 1.0, "candle": 0.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "RSI Red - Without ADR", detail: "33 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 33 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "rsi_on": 1.0, "candle": 1.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Random ADX", detail: "258 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 258 reproduced exactly by the software (entry, exit, prices); settings grid (AFL ADX 8 / 20)", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "adx_on": 1.0, "candle": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Green ADX", detail: "62 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 62 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "adx_on": 1.0, "candle": 0.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "Red ADX", detail: "76 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 76 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "adx_on": 1.0, "candle": 1.0, "candle_days": 2.0}},
      {file: "5DP + ADR/Test.xlsx", sheet: "ROC Above", detail: "89 NSENIFTY trades, 2000-01-01 to 2024-06-21: all 89 reproduced exactly by the software (entry, exit, prices); settings grid", params: {"entry_day": 5.0, "exit_day": 2.0, "pt": 0.0, "hsl": 0.0, "thr": 50.0, "roc_on": 1.0, "candle": 2.0, "roc_day": 5.0, "roc_thr": 1.0}},
    ],
  },
};

export const adrReference = (id: string | undefined): BtstReference | null =>
  id && Object.prototype.hasOwnProperty.call(ADR_REFERENCES, id) ? ADR_REFERENCES[id] : null;
