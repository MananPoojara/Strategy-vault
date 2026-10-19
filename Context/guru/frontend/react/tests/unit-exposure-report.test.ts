import {describe,it,expect} from 'vitest';
import {parseUnitExposureReport,parseUnitExposureAdapter,unitExposureInstrumentContext,unitStatisticDisplay} from '../src/services/unit-exposure-report';
import {roundingCell,unitReturnConsistent,unitPriceReturnConsistent,unitFeeConsistent,unitProductConsistent,unitCurveConsistent} from '../src/services/unit-exposure-rounding';
import {unitExposureFixture,UNIT_FIXTURE_BINDING} from './helpers/unit-exposure-fixture';
const parse = (change?: (data: any, admission: any) => void) => {
  const {data,admission}=unitExposureFixture();change?.(data,admission);
  return parseUnitExposureAdapter(data,admission,UNIT_FIXTURE_BINDING);
};
describe('separate typed unit-exposure contract',()=>{
  it('retains independently derived economic break-even as a closed non-win',()=>{
    const view=parse();expect(view.closedTrades).toBe(1);expect(view.closedReturnSum).toBe(0);
    expect(Object.is(view.report.trade_returns[0].net_return,0)).toBe(true);
    expect(view.report.days.map(d=>d.equity)).toEqual([1,.9995,1]);
    expect(view.report.days.at(-1)?.gross_equity).toBe(1.001);
  });
  it('never admits synthetic adapter metadata through public entry point',()=>{
    const {data,admission}=unitExposureFixture();expect(()=>parseUnitExposureReport(data,admission)).toThrow(/unit-exposure report/);
    expect(()=>parseUnitExposureReport(data,{...admission,adapter_contract:UNIT_FIXTURE_BINDING} as any)).toThrow();
  });
  it.each([
    ['missing report',(d:any)=>{delete d.unit_exposure_report;}],
    ['unknown model',(d:any)=>{d.unit_exposure_report.model='cash_or_unknown';}],
    ['wrong method',(d:any)=>{d.id='hbqts_stock_example';}],
    ['wrong book',(d:any)=>{d.book='hbiab';}],
    ['wrong convention',(d:any)=>{d.unit_exposure_report.source_convention='v1';}],
    ['cash coexistence',(d:any)=>{d.source_account=null;}],
    ['basis disagreement',(d:any)=>{d.accounting_basis='other';}],
    ['source performance claim',(d:any)=>{d.unit_exposure_report.source_performance_reproduced=true;}],
    ['native parity claim',(d:any)=>{d.unit_exposure_report.native_amibroker_parity=true;}],
    ['gapped source indices',(d:any)=>{d.unit_exposure_report.days[1].source_index++;}],
    ['date mismatch',(d:any)=>{d.unit_exposure_report.days[1].date='2024-01-05';}],
    ['outer curve mismatch',(d:any)=>{d.equity[1]=1;}],
    ['nonpositive curve',(d:any)=>{d.unit_exposure_report.days[1].equity=0;}],
    ['trade coordinate mismatch',(d:any)=>{d.trades[0].legs[0][0]=0;}],
    ['next-open phase',(d:any)=>{d.trades[0].entry_kind='open';}],
    ['different return visible in tradesheet',(d:any)=>{d.trades[0].ret_full_precision=Number.MIN_VALUE;}],
    ['invented monetary result',(d:any)=>{d.trades[0].accounting.gross_pnl=1000;}],
    ['unexecuted exit mislabel',(d:any)=>{d.trades[0].accounting.cost_status='paid_cash_exit';}],
    ['hidden tiny cost',(d:any)=>{d.unit_exposure_report.trade_returns[0].slippage_return=0;}],
    ['false winner statistic',(d:any)=>{d.stats.win_rate=1;}],
    ['forced zero undefined t statistic',(d:any)=>{d.stats.t_stat=0;}],
    ['forced zero undefined winning average',(d:any)=>{d.stats.avg_win=0;}],
    ['negative model zero',(d:any)=>{d.unit_exposure_report.trade_returns[0].net_return=-0;}],
    ['invented quantity field',(d:any)=>{d.unit_exposure_report.trade_returns[0].quantity=1;}],
  ])('withholds %s rather than using legacy financial fields',(_label,change)=>{expect(()=>parse(change)).toThrow(/unit-exposure report/);});
  it('checks effective defaults and policy without inventing metadata model fields',()=>{
    expect(()=>parse((_d,a)=>{a.parameters.hold=2;})).toThrow();
    expect(()=>parse((_d,a)=>{a.definition.instrument_policy.account_model='canonical_decimal_unit_exposure_v2';})).toThrow();
    expect(()=>parse((_d,a)=>{a.definition.instrument_policy.independent_selected_ledger_checked=false;})).toThrow();
    expect(()=>parse((_d,a)=>{a.costs.slippage_bps=0;})).toThrow();
  });
  it('requires a separately hash-bound reference projection adapter',()=>{
    const {data,admission}=unitExposureFixture();expect(()=>parseUnitExposureAdapter(data,admission,{})).toThrow();
    expect(()=>parseUnitExposureAdapter(data,admission,{...UNIT_FIXTURE_BINDING,reference_sha256:'unbound'})).toThrow();
  });
  it('only identifies real regional index-selector rows and keeps their metadata unchanged',()=>{
    const spx={ticker:'SPX',first:'1983-01-03',last:'2026-09-24'};
    const nifty={ticker:'NIFTY 50',first:'2000-01-03',last:'2026-10-07'};
    expect(unitExposureInstrumentContext('SPX','us',[spx],[spx])?.metadata).toBe(spx);
    expect(unitExposureInstrumentContext('NIFTY 50','in',[nifty],[nifty])?.metadata).toBe(nifty);
    expect(unitExposureInstrumentContext('SPX','in',[spx],[spx])).toBeNull();
    expect(unitExposureInstrumentContext('NIFTY50','in',[nifty],[nifty])).toBeNull();
    expect(unitExposureInstrumentContext('SPY','us',[{...spx,ticker:'SPY'}],[])).toBeNull();
    expect(unitExposureInstrumentContext('SPX','us',[spx],[{...spx}])).toBeNull();
    const disguised={...spx,asset_type:'etf'};expect(unitExposureInstrumentContext('SPX','us',[disguised],[disguised])).toBeNull();
  });
});
describe('pre-observation exact dyadic consistency bounds',()=>{
  it('keeps half-subnormal rounding-cell endpoints exactly',()=>{
    expect(roundingCell(Number.MIN_VALUE)).toEqual([1n,3n]);expect(roundingCell(-Number.MIN_VALUE)).toEqual([-3n,-1n]);expect(roundingCell(0)).toEqual([0n,0n]);
  });
  it('reconciles independently rounded fractions without changing the audited sign',()=>{
    expect(unitReturnConsistent(.001,0,0,.001)).toBe(true);
    expect(unitPriceReturnConsistent(99,99.099,.001)).toBe(true);
    expect(unitPriceReturnConsistent(99,99.099,.002)).toBe(false);
    expect(unitPriceReturnConsistent(99,99.099,0)).toBe(false);
    expect(unitFeeConsistent(0,0)).toBe(true);expect(unitFeeConsistent(5,.001)).toBe(true);expect(unitFeeConsistent(5,0)).toBe(false);
  });
  it('requires exact one-product contributions including supported subnormals',()=>{
    expect(unitProductConsistent(1,Number.MIN_VALUE,Number.MIN_VALUE)).toBe(true);
    expect(unitProductConsistent(Number.MIN_VALUE,.5,0)).toBe(false);
    expect(unitProductConsistent(1,0,-0)).toBe(false);
  });
  it('does not reconstruct a factor by adding one to rounded return',()=>{
    expect(unitCurveConsistent(1,Number.MIN_VALUE,-1)).toBe(true);
    expect(unitCurveConsistent(1,1,0)).toBe(true);
    expect(unitCurveConsistent(1,1+Number.EPSILON,0)).toBe(false);
    expect(unitCurveConsistent(1,1,NaN)).toBe(false);
  });
  it('preserves scoped six-decimal null and finite-huge serialization semantics',()=>{
    expect(unitStatisticDisplay(Infinity)).toBeNull();expect(unitStatisticDisplay(NaN)).toBeNull();
    expect(unitStatisticDisplay(1e308)).toBe(1e308);expect(unitStatisticDisplay(-1e308)).toBe(-1e308);
    expect(unitStatisticDisplay(.0000005)).toBe(.000001);expect(unitStatisticDisplay(-.0000005)).toBe(-.000001);
    expect(Object.is(unitStatisticDisplay(-.0000001),-0)).toBe(true);
  });
});
