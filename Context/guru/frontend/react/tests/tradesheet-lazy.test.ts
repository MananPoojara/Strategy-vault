import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {expect, it} from 'vitest';

// Evaluates the real presenter and export rows from source; report-data.js is the real report module.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const VaultReport = createRequire(import.meta.url)('../../report-data.js');
const part = (from: string, to: string) => runtime.slice(runtime.indexOf(from), runtime.indexOf(to));
const rows = runtime.slice(runtime.indexOf(' const tradeHeadersFor='), runtime.indexOf('\n', runtime.indexOf(' const tradeRowsFor=')));
const presenter = part('// Tradelist columns', ' function equityNavigator(');
import * as fmt from '../src/services/tradelist-format';
const deps = {
  ...fmt,   VaultReport, sourceTradeHeaders: (h: string[]) => h, sourceAccountCopy: () => ({tradeNote: ''}), tradeExitCheckMarkup: () => '',
  escText: (v: unknown) => String(v ?? ''), label: (n: number) => String(n), csvSafeRows: (r: unknown[][]) => r,
};
const make = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${rows}${presenter};return {tradeHTML,tradeTableHTML,tradeHeadersFor,tradeRowsFor};`);
const ui = make(deps);
const trades = Array.from({length: 7706}, (_, i) => ({entry: '2020-01-01', exit: '2020-01-02', avg_px: 100 + i, exit_px: 101 + i, ret: 0.01, dir: 1, accounting: {pts: 1}}));
const run = {trades};

it('a report holds no rows until the tab opens, but still states how many there are', () => {
  const html = ui.tradeHTML(run);
  expect(html).toContain('7706 trades');
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

it('the on-screen table has exactly the 17 Tradelist columns, formatted, with no explanatory paragraph', () => {
  const t = {entry: '2008-01-22', exit: '2008-01-31', avg_px: 4894.6, exit_px: 5137.45, ret: 0.046, dir: 1, reason: 'Base2 Bull Exit',
    accounting: {symbol: 'NIFTY31JAN2008FUT', signal_side: 'Long', pts: 242.85, gross_pnl: 1650000, gross_ret: 0.05, days_held: 9, taxes: 14250, slippage: 99850, slippage_ret: 0.001, net_pnl: 1540000, net_ret: 0.046, mae: 0.007, mfe: 0.106}};
  const r = {trades: [t]};
  expect(ui.tradeHTML(r)).not.toContain('rnote');
  const html = ui.tradeTableHTML(r, 'NIFTY 50');
  expect([...html.matchAll(/<th /g)]).toHaveLength(17);
  expect([...html.matchAll(/<th [^>]*>([^<]*)</g)].map(m => m[1]).join('|')).toBe('Symbol|Signal|Entry Date|Entry Price|Exit Date|Exit Price|Pts|Gross P&L|Gross P&L %|Days Held|Taxes|Slippage|Net P&L|Net P&L %|MAE|MFE|Exit Reason');
  const cells = [...html.split('<tbody>')[1].matchAll(/<td [^>]*>(.*?)<\/td>/g)].map(m => m[1]);
  expect(cells).toEqual(['NIFTY31JAN2008FUT', 'Long', '22/01/2008', '4894.60', '31/01/2008', '5137.45', '242.85', '₹16.5 L', '5.0%', '9', '₹14,250', '₹99,850', '₹15.4 L', '4.6%', '0.7%', '10.6%', 'Base2 Bull Exit']);
  const bare = {trades: [{...t, accounting: {...t.accounting, gross_pnl: null, taxes: null, slippage: null, net_pnl: null}}]};
  const c2 = [...ui.tradeTableHTML(bare, 'X').split('<tbody>')[1].matchAll(/<td [^>]*>(.*?)<\/td>/g)].map(m => m[1]);
  expect([c2[7], c2[10], c2[11], c2[12]]).toEqual(['—', '—', '—', '—']);
});

it('Excel, CSV and optimizer exports take every row from the run data, not from the page', () => {
  const exported = [ui.tradeHeadersFor(run), ...ui.tradeRowsFor(run, 'NIFTY 50')];
  expect(exported).toHaveLength(7706 + 1);
  expect(VaultReport.csv(exported).trim().split(/\r?\n/)).toHaveLength(7706 + 1);
  const handlers = runtime.slice(runtime.indexOf('async function exportRunTrades'), runtime.indexOf('const refreshHeat='));
  expect(handlers.match(/csvSafeRows\(tradeExportRows\(st\)\)/g)).toHaveLength(1); // CSV
  expect(handlers.match(/reportTable\('trades',st\)/g)).toHaveLength(2); // optimizer, Excel (Export everything uses reportTable(kind,st))
  expect(runtime.slice(runtime.indexOf(' async function reportTable('), runtime.indexOf(' async function xlsx('))).toContain("if(kind==='trades')return ['trades',tradeExportRows(st),");
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
