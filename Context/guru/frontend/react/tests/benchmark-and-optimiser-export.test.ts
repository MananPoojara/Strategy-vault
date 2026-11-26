import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {expect, it, vi} from 'vitest';
import {returnDisplay} from '../src/services/return-display';

// Site bugs from the verification tracker (13 Nov 2026), checked against the production runtime source.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const VaultReport = createRequire(import.meta.url)('../../report-data.js');
const part = (from: string, to: string) => {const a = runtime.indexOf(from), b = runtime.indexOf(to, a); expect(a).toBeGreaterThan(-1); expect(b).toBeGreaterThan(a); return runtime.slice(a, b);};

const dates = ['2026-10-07', '2026-10-08', '2026-10-09', '2026-10-12', '2026-11-12'];
const run = {dates, stats: {total_return: 0.1}};
const prices = (d: string[]) => ({d, c: d.map((_, i) => 100 + i)});

it('benchmark prices come from the engine series, the static file only as a fallback', async () => {
  const line = part('const benchmarkOhlc', '\n');
  const engine = prices(dates), stale = prices(dates.slice(0, 3));
  const make = (full: () => Promise<unknown>) => new Function('fullOhlc', 'staticOhlc', 'API', `${line};return benchmarkOhlc;`)(full, async () => stale, '/api/in');
  expect(await make(async () => engine)('NIFTY 50')).toBe(engine);
  expect(await make(async () => {throw Error('down');})('NIFTY 50')).toBe(stale);
  // the strategy page, the basket page and the export all read it; none reads the static snapshot directly
  expect(runtime).toContain("await benchmarkOhlc(isUS()?'SPX':'NIFTY 50',runApi)");
  expect(runtime).toContain('await benchmarkOhlc(benchmark,api)');
  expect(runtime).toContain('await benchmarkOhlc(st.benchmarkTk)');
  expect(runtime).not.toMatch(/await ohlc\((isUS\(\)|benchmark|st\.benchmarkTk)/);
});

it('a benchmark that ends before the strategy is withheld with a reason, never shown for another period', () => {
  const benchmarkFor = new Function('VaultReport', `${part(' function benchmarkFor(', ' function benchmarkHTML(')};return benchmarkFor;`)(VaultReport);
  const short = benchmarkFor(run, prices(dates.slice(0, 3)));
  expect(short.b).toBeNull();
  expect(short.note).toContain('2026-10-07 to 2026-10-09');
  expect(short.note).toContain('2026-10-07 to 2026-11-12');
  const full = benchmarkFor(run, prices(dates));
  expect(full.note).toBeNull();
  expect([full.b.from, full.b.to]).toEqual(['2026-10-07', '2026-11-12']);
  expect(full.b.total_return).toBeCloseTo(0.04, 12);
  expect(benchmarkFor(run, null).note).toBe('Benchmark price coverage is missing.');
});

it('a run ending a day past the benchmark\'s last bar still gets the benchmark, with its real dates', () => {
  const benchmarkFor = new Function('VaultReport', `${part(' function benchmarkFor(', ' function benchmarkHTML(')};return benchmarkFor;`)(VaultReport);
  const lag = benchmarkFor({dates: [...dates.slice(0, 4), '2026-10-13'], stats: {}}, prices(dates.slice(0, 4)));
  expect(lag.note).toBeNull();
  expect([lag.b.from, lag.b.to]).toEqual(['2026-10-07', '2026-10-12']);
  expect(benchmarkFor({dates: [...dates.slice(0, 4), '2026-10-30'], stats: {}}, prices(dates.slice(0, 4))).b).toBeNull();
});

it('optimiser trades export carries the Net Profit display and report link of the optimised run', async () => {
  const optTrades = part('async function optTrades(', '\nconst setLine');
  const defaultDisplay = {value: 0.5, reportMode: 'default run'};
  const d = {trades: [{open: false, ret: 0.02}, {open: false, ret: 0.03}, {open: true, ret: 0.9}]};
  const exportRunTrades = vi.fn(async () => undefined);
  const deps = {
    STRAT: {v: {id: 'cr2'}, tk: 'NIFTY 50', display: defaultDisplay},
    S: {tk: 'NIFTY 50', from: '2010-01-01', to: '', cost: '5', report_mode: 'fixed', p: ''},
    runData: async () => d, decodeP: () => ({len: 3}), bookDefaults: () => ({len: 2}), returnDisplay, sizingOf: () => ({weight: 1}),
    settingsTag: new Function(`${part('const settingsTag', '\n')};return settingsTag;`)(),
    VaultUI: {costState: () => ({slippage: '0', rollover: '0'}), exportRunTrades},
    href: (path: string, st: Record<string, string>) => '#' + path + '?' + new URLSearchParams(st),
    API: '/api/in', location: {href: 'http://vault/app/#/s/cr2/optim'},
  };
  const run = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${optTrades};return optTrades;`)(deps);
  await run({textContent: 'Download trades', dataset: {trades: 'len=3'}}, {exec: {slippage: '2', rollover: '0'}});
  const sent = (exportRunTrades.mock.calls[0] as unknown[])[0] as {display: {value: number}; url: string; d: unknown};
  expect(sent.d).toBe(d);
  expect(sent.display.value).toBeCloseTo(0.05, 12);
  expect(sent.display).not.toBe(defaultDisplay);
  expect(sent.url).toBe('http://vault/app/#/s/cr2?tk=NIFTY+50&from=2010-01-01&to=&cost=5&report_mode=fixed&p=len%3D3&slippage=2&rollover=0');
  // snapshotContext writes the run's own link into the Metadata sheet
  expect(part('function snapshotContext(', ' const filePart=')).toContain('url:run.url||location.href');
});

it('ohlc(): the engine series first (daily, and weekly aggregation), the static snapshot only when the engine fails or cannot aggregate', async () => {
  const line = part('const ohlc = async', '\n');
  // 2026-11-16 and 2026-11-23 are Mondays: two weeks of three bars
  const d = ['2026-11-16', '2026-11-17', '2026-11-18', '2026-11-23', '2026-11-24', '2026-11-25'];
  const engine = {d, o: [10, 11, 12, 20, 21, 22], h: [15, 16, 17, 25, 26, 27], l: [9, 8, 10, 19, 18, 20], c: [11, 12, 13, 21, 22, 23]};
  const stale = {d: ['2026-10-09'], o: [1], h: [2], l: [0], c: [1]};
  const make = (full: () => Promise<unknown>) => new Function('fullOhlc', 'staticOhlc', 'VaultReport', `${line};return ohlc;`)(full, async () => stale, VaultReport);
  expect(await make(async () => engine)('NIFTY 50')).toBe(engine);
  const w = await make(async () => engine)('NIFTY 50', 'W');
  expect(w.d).toHaveLength(2);
  expect([w.o[0], w.h[0], w.l[0], w.c[0]]).toEqual([10, 17, 8, 13]);
  expect([w.o[1], w.h[1], w.l[1], w.c[1]]).toEqual([20, 27, 18, 23]);
  expect(await make(async () => {throw Error('down');})('NIFTY 50')).toBe(stale);
  expect(await make(async () => ({d, c: engine.c}))('NIFTY 50', 'W')).toBe(stale);
});
