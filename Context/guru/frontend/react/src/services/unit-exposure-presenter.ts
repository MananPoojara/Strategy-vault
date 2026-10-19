import type {ReturnDisplay,ReturnDisplayMode} from './return-display';
import {isValidatedUnitExposureView,type UnitExposureView,type UnitExposureReport} from './unit-exposure-report';
export const UNIT_VISUAL_BASIS = 'Charts and calendar tables show rounded model values; statistics and the daily audit export use engine calculations.';
export const UNIT_CLOSED_BASIS = 'Closed trades only; terminal-open marks are excluded. Compounded closed model uses engine equity at the last closed exit, preserving independently rounded factors.';
/** The engine has already applied independently rounded factors; never reconstruct1+r. */
export function closedUnitEquity(report:UnitExposureReport):number {
  const last=report.trade_returns.filter(t=>!t.open).at(-1);
  return last?report.days[last.exit_source_index-report.days[0].source_index].equity:1;
}
export function unitExposureDisplay(data: unknown,mode: ReturnDisplayMode,view: UnitExposureView): ReturnDisplay {
  const result:ReturnDisplay={mode,label:mode==='compound'?'Compounded closed model':'Closed return sum',value:null,closedTrades:null,
    reportMode:mode==='compound'?'Engine factors · closed trades only':'Audited closed-return sum',description:UNIT_CLOSED_BASIS,unavailableReason:null};
  if (!isValidatedUnitExposureView(view) || !data || typeof data!=='object'
    || (data as {unit_exposure_report?:unknown}).unit_exposure_report!==view.report)
    return {...result,unavailableReason:'A validated unit-exposure report is required; legacy returns cannot replace it.'};
  const closed=view.report.trade_returns.filter(t=>!t.open),lastEquity=closedUnitEquity(view.report);
  const value=mode==='compound'?lastEquity-1:view.closedReturnSum;
  return {...result,value:Number.isFinite(value)?value:null,closedTrades:closed.length,
    unavailableReason:Number.isFinite(value)?null:'The closed display is outside finite arithmetic bounds.'};
}
export function unitExposureTables(view: UnitExposureView) {
  if (!isValidatedUnitExposureView(view)) throw Error('A validated unit-exposure report is required for export.');
  return {
    audit:{name:'Unit return audit',headers:['Trade #','Entry source index','Exit source index','Open','Net pretrade allocation (unit)','Gross return','Net return','Fee return','Slippage return','Gross contribution (unit)','Net contribution (unit)','Fee contribution (unit)','Slippage contribution (unit)'],
      rows:view.report.trade_returns.map(t=>[t.trade_index,t.entry_source_index,t.exit_source_index,t.open,t.entry_allocation,t.gross_return,t.net_return,t.fee_return,t.slippage_return,t.gross_contribution,t.net_contribution,t.fee_contribution,t.slippage_contribution]),percentColumns:[5,6,7,8],numericColumns:[0,1,2,4,9,10,11,12]},
    daily:{name:'Unit daily report',headers:['Source index','Date','Net equity (unit)','Independent gross equity (unit)','Index buy & hold (unit)','Net daily return','Gross daily return','Index daily return'],
      rows:view.report.days.map(d=>[d.source_index,d.date,d.equity,d.gross_equity,d.benchmark_equity,d.net_daily_return,d.gross_daily_return,d.benchmark_daily_return]),percentColumns:[5,6,7],numericColumns:[0,2,3,4]},
  };
}
export function unitExposureMetadata(view: UnitExposureView): (string|number|boolean|null)[][] {
  if (!isValidatedUnitExposureView(view)) throw Error('A validated unit-exposure report is required for export.');
  return [['Unit exposure model',view.model],['Source convention',view.report.source_convention],['Arithmetic',view.report.arithmetic],['Unit exposure basis',view.basis],['Closed display basis',UNIT_CLOSED_BASIS],['Chart/calendar presentation',UNIT_VISUAL_BASIS],['Gross basis','Independent zero-cost compounding on the same fills; contributions use actual net pretrade allocation.'],['Open exit cost','Estimated second-side liquidation deduction; terminal mark is not a filled sell.'],['Observed-time basis','Exposure includes entry-through-exit observed sessions; drawdown duration counts observed sessions. Days Held uses calendar differences.'],['Closed zero-return grouping','Exact-zero closed returns join non-wins.'],['Money basis','Dimensionless initial model unit1; no funded capital, share quantities or statutory taxes.'],['Original performance reproduced',false],['Native AmiBroker parity',false]];
}
