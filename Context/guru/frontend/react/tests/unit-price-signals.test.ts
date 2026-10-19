import {describe,it,expect} from 'vitest';
import {validatedUnitExposureReport} from '../src/services/unit-exposure-report';
import {unitPriceSignals} from '../src/services/unit-exposure-presenter';
import {unitRuntimeFixture} from './helpers/unit-runtime-fixture';

describe('audited unit-model price annotations',()=>{
 it('groups multiple weekly closed returns by addition without reconstructing model factors',()=>{
  const {data,admission}=unitRuntimeFixture(true),view=validatedUnitExposureReport(data,admission);
  const signals=unitPriceSignals(data,view,[0,0,1,1,1,1,1,2]);
  const closed=signals.zones.filter(z=>z.b===1&&z.outcome!=='open').map(z=>z.t);
  expect(closed).toHaveLength(3);
  const expected=data.trades.slice(0,3).reduce((sum:number,t:any)=>sum+t.ret_full_precision,0);
  expect(signals.closedReturnSum(closed)).toBe(expected);
  expect(signals.closedReturnSum(closed)).not.toBe(data.trades.slice(0,3).reduce((factor:number,t:any)=>factor*(1+t.ret),1)-1);
  expect(signals.closedReturnSum([])).toBe(0);
  expect(()=>signals.closedReturnSum([data.trades.at(-1)])).toThrow(/closed/);
  expect(()=>signals.closedReturnSum([closed[0],closed[0]])).toThrow(/distinct/);
 });
 it('classifies an exact-zero closed return as a non-win, without rounding it to a winner',()=>{
  const {data,admission}=unitRuntimeFixture(),view=validatedUnitExposureReport(data,admission);
  expect(data.trades[0].ret_full_precision).toBe(0);
  const signals=unitPriceSignals(data,view,[0,1,2]);
  expect(signals.annotation(data.trades[0])).toMatchObject({return:0,outcome:'nonwin',a:1,b:2});
  expect(signals.closedReturnSum(data.trades)).toBe(0);
 });
 it('caps terminal-open shading and hover at the selected mark inside a longer price history',()=>{
  const {data,admission}=unitRuntimeFixture(true),view=validatedUnitExposureReport(data,admission);
  // The selected January window sits inside a much longer full-prefix daily chart.
  const barOf=data.dates.map((_:string,i:number)=>1000+i),signals=unitPriceSignals(data,view,barOf);
  const open=data.trades.at(-1),zone=signals.annotation(open);
  expect(zone).toMatchObject({a:1007,b:1007,outcome:'open'});
  expect(signals.positionAt(1007)).toBe(open);
  expect(signals.positionAt(1008)).toBeUndefined();
  expect(signals.positionAt(11045)).toBeUndefined();
  expect(signals.zones.filter(z=>z.outcome==='win')).toHaveLength(3);
  expect(()=>signals.closedReturnSum([open])).toThrow();
 });
 it('rejects unrelated bodies, forged views/trades and unmapped selected dates',()=>{
  const {data,admission}=unitRuntimeFixture(),view=validatedUnitExposureReport(data,admission);
  expect(()=>unitPriceSignals({...data},view,[0,1,2])).toThrow(/paired/);
  expect(()=>unitPriceSignals(data,{...view},[0,1,2])).toThrow(/paired/);
  expect(()=>unitPriceSignals(data,view,[0,1])).toThrow(/coordinates/);
  const signals=unitPriceSignals(data,view,[0,1,2]);
  expect(()=>signals.annotation({...data.trades[0]})).toThrow(/belong/);
  expect(()=>signals.closedReturnSum([{...data.trades[0]}])).toThrow(/belong/);
 });
});
