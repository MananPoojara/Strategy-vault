/** Distinct cash-account contract. These checks validate reporting consistency;
 * they do not certify book figures, vendor prices or source performance. */
export const BANDY_ACCOUNT_MODEL = "bandy_fixed_fraction_fractional_cash_v1";
export const BANDY_ACCOUNT_METHODS = ["hbiab_ma5_20", "hbiab_ma7_4", "hbiab_macd_example", "hbiab_simple_ma25"] as const;
export const bandyAccountMethod = (id?: string, book?: string) => book === "hbiab"
  && BANDY_ACCOUNT_METHODS.some(method => method === id);
export const BANDY_SIMPLE_METHOD = "hbiab_simple_ma25";
export const BANDY_SIMPLE_CONVENTION = "bandy_simple_raw_buy_clock_fractional_cash_v1";
export const BANDY_SIMPLE_BASIS = "Bandy Introduction to AmiBroker (2008), printed527-528, SimpleSystem.afl interpretation/commentary teaching example shown on actual SPY; no printed performance study. MA25=MA(C,25), raw Buy=Close>MA25, Sell=BarsSince(raw Buy)>=5; every true raw Buy resets the clock even while held, and equality is false. Twenty-five genuine observations mature MA; no actionable sell before the first observed Buy and no native BarsSince sentinel claim. Daily finalized same-close fills, one long without pyramiding, 100% fractional current-equity target with entry-cost cash reserve are declared research conventions. Normalized initial equity1, full supplied signal history and flat selected account start; terminal entries remain open and pay actual entry cost only. Gross removes paid costs from identical accepted quantities; liquidation estimate separate. Actual stocks/ordinary equity ETFs only; no cash-index, basket portfolio or derivative substitution. No source performance, funded capital, native AmiBroker binary parity or broker-execution claim.";
/** Catalogue identity and effective selected settings are separate from the account wire. */
export interface BandyAdmissionContext { definition: unknown; parameters: unknown; instrument: unknown }
const cashIndices=new Set(["NIFTY 50","BANK NIFTY","MIDCAP 100","NIFTY BANK","NIFTY MIDCAP 100","SPX","NDX","DJI"]);
/** The row is retained unchanged. ETF eligibility comes from the existing UI
 * selector, because older runtime ETF rows do not report their asset type. */
export function simpleInstrumentContext(ticker: string, rows: readonly unknown[], eligibleEtfs: readonly unknown[]) {
  const metadata=rows.find(row=>object(row) && row.ticker === ticker);
  if (!object(metadata) || cashIndices.has(ticker)) return null;
  if (metadata.asset_type === "individual_stock") return {
    kind:"individual_stock", metadata, classification_basis:"native_stock_metadata",
  };
  const eligible=eligibleEtfs.filter(row=>object(row) && rows.includes(row)).map(row=>(row as Fields).ticker);
  return !ticker.startsWith("basket:") && eligible.includes(ticker) && (metadata.asset_type === undefined || metadata.asset_type === "etf")
    ? {kind:"etf",metadata,classification_basis:"existing_runtime_etf_selector",eligible_etf_tickers:eligible} : null;
}
export interface BandyAccountDay {
  date: string; source_index: number; equity: number; gross_same_quantity_equity: number;
  costs_paid: number; free_cash: number; quantity: number; position_value: number; direction: 0 | 1;
}
export interface BandyTradeAccount {
  entry: string; exit: string; open: boolean; entry_signal_date: string; exit_signal_date: string | null;
  entry_phase: "close" | "open"; exit_phase: "close" | "open" | "mark";
  raw_instrument_gross_return: number; entry_equity_units: number; entry_allocation_fraction: number;
  quantity: number; entry_notional_units: number; exit_notional_units: number | null;
  gross_account_contribution: number; net_account_contribution: number;
  entry_cost_account_contribution: number; exit_cost_account_contribution: number;
  gross_fraction_of_actual_entry_equity: number; net_fraction_of_actual_entry_equity: number;
  open_liquidation_cost_estimate: number;
}
export interface BandySourceAccount {
  model: typeof BANDY_ACCOUNT_MODEL; book_performance_reproduced: false;
  basis: string; account_contribution_basis: string; rejected_entries: number;
  sizing: {target_equity_fraction: 1 | 0.25; origin: "source_rule" | "declared_research_convention";
    cash_reserved: true; fractional_positions: true};
  terminal_entry_pending: boolean; terminal_exit_pending: boolean;
  pending_order: null | {side: "buy" | "sell"; signal_date: string; fill_phase: "open"; filled: false};
  days: BandyAccountDay[]; trade_accounts: BandyTradeAccount[];
}
type Fields = Record<string, unknown>;
const object = (v: unknown): v is Fields => Boolean(v) && typeof v === "object" && !Array.isArray(v);
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const dayString = (v: unknown): v is string => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v)
  && Number.isFinite(Date.parse(v)) && new Date(v).toISOString().slice(0, 10) === v;
const count = (v: unknown): v is number => finite(v) && Number.isSafeInteger(v) && v >= 0;
const close = (a: number, b: number) => Math.abs(a-b) <= 1e-10 * Math.max(1, Math.abs(a), Math.abs(b));
// A positive modeled charge must not disappear inside an absolute NAV tolerance.
const paidCost = (actual: number, expected: number) => expected === 0 ? actual === 0
  : actual > 0 && Math.abs(actual-expected) <= 1e-10 * Math.max(Math.abs(actual),Math.abs(expected));
// Only existing six-decimal prices/statistics need display-rounding tolerance.
const displayed = (a: number, b: number) => Math.abs(a-b) <= 2.01e-6 * Math.max(1, Math.abs(a), Math.abs(b));
const invalid = (): never => {throw new Error("The Bandy cash account response is missing or inconsistent. Results remain unavailable; reload after the account contract is restored.");};
const monthlyOrStop = (v: Fields) => ["stop", "initial_stop", "final_stop", "monthly_reference", "monthly_realized_losses",
  "outstanding_planned_risk", "monthly_halted", "planned_risk_fraction_of_month_reference", "risk_budget_fraction_of_month_reference"]
  .some(key => Object.hasOwn(v, key));

export function parseBandySourceAccount(data: unknown, id?: string, book?: string, selectedTicker?: string, admission?: unknown): BandySourceAccount {
  return parseCashAccount(data,id,book,selectedTicker,admission,false);
}
/** Private/offline contract checks only. Public sourceAccount always calls the
 * strict entry point above and can never enable a synthetic instrument. */
export function parseBandyAdapterContract(data: unknown, id: string, selectedTicker: string, admission: unknown): BandySourceAccount {
  if (id !== BANDY_SIMPLE_METHOD || !object(admission) || !object(admission.adapter_contract)
    || admission.adapter_contract.scope !== "independently_bound_reporting_fixture"
    || typeof admission.adapter_contract.case_id !== "string" || !admission.adapter_contract.case_id
    || !["input_sha256","request_sha256","capture_sha256"].every(key=>typeof (admission.adapter_contract as Fields)[key] === "string"
      && /^[a-f0-9]{64}$/.test((admission.adapter_contract as Fields)[key] as string))) return invalid();
  return parseCashAccount(data,id,"hbiab",selectedTicker,admission,true);
}
function parseCashAccount(data: unknown, id?: string, book?: string, selectedTicker?: string, admission?: unknown, adapter=false): BandySourceAccount {
  if (!bandyAccountMethod(id, book) || !object(data) || data.kind === "basket" || !object(data.source_account)) return invalid();
  if (data.id !== id || data.book !== book) return invalid();
  // The new teaching example cannot inherit the three older methods' basis or settings.
  // Existing methods deliberately retain their original contract and checks.
  if (id === BANDY_SIMPLE_METHOD) {
    if (data.kind !== undefined || data.model_initial_equity !== 1 || data.accounting_basis !== BANDY_SIMPLE_BASIS
      || data.source_account.basis !== BANDY_SIMPLE_BASIS || !object(admission)
      || !object(admission.definition) || admission.definition.id !== id || admission.definition.book !== book
      || !object(admission.definition.instrument_policy) || !object(admission.parameters) || !object(admission.instrument)) return invalid();
    const policy=admission.definition.instrument_policy, parameters=admission.parameters;
    if (!Array.isArray(policy.allowed_types) || policy.allowed_types.length !== 2
      || !policy.allowed_types.includes("individual_stock") || !policy.allowed_types.includes("etf")
      || policy.execution_scope !== "single_symbol_source_account" || policy.account_model !== BANDY_ACCOUNT_MODEL
      || policy.source_basis !== BANDY_SIMPLE_BASIS || policy.source_convention !== BANDY_SIMPLE_CONVENTION
      || policy.source_performance_reproduced !== false || policy.source_rule_reviewed !== true
      || policy.independent_selected_ledger_checked !== true || Object.keys(parameters).length !== 2
      || parameters.ma !== 25 || parameters.hold !== 5 || !selectedTicker) return invalid();
    const instrument=admission.instrument, metadata=instrument.metadata;
    if (!object(metadata) || metadata.ticker !== selectedTicker || cashIndices.has(selectedTicker)
      || !["individual_stock","etf",...(adapter ? ["synthetic_test_only"] : [])].includes(instrument.kind as string)) return invalid();
    if (instrument.kind === "individual_stock" && (instrument.classification_basis !== "native_stock_metadata" || metadata.asset_type !== "individual_stock")) return invalid();
    if (instrument.kind === "etf" && (instrument.classification_basis !== "existing_runtime_etf_selector"
      || selectedTicker.startsWith("basket:") || !Array.isArray(instrument.eligible_etf_tickers) || !instrument.eligible_etf_tickers.includes(selectedTicker)
      || (metadata.asset_type !== undefined && metadata.asset_type !== "etf"))) return invalid();
    if (instrument.kind === "synthetic_test_only" && (!adapter || instrument.classification_basis !== "independent_synthetic_fixture")) return invalid();
    if (selectedTicker.startsWith("basket:") && (object(data.provenance) ? data.provenance.asset_type !== "individual_stock" : !adapter)) return invalid();
    const symbol=typeof metadata.symbol === "string" ? metadata.symbol : selectedTicker.split(":").at(-1)!;
    if (!symbol || (data.ticker !== undefined && data.ticker !== selectedTicker && data.ticker !== symbol)) return invalid();
    const source=object(metadata.provenance) ? metadata.provenance : {};
    for (const origin of [metadata,source]) {
      if (origin.symbol !== undefined && origin.symbol !== symbol || origin.qualified_ticker !== undefined && origin.qualified_ticker !== selectedTicker) return invalid();
      if (origin.asset_type !== undefined && origin.asset_type !== instrument.kind) return invalid();
      if (object(data.provenance)) for (const key of ["symbol","qualified_ticker","asset_type","trusted_market","currency","source_sha256","csv_sha256"]) {
        if (origin[key] !== undefined && origin[key] !== data.provenance[key]) return invalid();
      }
    }
    if (object(data.provenance) && data.provenance.asset_type !== undefined && data.provenance.asset_type !== instrument.kind) return invalid();
    if (!Array.isArray(data.trades)) return invalid();
    for (const trade of data.trades) if (!object(trade) || !object(trade.accounting) || trade.accounting.symbol !== symbol
      || ["ticker","symbol"].some(key=>trade[key] !== undefined && trade[key] !== selectedTicker && trade[key] !== symbol)) return invalid();
  }
  const a=data.source_account, macd=id === "hbiab_macd_example", fraction=macd ? .25 : 1;
  if (a.model !== BANDY_ACCOUNT_MODEL || a.book_performance_reproduced !== false
    || typeof a.basis !== "string" || !a.basis.trim() || typeof a.account_contribution_basis !== "string" || !a.account_contribution_basis.trim()
    || !count(a.rejected_entries) || !object(a.sizing) || a.sizing.target_equity_fraction !== fraction
    || a.sizing.origin !== (macd ? "source_rule" : "declared_research_convention")
    || a.sizing.cash_reserved !== true || a.sizing.fractional_positions !== true
    || typeof a.terminal_entry_pending !== "boolean" || typeof a.terminal_exit_pending !== "boolean"
    || !Array.isArray(a.days) || !Array.isArray(a.trade_accounts)
    || !Array.isArray(data.dates) || !Array.isArray(data.equity) || !Array.isArray(data.gross_equity)
    || !Array.isArray(data.trades) || !Array.isArray(data.close) || !object(data.stats) || !object(data.gross_stats) || !object(data.costs)
    || data.dates.length < 1 || [a.days,data.equity,data.gross_equity,data.close].some(rows => rows.length !== (data.dates as unknown[]).length)
    || a.trade_accounts.length !== data.trades.length) return invalid();
  const provenance=object(data.provenance) ? data.provenance : null;
  if (selectedTicker?.startsWith("basket:") && (provenance ? provenance.qualified_ticker !== selectedTicker
    || typeof provenance.symbol !== "string" || !provenance.symbol.trim() : !adapter)) return invalid();
  if (selectedTicker && provenance && typeof provenance.qualified_ticker === "string" && provenance.qualified_ticker !== selectedTicker) return invalid();
  const expectedSymbol=provenance && typeof provenance.symbol === "string" ? provenance.symbol
    : selectedTicker && !selectedTicker.startsWith("basket:") ? selectedTicker : null;
  const costs=data.costs;
  if (!finite(costs.fees_bps) || !finite(costs.slippage_bps) || costs.fees_bps < 0 || costs.slippage_bps < 0
    || costs.fees_bps+costs.slippage_bps > 5000 || costs.rollover_bps !== 0
    || !finite(costs.total_side_bps) || !paidCost(costs.total_side_bps,costs.fees_bps+costs.slippage_bps)) return invalid();
  const side=(costs.fees_bps+costs.slippage_bps)/10000;
  const indices=new Map<string,number>(), numericDay=["equity","gross_same_quantity_equity","costs_paid","free_cash","quantity","position_value"];
  for (let i=0;i<a.days.length;i++) {
    const d: unknown=a.days[i];
    if (!object(d) || monthlyOrStop(d) || !dayString(d.date) || d.date !== data.dates[i] || !count(d.source_index)
      || (i > 0 && (!object(a.days[i-1]) || d.date <= (a.days[i-1] as Fields).date! || d.source_index !== ((a.days[i-1] as Fields).source_index as number)+1))
      || numericDay.some(key => !finite(d[key])) || !finite(data.close[i]) || data.close[i] <= 0
      || !finite(data.equity[i]) || !finite(data.gross_equity[i]) || d.equity !== data.equity[i] || d.gross_same_quantity_equity !== data.gross_equity[i]
      || (d.equity as number) <= 0 || (d.gross_same_quantity_equity as number) <= 0 || (d.costs_paid as number) < 0
      || (d.free_cash as number) < -1e-12 || (d.quantity as number) < 0 || (d.position_value as number) < 0
      || d.direction !== ((d.quantity as number) > 0 ? 1 : 0)
      || !close(d.equity as number,(d.free_cash as number)+(d.position_value as number))
      || !close((d.gross_same_quantity_equity as number)-(d.equity as number),d.costs_paid as number)
      || !displayed(d.position_value as number,(d.quantity as number)*data.close[i])) return invalid();
    indices.set(d.date,i);
  }
  const entries=new Map<number,BandyTradeAccount>(), exits=new Map<number,BandyTradeAccount>();
  const numericTrade=["raw_instrument_gross_return","entry_equity_units","entry_allocation_fraction","quantity","entry_notional_units",
    "gross_account_contribution","net_account_contribution","entry_cost_account_contribution","exit_cost_account_contribution",
    "gross_fraction_of_actual_entry_equity","net_fraction_of_actual_entry_equity","open_liquidation_cost_estimate"];
  let lastExit=-1, closed=0, sum=0, grossSum=0, netSum=0, openCount=0, ledgerSymbol="";
  for (let i=0;i<a.trade_accounts.length;i++) {
    const row: unknown=a.trade_accounts[i], trade: unknown=data.trades[i];
    if (!object(row) || monthlyOrStop(row) || !object(trade) || !object(trade.accounting)
      || !dayString(row.entry) || !dayString(row.exit) || !dayString(row.entry_signal_date)
      || !indices.has(row.entry) || !indices.has(row.exit) || row.entry !== trade.entry || row.exit !== trade.exit
      || typeof row.open !== "boolean" || row.open !== trade.open || row.entry > row.exit
      || numericTrade.some(key => !finite(row[key])) || (row.entry_equity_units as number) <= 0 || (row.quantity as number) <= 0
      || (row.entry_notional_units as number) <= 0 || (row.entry_allocation_fraction as number) <= 0 || (row.entry_allocation_fraction as number) > fraction+1e-12
      || row.entry_phase !== (macd ? "open" : "close") || row.exit_phase !== (row.open ? "mark" : macd ? "open" : "close")
      || (row.entry_signal_date as string) > row.entry || (macd ? row.entry_signal_date === row.entry : row.entry_signal_date !== row.entry)
      || !finite(trade.ret_full_precision) || !close(row.net_fraction_of_actual_entry_equity as number,trade.ret_full_precision)
      || trade.dir !== 1 || trade.entry_kind !== row.entry_phase || !Array.isArray(trade.legs) || trade.legs.length !== 1
      || !Array.isArray(trade.legs[0]) || !finite(trade.legs[0][1]) || trade.legs[0][1] <= 0 || !finite(trade.exit_px) || trade.exit_px <= 0
      || ["gross_pnl","net_pnl","taxes","fees","slippage","mae","mfe"].some(key => (trade.accounting as Fields)[key] !== null)
      || (row.entry_cost_account_contribution as number) < 0 || (row.exit_cost_account_contribution as number) < 0 || (row.open_liquidation_cost_estimate as number) < 0) return invalid();
    const t=row as unknown as BandyTradeAccount, ti=indices.get(t.entry)!, xi=indices.get(t.exit)!, price=trade.legs[0][1], exitPrice=trade.exit_px;
    if (ti <= lastExit || entries.has(ti) || (!t.open && exits.has(xi))
      || trade.legs[0][0] !== ti || !finite(trade.legs[0][2]) || !close(trade.legs[0][2],t.entry_allocation_fraction)
      || trade.exit_i !== xi || trade.signal !== (t.open ? t.exit : t.exit_signal_date)
      || (macd && indices.get(t.entry_signal_date) !== ti-1)
      || typeof trade.accounting.symbol !== "string" || !trade.accounting.symbol.trim()
      || (expectedSymbol !== null && trade.accounting.symbol !== expectedSymbol)
      || (ledgerSymbol !== "" && trade.accounting.symbol !== ledgerSymbol)
      || trade.accounting.signal_side !== "Long"
      || !finite(trade.accounting.pts) || !close(trade.accounting.pts,exitPrice-price)
      || trade.accounting.days_held !== (Date.parse(t.exit)-Date.parse(t.entry))/86400000
      || trade.accounting.cost_status !== (t.open ? "actual_entry_cost_only_open_mark" : "modeled_filled_side_costs")
      // shared fill-aware MAE/MFE (Rust accounting::excursion): either unavailable, or finite with MAE <= MFE
      || !((trade.accounting.excursion_bars === 0 && trade.accounting.excursion_status === 'unavailable' && trade.accounting.mae_ret_observed === null && trade.accounting.mfe_ret_observed === null) || (Number.isInteger(trade.accounting.excursion_bars) && (trade.accounting.excursion_bars as number) > 0 && trade.accounting.excursion_status === 'fill_aware_ohlc_path' && Number.isFinite(trade.accounting.mae_ret_observed) && Number.isFinite(trade.accounting.mfe_ret_observed) && (trade.accounting.mae_ret_observed as number) <= (trade.accounting.mfe_ret_observed as number)))) return invalid();
    lastExit=xi;ledgerSymbol=trade.accounting.symbol;
    if (t.open) {
      openCount++;
      if (i !== a.trade_accounts.length-1 || xi !== a.days.length-1 || t.exit_notional_units !== null || t.exit_signal_date !== null
        || t.exit_cost_account_contribution !== 0 || !paidCost(t.open_liquidation_cost_estimate,t.quantity*exitPrice*side)) return invalid();
    } else {
      closed++;sum+=t.net_fraction_of_actual_entry_equity;
      if (!dayString(t.exit_signal_date) || t.exit_signal_date < t.entry_signal_date || t.exit_signal_date > t.exit
        || (macd ? indices.get(t.exit_signal_date) !== xi-1 : t.exit_signal_date !== t.exit)
        || !finite(t.exit_notional_units) || t.exit_notional_units <= 0 || !close(t.exit_notional_units,t.quantity*exitPrice)
        || !paidCost(t.exit_cost_account_contribution,t.exit_notional_units*side) || t.open_liquidation_cost_estimate !== 0) return invalid();
      exits.set(xi,t);
    }
    const account=trade.accounting;
    const paidNotional=t.entry_notional_units+(t.exit_notional_units || 0);
    if (!close(t.entry_notional_units,t.quantity*price) || !close(t.entry_allocation_fraction,t.entry_notional_units/t.entry_equity_units)
      || !close(t.entry_allocation_fraction,macd ? .25 : 1/(1+side))
      || !paidCost(t.entry_cost_account_contribution,t.entry_notional_units*side)
      || !close(t.raw_instrument_gross_return,exitPrice/price-1)
      || !close(t.gross_account_contribution,t.quantity*(exitPrice-price))
      || !close(t.gross_account_contribution-t.entry_cost_account_contribution-t.exit_cost_account_contribution,t.net_account_contribution)
      || !close(t.gross_fraction_of_actual_entry_equity,t.gross_account_contribution/t.entry_equity_units)
      || !close(t.net_fraction_of_actual_entry_equity,t.net_account_contribution/t.entry_equity_units)
      || !finite(account.gross_ret) || !finite(account.net_ret) || !finite(account.fee_ret) || !finite(account.slippage_ret)
      || !close(account.gross_ret,t.gross_fraction_of_actual_entry_equity) || !close(account.net_ret,t.net_fraction_of_actual_entry_equity)
      || !paidCost(account.fee_ret,paidNotional*costs.fees_bps/10000/t.entry_equity_units)
      || !paidCost(account.slippage_ret,paidNotional*costs.slippage_bps/10000/t.entry_equity_units)) return invalid();
    grossSum+=t.gross_account_contribution;netSum+=t.net_account_contribution;entries.set(ti,t);
  }
  let cash=1,paid=0,quantity=0;
  for (let i=0;i<a.days.length;i++) {
    const d=a.days[i] as unknown as BandyAccountDay, enter=entries.get(i), exit=exits.get(i);
    if (exit) {cash+=exit.exit_notional_units!-exit.exit_cost_account_contribution;paid+=exit.exit_cost_account_contribution;quantity=0;}
    if (enter) {
      if (quantity !== 0 || !close(enter.entry_equity_units,cash)) return invalid();
      cash-=enter.entry_notional_units+enter.entry_cost_account_contribution;paid+=enter.entry_cost_account_contribution;quantity=enter.quantity;
    }
    if (!close(cash,d.free_cash) || !paidCost(d.costs_paid,paid) || !close(quantity,d.quantity)) return invalid();
  }
  const pending=a.pending_order;
  if (pending === null) {if (a.terminal_entry_pending || a.terminal_exit_pending) return invalid();}
  else if (!macd || !object(pending) || !["buy","sell"].includes(pending.side as string)
    || pending.fill_phase !== "open" || pending.filled !== false || pending.signal_date !== data.dates.at(-1)
    || a.terminal_entry_pending !== (pending.side === "buy") || a.terminal_exit_pending !== (pending.side === "sell")
    || (pending.side === "sell" ? openCount !== 1 : openCount !== 0)) return invalid();
  const final=a.days.at(-1) as unknown as BandyAccountDay;
  if (openCount > 1 || !finite(data.stats.total_return) || !displayed(final.equity-1,data.stats.total_return)
    || !finite(data.gross_stats.total_return) || !displayed(final.gross_same_quantity_equity-1,data.gross_stats.total_return)
    || data.stats.trades !== closed || !finite(data.stats.net_profit) || !displayed(data.stats.net_profit,sum)
    || !finite(data.stats.closed_return_sum) || !displayed(data.stats.closed_return_sum,sum)
    || (closed > 0 && (!finite(data.stats.avg_net_pnl) || !displayed(data.stats.avg_net_pnl,sum/closed)))
    || !close(grossSum,final.gross_same_quantity_equity-1) || !close(netSum,final.equity-1)) return invalid();
  return a as unknown as BandySourceAccount;
}
