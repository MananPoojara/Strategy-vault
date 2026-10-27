import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import type {
  Catalogue, FidelityResult, Indicator, InstrumentsResponse, LabStrategy, OptimizeResult, PublishedStrategy,
  RunResult, SeriesResponse, TemplateResponse, TemplatesResponse, TickerRow,
} from "../../src/features/lab/types";

import { MODEL_NOTE, MONEY_BASIS_HINT } from "../../src/features/lab/labels";

const needs = { volume: false, intraday: false, breadth: false, second_symbol: false, fundamentals: false, derivatives: false, onchain: false, order_flow: false, order_book: false };
const ind = (patch: Partial<Indicator> & Pick<Indicator, "id" | "name" | "category">): Indicator => ({
  placement: "price", needs, computed: true, params: [], outputs: [patch.id], first: [0], sparse: false,
  definition: `${patch.id}(close)`, note: "", tv: "", ab: "", ...patch,
});
const len = (def: number) => ({ key: "length", label: "Length", default: def, min: 1, max: 5000, step: 1, integer: true });

export const catalogueFixture: Catalogue = {
  label: "User-defined indicator strategy · simulated · not a book method",
  price_sources: ["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"],
  operators: ["crosses_above", "crosses_below", ">", "<", ">=", "<="],
  timeframes: ["D", "W", "M"], max_conditions: 12, max_spec_bytes: 8192, max_combos: 50000,
  indicators: [
    ind({ id: "ema", name: "Exponential Moving Average", category: "Moving averages", params: [len(9)], tv: "ta.ema", ab: "EMA" }),
    ind({ id: "rsi", name: "Relative Strength Index", category: "Oscillators", placement: "pane", params: [len(14)], tv: "ta.rsi", ab: "RSI" }),
    ind({ id: "macd", name: "MACD", category: "Oscillators", placement: "pane", outputs: ["macd", "signal", "hist"], first: [25, 33, 33],
      params: [{ key: "fast", label: "Fast", default: 12, min: 1, max: 500, step: 1, integer: true },
               { key: "slow", label: "Slow", default: 26, min: 1, max: 500, step: 1, integer: true },
               { key: "signal", label: "Signal", default: 9, min: 1, max: 500, step: 1, integer: true }] }),
    ind({ id: "stoch", name: "Stochastic", category: "Oscillators", placement: "pane", outputs: ["k", "d"],
      params: [len(14), { key: "variant", label: "Variant (0 TradingView, 1 AmiBroker ratio of sums)", default: 0, min: 0, max: 1, step: 1, integer: true }] }),
    ind({ id: "obv", name: "On-Balance Volume", category: "Volume", placement: "pane", needs: { ...needs, volume: true } }),
    ind({ id: "ad_line", name: "Advance/Decline Line", category: "Breadth", placement: "pane", needs: { ...needs, breadth: true } }),
    ind({ id: "fractals", name: "Williams Fractals", category: "Patterns", outputs: ["up", "down"], sparse: true }),
    ind({ id: "doji", name: "Doji", category: "Patterns", outputs: ["doji"] }),
    ind({ id: "or_range", name: "Opening Range", category: "Opening range", computed: false, first: [null],
      needs: { ...needs, intraday: true }, note: "Needs intraday (minute) bars; not available yet." }),
  ],
};

export const tickersFixture: TickerRow[] = [
  { ticker: "NIFTY 50", first: "2000-01-03", last: "2026-10-14", has_volume: false, series: [] },
  { ticker: "NIFTYBANK", first: "2000-01-03", last: "2026-10-14", has_volume: false, series: [] },
  { ticker: "NSE:RELIANCE", symbol: "RELIANCE", basket: "NIFTY 50", first: "2005-01-03", last: "2026-10-14",
    has_volume: true, series: [], native: true, asset_type: "individual_stock", price_status: "ready" },
];

export const glInstrumentsFixture: InstrumentsResponse = {
  provisional: true, provisional_note: "Provisional · TradingView export · internal research only, not publishable to YT Strategies",
  label: "User-defined indicator strategy · simulated · not a book method",
  instruments: [
    { status: "ready", first: "2014-01-02", last: "2026-10-10", bars: 4300, tradable_from: "2014-02-01",
      instrument: { ticker: "CRYPTO:BTCUSD", type: "crypto", native: false, market: "crypto", currency: "USD", name: "Bitcoin / US dollar",
        has_volume: false, provisional: true, label: "Provisional · TradingView export CRYPTO:BTCUSD, daily" } },
    { status: "needs_data", reason: "IG:DAX is not staged on this server",
      instrument: { ticker: "IG:DAX", type: "cfd", native: false, market: "gl", currency: "EUR", name: "DAX", has_volume: false, provisional: true,
        label: "Provisional · TradingView export IG:DAX, daily" } },
  ],
};

export const dates = ["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-15", "2024-02-01", "2024-02-02"];
export const seriesFixture = (id: string, outputs: string[], values: Record<string, (number | null)[]>): SeriesResponse => ({
  label: catalogueFixture.label, id, tf: "D", params: {}, outputs, dates, series: values, provenance: null,
  instrument: { ticker: "NIFTY 50", type: "index", native: false, market: "in", currency: "INR" },
});

export const runFixture: RunResult = {
  label: "User-defined indicator strategy · simulated · not a book method", kind: "lab_simulation", origin: null,
  instrument: { ticker: "NIFTY 50", type: "index", native: false, market: "in", currency: "INR" },
  from: dates[0], to: dates[5], start: { date: "2024-01-08", reason: "stale opens before 2024-01-08" }, dates,
  close: [100, 101, 103, 102, 105, 106], equity: [1, 1, 1.02, 1.01, 1.04, 1.05], gross_equity: [1, 1, 1.021, 1.011, 1.041, 1.051],
  bh: [1, 1.01, 1.03, 1.02, 1.05, 1.06],
  trades: [
    { entry: "2024-01-08", exit: "2024-01-15", entry_i: 1, exit_i: 3, entry_px: 101, exit_px: 102, ret: 0.0099, bars: 2, open: false,
      signal: "2024-01-09", reason: "exit.any[0]: bars_held >= 2", dir: 1, entry_kind: "next_open" },
    { entry: "2024-02-01", exit: "2024-02-02", entry_i: 4, exit_i: 5, entry_px: 105, exit_px: 106, ret: -0.0095, bars: 1, open: true,
      signal: "", reason: "open", dir: -1, entry_kind: "close" },
  ],
  stats: { total_return: 0.05, cagr: 0.31, max_dd: -0.0098, sharpe: 1.42, trades: 2, win_rate: 0.5, avg_trade: 0.0002, bh_cagr: 0.4,
    exposure: 0.6, gross_total_return: 0.051 },
  gross_stats: { total_return: 0.051 },
  costs: { fees_bps: 5, slippage_bps: 2, rollover_bps: 0, total_side_bps: 7, taxes: null, rolls: null, basis: "per side",
    rollover_status: "none", slippage_status: "applied", money_status: "normalized", open_trade_cost_basis: "entry only" },
  provenance: null,
};

const P = "entry.all[0].rhs.p.length";
export const optimizeFixture: OptimizeResult = {
  label: runFixture.label, kind: "lab_optimization", origin: null, instrument: runFixture.instrument,
  from: "2010-01-04", to: "2026-10-14", start: null, split: "2022-01-03", combos: 3, threads: 2, ms: 900,
  dims: [{ path: P, values: [10, 20, 30] }],
  cols: [P, "total_return", "cagr", "sharpe", "max_dd", "trades", "monthly_thickness", "expectancy_ratio", "mar",
    "is_total_return", "is_cagr", "is_sharpe", "is_trades", "oos_total_return", "oos_cagr", "oos_sharpe", "oos_trades"],
  base: [0.5, 0.05, 0.6, -0.2, 40, 0.3, 0.1, 0.25, 0.4, 0.05, 0.6, 30, 0.1, 0.04, 0.5, 10],
  ranked_on: "in_sample",
  rank: { eligible: 3, distinct: 3, total: 3, min_trades: 10, per_year: 1, stats: ["cagr", "sharpe", "monthly_thickness", "expectancy_ratio", "mar"],
    groups: [], pool: [],
    top3: [{ row: 1, score: 0.9, weakest: 0.8, pct: [90, 85, 80, 95, 88], vs_leader: [null, null, null, null, null], groups: [] },
           { row: 0, score: 0.7, weakest: 0.6, pct: [70, 65, 60, 75, 68], vs_leader: [-0.01, -0.1, -0.05, -0.02, -0.03], groups: [] }] },
  picked: {
    "1": [20, 0.7, 0.07, 0.8, -0.18, 35, 0.35, 0.12, 0.39, 0.6, 0.07, 0.8, 28, 0.08, 0.03, 0.4, 7],
    "0": [10, 0.6, 0.06, 0.7, -0.19, 45, 0.31, 0.11, 0.32, 0.5, 0.06, 0.7, 33, 0.09, 0.035, 0.45, 12],
  },
  rows: [], costs: runFixture.costs, provenance: null,
};

const method = { id: "rsi2_4", name: "RSI 2 · 4", book: "Short Term Trading Strategies That Work", group: "RSI", dir: "long" };
export const templatesFixture: TemplatesResponse = {
  label: "Book template · declared adaptation into lab rules · not a verification", methods: 4365, admitted: 164,
  templates: [
    { method, template_version: 1, requirement: "index or ETF", allowed_types: ["index", "etf"] },
    { method: { ...method, id: "hpetf_rsi", name: "High-probability ETF RSI", book: "High Probability ETF Trading" }, template_version: 1,
      requirement: "actual ETF prices", allowed_types: ["etf"] },
  ],
};
export const templateFixture: TemplateResponse = {
  label: templatesFixture.label, method, status: "available",
  instrument: { requirement: "index or ETF", allowed_types: ["index", "etf"] },
  template: { version: 1, notes: ["Entry at the close, as in the book."],
    spec: { v: 1, origin: { book: "ststw", method: "rsi2_4", template_version: 1 }, execution: { fill: "close" },
      entry: { all: [{ lhs: { s: "rsi", p: { length: 2 } }, op: "<", rhs: { value: 4 } }] },
      exit: { any: [{ lhs: { s: "close" }, op: ">", rhs: { s: "sma", p: { length: 5 } } }] } } },
};
export const unavailableTemplateFixture: TemplateResponse = { label: templatesFixture.label, method, status: "unavailable", reason: "the book method is not admitted" };
export const fidelityFixture: FidelityResult = {
  label: "Template fidelity · display only · not a verification", kind: "lab_template_fidelity", method, template_version: 1,
  instrument: runFixture.instrument, from: "2000-01-03", to: "2026-10-14", book_trades: 12, template_trades: 12, matched: 12,
  reproduces: true, differences_total: 0, differences: [],
  note: "Compares the template's simulated trades with the book engine's simulated trades on the same data, window and default parameters, before costs. It does not verify the book method, its published figures or either engine.",
};
export const strategyFixture = (patch: Partial<LabStrategy> = {}): LabStrategy => ({
  id: 7, owner: "u1", ownerName: "Nandini", name: "EMA pullback", spec: { v: 1,
    entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }] },
    exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] } },
  origin: null, market: "in", instrument: "NIFTY 50", status: "draft", version: 3, submittedAt: null, reviewedAt: null,
  reviewNote: null, createdAt: "2026-10-10T10:00:00Z", updatedAt: "2026-10-12T10:00:00Z", publishable: true, ...patch,
});
export const publishedFixture: PublishedStrategy[] = [
  { snapshotId: 11, id: 7, version: 3, name: "EMA pullback", spec: strategyFixture().spec, origin: null, market: "in",
    instrument: "NIFTY 50", ownerName: "Nandini", publishedAt: "2026-10-13T09:00:00Z" },
  { snapshotId: 12, id: null, version: 1, name: "RSI 2 variant", spec: templateFixture.status === "available" ? templateFixture.template.spec : strategyFixture().spec,
    origin: { book: "ststw", method: "rsi2_4", template_version: 1 }, market: "us", instrument: "SPY", ownerName: null,
    publishedAt: "2026-10-14T09:00:00Z" },
];

export const jsonResponse = (body: unknown, status = 200, headers: Record<string, string> = {}) =>
  new Response(body === null ? null : JSON.stringify(body), { status, headers: { "Content-Type": "application/json", ...headers } });

/** fetch stub: first route whose pattern matches the URL (string = prefix) answers; unmatched → 404. Records calls. */
export function fetchRouter(routes: [RegExp | string, (url: string, init?: RequestInit) => Response | Promise<Response>][]) {
  const calls: { url: string; init?: RequestInit }[] = [];
  const fn = async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    const hit = routes.find(([p]) => (typeof p === "string" ? url.startsWith(p) : p.test(url)));
    return hit ? hit[1](url, init) : jsonResponse({ error: "Unknown endpoint." }, 404);
  };
  return Object.assign(fn, { calls });
}

let root: Root | undefined;
let host: HTMLDivElement | undefined;
/** act/createRoot mount (pattern of tests/library-controls.test.tsx). Call unmount() in afterEach. */
export async function mount(node: ReactNode) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(node));
  return host;
}
export async function unmount() {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
}
/** Let pending promises and effects settle inside act. */
export const flush = () => act(async () => { for (let i = 0; i < 5; i++) await Promise.resolve(); await new Promise((r) => setTimeout(r, 0)); });

/** Synthetic Phase 7 contracts for UI tests; money rows are not a validated ledger export. */
export const catalogue7Fixture: Catalogue = {
  ...catalogueFixture,
  operators: ["crosses_above", "crosses_below", ">", "<", ">=", "<=", "=", "!=", "crosses", "between", "outside", "rising", "falling"],
  timeframes: ["D", "W", "M", "Q", "Y"], custom_days: [2, 63], expand_modes: ["last", "first", "point"],
  max_leaves: 40, max_nodes: 120, node_kinds: ["all", "any", "not", "at_least", "then"],
  expressions: ["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change",
    "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"],
  max_query_bytes: 16384,
  execution: {
    direction: ["long", "short", "both"], fill: ["next_open", "close"], opposite: ["reverse", "exit", "ignore"], entry_on: ["level", "edge"],
    intrabar: ["stop_first", "ohlc_path"], sizing: ["pct_equity", "risk_pct"], distances: ["pct", "atr", "points", "at"],
    max_take_profits: 3, max_legs: 5, max_valid_bars: 20, max_delay_bars: 10,
    cooldown_after: ["any", "loss", "win"], max_cooldown_bars: 1000, max_trades_per: ["W", "M", "Q", "Y"], max_trades_count: 1000,
    max_consecutive_losses: 1000, max_pause_bars: 100000, max_drawdown_pct_below: 100,
  },
  indicators: [...catalogueFixture.indicators,
    ind({ id: "calendar", name: "Calendar", category: "Periods & calendar", outputs: ["dow", "day", "month", "year", "doy", "bar_index", "days_left"], first: [0, 0, 0, 0, 0, 0, 0] }),
    ind({ id: "session", name: "Session", category: "Periods & calendar",
      outputs: ["tdom", "tdoq", "tdoy", "tdow", "week_of_month", "quarter", "weekdays_left_m", "weekdays_left_q", "weekdays_left_y"],
      first: [31, 91, 366, 5, 31, 0, 0, 0, 0] }),
    ind({ id: "sessions_left", name: "Sessions left", category: "Periods & calendar", outputs: ["m", "q", "y"], first: [0, 0, 0], sparse: true }),
  ],
};
export const run7Fixture: RunResult = {
  ...runFixture,
  execution_model: { kind: "index", mode: "unit", executable: false, notes: [MODEL_NOTE.index], events: ["risk pause after 2 losing trades on 2024-01-15"] },
  adaptations: ["sessions_left reads this instrument's future session dates"],
  trades: [
    { ...runFixture.trades[0], legs: [["2024-01-08", 101, 0.5], ["2024-01-09", 101.5, 0.5]],
      fills: [["2024-01-10", 103, 0.5], ["2024-01-15", 102, 0.5]] },
    { ...runFixture.trades[1], entry_kind: "reverse" },
  ],
};
export const money7Fixture: RunResult = {
  ...runFixture,
  instrument: { ticker: "SPY", type: "etf", native: true, market: "us", currency: "USD" },
  execution_model: { kind: "etf", mode: "money", executable: true, notes: [], events: [] },
  money: { capital: 100000, final_equity: 101234.5, costs_total: 42, skipped: 1, currency: "USD",
    basis: MONEY_BASIS_HINT + " Quantities are computed on the admitted price series." },
  costs: { commission: { per_order: 21, per_unit: 0, pct: 0 }, slippage: { bps: 0 }, taxes: [], borrow_pct_year: null, basis: MONEY_BASIS_HINT },
  ledger: [
    { entry: "2024-01-08", qty: 10, notional: 1010, costs: 21, pnl: 9.4 },
    { entry: "2024-02-01", qty: 10, notional: 1050, costs: 21, pnl: -10.5 },
  ],
};

/** Picks an indicator in a rule's searchable picker (the "Indicator" button opens the list; an option is chosen on mousedown). */
export async function pickIndicator(scope: ParentNode, id: string) {
  await act(async () => scope.querySelector<HTMLButtonElement>('button[aria-label="Indicator"]')!.click());
  await act(async () => { scope.querySelector<HTMLElement>(`[role="option"][data-id="${id}"]`)!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); });
}
