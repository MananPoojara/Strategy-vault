/** Distinct cash-account contract. These checks validate reporting consistency;
 * they do not certify book figures, vendor prices or source performance. */
export const BANDY_ACCOUNT_MODEL = "bandy_fixed_fraction_fractional_cash_v1";
export const BANDY_ACCOUNT_METHODS = ["hbiab_ma5_20", "hbiab_ma7_4", "hbiab_macd_example"] as const;
export const bandyAccountMethod = (id?: string, book?: string) => book === "hbiab"
  && BANDY_ACCOUNT_METHODS.some(method => method === id);
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
// Only existing six-decimal prices/statistics need display-rounding tolerance.
const displayed = (a: number, b: number) => Math.abs(a-b) <= 2.01e-6 * Math.max(1, Math.abs(a), Math.abs(b));
const invalid = (): never => {throw new Error("The Bandy cash account response is missing or inconsistent. Results remain unavailable; reload after the account contract is restored.");};
const monthlyOrStop = (v: Fields) => ["stop", "initial_stop", "final_stop", "monthly_reference", "monthly_realized_losses",
  "outstanding_planned_risk", "monthly_halted", "planned_risk_fraction_of_month_reference", "risk_budget_fraction_of_month_reference"]
  .some(key => Object.hasOwn(v, key));

export function parseBandySourceAccount(data: unknown, id?: string, book?: string): BandySourceAccount {
  if (!bandyAccountMethod(id, book) || !object(data) || data.kind === "basket" || !object(data.source_account)) return invalid();
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
    || data.dates.length < 2 || [a.days,data.equity,data.gross_equity,data.close].some(rows => rows.length !== (data.dates as unknown[]).length)
    || a.trade_accounts.length !== data.trades.length) return invalid();
  const costs=data.costs;
  if (!finite(costs.fees_bps) || !finite(costs.slippage_bps) || costs.fees_bps < 0 || costs.slippage_bps < 0
    || costs.fees_bps+costs.slippage_bps > 5000 || costs.rollover_bps !== 0
    || costs.total_side_bps !== costs.fees_bps+costs.slippage_bps) return invalid();
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
  let lastExit=-1, closed=0, sum=0, grossSum=0, netSum=0, openCount=0;
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
      || (typeof data.benchmark_symbol === "string" && trade.accounting.symbol !== data.benchmark_symbol)
      || (object(data.provenance) && typeof data.provenance.symbol === "string" && trade.accounting.symbol !== data.provenance.symbol)
      || trade.accounting.signal_side !== "Long"
      || trade.accounting.cost_status !== (t.open ? "actual_entry_cost_only_open_mark" : "modeled_filled_side_costs")
      || trade.accounting.excursion_bars !== 0 || trade.accounting.excursion_status !== "unavailable"
      || trade.accounting.mae_ret_observed !== null || trade.accounting.mfe_ret_observed !== null) return invalid();
    lastExit=xi;
    if (t.open) {
      openCount++;
      if (i !== a.trade_accounts.length-1 || xi !== a.days.length-1 || t.exit_notional_units !== null || t.exit_signal_date !== null
        || t.exit_cost_account_contribution !== 0 || !close(t.open_liquidation_cost_estimate,t.quantity*exitPrice*side)) return invalid();
    } else {
      closed++;sum+=t.net_fraction_of_actual_entry_equity;
      if (!dayString(t.exit_signal_date) || t.exit_signal_date < t.entry_signal_date || t.exit_signal_date > t.exit
        || (macd ? indices.get(t.exit_signal_date) !== xi-1 : t.exit_signal_date !== t.exit)
        || !finite(t.exit_notional_units) || t.exit_notional_units <= 0 || !close(t.exit_notional_units,t.quantity*exitPrice)
        || !close(t.exit_cost_account_contribution,t.exit_notional_units*side) || t.open_liquidation_cost_estimate !== 0) return invalid();
      exits.set(xi,t);
    }
    const account=trade.accounting;
    const paidNotional=t.entry_notional_units+(t.exit_notional_units || 0);
    if (!close(t.entry_notional_units,t.quantity*price) || !close(t.entry_allocation_fraction,t.entry_notional_units/t.entry_equity_units)
      || !close(t.entry_allocation_fraction,macd ? .25 : 1/(1+side))
      || !close(t.entry_cost_account_contribution,t.entry_notional_units*side)
      || !close(t.raw_instrument_gross_return,exitPrice/price-1)
      || !close(t.gross_account_contribution,t.quantity*(exitPrice-price))
      || !close(t.gross_account_contribution-t.entry_cost_account_contribution-t.exit_cost_account_contribution,t.net_account_contribution)
      || !close(t.gross_fraction_of_actual_entry_equity,t.gross_account_contribution/t.entry_equity_units)
      || !close(t.net_fraction_of_actual_entry_equity,t.net_account_contribution/t.entry_equity_units)
      || !finite(account.gross_ret) || !finite(account.net_ret) || !finite(account.fee_ret) || !finite(account.slippage_ret)
      || !close(account.gross_ret,t.gross_fraction_of_actual_entry_equity) || !close(account.net_ret,t.net_fraction_of_actual_entry_equity)
      || !close(account.fee_ret,paidNotional*costs.fees_bps/10000/t.entry_equity_units)
      || !close(account.slippage_ret,paidNotional*costs.slippage_bps/10000/t.entry_equity_units)) return invalid();
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
    if (!close(cash,d.free_cash) || !close(paid,d.costs_paid) || !close(quantity,d.quantity)) return invalid();
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
