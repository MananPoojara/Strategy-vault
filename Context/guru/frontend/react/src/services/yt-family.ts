/**
 * YT strategy families shown as a hierarchy: family card (e.g. BTST) -> main variant cards -> sub-variant cards.
 * The engine puts the main variant in the catalogue `group`: "YT Strategies · <family> · <main variant>".
 */
export interface CatalogueLike {id: string; family?: string; group?: string; status?: string}
export interface MainVariant {name: string; ids: string[]; testable: number}
export interface Preset {label: string; params: Record<string, number>; ticker?: string; note: string}

/** The main-variant name in a YT group label, or '' when the group has no third part. */
export function ytMainVariant(group: string | undefined): string {
  const parts = (group || '').split(' · ');
  return parts.length >= 3 ? parts.slice(2).join(' · ') : '';
}

/** Main variants of a family in catalogue order, with their sub-variant ids and how many are runnable. */
export function ytMainVariants(defs: CatalogueLike[], family: string): MainVariant[] {
  const out = new Map<string, MainVariant>();
  for (const def of defs) {
    if (def.family !== family) continue;
    const name = ytMainVariant(def.group);
    if (!name) continue;
    const entry = out.get(name) || {name, ids: [], testable: 0};
    entry.ids.push(def.id);
    if (def.status === 'live') entry.testable += 1;
    out.set(name, entry);
  }
  return [...out.values()];
}

export const MAIN_VARIANT_NOTES: Record<string, string> = {
  'Up-close': 'Buy near the close after rising daily closes with increasing changes; sell next morning at the exit time, or earlier at profit booking or the hard stop. The core rule plus its add-on conditions and filters.',
  'Second-last candle': 'Buy on the second-last candle of every day and sell on the next day\'s first candle; optional monthly Supertrend (bull or bear) filter.',
  'Candle-count': 'Enter a set number of candles before the day\'s close and exit after a set number of candles on later days, at a profit target, or when the Supertrend turns down.',
  // ADR family (Advance Decline Ratio of the Nifty 50)
  'Basic Long/Short': 'Go long when the Nifty 50 advance / decline ratio is at or below a lower threshold, short when it is at or above an upper threshold, and exit after a set number of candles.',
  'Versions (ST and RSI filters)': 'The ADR entry (ratio below a threshold, on consecutive or look-back candles, or on its average) with Supertrend and RSI filters and exits by candles, ADR ratio, profit booking or hard stop; the "with 5DP exit" copy adds the Five Day Phenomena exit.',
  'Without RSI': 'The ADR long entry without the RSI filter: exits by N candles, green candles, up-close candles or the ADR ratio, with Supertrend filters (one or two), profit booking and hard stop. The short side does not run in these AFLs.',
  '2-11 / 5DP patch': 'ADR long entries only inside a calendar patch of the month (2-11 or the Five Day Phenomena window), with ADR, RSI, ADX and Supertrend filters; the ROC copy adds a ROC filter.',
  'Five Day Phenomena': 'Buy near the month end when the ADR test holds (plus candle colour, RSI, ADX and ROC tests) and sell a few days into the next month, with ATHC, Supertrend and bull / bear filters.',
  // ATH family (all-time-high close)
  'ATHC: %Swing exit': 'Buy when the close makes a new all-time-high close; sell when the close falls below the %swing line (a band under the close that only moves up). The core ATH rule with its add-ons: scale-in, hard stop and profit target, ROC, VIX and ADX filters, turnover and penny-stock filters.',
  'ATHC+% buy: %Swing exit': 'Buy only when the close is a set percentage above (or below) the all-time-high close instead of at the all-time-high close itself; exit by the %swing line.',
  'ATHC: Supertrend exit': 'Buy a new all-time-high close and sell when the Supertrend (for example 10,4 or 5,2 / 5,4) turns down.',
  'ATHC: Other exits': 'Buy a new all-time-high close and sell on a moving average, channel, band, oscillator, lower-low, previous-day-low, ATR or time-based exit instead of the %swing line.',
  'After consecutive losses': 'Take the all-time-high entry only after one or two consecutive losing trades of the %swing or Supertrend version (the "probability" variants).',
  'Bull / Bear market': 'The all-time-high strategy gated by an index regime (Supertrend or %swing on the NSE 500 or Nifty): trade only in a bull or a bear market, or exit all trades when the market state ends.',
  'Delay entry': 'Buy some bars after the all-time-high close, only if the stock still qualifies, then exit with the %swing line.',
  'No-January trade': 'The all-time-high strategy with a seasonal rule: no new entries in January; new or running all-time-high buys only from February.',
  'Ranking & stock selection': 'Which stocks get the limited positions: ranking by turnover, average turnover, market cap, ROC, RSI, positive closes, or large / mid / small-cap index membership.',
  'Reshuffling': 'Periodic reshuffling: rank stocks by the close-to-high ratio (52-week high or all-time high) and hold the top ones, with ratio ranges, month-end exits and stop variants.',
  'Intraday & next-day (BTST)': 'All-time-high entries taken on live market prices or intraday, with exits the next day or at the intraday close.',
  'Long-term highs': 'Entry on a 52-week, 3 / 5-year or 250-day high instead of the all-time-high close.',
  'Short & combos': 'The short side of the all-time-high idea and long / short or long / other-asset combinations.',
  'Scanners & tools': 'Helpers around the ATH strategies (stock count, scanners, position optimisation); not trading rule sets.',
};

/** Team settings offered as one-click presets on a sub-variant page. Empty params = the sub-variant's own defaults. */
export const YT_PRESETS: Record<string, Preset[]> = {
  ytadr_versions: [
    {label: 'Best setting A', params: {st_on: 1, lookback: 1, st_tf: 1, st_fact: 2.5, thr: 0.2, consec: 1, n_entry: 1, exit_n: 16}, ticker: 'NIFTY 50', note: 'Team sheet "Best Setting Tradelist" A: weekly Supertrend 2.5, threshold 0.2, exit after 16 candles (122 trades).'},
    {label: 'Best setting B', params: {st_on: 1, lookback: 1, st_tf: 0, st_fact: 3, thr: 0.3, consec: 1, n_entry: 1, exit_n: 20}, ticker: 'NIFTY 50', note: 'Sheet B: daily Supertrend 3, threshold 0.3, exit after 20 candles (135 trades).'},
    {label: 'Best setting G', params: {st_on: 1, lookback: 7, st_tf: 2, st_fact: 1.5, consec: 0, n_entry: 5, thr: 0.6, exit_n: 8}, ticker: 'NIFTY 50', note: 'Sheet G: monthly Supertrend 1.5, 5 of the last 7 candles, threshold 0.6, exit after 8 candles (67 trades).'},
  ],
  ytadr_without_rsi: [
    {label: 'Up-close v1', params: {st_on: 0, exit_type: 3, consec: 1, lookback: 1, thr: 0.7, n_entry: 1, exit_n: 4}, ticker: 'NIFTY 50', note: 'Team sheet "Nifty_Advance_Decline_Ratio-Stats" Upclose v1 (575 trades).'},
    {label: 'Green candle', params: {st_on: 0, exit_type: 2, consec: 0, thr: 0.9, n_entry: 3, exit_n: 4, lookback: 6}, ticker: 'NIFTY 50', note: 'The same workbook, sheet "green" (463 trades).'},
  ],
  ytadr_5dp: [
    {label: 'With ADR', params: {entry_day: 5, exit_day: 2, pt: 0, hsl: 0, thr: 0.5, n_entry: 1, candle: 2}, ticker: 'NIFTY 50', note: 'Team sheet "5DP + ADR / Test" with ADR (88 trades).'},
  ],
  ytbtst_core: [
    {label: 'Team Nifty list', params: {}, ticker: 'NIFTY 50', note: 'Lookback 2, 14:50 to 09:25, no profit booking, stop or filter: the 456-trade NIFTY list (2007-2021).'},
    {label: 'Team BankNifty list', params: {entry_time: 1510}, ticker: 'NIFTYBANK', note: 'Lookback 2, 15:10 to 09:25: the 473-trade BANKNIFTY list (2007-2021).'},
    {label: 'Lookback 3', params: {days: 3}, note: 'Three rising closes, otherwise the team Nifty settings.'},
    {label: 'Lookback 4', params: {days: 4}, note: 'Four rising closes.'},
    {label: 'Weekly Supertrend 20,3', params: {flt_type: 1, flt_tf: 2, st_period: 20, st_factor: 3}, note: 'The team sheet "NIFTY_With ST 20,3 weekly", lookback 2.'},
    {label: 'Lookback 3 + weekly Supertrend', params: {days: 3, flt_type: 1, flt_tf: 2, st_period: 20, st_factor: 3}, note: 'The same sheet, lookback 3 block.'},
    {label: 'AFL defaults', params: {days: 3, entry_time: 1515, exit_time: 920, pb_pct: 1, flt_type: 1, flt_tf: 1, st_period: 20, st_factor: 1}, note: 'The values written in the AFL itself.'},
  ],
};

/** True when the page's parameter overrides are exactly the preset's (order-independent). */
export function presetActive(current: Record<string, number>, preset: Preset): boolean {
  const a = Object.entries(current).filter(([, v]) => Number.isFinite(v));
  const b = Object.entries(preset.params);
  return a.length === b.length && b.every(([k, v]) => current[k] === v);
}
