/** Visual checks for the existing Connors engines. Arithmetic and warmup match
 * backend/src/connors.rs; this module does not generate orders or alter a run.
 * Always calculate from the engine's full source history, then sample the visible
 * dates. Restarting Wilder smoothing at the selected From date is incorrect. */
export interface PriceHistory {
  readonly d: readonly string[];
  readonly o: readonly number[];
  readonly h: readonly number[];
  readonly l: readonly number[];
  readonly c: readonly number[];
}
export interface IndicatorPoint { time: string; value: number | null }
export interface IndicatorThreshold {
  value: number; label: string; kind: "entry" | "exit" | "reference";
}
export interface StrategyIndicator {
  key: string;
  label: string;
  placement: "price" | "pane";
  kind: "sma" | "rsi" | "di" | "pctb" | "sum";
  points: IndicatorPoint[];
  thresholds: IndicatorThreshold[];
}
export interface StrategyIndicators {
  status: "ready" | "unavailable" | "unsupported";
  series: StrategyIndicator[];
  note: string;
}
export interface IndicatorRequest {
  id: string;
  prices: PriceHistory | null;
  /** Must be true only for the complete source supplied to the calculation engine. */
  fullHistory: boolean;
  /** Schema defaults merged with the selected run's custom parameters. */
  parameters: Readonly<Record<string, number>>;
  /** Daily dates or actual last-trading dates of displayed W/M candles. */
  displayDates?: readonly string[];
}

function assertPeriod(period: number) {
  if (!Number.isInteger(period) || period < 1 || period > 10000)
    throw new Error("Indicator period must be an integer from 1 to 10000.");
}

export function simpleMovingAverage(values: readonly number[], period: number): (number | null)[] {
  assertPeriod(period);
  const output: (number | null)[] = Array(values.length).fill(null);
  let sum = 0;
  for (let i = 0; i < values.length; i++) {
    sum += values[i];
    if (i >= period) sum -= values[i - period];
    if (i + 1 >= period) output[i] = sum / period;
  }
  return output;
}

function wilder(values: readonly (number | null)[], period: number): (number | null)[] {
  assertPeriod(period);
  const output: (number | null)[] = Array(values.length).fill(null);
  const start = values.findIndex((v) => v !== null);
  if (start < 0 || values.length < start + period) return output;
  const seed = values.slice(start, start + period);
  if (seed.some((v) => v === null)) return output;
  let previous = seed.reduce<number>((sum, v) => sum + v!, 0) / period;
  output[start + period - 1] = previous;
  for (let i = start + period; i < values.length; i++) {
    if (values[i] === null) break;
    previous = (previous * (period - 1) + values[i]!) / period;
    output[i] = previous;
  }
  return output;
}

export function wilderRsi(closes: readonly number[], period: number): (number | null)[] {
  const up: (number | null)[] = Array(closes.length).fill(null);
  const down: (number | null)[] = Array(closes.length).fill(null);
  for (let i = 1; i < closes.length; i++) {
    const change = closes[i] - closes[i - 1];
    up[i] = Math.max(change, 0);
    down[i] = Math.max(-change, 0);
  }
  const averageUp = wilder(up, period), averageDown = wilder(down, period);
  return averageUp.map((u, i) => {
    const d = averageDown[i];
    if (u === null || d === null) return null;
    if (d === 0) return u === 0 ? 50 : 100;
    return 100 - 100 / (1 + u / d);
  });
}

export function directionalPlus(prices: PriceHistory, period: number): (number | null)[] {
  const movement: (number | null)[] = Array(prices.d.length).fill(null);
  const ranges: (number | null)[] = Array(prices.d.length).fill(null);
  for (let i = 1; i < prices.d.length; i++) {
    const up = prices.h[i] - prices.h[i - 1], down = prices.l[i - 1] - prices.l[i];
    movement[i] = up > down && up > 0 ? up : 0;
    ranges[i] = Math.max(prices.h[i] - prices.l[i],
      Math.abs(prices.h[i] - prices.c[i - 1]), Math.abs(prices.l[i] - prices.c[i - 1]));
  }
  const smoothedMove = wilder(movement, period), smoothedRange = wilder(ranges, period);
  return smoothedRange.map((range, i) => range !== null && range > 0 && smoothedMove[i] !== null
    ? 100 * smoothedMove[i]! / range : null);
}

/** connors.rs pct_b: population variance and symmetric k-sigma bands. */
export function bollingerPercentB(closes: readonly number[], period: number, deviations: number): (number | null)[] {
  assertPeriod(period);
  if (!Number.isFinite(deviations) || deviations <= 0) throw new Error("Bollinger band width must be positive.");
  const output: (number | null)[] = Array(closes.length).fill(null);
  for (let i = period - 1; i < closes.length; i++) {
    const window = closes.slice(i + 1 - period, i + 1);
    const mean = window.reduce((sum, value) => sum + value, 0) / period;
    const sd = Math.sqrt(window.reduce((sum, value) => sum + (value - mean) ** 2, 0) / period);
    if (sd > 0) output[i] = (closes[i] - (mean - deviations * sd)) / (2 * deviations * sd);
  }
  return output;
}

function rollingSum(values: readonly (number | null)[], period: number): (number | null)[] {
  assertPeriod(period);
  return values.map((_, i) => {
    if (i + 1 < period) return null;
    let sum = 0;
    // The entry engine sums backward from today's value; preserve its arithmetic.
    for (let k = 0; k < period; k++) {
      const value = values[i - k]; if (value === null) return null;
      sum += value;
    }
    return sum;
  });
}

const connorsIds = new Set([
  "f1a", "f3a", "f4a", "f4c", "f4d", "rsi2_5d", "r3", "tps_stocks", "x555", "h10",
  "tps_long", "tps_short", "s12345", "eom_co", "eom_rsi", "eom_cc", "eom_und",
  "rsi2_4", "rsi4_25", "rsi_div8", "rsi_div2", "rsi_1", "rsi_2", "rsi_3", "price",
  "pctb_1", "pctb_2", "dmi_1", "dmi_2", "dmi_3", "dmi_4", "dmi_5",
  "short_4up", "short_strong", "ss_long", "ss_short", "ss_both",
  "vix200", "vix2", "vix4", "vix7", "vix8", "vix53_short", "vix101_short",
  "trin4", "trin4_12", "trin3_10", "trin3_11", "trin2_11", "trin_1", "trin_2",
  "trin_3", "trin_4", "trin_5", "short_vix53", "short_vix101", "tick", "nyse_adv",
]);
const hpetfIds = /^hpetf_(3dhl|rsi25|r3|pctb|mdd|rsi10|tps)_(long|short)(?:_aggr)?$/;

export function buildStrategyIndicators(request: IndicatorRequest): StrategyIndicators {
  const unavailable = (note: string): StrategyIndicators => ({ status: "unavailable", series: [], note });
  const baseId = request.id.replace(/^spmt_/, ""), etf = hpetfIds.test(request.id);
  if (!connorsIds.has(baseId) && !etf)
    return { status: "unsupported", series: [], note: "Strategy indicator overlays for this method are awaiting implementation. Trade markers remain available." };
  if (!request.fullHistory)
    return unavailable("Exact indicator values need the full source history used by the engine, including warmup before the selected From date. Overlays are withheld for this sliced price series.");
  const px = request.prices;
  if (!px || !px.d.length || [px.o, px.h, px.l, px.c].some((v) => v.length !== px.d.length || v.some((x) => !Number.isFinite(x)))
      || px.d.some((d, i) => !/^\d{4}-\d{2}-\d{2}$/.test(d) || (i > 0 && d <= px.d[i - 1]))
      || px.d.some((_, i) => px.h[i] < Math.max(px.o[i], px.c[i], px.l[i]) || px.l[i] > Math.min(px.o[i], px.c[i], px.h[i])))
    return unavailable("A complete, ordered OHLC series is required. Missing or inconsistent prices are not filled.");
  const dates = request.displayDates ?? px.d;
  const byDate = new Map(px.d.map((date, i) => [date, i]));
  if (dates.some((date, i) => !byDate.has(date) || (i > 0 && date <= dates[i - 1])))
    return unavailable("Displayed candle dates do not match the full source history. Indicator values are not interpolated.");
  const p = request.parameters;
  const parameter = (key: string) => {
    if (!Object.hasOwn(p, key) || !Number.isFinite(p[key])) throw new Error(`Resolved run parameter '${key}' is unavailable.`);
    return p[key];
  };
  const period = (key: string) => { const value = Math.max(1, Math.round(parameter(key))); assertPeriod(value); return value; };
  const series: StrategyIndicator[] = [];
  const add = (key: string, label: string, kind: StrategyIndicator["kind"], values: (number | null)[], thresholds: IndicatorThreshold[] = []) => {
    series.push({ key, label, kind, placement: kind === "sma" ? "price" : "pane", thresholds,
      points: dates.map((time) => ({ time, value: values[byDate.get(time)!] })) });
  };
  const threshold = (key: string, label: string, kind: IndicatorThreshold["kind"]): IndicatorThreshold[] =>
    Object.hasOwn(p, key) ? [{ value: parameter(key), label, kind }] : [];
  try {
    const required = baseId.startsWith("dmi_") ? ["trend", "rsi_n", "exit_rsi", "di_n", "di_lt"]
      : request.id === "r3" ? ["trend", "ma", "rsi_n", "rsi_lt", "first_lt", "exit_rsi"]
      : request.id.startsWith("hpetf_r3_") ? ["trend", "first", "entry", "exit"] : [];
    for (const key of required) parameter(key);
    if (Object.hasOwn(p, "trend")) { const n = period("trend"); add("trend", `SMA(${n})`, "sma", simpleMovingAverage(px.c, n)); }
    if (Object.hasOwn(p, "ma")) { const n = period("ma"); add("ma", `SMA(${n})`, "sma", simpleMovingAverage(px.c, n)); }
    if (Object.hasOwn(p, "rsi_n") || (etf && /_(r3|rsi10|tps)_/.test(request.id))) {
      const n = etf ? request.id.startsWith("hpetf_rsi25_") ? Math.max(2, period("rsi_n")) : 2 : period("rsi_n");
      const thresholds = etf
        ? [...threshold("entry", "Entry", "entry"), ...threshold("first", "First RSI move", "reference"), ...threshold("add", "Scale-in", "reference"), ...threshold("exit", "Exit", "exit")]
        : [...threshold("rsi_lt", "Entry below", "entry"), ...threshold("rsi_gt", "Entry above", "entry"), ...threshold("first_lt", "First falling day", "reference"), ...threshold("exit_rsi", baseId.startsWith("dmi_") ? "Exit / entry cap" : "Exit", "exit")];
      add("rsi", `RSI(${n})`, "rsi", wilderRsi(px.c, n), thresholds);
    }
    if (Object.hasOwn(p, "slow_n")) { const n = period("slow_n"); add("slow-rsi", `RSI(${n}) · second condition`, "rsi", wilderRsi(px.c, n), threshold("slow_lt", "Second RSI below", "entry")); }
    if (baseId.startsWith("dmi_")) {
      const n = period("di_n");
      add("plus-di", `+DI(${n})`, "di", directionalPlus(px, n), threshold("di_lt", "Entry below", "entry"));
    }
    if (baseId === "pctb_1" || baseId === "pctb_2" || request.id.startsWith("hpetf_pctb_")) {
      const n = period(etf ? "n" : "pctb_n"), k = request.id.startsWith("spmt_") ? parameter("pctb_k") : 1;
      add("percent-b", `%b(${n}, ${k}σ)`, "pctb", bollingerPercentB(px.c, n, k),
        etf ? [...threshold("entry", "Entry", "entry"), ...threshold("exit", "Exit", "exit")]
          : threshold("pctb_lt", "Entry below", "entry"));
    }
    if (/^rsi_[123]$/.test(baseId)) {
      const n = request.id.startsWith("spmt_") || baseId === "rsi_3" ? period("sum_n") : period("rsi_n");
      const days = period(request.id.startsWith("spmt_") ? "sum_days" : "days");
      add("rsi-sum", `RSI(${n}) · ${days}-day sum`, "sum", rollingSum(wilderRsi(px.c, n), days), threshold("sum_lt", "Entry below", "entry"));
    }
  } catch (error) { return unavailable(error instanceof Error ? error.message : "Resolved indicator parameters are unavailable."); }
  if (!series.length) return unavailable("Resolved strategy parameters for the indicator chart are unavailable.");
  const auxiliary = /vix|trin|tick|nyse_adv/.test(baseId);
  return { status: "ready", series, note: "Daily indicator values use the full local source history and the selected run parameters. Weekly/monthly candles show the daily value at their last trading date. "
    + (auxiliary ? "Auxiliary market series are not plotted here; check the complete Rules for those inputs." : "Only the listed indicators are plotted; price-pattern, liquidity and position-management conditions remain in Rules.") };
}
