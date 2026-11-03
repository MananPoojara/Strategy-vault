import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {expect, it} from 'vitest';

// Evaluates the real presenter and export rows from source; report-data.js is the real report module.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const VaultReport = createRequire(import.meta.url)('../../report-data.js');
const part = (from: string, to: string) => runtime.slice(runtime.indexOf(from), runtime.indexOf(to));
const rows = runtime.slice(runtime.indexOf(' const tradeHeadersFor='), runtime.indexOf('\n', runtime.indexOf(' const tradeRowsFor=')));
const presenter = part('// Tradesheet number columns', ' function equityNavigator(');
const deps = {
  VaultReport, sourceTradeHeaders: (h: string[]) => h, sourceAccountCopy: () => ({tradeNote: ''}), tradeExitCheckMarkup: () => '',
  escText: (v: unknown) => String(v ?? ''), label: (n: number) => String(n), csvSafeRows: (r: unknown[][]) => r,
};
const make = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${rows}${presenter};return {tradeHTML,tradeTableHTML,tradeHeadersFor,tradeRowsFor};`);
const ui = make(deps);
const trades = Array.from({length: 7706}, (_, i) => ({entry: '2020-01-01', exit: '2020-01-02', avg_px: 100 + i, exit_px: 101 + i, ret: 0.01, dir: 1, accounting: {pts: 1}}));
const run = {trades};

it('a report holds no rows until the tab opens, but still states how many there are', () => {
  const html = ui.tradeHTML(run);
  expect(html).toContain('7706 positions');
  expect(html).toContain('id="trades-xlsx"');
  expect(html).toContain('id="csv"');
  expect(html).toContain('data-tradesheet-table');
  expect(html).not.toContain('<td');
  expect(html).not.toContain('<table');
});

it('the table built on opening has every row, newest first', () => {
  const html = ui.tradeTableHTML(run, 'NIFTY 50');
  expect(html.match(/<tr>/g)).toHaveLength(7706 + 1);
  expect(html.indexOf('7805.00')).toBeLessThan(html.indexOf('>100.00<'));
});

it('Excel, CSV and optimizer exports take every row from the run data, not from the page', () => {
  const exported = [ui.tradeHeadersFor(run), ...ui.tradeRowsFor(run, 'NIFTY 50')];
  expect(exported).toHaveLength(7706 + 1);
  expect(VaultReport.csv(exported).trim().split(/\r?\n/)).toHaveLength(7706 + 1);
  const handlers = runtime.slice(runtime.indexOf('async function exportRunTrades'), runtime.indexOf('const refreshHeat='));
  expect(handlers.match(/\[tradeHeadersFor\(st\.d\),\.\.\.tradeRowsFor\(st\.d,st\.tk\)\]/g)).toHaveLength(3); // optimizer, CSV, Excel
  expect(handlers).not.toMatch(/#tradesheet|data-tradesheet/);
  expect(handlers).not.toContain('tradeTableHTML');
});

it('is filled for a PDF or print even while its tab is closed', () => {
  const showing = new Function('document', `${part('const panelShowing=', 'async function drawVisibleReportCharts')};return panelShowing;`);
  const body = document.createElement('body');
  body.innerHTML = '<div data-report-panel="positions" hidden><div id="closed"></div></div><div data-report-panel="positions"><div id="open"></div></div><div id="plain"></div>';
  const check = showing({body});
  const el = (id: string) => body.querySelector('#' + id)!;
  expect(check(el('closed'))).toBe(false);
  expect(check(el('open'))).toBe(true);
  expect(check(el('plain'))).toBe(true);
  for (const mode of ['pdf-capture', 'report-print']) {
    body.className = mode;
    expect(check(el('closed'))).toBe(true);
  }
});
