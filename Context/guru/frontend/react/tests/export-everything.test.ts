import {readFileSync} from 'node:fs';
import {expect, it} from 'vitest';
import {exportEverything, EXPORT_KINDS, MANIFEST_HEADERS, type ExportRun} from '../src/services/export-everything';

// V4 (13 Nov 2026): verifier exports. Checked against the production runtime source and the export-everything service.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const part = (from: string, to: string) => {const a = runtime.indexOf(from), b = runtime.indexOf(to, a); expect(a).toBeGreaterThan(-1); expect(b).toBeGreaterThan(a); return runtime.slice(a, b);};
const line = (from: string) => part(from, '\n');

it('bug #3: the Parameters sheet gets one row per setting and the file name carries applied settings', () => {
  const decodeP = new Function(`${line('const decodeP')};return decodeP;`)();
  // (settingsTag is new with the fix; before it, the test still runs and fails on the split settings)
  const settingsTag = runtime.includes('const settingsTag') ? new Function(`${line('const settingsTag')};return settingsTag;`)() : () => undefined;
  const deps = {
    S: {region: 'in', p: 'trend:50,stop:2'}, decodeP, settingsTag, STRAT: null, currentBenchmark: null, LABEL: {},
    unitFor: () => null, immutable: (x: unknown) => x, location: {href: 'http://vault/#/s/x'}, document: {querySelector: () => null},
  };
  const snapshotContext = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${part(' function snapshotContext(', ' const filePart=')};return snapshotContext;`)(deps);
  const d = {dates: ['2010-01-04'], trades: []}, v = {id: 'x'};
  // the page run: its full settings (book defaults + applied) — not the URL text
  const page = snapshotContext({d, v, tk: 'NIFTY 50', indicatorParameters: {trend: 50, stop: 2, exit: 5}});
  expect(page.params).toEqual({trend: 50, stop: 2, exit: 5});
  expect(page.fileTag).toBe('trend50_stop2');
  // without a settings object the applied settings are decoded, never split into characters
  const bare = snapshotContext({d, v, tk: 'NIFTY 50'});
  expect(Object.entries(bare.params)).toEqual([['trend', 50], ['stop', 2]]);
  expect(part(' async function xlsx(', ' async function pristineExport(')).toContain("[['Parameter','Value'],...Object.entries(st.params)]");
  // default settings keep the old file name; explicit tags (optimiser rows, Export everything) win
  deps.S.p = '';
  expect(snapshotContext({d, v, tk: 'NIFTY 50', indicatorParameters: {trend: 20}}).fileTag).toBeUndefined();
  expect(snapshotContext({d, v, tk: 'NIFTY 50', fileTag: 'book'}).fileTag).toBe('book');
});

it('page buttons and Export everything build each workbook with the same table code', () => {
  const handler = part("document.addEventListener('click',async event=>{const b=event.target.closest('[data-action],[data-chart-snapshot]", '\n');
  expect(handler).toContain("if(b.id==='trades-xlsx')return await xlsx(...await reportTable('trades',st));");
  expect(handler).toContain('if(REPORT_KINDS.includes(kind))return await xlsx(...await reportTable(kind,st));');
  expect(line(' const REPORT_KINDS=')).toContain(EXPORT_KINDS.map(k => `'${k}'`).join(','));
  expect(part(' async function exportAll(', ' document.addEventListener(')).toContain('await reportTable(kind,st);const book=await xlsx(k,rows,pc,st,types,false)');
  expect(line(' async function exportRunTrades(')).toContain("xlsx(...await reportTable('trades',st))");
  // the zip writer drops ArrayBuffer entries: workbooks are handed over as Uint8Array
  expect(part(' async function xlsx(', ' async function pristineExport(')).toContain("if(!save)return {name:reportName(st,kind,'xlsx'),bytes:new Uint8Array(bytes)};");
});

it('static price files are read only as a fallback after the engine series', () => {
  const reads = runtime.split('\n').filter(l => /[^\w.]ohlc\(/.test(l) && !l.startsWith('const ohlc'));
  expect(reads).toHaveLength(3);
  expect(reads[0]).toContain('const benchmarkOhlc = (tk, api = API) => fullOhlc(tk, api).catch(() => ohlc(tk));');
  expect(reads[1]).toContain('const px=optimization?null:(await chartRequest)||d.ohlc||await ohlc(tk);');
  expect(reads[2]).toContain("STRAT.px?.full_history||STRAT.d.ohlc?VaultReport.aggregate(STRAT.px,tf):await ohlc(STRAT.tk, tf)");
});

type Run = ExportRun & {tk: string; from: string; p: string};
const fake = (over: Partial<Parameters<typeof exportEverything<Run>>[0]> = {}) => {
  let active = 0, peak = 0;
  const calls: string[] = [];
  const deps = {
    admit: () => null, ready: async () => null,
    optimiserTop: async (tk: string, from: string) => { calls.push(`opt ${tk} ${from}`); return 'len:7'; },
    run: async (tk: string, from: string, p: string) => { active++; peak = Math.max(peak, active); await Promise.resolve(); active--; calls.push(`run ${tk} ${from} ${p}`);
      return {tk, from, p, dates: [from || '1995-11-03', '2026-11-12'], params: p ? {len: 7} : {len: 5}, ms: 12}; },
    workbook: async (kind: string, run: Run) => { active++; peak = Math.max(peak, active); await Promise.resolve(); active--; return {name: `x-${run.tk}-${kind}${run.p ? '-len7' : ''}.xlsx`, bytes: new Uint8Array([1])}; },
    now: () => '2026-11-13T10:00:00Z',
    ...over,
  };
  return {deps, calls, peak: () => peak};
};

it('runs the whole verifier matrix one run at a time and lists every file in the manifest', async () => {
  const f = fake(), progress: number[] = [];
  const {files, manifest} = await exportEverything<Run>({...f.deps, progress: done => progress.push(done)});
  expect(files).toHaveLength(3 * 2 * 2 * 5);
  expect(new Set(files.map(x => x.path)).size).toBe(60);
  expect(files[0].path).toBe('NIFTY50/full-history/book/x-NIFTY 50-trades.xlsx');
  expect(files.at(-1)!.path).toBe('NIFTYMIDCAP100/from-2010-01-04/optimiser-1/x-NIFTYMIDCAP100-details-len7.xlsx');
  expect(f.peak()).toBe(1);
  expect(f.calls.filter(c => c.startsWith('run'))).toHaveLength(12);
  expect(f.calls).toContain('opt NIFTYBANK 2010-01-04');
  expect(f.calls).toContain('run NIFTYBANK 2010-01-04 len:7');
  expect(manifest[0]).toEqual(MANIFEST_HEADERS);
  expect(manifest).toHaveLength(61);
  expect(manifest[1]).toEqual(['NIFTY50/full-history/book/x-NIFTY 50-trades.xlsx', 'Nifty 50', 'NIFTY 50', 'Full history', '', 'Book settings', 'len=5', 'trades', '1995-11-03', '2026-11-12', 12, '2026-11-13T10:00:00Z', 'ok', '']);
  expect(manifest.at(-1)!.slice(1, 8)).toEqual(['Midcap 100', 'NIFTYMIDCAP100', 'From 2010-01-04', '2010-01-04', 'Optimiser #1', 'len=7', 'details']);
  expect(progress.at(-1)).toBe(12);
});

it('records admission, data, fixed-configuration and failure reasons instead of files', async () => {
  const f = fake({
    admit: tk => tk === 'NIFTYBANK' ? 'Not admitted on Bank Nifty.' : null,
    ready: async (tk, from) => tk === 'NIFTYMIDCAP100' && from === '' ? 'Needs data: Midcap 100 before 2005.' : null,
    run: async (tk, from, p) => { if (tk === 'NIFTYMIDCAP100' && p) throw Error('engine down'); return {tk, from, p, dates: ['2010-01-04', '2026-11-12'], params: {}, fixed: tk === 'NIFTY 50' ? 'Fixed reviewed configuration.' : undefined}; },
    workbook: async (kind, run) => kind === 'trades' && run.tk === 'NIFTY 50' ? {skip: 'trades paged'} : {name: `${run.tk}-${kind}.xlsx`, bytes: new Uint8Array()},
  });
  const {files, manifest} = await exportEverything<Run>(f.deps);
  const notes = manifest.slice(1).filter(r => r[12] === 'skipped').map(r => `${r[2]}|${r[3]}|${r[5]}|${r[13]}`);
  expect(notes).toEqual([
    'NIFTY 50|Full history|Book settings|trades paged', 'NIFTY 50|Full history|Optimiser #1|Fixed reviewed configuration.',
    'NIFTY 50|From 2010-01-04|Book settings|trades paged', 'NIFTY 50|From 2010-01-04|Optimiser #1|Fixed reviewed configuration.',
    'NIFTYBANK|Full history|Book settings|Not admitted on Bank Nifty.', 'NIFTYBANK|Full history|Optimiser #1|Not admitted on Bank Nifty.',
    'NIFTYBANK|From 2010-01-04|Book settings|Not admitted on Bank Nifty.', 'NIFTYBANK|From 2010-01-04|Optimiser #1|Not admitted on Bank Nifty.',
    'NIFTYMIDCAP100|Full history|Book settings|Needs data: Midcap 100 before 2005.', 'NIFTYMIDCAP100|Full history|Optimiser #1|Needs data: Midcap 100 before 2005.',
    'NIFTYMIDCAP100|From 2010-01-04|Optimiser #1|Failed: engine down',
  ]);
  expect(f.calls).not.toContain('opt NIFTYMIDCAP100 '); // book data not ready: the optimiser is not asked
  expect(files).toHaveLength(4 + 4 + 5);
});

it('cancel stops before the next engine call and returns nothing', async () => {
  const control = new AbortController();
  const f = fake({run: async (tk, from, p) => { control.abort(); return {tk, from, p, dates: ['2010-01-04'], params: {}}; }});
  await expect(exportEverything<Run>({...f.deps, signal: control.signal})).rejects.toThrow();
  expect(f.calls.filter(c => c.startsWith('opt'))).toHaveLength(0);
});
