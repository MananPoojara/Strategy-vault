/** Lab API and rule-spec types (spec §5–§6; backend contract in Context/guru/backend/src/lab/api.rs). */
export type Market = "in" | "us" | "gl";
export type SavedMarket = Market | "crypto";
export type Role = "admin" | "researcher" | "viewer";
/** 'D' | 'W' | 'M' today; Phase 7 may add more through catalogue.timeframes. */
export type Timeframe = string;
export type ChartRange = "1Y" | "3Y" | "All";

export interface Provisional { provisional?: true; provisional_note?: string }
export interface IndicatorParam { key: string; label: string; default: number; min: number; max: number; step: number | null; integer: boolean }
export interface Needs { volume: boolean; intraday: boolean; breadth: boolean; second_symbol: boolean; fundamentals: boolean; derivatives: boolean; onchain: boolean }
export interface Indicator {
  id: string; name: string; category: string; placement: "price" | "pane"; needs: Needs; computed: boolean;
  params: IndicatorParam[]; outputs: string[]; first: (number | null)[]; sparse: boolean;
  definition: string; note: string; tv: string; ab: string;
}
export interface Catalogue extends Provisional {
  label: string; price_sources: string[]; operators: string[]; timeframes: Timeframe[];
  max_conditions: number; max_spec_bytes: number; max_combos: number; indicators: Indicator[];
}
export interface Instrument {
  ticker: string; type: string; native: boolean; market: SavedMarket | null; currency: string | null;
  name?: string; timezone?: string; calendar?: string; bars_per_year?: number; price?: string;
  has_volume?: boolean; provisional?: true; label?: string; source?: string; quirks?: string;
}
export interface TickerRow {
  ticker: string; first: string; last: string; has_volume: boolean; series: string[];
  symbol?: string; basket?: string; native?: boolean; asset_type?: string; price_status?: string;
}
export type GlInstrumentRow =
  | { instrument: Instrument; status: "ready"; first: string; last: string; bars: number; tradable_from: string | null }
  | { instrument: Instrument; status: "needs_data"; reason: string };
export interface InstrumentsResponse extends Provisional { label: string; instruments: GlInstrumentRow[] }
export interface SeriesResponse extends Provisional {
  label: string; instrument: Instrument; id: string; tf: Timeframe; params: Record<string, number>;
  outputs: string[]; dates: string[]; series: Record<string, (number | null)[]>; provenance: unknown;
}

// ---- rule spec v1 (Context/guru/backend/src/lab/rules.rs). Unknown keys are kept by every editor.
export interface ValueOperand { value: number }
export interface SeriesOperand { s: string; p?: Record<string, number>; tf?: Timeframe; out?: string; offset?: number }
export type Operand = ValueOperand | SeriesOperand;
export interface Cond { lhs: Operand; op: string; rhs: Operand; [extra: string]: unknown }
export type Dist = { pct: number } | { atr: number; atr_length?: number };
export interface Origin { book: string; method: string; template_version: number }
export interface OptimizeDim { path: string; from: number; to: number; step: number }
export type Fill = "next_open" | "close";
export interface Spec {
  v: 1;
  direction?: "long" | "short";
  origin?: Origin;
  entry: { all: Cond[]; limit?: Dist; [extra: string]: unknown };
  exit: { any: Cond[]; stop_loss?: Dist; target?: Dist; trailing?: Dist; [extra: string]: unknown };
  execution?: { fill?: Fill; exit_fill?: Fill; cost_bps?: number; [extra: string]: unknown };
  optimize?: OptimizeDim[];
  [extra: string]: unknown;
}

// ---- run / optimize
export interface Trade {
  entry: string; exit: string; entry_i: number; exit_i: number; entry_px: number | null; exit_px: number | null;
  ret: number | null; bars: number; open: boolean; signal: string; reason: string; dir: 1 | -1; entry_kind: string;
}
export type Stats = Record<string, number | null>;
export interface Costs {
  fees_bps: number; slippage_bps: number; rollover_bps: number; total_side_bps: number; taxes: null; rolls: null;
  basis: string; rollover_status: string; slippage_status: string; money_status: string; open_trade_cost_basis: string;
}
export interface BookMethod { id: string; name: string; book: string; group: string; dir: string }
export interface ResponseOrigin { method: BookMethod; template_version: number }
export type Start = null | { date: string; reason: string };
export interface RunResult extends Provisional {
  label: string; kind: "lab_simulation"; origin: ResponseOrigin | null; instrument: Instrument;
  from: string; to: string; start: Start; dates: string[]; close: number[]; equity: number[];
  gross_equity: number[]; bh: number[]; trades: Trade[]; stats: Stats; gross_stats: Stats; costs: Costs; provenance: unknown;
}
export interface Cand { row: number; score: number; weakest: number; pct: number[]; vs_leader: (number | null)[]; groups: string[] }
export interface Rank {
  eligible: number; distinct: number; total: number; min_trades: number; per_year: number; stats?: string[];
  groups: { stat: string; label: string; rows: number[] }[]; pool: Cand[]; top3: Cand[];
}
export interface OptimizeResult extends Provisional {
  label: string; kind: "lab_optimization"; origin: ResponseOrigin | null; instrument: Instrument;
  from: string; to: string; start: Start; split: string | null; combos: number; threads: number; ms: number;
  dims: { path: string; values: number[] }[];
  /** dims paths first, then metric names; `rows`/`picked` rows align with cols; `base` aligns with cols after the dims. */
  cols: string[]; base: number[]; ranked_on: "in_sample" | "full"; rank: Rank;
  picked: Record<string, number[]>; rows: number[][]; costs: Costs; provenance: unknown;
}

// ---- Customize
export interface TemplateListItem { method: BookMethod; template_version: number; requirement: string; allowed_types: string[] }
export interface TemplatesResponse { label: string; methods: number; admitted: number; templates: TemplateListItem[] }
export type TemplateResponse =
  | { label: string; method: BookMethod; status: "unavailable"; reason: string }
  | { label: string; method: BookMethod; status: "available"; instrument: { requirement: string; allowed_types: string[] };
      template: { version: number; spec: Spec; notes: string[] } };
export interface FTrade { entry: string; exit: string; entry_px: number | null; exit_px: number | null; dir: number; legs: number; open: boolean }
export interface FidelityResult {
  label: string; kind: "lab_template_fidelity"; method: BookMethod; template_version: number; instrument: Instrument;
  from: string; to: string; book_trades: number; template_trades: number; matched: number; reproduces: boolean;
  differences_total: number; differences: { book: FTrade | null; template: FTrade | null }[]; note: string;
}

// ---- saved strategies (/api/app/lab/strategies)
export type StrategyStatus = "draft" | "submitted" | "published" | "rejected";
export interface LabStrategy {
  id: number; owner: string; ownerName: string; name: string; spec: Spec; origin: Origin | null;
  market: SavedMarket; instrument: string; status: StrategyStatus; version: number;
  submittedAt: string | null; reviewedAt: string | null; reviewNote: string | null;
  createdAt: string; updatedAt: string; publishable: boolean;
}
export interface PublishedStrategy {
  snapshotId: number; id: number | null; version: number; name: string; spec: Spec; origin: Origin | null;
  market: "in" | "us"; instrument: string; ownerName: string | null; publishedAt: string;
}
export interface StrategyDraft { name: string; spec: Spec; market: SavedMarket; instrument: string }

// ---- errors (LabError in client.ts implements this; UI reads it duck-typed)
export type EngineStatus = "invalid" | "unsupported" | "needs_data" | "not_found";
export interface LabErrorFields {
  message: string; status: number; kind: "engine" | "gateway" | "auth";
  engineStatus?: EngineStatus; label?: string; provisionalNote?: string; retryAfter?: number;
}
