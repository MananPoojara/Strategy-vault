import type { Catalogue, Operand } from "./types";
import { isSeries } from "./spec-model";

/** Lines of price-chart indicators that are not prices: ±1 trend states, 0/1 signals, counts and sizes in points (ATR, widths,
 *  slopes). Measured on NIFTY 50 (2026-10-30): every line of every price-chart indicator whose values are only −1/0/1 or sit
 *  under 10 % of the close. Comparing one of these with the close never holds, so the rule reads it against a Threshold. */
export const NOT_PRICE_LINES: Record<string, readonly string[]> = {
  supertrend: ["direction"], ma_cross: ["cross"], chandelier: ["direction"], vstop: ["direction"], wilder_vi: ["direction", "vi"],
  moon_phases: ["new_moon", "full_moon"], alphatrend: ["signal"], clear_method: ["direction"], atr_trailing_stop: ["direction"],
  gann_hilo: ["direction"], ha_supertrend: ["direction"], halftrend: ["direction"], nrtr: ["trend"], pp_supertrend: ["trend"],
  ott: ["direction"], range_filter: ["direction"], ssl: ["direction"], utbot: ["buy", "sell"], trend_magic: ["direction"],
  mavg_bands: ["width"], leavitt_conv: ["slope", "accel"], darvas: ["breakout"], gaps: ["gap_up", "gap_down", "filled_up", "filled_down"],
  fvg: ["bull", "bear"], fib_retracement: ["direction"], block_ma: ["dir1", "dir2"], super_trend_stop: ["trend"], sarext: ["direction"],
  kiwis_trailing_stop: ["direction"], pmax: ["direction"], hwc: ["width", "pct"], volatility_trend: ["direction"],
  liquidity_sweep: ["bull", "bear"], madrid_ribbon: Array.from({ length: 18 }, (_, i) => `state${5 * (i + 1)}`),
  smc_bos_choch: ["bull_bos", "bull_choch", "bear_bos", "bear_choch", "trend"], smc_equal_hl: ["bull_eql", "bear_eqh"],
  smc_swings: ["bull", "bear"], order_block: ["bull", "bear", "bull_mitigated", "bear_mitigated"], ifvg: ["bull", "bear"],
  breaker_block: ["bull", "bear"], smc_strong_weak: ["bias"], zigzag: ["direction", "bars_since", "new_high", "new_low", "moved"],
  rob_booker_adx_breakout: ["adx", "consolidation", "box_width", "buy", "sell"],
};
/** A trend-state name on an indicator added after the measurement is not a price either. */
const STATE_NAMES = new Set(["direction", "trend", "bias"]);

/** True when the operand is an indicator line that is not on the price scale: any line of an indicator drawn in its own pane, or a
 *  listed / trend-state line of a price-chart indicator. Prices and price-level lines return false. */
export function notPriceLine(o: Operand, cat: Catalogue): boolean {
  if (!isSeries(o)) return false;
  const ind = cat.indicators.find((i) => i.id === o.s);
  if (!ind) return false;
  if (ind.placement !== "price") return true;
  const out = o.out ?? ind.outputs[0];
  return (NOT_PRICE_LINES[ind.id]?.includes(out) ?? false) || STATE_NAMES.has(out);
}
