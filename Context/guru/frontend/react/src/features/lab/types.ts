/** Lab API and rule-spec types (spec §5–§6; backend contract in Context/guru/backend/src/lab/api.rs). */
export type Market = "in" | "us" | "gl";
export type SavedMarket = Market | "crypto";
export type Role = "admin" | "researcher" | "viewer";
/** 'D' | 'W' | 'M' | 'Q' | 'Y' | '<n>D' (2–63). */
export type Timeframe = string;
export type ChartRange = "1Y" | "3Y" | "All";

export interface ForeignDisclosure { ticker: string; type: string; alignment: "same_date" | "prior_date"; fixup: boolean; first: string; last: string; uses?: {path:string;source:string;source_scope?:"portfolio_member";alignment:"same_date"|"prior_date";fixup:boolean}[] }
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
  custom_days?: [number, number]; expand_modes?: string[]; max_leaves?: number; max_nodes?: number;
  node_kinds?: string[]; expressions?: string[]; max_query_bytes?: number; execution?: CatalogueExecution;
  max_foreign?: number; foreign_volatility?: string | null; portfolio?: PortfolioCapability | null;
  max_conditions: number; max_spec_bytes: number; max_combos: number; indicators: Indicator[];
}
export interface CatalogueExecution {
  direction: string[]; fill: string[]; opposite: string[]; entry_on: string[]; intrabar: string[]; sizing: string[]; distances: string[];
  max_take_profits: number; max_legs: number; max_valid_bars: number; max_delay_bars: number;
  cooldown_after?: string[]; max_cooldown_bars?: number; max_trades_per?: string[]; max_trades_count?: number;
  max_consecutive_losses?: number; max_pause_bars?: number; max_drawdown_pct_below?: number;
}
export interface Instrument {
  ticker: string; type: string; native: boolean; market: SavedMarket | null; currency: string | null;
  name?: string; timezone?: string; calendar?: string; bars_per_year?: number; price?: string;
  has_volume?: boolean; provisional?: true; label?: string; source?: string; quirks?: string;
}
export interface TickerRow {
  ticker: string; first: string; last: string; has_volume: boolean; series: string[];
  symbol?: string; basket?: string; native?: boolean; asset_type?: string; price_status?: string; market?: SavedMarket; trusted_market?: Market;
}
export type GlInstrumentRow =
  | { instrument: Instrument; status: "ready"; first: string; last: string; bars: number; tradable_from: string | null }
  | { instrument: Instrument; status: "needs_data"; reason: string };
export interface InstrumentsResponse extends Provisional { label: string; instruments: GlInstrumentRow[] }
export interface SeriesResponse extends Provisional {
  label: string; instrument: Instrument; id: string; tf: Timeframe; params: Record<string, number>;
  adaptations?: string[]; foreign?: ForeignDisclosure[]; outputs: string[]; dates: string[]; series: Record<string, (number | null)[]>; provenance: unknown;
}

// ---- rule spec (Context/guru/backend/src/lab/rules.rs; Phase 7 spec §4–§7). Unknown keys are kept by every editor.
export type ExpandMode = "last" | "first" | "point";
export interface ValueOperand { value: number }
export interface SeriesOperand { s: string; p?: Record<string, number>; tf?: Timeframe; out?: string; offset?: number; expand?: ExpandMode; symbol?: string; vs?: string; fixup?: "none" | "previous" }
/** An expression or state operand: one key of rule-tree EXPR_KEYS plus its fields; read it through rule-tree.ts. */
export type ExprOperand = { [key: string]: unknown };
export type Operand = ValueOperand | SeriesOperand | ExprOperand;
export interface Cond { lhs: Operand; op: string; rhs: Operand; [extra: string]: unknown }
export interface RangeNode { lhs: Operand; op: "between" | "outside"; lo: Operand; hi: Operand; [extra: string]: unknown }
export interface TrendNode { lhs: Operand; op: "rising" | "falling"; bars: number; [extra: string]: unknown }
export interface AllNode { all: RuleNode[]; [extra: string]: unknown }
export interface AnyNode { any: RuleNode[]; [extra: string]: unknown }
export interface NotNode { not: RuleNode; [extra: string]: unknown }
export interface AtLeastNode { at_least: number; of: number; when: RuleNode; [extra: string]: unknown }
export interface ThenNode { then: RuleNode[]; within: number; [extra: string]: unknown }
export type RuleNode = Cond | RangeNode | TrendNode | AllNode | AnyNode | NotNode | AtLeastNode | ThenNode;
export type Dist = { pct: number } | { atr: number; atr_length?: number } | { points: number };
export type EntryLevel = Dist | { at: Operand };
export type TakeProfit = Dist & { size: number };
export interface ScaleIn { legs: number[]; min_bars?: number; price?: "any" | "better" | "worse" }
export interface EntrySpec { all: RuleNode[]; limit?: EntryLevel; stop?: EntryLevel; valid_bars?: number; scale_in?: ScaleIn; [extra: string]: unknown }
export interface ExitSpec {
  any: RuleNode[]; stop_loss?: Dist; target?: Dist; trailing?: Dist & { activate?: Dist }; breakeven?: { after: Dist; plus?: Dist };
  take_profits?: TakeProfit[]; min_bars?: number; [extra: string]: unknown;
}
export interface ShortSide { entry: EntrySpec; exit: ExitSpec; [extra: string]: unknown }
export type Sizing = { pct_equity: number } | { risk_pct: number } | { units: number } | { value: number };
export interface MoneyCosts {
  commission?: { per_order?: number; per_unit?: number; pct?: number }; slippage?: { bps: number };
  taxes?: { name: string; pct: number; side: "buy" | "sell" | "both" }[]; borrow_pct_year?: number;
}
export interface Execution {
  fill?: Fill; exit_fill?: Fill; cost_bps?: number; opposite?: "reverse" | "exit" | "ignore"; delay_bars?: number;
  entry_on?: "level" | "edge"; intrabar?: "stop_first" | "ohlc_path"; sizing?: Sizing; capital?: number; costs?: MoneyCosts;
  cooldown?: { bars: number; after?: "any" | "loss" | "win" }; max_trades?: { count: number; per: "W" | "M" | "Q" | "Y" };
  risk?: { max_consecutive_losses?: number; pause_bars?: number; max_drawdown_pct?: number };
  [extra: string]: unknown;
}
export interface Origin { book: string; method: string; template_version: number }
export interface OptimizeDim { path: string; from: number; to: number; step: number }
export type Fill = "next_open" | "close";
export interface Spec {
  v: 1; direction?: "long" | "short" | "both"; origin?: Origin; entry: EntrySpec; exit: ExitSpec; short_side?: ShortSide;
  execution?: Execution; optimize?: OptimizeDim[]; portfolio?: PortfolioSpec; [extra: string]: unknown;
}

// ---- run / optimize
export type Piece = [date: string, price: number, weight: number];
export interface Trade {
  legs?: Piece[]; fills?: Piece[]; weight?: number;
  entry: string; exit: string; entry_i: number; exit_i: number; entry_px: number | null; exit_px: number | null;
  ret: number | null; bars: number; open: boolean; signal: string; reason: string; dir: 1 | -1; entry_kind: string;
}
export type Stats = Record<string, number | null>;
export interface UnitCosts {
  fees_bps: number; slippage_bps: number; rollover_bps: number; total_side_bps: number; taxes: null; rolls: null;
  basis: string; rollover_status: string; slippage_status: string; money_status: string; open_trade_cost_basis: string;
}
export interface DeclaredCosts {
  commission: { per_order: number; per_unit: number; pct: number }; slippage: { bps: number };
  taxes: { name: string; pct: number; side: "buy" | "sell" | "both" }[]; borrow_pct_year: number | null; basis: string;
  fees_bps?: never; slippage_bps?: never;
}
export type Costs = UnitCosts | DeclaredCosts;
export interface BookMethod { id: string; name: string; book: string; group: string; dir: string }
export interface ResponseOrigin { method: BookMethod; template_version: number }
export type Start = null | { date: string; reason: string };
export interface ExecutionModel { kind: string; mode: "unit" | "money"; executable: boolean; notes: string[]; events: string[] | null }
export interface MoneySummary { capital: number; final_equity: number; costs_total: number; skipped: number; currency: string | null; basis: string }
export interface LedgerFill { date: string; px: number; qty: number; commission: number; taxes: number; slippage: number }
export interface LedgerRow {
  entry: string; qty: number; notional: number; costs: number; pnl: number;
  borrow?: number; open?: boolean; entries?: LedgerFill[]; exits?: LedgerFill[];
}
export interface RunResult extends Provisional {
  execution_model?: ExecutionModel; money?: MoneySummary; ledger?: LedgerRow[]; adaptations?: string[]; foreign?: ForeignDisclosure[];
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
  money?: Pick<MoneySummary, "capital" | "basis">;
  execution_model?: ExecutionModel; adaptations?: string[]; foreign?: ForeignDisclosure[];
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
  engineStatus?: EngineStatus; label?: string; provisionalNote?: string; retryAfter?: number; read?: boolean;
}

// Phase 8 portfolios have their own result kind and never invent a single execution instrument.
export interface PortfolioCapability { modes: string[]; max_universe: number; max_universe_list: number; max_open: number; max_combos: number; max_work: number; max_source_bars?:number;max_member_bars?:number;max_compiled_cells?:number;max_workers?:number; rebalance: string[]; instruments?: {ticker:string;type:string;market:Market}[]; baskets: {id:string;members:number;available_members?:number;membership_kind:string;supported?:boolean}[] }
export interface PortfolioSpec { universe: {basket:string;tickers?:never}|{tickers:string[];basket?:never}; mode?: "signals"|"rotation"; score?: Operand; rank?: "highest"|"lowest"; max_open?:number; worst_rank_held?:number; rebalance?:"bar"|"W"|"M"|"Q"|"Y"; [extra:string]:unknown }
export interface PortfolioMeta {mode:"signals"|"rotation";max_open:number;worst_rank_held:number;rank:"highest"|"lowest";rebalance:string|null;universe:{basket:string|null;members:number;admitted:number;refused:{ticker:string;reason:string}[]}}
export interface Membership {kind:string;historical_membership_verified:false;note:string}
export interface PortfolioTrade extends Trade {ticker:string;qty:number;notional:number;costs:number;rank:number;score:number|null;entry_signal:string}
export interface PortfolioRunResult extends Provisional {kind:"lab_portfolio_simulation";label:string;origin:null;portfolio:PortfolioMeta;membership:Membership;reference:{ticker:string;type:"index"};foreign?:ForeignDisclosure[];execution_model:ExecutionModel;adaptations?:string[];from:string;to:string;dates:string[];equity:number[];gross_equity:number[];cash:number[];exposure:number[];positions:number[];bh:number[];trades:PortfolioTrade[];skipped:Record<string,number>;stats:Stats;gross_stats:Stats;money:{capital:number;final_equity:number;costs_total:number;currency:string};provenance:unknown}
export type PortfolioOptimizeResult = Omit<OptimizeResult,"kind"|"instrument"|"start"|"costs"|"provenance"|"money"> & {kind:"lab_portfolio_optimization";portfolio:PortfolioMeta;membership:Membership;reference:{ticker:string;type:"index"};money:{capital:number;basis?:string}};
export type LabRunResult = RunResult | PortfolioRunResult;
export type LabOptimizeResult = OptimizeResult | PortfolioOptimizeResult;
