import {
  isValidatedPristinePair, type PristineView, type PristineReport,
  type PristineStatMap, type PristineSpan,
} from './pristine-report';

export type PristineCell = string | number | boolean | null;
export interface PristineTable {
  readonly name: string;
  readonly headers: readonly string[];
  readonly rows: readonly (readonly PristineCell[])[];
  readonly percentColumns: readonly number[];
}
export interface PristineStrategySnapshot {
  readonly data: unknown | null;
  readonly view: PristineView | null;
  readonly definition: {
    readonly id: string; readonly book: string; readonly name?: string;
    readonly family_name?: string; readonly rules?: readonly string[];
    readonly notes?: string;
  };
  readonly selection: Readonly<Record<string, string>>;
  readonly stocks: readonly {readonly ticker: string; readonly symbol: string}[];
  readonly availability: {readonly status: string; readonly note: string} | null;
  readonly optimization: boolean;
}
export const PRISTINE_CLOSED_BASIS = 'Closed trades only. Closed sum and compounded closed return are the audited engine fields; terminal-open marks and their estimated exit side are excluded.';
export const PRISTINE_VISUAL_BASIS = 'Charts and calendar returns are visual summaries of recorded model values. Full-precision engine statistics and exact source audits remain separate.';
export const PRISTINE_COST_BASIS = 'Costs are normalized return deductions on deployed entry allocation. Closed trades pay two sides; a terminal-open mark contains one paid entry side and one separately estimated exit side, without an exit transaction.';
export const pristineNumber = (value: number | null | undefined, percent = false) => value == null || !Number.isFinite(value)
  ? '—' : percent ? (Number.isFinite(value * 100) ? `${(value * 100).toFixed(2)}%` : 'Not available (display overflow)') : value.toLocaleString('en-US', {maximumFractionDigits: 8});
function paired(data: unknown, view: PristineView): PristineReport {
  if (!isValidatedPristinePair(data, view) || Reflect.ownKeys(data as object).length !== 1 || !Object.hasOwn(data as object,'pristine_report')) throw Error('A paired, protected sole-model Pristine response is required.');
  return view.report;
}
const table = (name: string, headers: string[], rows: PristineCell[][], percentColumns: number[] = []): PristineTable => Object.freeze({
  name, headers:Object.freeze(headers),rows:Object.freeze(rows.map(row=>Object.freeze(row))),percentColumns:Object.freeze(percentColumns),
});
const exactText = (value: {negative:boolean;numerator_hex:string;denominator_hex:string} | null) => value === null ? null : `${value.negative?'-':''}0x${value.numerator_hex}/0x${value.denominator_hex}`;
const datesFor = (report: PristineReport, span: PristineSpan) => ({entry:report.rows[span.entry_i-report.from_source_index].date,exit:report.rows[span.exit_i-report.from_source_index].date});
/** Private visual-only helper; copied-module test alias does not grant admission. */
function pristineVisualPeriods(dates: readonly string[], length: number, values: readonly number[]) {
    const result: Record<string,number|null> = {}, unavailable: Record<string,string> = {};
    let base=1,key=dates[0].slice(0,length);
    const record=(last:number)=>{const ratio=last/base;if(!Number.isFinite(ratio)||ratio===0){result[key]=null;unavailable[key]=ratio===0?'Visual ratio underflow; engine statistics and recorded endpoints are unchanged.':'Visual ratio overflow; engine statistics and recorded endpoints are unchanged.';}else result[key]=ratio-1;};
    dates.forEach((date,i)=>{const next=date.slice(0,length);if(next!==key){record(values[i-1]);base=values[i-1];key=next;}});
    record(values.at(-1)!);return {values:Object.freeze(result),unavailable:Object.freeze(unavailable)};
}

/** All arrays here are view models, never a legacy Run or a new return engine. */
export function pristinePresentation(data: unknown, view: PristineView) {
  const report = paired(data,view);
  const dates = report.days.map(day=>day.date), equity=report.days.map(day=>day.equity), gross=report.days.map(day=>day.gross_equity), benchmark=report.days.map(day=>day.benchmark_equity);
  let peak = 1;
  const drawdown = equity.map(value=>{peak=Math.max(peak,value);return value/peak-1;});

  const months=pristineVisualPeriods(dates,7,equity),years=pristineVisualPeriods(dates,4,equity),benchmarkYears=pristineVisualPeriods(dates,4,benchmark);
  const trades = report.spans.map((span,i)=>{
    const days=datesFor(report,span),trade=report.trade_returns[i],extra=report.trade_supplements[i];
    return Object.freeze({index:i,span,trade,extra,...days,outcome:span.open?'open':trade.net_return>0?'win':'nonwin'} as const);
  });
  return Object.freeze({report,dates:Object.freeze(dates),equity:Object.freeze(equity),gross:Object.freeze(gross),benchmark:Object.freeze(benchmark),drawdown:Object.freeze(drawdown),
    months:months.values,years:years.values,benchmarkYears:benchmarkYears.values,visualPeriodReasons:Object.freeze({months:months.unavailable,years:years.unavailable,benchmarkYears:benchmarkYears.unavailable}),trades:Object.freeze(trades),
    closedSum:report.closed_stats.net.closed_return_sum,closedCompound:report.closed_stats.net.closed_compound_return,
    benchmarkLabel:`${report.symbol} buy & hold`,basis:report.basis});
}
export type PristinePresentation = ReturnType<typeof pristinePresentation>;
export function pristineTradeTable(data: unknown, view: PristineView): PristineTable {
  const report=paired(data,view);
  return table('Positions', ['Symbol','Signal date','Entry date','Entry phase','Entry price','Exit / mark date','Exit / mark phase','Exit / mark price','Open','Net return','Gross return','Fee deduction','Slippage deduction','Calendar days held','Observed sessions','Paid sides','Estimated exit sides','Net entry allocation (unit)','Net contribution (unit)'],
    report.spans.map((span,i)=>{const days=datesFor(report,span),t=report.trade_returns[i],x=report.trade_supplements[i];return [report.symbol,report.rows[span.signal_i-report.from_source_index].date,days.entry,span.entry_phase,span.entry_price,days.exit,span.exit_or_mark_phase,span.exit_or_mark_price,span.open,t.net_return,t.gross_return,t.fee_return,t.slippage_return,x.calendar_days_held,x.observed_exposure_sessions,x.paid_sides,x.estimated_exit_sides,t.entry_allocation,t.net_contribution];}),[9,10,11,12]);
}
/** Full raw values are exported; formatting never feeds a return calculation. */
export function pristineWorkbook(data: unknown, view: PristineView): readonly PristineTable[] {
  const r=paired(data,view),provenance=JSON.parse(r.provenance_json) as Record<string,unknown>;
  const statRows:PristineCell[][]=[];
  for(const [label,map] of Object.entries(r.statistics) as [string,PristineStatMap<string>][])for(const [key,value] of Object.entries(map.values)) statRows.push([label,key,value,map.display_values[key],map.null_reasons[key]||null]);
  const metadata:PristineCell[][]=[['Model',r.model],['Convention',r.convention],['Method',r.id],['Book',r.book],['Research mode',r.research_mode],['Qualified ticker',r.qualified_ticker],['Actual stock',r.symbol],['Benchmark',`${r.symbol} buy & hold`],['Currency label','USD-labelled supplied series units; historical-dollar equivalence unverified'],['Model units','Dimensionless initial1; no funded capital, share quantity or taxes'],['Price basis',r.basis],['Cost basis',r.costs.basis],['Closed display',PRISTINE_CLOSED_BASIS],['Visual calculations',PRISTINE_VISUAL_BASIS],['Original performance reproduced',false],['Native platform parity',false],['Native historical dollars verified',false],['Historical membership verified',false],['Source rows',r.source_row_count],['Relevant source prefix rows',r.source_prefix_row_count],['Volume column present',r.volume_column_present],['Fees bps per side',r.costs.fees_bps],['Slippage bps per side',r.costs.slippage_bps],['Rollover bps',r.costs.rollover_bps],['Original provenance JSON',r.provenance_json]];
  for(const [key,value]of Object.entries(provenance))metadata.push([`Provenance: ${key}`,value===null||['string','number','boolean'].includes(typeof value)?value as PristineCell:JSON.stringify(value)]);
  for(const [label,summary]of [['net',r.closed_stats.net],['gross',r.closed_stats.gross]]as const)for(const [key,value]of Object.entries(summary))metadata.push([`Closed ${label}: ${key}`,value]);
  metadata.push(['Closed summary: net_max_drawdown',r.closed_stats.net_max_drawdown],['Closed summary: gross_max_drawdown',r.closed_stats.gross_max_drawdown],['Closed summary: observed_exposure_sessions',r.closed_stats.observed_exposure_sessions],['Closed summary: selected_sessions',r.closed_stats.selected_sessions],['Closed summary: exposure',r.closed_stats.exposure]);
  const tradeFields=['trade_index','entry_source_index','exit_source_index','open','entry_allocation','gross_return','net_return','fee_return','slippage_return','gross_contribution','net_contribution','fee_contribution','slippage_contribution','gross_factor','net_factor','gross_curve_entry_allocation','paid_sides','estimated_exit_sides','calendar_days_held','observed_exposure_sessions','paid_fee_return','paid_fee_contribution','paid_slippage_return','paid_slippage_contribution','estimated_exit_fee_return','estimated_exit_fee_contribution','estimated_exit_slippage_return','estimated_exit_slippage_contribution']as const;
  return Object.freeze([
    pristineTradeTable(data,view),
    table('Trade audit',[...tradeFields],r.trade_returns.map((trade,i)=>{const audit={...trade,...r.trade_supplements[i]};return tradeFields.map(key=>audit[key]);})),
    table('Daily audit',['Source index','Date','Net equity (unit)','Independent gross equity (unit)',`${r.symbol} buy & hold (unit)`,'Net daily return','Gross daily return','Stock daily return','Observed exposure','Paid fee contribution','Paid slippage contribution','Estimated exit fee contribution','Estimated exit slippage contribution'],r.days.map((day,i)=>{const x=r.day_supplements[i];return[day.source_index,day.date,day.equity,day.gross_equity,day.benchmark_equity,day.net_daily_return,day.gross_daily_return,day.benchmark_daily_return,x.observed_exposure_session,x.paid_fee_contribution,x.paid_slippage_contribution,x.estimated_exit_fee_contribution,x.estimated_exit_slippage_contribution];}),[5,6,7]),
    table('Source events',['Source index','Date','Kind','Reason','Phase','Signal index','Entry index','Actual price','Exact price','Stop','Target','Previous stop','Setup JSON','OHLC ambiguity'],r.source.events.map(e=>[e.i,r.rows[e.i-r.from_source_index].date,e.kind,e.reason,e.phase,e.signal_i,e.entry_i,e.price,exactText(e.exact_price),exactText(e.stop),exactText(e.target),exactText(e.previous_stop),e.setup?JSON.stringify(e.setup):null,e.ambiguity.join('; ')])),
    table('Source signals',['Signal index','Date','Flat permission','Exact trigger','Exact stop','Exact target','Exact signal close'],r.source.signals.map(s=>[s.setup.signal_i,r.rows[s.setup.signal_i-r.from_source_index].date,s.flat_permission,exactText(s.setup.trigger),exactText(s.setup.stop),exactText(s.setup.target),exactText(s.setup.signal_close)])),
    table('Source rows',['Source index','Date','Open','High','Low','Close','Volume state','Volume value','Original invalid volume token'],r.rows.map(row=>[row.source_index,row.date,row.open,row.high,row.low,row.close,row.volume.state,row.volume.value,row.volume.token])),
    table('Statistics',['Model','Statistic','Raw value','Display value','Unavailable reason'],statRows),
    table('Metadata',['Field','Value'],metadata),
  ]);
}
