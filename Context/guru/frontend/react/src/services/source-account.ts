import {BANDY_ACCOUNT_MODEL,bandyAccountMethod,parseBandySourceAccount,type BandySourceAccount} from "./bandy-source-account";
export {BANDY_ACCOUNT_MODEL,bandyAccountMethod} from "./bandy-source-account";
/** Preserve the protected Elder contract and the distinct Bandy cash contract.
 * Unknown or stale payloads never fall back to the legacy unit-exposure presenter. */
export const ELDER_ACCOUNT_MODEL = "elder_protected_impulse_fractional_cash_v1";
export const ELDER_METHOD = "elcim_impulse";

export interface SourceAccountDay {
  date: string;
  equity: number;
  gross_same_quantity_equity: number;
  costs_paid: number;
  free_cash: number;
  reserved_short_principal: number;
  restricted_short_proceeds: number;
  direction: -1 | 0 | 1;
  stop: number | null;
  monthly_reference: number;
  monthly_realized_losses: number;
  outstanding_planned_risk: number;
  monthly_halted: boolean;
  liquidation_pending: boolean;
}
export interface SourceTradeAccount {
  entry: string;
  exit: string;
  open: boolean;
  raw_instrument_gross_return: number;
  entry_equity_units: number;
  entry_allocation_fraction: number;
  initial_stop: number;
  final_stop: number;
  planned_risk_fraction_of_month_reference: number;
  risk_budget_fraction_of_month_reference: number;
  gross_account_contribution: number;
  net_account_contribution: number;
  entry_cost_account_contribution: number;
  exit_cost_account_contribution: number;
  gross_fraction_of_actual_entry_equity: number;
  net_fraction_of_actual_entry_equity: number;
}
export interface ElderSourceAccount {
  model: typeof ELDER_ACCOUNT_MODEL;
  book_performance_reproduced: false;
  basis: string;
  account_contribution_basis: string;
  rejected_entries: number;
  terminal_liquidation_pending: boolean;
  days: SourceAccountDay[];
  trade_accounts: SourceTradeAccount[];
}
export type SourceAccount = ElderSourceAccount | BandySourceAccount;
interface SourceAccountViewCore {
  basis: string;
  actualReturn: number;
  grossReturn: number;
  closedTradeCount: number;
}
export type SourceAccountView = SourceAccountViewCore & (
  {model: typeof ELDER_ACCOUNT_MODEL} | {model: typeof BANDY_ACCOUNT_MODEL; sizing: BandySourceAccount["sizing"]}
);
type Fields = Record<string, unknown>;
const object = (v: unknown): v is Fields => Boolean(v) && typeof v === "object" && !Array.isArray(v);
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const date = (v: unknown): v is string => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v);
const count = (v: unknown): v is number => finite(v) && Number.isInteger(v) && v >= 0;
const near = (a: number, b: number, tolerance = 2.01e-6) => Math.abs(a - b) <= tolerance * Math.max(1, Math.abs(a), Math.abs(b));
const invalid = (): never => { throw new Error("The protected Elder account response is missing or inconsistent. Results remain unavailable; reload after the account contract is restored."); };

export function sourceAccount(data: unknown, id?: string, book?: string, selectedTicker?: string): SourceAccount | null {
  if (bandyAccountMethod(id,book) || (object(data) && object(data.source_account) && data.source_account.model === BANDY_ACCOUNT_MODEL)) return parseBandySourceAccount(data,id,book,selectedTicker);
  return elderSourceAccount(data,id,book);
}
function elderSourceAccount(data: unknown, id?: string, book?: string): ElderSourceAccount | null {
  const expected = id === ELDER_METHOD && book === "elcim";
  if (!object(data)) return expected ? invalid() : null;
  if (!expected && data.source_account == null) return null;
  if (!expected || !object(data.source_account) || data.kind === "basket") return invalid();
  const account = data.source_account;
  if (account.model !== ELDER_ACCOUNT_MODEL || account.book_performance_reproduced !== false
    || typeof account.basis !== "string" || !account.basis.trim()
    || typeof account.account_contribution_basis !== "string"
    || !count(account.rejected_entries) || typeof account.terminal_liquidation_pending !== "boolean"
    || !Array.isArray(account.days) || !Array.isArray(account.trade_accounts)
    || !Array.isArray(data.dates) || !Array.isArray(data.equity) || !Array.isArray(data.gross_equity)
    || !Array.isArray(data.trades) || !object(data.stats) || !finite(data.stats.total_return)
    || data.dates.length < 2 || account.days.length !== data.dates.length
    || data.equity.length !== data.dates.length || data.gross_equity.length !== data.dates.length
    || account.trade_accounts.length !== data.trades.length) return invalid();
  const daily = ["equity", "gross_same_quantity_equity", "costs_paid", "free_cash", "reserved_short_principal",
    "restricted_short_proceeds", "monthly_reference", "monthly_realized_losses", "outstanding_planned_risk"];
  for (let i = 0; i < account.days.length; i++) {
    const day: unknown = account.days[i];
    if (!object(day) || !date(day.date) || day.date !== data.dates[i]
      || daily.some(k => !finite(day[k])) || !finite(data.equity[i]) || !finite(data.gross_equity[i])
      || day.equity !== data.equity[i] || day.gross_same_quantity_equity !== data.gross_equity[i]
      || (day.equity as number) <= 0 || (day.monthly_reference as number) <= 0
      || ![-1, 0, 1].includes(day.direction as number)
      || (day.direction === 0 ? day.stop !== null : !finite(day.stop))
      || typeof day.monthly_halted !== "boolean" || typeof day.liquidation_pending !== "boolean"
      || !near((day.gross_same_quantity_equity as number) - (day.equity as number), day.costs_paid as number)) return invalid();
  }
  const numeric = ["raw_instrument_gross_return", "entry_equity_units", "entry_allocation_fraction", "initial_stop", "final_stop",
    "planned_risk_fraction_of_month_reference", "risk_budget_fraction_of_month_reference", "gross_account_contribution",
    "net_account_contribution", "entry_cost_account_contribution", "exit_cost_account_contribution",
    "gross_fraction_of_actual_entry_equity", "net_fraction_of_actual_entry_equity"];
  for (let i = 0; i < account.trade_accounts.length; i++) {
    const row: unknown = account.trade_accounts[i], trade: unknown = data.trades[i];
    if (!object(row) || !object(trade) || !object(trade.accounting) || !date(row.entry) || !date(row.exit)
      || row.entry !== trade.entry || row.exit !== trade.exit || typeof row.open !== "boolean" || row.open !== trade.open
      || numeric.some(k => !finite(row[k])) || (row.entry_equity_units as number) <= 0
      || (row.entry_allocation_fraction as number) <= 0 || (row.entry_allocation_fraction as number) > 1.000001
      || (row.open && row.exit_cost_account_contribution !== 0)
      || ["gross_pnl", "net_pnl", "taxes", "fees", "slippage"].some(k => (trade.accounting as Fields)[k] != null)
      || !finite(trade.ret_full_precision) || !near(row.net_fraction_of_actual_entry_equity as number, trade.ret_full_precision)
      || !near((row.gross_account_contribution as number) - (row.entry_cost_account_contribution as number)
        - (row.exit_cost_account_contribution as number), row.net_account_contribution as number)) return invalid();
  }
  const finalEquity = data.equity.at(-1) as number;
  if (!near(finalEquity - 1, data.stats.total_return)) return invalid();
  return account as unknown as ElderSourceAccount;
}

export function sourceAccountView(account: SourceAccount): SourceAccountView {
  const view = {model: account.model, basis: account.basis, actualReturn: account.days.at(-1)!.equity - 1,
    grossReturn: account.days.at(-1)!.gross_same_quantity_equity - 1,
    closedTradeCount: account.trade_accounts.filter(t => !t.open).length};
  return account.model === BANDY_ACCOUNT_MODEL ? {...view,model:account.model,sizing:account.sizing} : {...view,model:account.model};
}

export const sourceAuditHeaders = ["Entry", "Exit / mark", "Open", "Raw instrument gross return", "Entry allocation fraction",
  "Gross / actual entry equity", "Net / actual entry equity (unrounded)", "Gross account contribution (initial unit)",
  "Net account contribution (initial unit)", "Entry costs (initial unit)", "Exit costs (initial unit)",
  "Planned risk / monthly reference", "Risk budget / monthly reference", "Initial stop", "Last known stop"];
export function sourceTradeHeaders(headers: readonly string[]): string[] {
  return headers.map((header, i) => ({8: "Gross return / actual entry equity", 11: "Slippage / actual entry equity",
    13: "Net return / actual entry equity", 14: "Observed adverse / actual entry equity",
    15: "Observed favourable / actual entry equity"}[i] || header));
}
export function sourceAuditRows(account: SourceAccount, fullPrecisionNetReturns: readonly number[]): (string | number | boolean | null)[][] {
  if (fullPrecisionNetReturns.length !== account.trade_accounts.length || fullPrecisionNetReturns.some(v => !finite(v))) return invalid();
  if (account.model === BANDY_ACCOUNT_MODEL) return account.trade_accounts.map((t,i) => [t.entry,t.open ? "Open mark: " + t.exit : t.exit,t.open,t.raw_instrument_gross_return,
    t.entry_allocation_fraction,t.gross_fraction_of_actual_entry_equity,fullPrecisionNetReturns[i],t.gross_account_contribution,
    t.net_account_contribution,t.entry_cost_account_contribution,t.exit_cost_account_contribution,t.entry_notional_units,
    t.exit_notional_units,t.quantity,t.entry_equity_units,t.entry_signal_date,t.exit_signal_date,t.entry_phase,t.exit_phase,t.open_liquidation_cost_estimate]);
  return account.trade_accounts.map((t, i) => [t.entry, t.open ? "Open mark: " + t.exit : t.exit, t.open, t.raw_instrument_gross_return,
    t.entry_allocation_fraction, t.gross_fraction_of_actual_entry_equity, fullPrecisionNetReturns[i],
    t.gross_account_contribution, t.net_account_contribution, t.entry_cost_account_contribution,
    t.exit_cost_account_contribution, t.planned_risk_fraction_of_month_reference,
    t.risk_budget_fraction_of_month_reference, t.initial_stop, t.final_stop]);
}
export const sourceRiskHeaders = ["Date", "Net account equity (initial unit)", "Gross same-quantity equity (initial unit)",
  "Costs paid (initial unit)", "Free cash (initial unit)", "Reserved short principal (initial unit)", "Restricted short proceeds (initial unit)",
  "Direction", "Known stop", "Monthly reference (initial unit)", "Monthly realized losses (initial unit)",
  "Outstanding planned risk (initial unit)", "Monthly halted", "Liquidation pending"];
export function sourceRiskRows(account: SourceAccount): (string | number | boolean | null)[][] {
  if (account.model === BANDY_ACCOUNT_MODEL) return account.days.map(d => [d.date,d.equity,d.gross_same_quantity_equity,d.costs_paid,d.free_cash,d.quantity,d.position_value,d.direction,d.source_index]);
  return account.days.map(d => [d.date, d.equity, d.gross_same_quantity_equity, d.costs_paid, d.free_cash,
    d.reserved_short_principal, d.restricted_short_proceeds, d.direction, d.stop, d.monthly_reference,
    d.monthly_realized_losses, d.outstanding_planned_risk, d.monthly_halted, d.liquidation_pending]);
}

export const bandyAuditHeaders = [...sourceAuditHeaders.slice(0,11), "Entry notional (initial unit)", "Filled exit notional (initial unit)",
  "Normalized fractional position units", "Actual entry equity (initial unit)", "Entry signal date", "Exit signal date", "Entry phase", "Exit / mark phase", "Uncharged open liquidation estimate (initial unit)"];
export const bandyDailyHeaders = ["Date", "Net cash-account equity (initial unit)", "Gross same-quantity equity (initial unit)",
  "Costs paid (initial unit)", "Free cash (initial unit)", "Normalized fractional position units", "Position value (initial unit)", "Direction", "Source index"];
export function sourceAccountTables(account: SourceAccount, returns: readonly number[]) {
  const bandy=account.model === BANDY_ACCOUNT_MODEL;
  return {
    audit:{name:"Account audit",headers:bandy ? bandyAuditHeaders : sourceAuditHeaders,rows:sourceAuditRows(account,returns),
      percentColumns:bandy ? [3,4,5,6] : [3,4,5,6,11,12],numericColumns:bandy ? [7,8,9,10,11,12,13,14,19] : [7,8,9,10,13,14]},
    daily:{name:bandy ? "Account daily" : "Account risk",headers:bandy ? bandyDailyHeaders : sourceRiskHeaders,rows:sourceRiskRows(account),
      numericColumns:bandy ? [1,2,3,4,5,6] : [1,2,3,4,5,6,8,9,10,11]},
  };
}
export function sourceAccountCopy(account: {model: SourceAccount["model"]; sizing?: BandySourceAccount["sizing"]}) {
  const bandy=account.model === BANDY_ACCOUNT_MODEL;
  const sourceQuarter=bandy && account.sizing?.origin === "source_rule";
  return {
    bandy,
    accountReturn:bandy ? "Cash-account return" : "Risk-sized account return",
    accountEquity:bandy ? "Cash-account equity" : "Risk-sized account equity",
    decline:bandy ? "Cash-account decline" : "Risk-sized account decline",
    sizing:bandy ? sourceQuarter ? "Source allocation: 25% of current equity; remaining cash stays idle."
      : "Declared research convention: 100% target allocation with entry costs reserved in cash. The MA examples do not specify position sizing."
      : "Protection and the 2% per-trade / 6% monthly risk limits stay active.",
    settingsNote:bandy ? "Display only. The reviewed fixed allocation, actual cash account, equity and risk statistics stay unchanged. Open holdings are excluded from this closed-return display."
      : "Display only. The mandatory 2%/6% risk controller, actual cash account, equity and risk statistics stay unchanged. Open holdings are excluded from this closed-return display.",
    displayNote:bandy ? " Open holdings are excluded. This display never changes the reviewed allocation, actual cash account or its risk statistics."
      : " Open holdings are excluded. This display never changes the source 2%/6% sizing, protection, actual cash account or its risk statistics.",
    curveNote:bandy ? "The separate cash-account curve follows actual cash and position marks, including paid entry costs on open holdings. Unfilled exits and separate hypothetical liquidation estimates are not deducted. Its total return is not the arithmetic closed-return sum."
      : "The separate risk-sized account curve follows the actual cash/risk ledger, including open price marks and actual entry costs. Its total return is not the arithmetic closed-return sum.",
    tradeNote:bandy ? "Gross/net returns and paid slippage are fractions of actual net entry equity, weighted by the accepted cash allocation. Raw instrument returns, actual notionals and initial-unit contributions are separate in the Account audit Excel sheet. Open marks pay entry costs only; hypothetical liquidation estimates are separate."
      : "Gross/net returns and slippage are fractions of actual net entry equity, weighted by the accepted risk allocation. Raw instrument returns and initial-unit contributions are separate in the Account audit Excel sheet. Open marks pay entry costs only.",
    exportCurve:bandy ? "Cash plus actual accepted position marks, with paid entry costs only on open holdings."
      : "Cash plus actual risk-sized position marks, with entry costs only on open holdings.",
    exportDisplay:bandy ? "Closed sum / hypothetical closed compounding never changes reviewed fixed sizing or the accepted ledger."
      : "Closed sum / hypothetical closed compounding never changes source 2%/6% sizing or the accepted ledger.",
  };
}
