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
  /** Frontend only: a "My indicators" formula drawn as a chart chip (series come from /lab/series?formula=…). */
  formula?: string;
}
export interface Catalogue extends Provisional {
  label: string; price_sources: string[]; operators: string[]; timeframes: Timeframe[];
  custom_days?: [number, number]; expand_modes?: string[]; max_leaves?: number; max_nodes?: number;
  node_kinds?: string[]; expressions?: string[]; max_query_bytes?: number; execution?: CatalogueExecution;
  max_foreign?: number; foreign_volatility?: string | null; portfolio?: PortfolioCapability | null;
  max_conditions: number; max_spec_bytes: number; max_combos: number; indicators: Indicator[];
  /** Formula-language help for the custom indicator editor. */
  formula_functions?: FormulaFunction[];
}
// ---- custom indicators (formula language; engine lab/formula.rs, storage /api/app/lab/indicators). Shapes are read in formula.ts only.
export interface FormulaFunction { name: string; args: string; desc: string }
export interface FormulaParam { key: string; default: number; min: number; max: number; step: number }
export interface FormulaInfo { params: FormulaParam[]; plots: string[]; placement: "pane" | "price" }
/** A rule operand carrying its own formula text, so a saved strategy keeps the formula it was saved with. */
export interface FormulaOperand { formula: string; name: string; p?: Record<string, number>; out?: string; tf?: Timeframe; offset?: number; expand?: ExpandMode; [extra: string]: unknown }
/** `blocks`: the block builder's layout (opaque here); null when the formula was typed. */
export interface SavedIndicator { id: number; name: string; formula: string; blocks?: unknown | null; version: number; createdAt: string; updatedAt: string }
export interface IndicatorDraft { name: string; formula: string; blocks: unknown | null }
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
  /** Present on a formula series. */
  formula?: FormulaInfo;
}

// ---- rule spec (Context/guru/backend/src/lab/rules.rs; Phase 7 spec §4–§7). Unknown keys are kept by every editor.
export type ExpandMode = "last" | "first" | "point";
export interface ValueOperand { value: number }
export interface SeriesOperand { s: string; p?: Record<string, number>; tf?: Timeframe; out?: string; offset?: number; expand?: ExpandMode; symbol?: string; vs?: string; fixup?: "none" | "previous" }
/** An expression or state operand: one key of rule-tree EXPR_KEYS plus its fields; read it through rule-tree.ts. */
export type ExprOperand = { [key: string]: unknown };
export type Operand = ValueOperand | SeriesOperand | FormulaOperand | ExprOperand;
export interface Cond { lhs: Operand; op: string; rhs: Operand; [extra: string]: unknown }
export interface RangeNode { lhs: Operand; op: "between" | "outside"; lo: Operand; hi: Operand; [extra: string]: unknown }
export interface TrendNode { lhs: Operand; op: "rising" | "falling"; bars: number; [extra: string]: unknown }
export interface AllNode { all: RuleNode[]; [extra: string]: unknown }
export interface AnyNode { any: RuleNode[]; [extra: string]: unknown }
export interface NotNode { not: RuleNode; [extra: string]: unknown }
export interface AtLeastNode { at_least: number; of: number; when: RuleNode; [extra: string]: unknown }
export interface ThenNode { then: RuleNode[]; within: number; [extra: string]: unknown }
export type RuleNode = Cond | RangeNode | TrendNode | AllNode | AnyNode | NotNode | AtLeastNode | ThenNode;
export type Dist = { pct: number } | { atr: number; atr_length?: number } | { points: number } | { profit_pct: number };
/** AmiBroker trade price fields (Analysis settings › Trades); average = (O+H+L+C)/4. */
export type TradePrice = "open" | "high" | "low" | "close" | "average";
export type TradeSide = "buy" | "sell" | "short" | "cover";
/** AmiBroker ApplyStop ExitAtStop: 0 tested and filled at the bar's trade price, 1 intraday at the stop, 2 High-Low then the next bar's trade price. */
export type ExitAtStop = 0 | 1 | 2;
export type StopKey = "stop_loss" | "target" | "trailing" | "bars";
export type EntryLevel = Dist | { at: Operand };
export type TakeProfit = Dist & { size: number };
export interface ScaleIn { legs: number[]; min_bars?: number; price?: "any" | "better" | "worse" }
export interface EntrySpec { all: RuleNode[]; limit?: EntryLevel; stop?: EntryLevel; valid_bars?: number; scale_in?: ScaleIn; [extra: string]: unknown }
export interface ExitSpec {
  any: RuleNode[]; stop_loss?: Dist; target?: Dist; trailing?: Dist & { activate?: Dist }; breakeven?: { after: Dist; plus?: Dist };
  take_profits?: TakeProfit[]; min_bars?: number;
  /** rules.rs exit.dual_atr: price source ∓ multiple × average true range, initial and favour bands; tf D (default), W or M. */
  dual_atr?: { initial: { period: number; mult: number; price?: string }; favour: { period: number; mult: number; price?: string }; tf?: "D" | "W" | "M"; period_end_only?: boolean };
  [extra: string]: unknown;
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
  cooldown?: { bars: number; after?: "any" | "loss" | "win" };
  prices?: Partial<Record<TradeSide, TradePrice>>; delays?: Partial<Record<TradeSide, number>>;
  reentry?: { stop_loss?: number; target?: number; trailing?: number; bars?: number }; max_trades?: { count: number; per: "W" | "M" | "Q" | "Y" };
  /** AmiBroker Stops (single instrument, both sides): ExitAtStop per stop (absent = 1), "Activate stops immediately" (absent = true, today's),
   *  N-bar "Has priority", General › "Allow same bar exit / entry signal" (absent = true, today's). */
  exit_at_stop?: Partial<Record<StopKey, ExitAtStop>>; activate_stops_immediately?: boolean; nbar_has_priority?: boolean; allow_same_bar_exit?: boolean; reenter_on_exit_bar?: boolean;
  /** "signal": bars_held counts from the entry signal bar (AmiBroker AFL BarsSince(Buy)); default the fill bar. */
  bars_held_from?: "fill" | "signal";
  risk?: { max_consecutive_losses?: number; pause_bars?: number; max_drawdown_pct?: number };
  /** AmiBroker position limits (money mode only): every field optional; absent = no limit. */
  limits?: PositionLimits;
  /** AmiBroker General › Tick size of a cash instrument (0 / absent: no rounding) and Pad and align all data to reference symbol. */
  tick_size?: number; pad_align?: boolean;
  /** AmiBroker Settings › Report risk-free rates in % (absent: 5 / 5.4); single-instrument runs only. */
  report?: { rf_sharpe?: number; rf_upi?: number };
  [extra: string]: unknown;
}
export interface PositionLimits {
  min_shares?: number; round_lot?: number; min_value?: number; max_value?: number; max_volume_pct?: number;
  shrink?: boolean; prev_bar_equity?: boolean; interest_pct?: number; margin_pct?: number; margin_rate_pct?: number; [extra: string]: unknown;
}
export interface Origin { book: string; method: string; template_version: number }
export interface OptimizeDim { path: string; from: number; to: number; step: number }
export type Fill = "next_open" | "close";
export interface Spec {
  v: 1; direction?: "long" | "short" | "both"; origin?: Origin; entry: EntrySpec; exit: ExitSpec; short_side?: ShortSide;
  execution?: Execution; optimize?: OptimizeDim[]; portfolio?: PortfolioSpec; date_filter?: DateFilter; [extra: string]: unknown;
}
/** A calendar window: from (inclusive) to `to` (the first day outside), YYYY-MM-DD. */
export interface DateWindow { from: string; to: string; label?: string }
/** A saved custom date filter (auth lab-date-sets.mjs): personal, or shared by an admin with everyone. */
export interface DateSet { id: number; name: string; shared: boolean; windows: DateWindow[]; version: number; mine: boolean; editable: boolean; createdAt: string; updatedAt: string }
/** rules.rs DateFilter: trade inside (bull) or outside (bear) the windows ([from, to] pairs); exit when a window ends / starts;
 *  `set` names the saved set the windows come from. */
export interface DateFilter { mode?: "inside" | "outside"; windows: [string, string][]; exit_at_change?: boolean; off?: boolean; set?: { id?: number; name?: string } }

// ---- run / optimize
export type Piece = [date: string, price: number, weight: number];
export interface Trade {
  legs?: Piece[]; fills?: Piece[]; weight?: number;
  entry: string; exit: string; entry_i: number; exit_i: number; entry_px: number | null; exit_px: number | null;
  ret: number | null; bars: number; open: boolean; signal: string; reason: string; dir: 1 | -1; entry_kind: string;
  /** Maximum adverse / favourable excursion, fractions of the entry price (AmiBroker MAE/MFE). */
  mae?: number | null; mfe?: number | null;
}
export interface ReportGroup { count: number | null; pct: number | null; total: number | null; profit?: number | null; avg: number | null; avg_bars: number | null; largest: number | null; bars_in_largest: number | null; max_consecutive: number | null }
export interface ReportSide { trades: number; avg: number | null; avg_bars: number | null; profit_factor: number | null; payoff: number | null; winners: ReportGroup; losers: ReportGroup }
/** AmiBroker-style backtest report (lab/report.rs). */
export interface LabReport { all: ReportSide; long: ReportSide; short: ReportSide; system: Record<string, number | null>; buy_hold?: Record<string, number | null>;
  /** Buy&Hold as one trade, same keys as `all` (null where not applicable); absent from older responses. */
  buyhold?: ReportSide | null;
  /** Risk-free rates (%) the Sharpe ratio of trades and the UPI used: execution.report or AmiBroker's 5 / 5.4. */
  rates?: { sharpe: number; upi: number }; notes: string[] }
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
export interface MoneySummary {
  capital: number; final_equity: number; costs_total: number; skipped: number; currency: string | null; basis: string;
  /** Only when execution.limits is set: total idle-cash interest and why entries were skipped; margin_interest only with a margin rate. */
  limits?: { interest: number; margin_interest?: number; skip_reasons: Record<string, number>; basis: string };
}
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
  report?: LabReport;
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
  /** Requested with table=1: every combination, dim values then `table_cols`, in the ranking period. */
  table_cols?: string[]; table?: (number | null)[][];
  /** AmiBroker Analysis "Info" tab: elapsed ms, combinations evaluated, threads, warnings. */
  info?: LabInfo;
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
export interface LabInfo { ms: number; combos: number; threads: number; warnings: string[] }
/** /lab/optimize?mode=individual (AmiBroker Individual Optimize): each universe symbol optimized alone. `best` and `table` rows: dim values then `cols` / `table_cols`. */
export interface IndividualSymbol { ticker: string; skipped?: string; from?: string; to?: string; eligible?: number; min_trades?: number; best?: number[] | null; table?: (number | null)[][]; table_rows?: number }
export interface IndividualOptimizeResult extends Provisional {
  label: string; kind: "lab_individual_optimization"; origin: null; universe: { basket: string | null; symbols: number }; combos: number;
  dims: { path: string; values: number[] }[]; cols: string[]; table_cols?: string[]; results: IndividualSymbol[]; notes: string[]; info: LabInfo; adaptations?: string[];
}

// ---- Walk-forward (/lab/walkforward): one optimizer run per in-sample window, its best setting run out-of-sample
export type WalkforwardTarget = "score" | "cagr" | "sharpe" | "monthly_thickness" | "expectancy_ratio" | "mar";
export interface WalkforwardOptions { is: number; oos: number; step: number; unit: "years" | "months"; anchored: boolean; target: WalkforwardTarget }
export type WalkforwardMetrics = Record<"total_return" | "cagr" | "sharpe" | "max_dd" | "mar" | "trades" | "win_rate" | "profit_factor", number | null>;
export interface WalkforwardWindow {
  is: { from: string; to: string }; oos: { from: string; to: string }; eligible: number;
  /** null when no setting was eligible in-sample; the out-of-sample window is then held flat (`skipped`). */
  params: Record<string, number> | null; is_metrics: WalkforwardMetrics | null; oos_metrics: WalkforwardMetrics | null; skipped?: string;
  /** a position was still open at the window's last bar: marked at that close, not counted in trades. */
  open_at_end?: boolean;
}
export interface WalkforwardResult extends Provisional {
  label: string; kind: "lab_walkforward"; target: WalkforwardTarget; unit: "years" | "months"; is: number; oos: number; step: number; anchored: boolean;
  combos: number; ms: number; windows: WalkforwardWindow[]; oos_equity: { dates: string[]; equity: number[] }; info?: LabInfo;
  summary: { total_return: number | null; cagr: number | null; max_dd: number | null; trades: number; win_rate: number | null; windows: number; skipped: number; open_at_edge?: number; gaps?: boolean; from: string; to: string };
  notes?: string[];
}
