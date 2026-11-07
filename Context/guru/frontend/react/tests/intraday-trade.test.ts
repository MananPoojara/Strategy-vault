import {describe, expect, it} from 'vitest';
import {entryHHMM, exitHHMM, isIntradayStrategy, layerMarkers, lowerBound, stampSeconds, stepTrade, tradeLayers, tradeSpans, tradeWindow} from '../src/services/intraday-trade';

const at = (d: string, hhmm: number) => stampSeconds(d, hhmm);
const times = [at('2024-01-02', 1510), at('2024-01-02', 1515), at('2024-01-03', 915), at('2024-01-03', 920), at('2024-01-03', 935), at('2024-01-04', 1515)];
const trade = {entry: '2024-01-02', exit: '2024-01-03', reason: 'Time exit 0920', legs: [[3, 106.5] as [number, number]], exit_px: 106.9};

describe('intraday trade layers', () => {
  it('detects an intraday strategy by its BTST_5M series', () => {
    expect(isIntradayStrategy(['BTST_5M.C1525', 'BTST_5M.C0915'])).toBe(true);
    expect(isIntradayStrategy([])).toBe(false);
    expect(isIntradayStrategy(undefined)).toBe(false);
    expect(isIntradayStrategy(['VIX'])).toBe(false);
  });
  it('uses the entry time in effect and the exit time from the reason', () => {
    expect(entryHHMM(undefined)).toBe(1515);
    expect(entryHHMM({entry_time: 1450})).toBe(1450);
    expect(exitHHMM('Time exit 0925', undefined)).toBe(925);
    expect(exitHHMM('Profit booking 1% at 0935', {exit_time: 930})).toBe(935);
    expect(exitHHMM('Hard stop 2% at 1520', undefined)).toBe(1520);
    expect(exitHHMM('Profit booking 1%', {exit_time: 930})).toBe(930);
    expect(exitHHMM('Profit booking 1%', undefined)).toBe(920);
  });
  it('binary-searches and stamps times', () => {
    expect(stampSeconds('1970-01-02', 1)).toBe(86400 + 60);
    expect(lowerBound(times, at('2024-01-03', 920))).toBe(3);
    expect(lowerBound(times, at('2024-01-03', 925))).toBe(4);
    expect(lowerBound(times, at('2025-01-01', 0))).toBe(times.length);
  });
  it('places entry at the entry-time bar and a time exit at its bar', () => {
    const [l] = tradeLayers([trade], times);
    expect([l.entryTime, l.exitTime, l.entryPx, l.exitPx, l.profitLine]).toEqual([times[1], times[3], 106.5, 106.9, null]);
    expect(tradeLayers([trade], times, {entry_time: 1510})[0].entryTime).toBe(times[0]);
  });
  it('profit booking exits at its own bar with the target level; a missing bar snaps forward; an open trade has no exit', () => {
    const [pb] = tradeLayers([{...trade, reason: 'Profit booking 1% at 0935', exit_px: 107.565}], times);
    expect(pb.exitTime).toBe(times[4]);
    expect(pb.profitLine).toBeCloseTo(107.565, 6);
    expect(tradeLayers([{...trade, reason: 'Time exit 0925'}], times)[0].exitTime).toBe(times[4]);
    expect(tradeLayers([{...trade, open: true, reason: 'open'}], times)[0].exitTime).toBeNull();
    expect(tradeLayers([{...trade, entry: '2024-02-01'}], times)[0].entryTime).toBeNull();
  });
  it('builds arrows (buy green up at entry, sell red down at exit) and spans', () => {
    const layers = tradeLayers([trade, {...trade, entry: '2024-01-04', exit: '', open: true, reason: 'open'}], times);
    const m = layerMarkers(layers, {buy: 'G', sell: 'R'});
    expect(m.map(k => [k.time, k.shape, k.position, k.color])).toEqual([
      [times[1], 'arrowUp', 'belowBar', 'G'], [times[3], 'arrowDown', 'aboveBar', 'R'], [times[5], 'arrowUp', 'belowBar', 'G']]);
    expect(tradeSpans(layers, times[5])).toEqual([{a: times[1], b: times[3], index: 0}, {a: times[5], b: times[5], index: 1}]);
  });
  it('zooms to a trade and steps within a list', () => {
    expect(tradeWindow(trade)).toEqual({from: at('2024-01-02', 0), to: at('2024-01-03', 0) + 86399});
    expect(stepTrade(0, -1, 5)).toBe(0);
    expect(stepTrade(2, 1, 5)).toBe(3);
  });
});
