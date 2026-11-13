import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {expect, it} from 'vitest';

// V7 (13 Nov 2026): scale-out trades carry their exit pieces in the trades export, so a checker can test each piece against its own day.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const VaultReport = createRequire(import.meta.url)('../../report-data.js');
// The page's own Excel library, loaded the way the browser loads it (a plain script, not a CommonJS module).
const XLSX = new Function('module', 'exports', 'define', 'require', readFileSync('../vendor/xlsx-js-style-1.2.0.min.js', 'utf8') + ';return XLSX;')();
const part = (from: string, to: string) => {const a = runtime.indexOf(from), b = runtime.indexOf(to, a); expect(a).toBeGreaterThan(-1); expect(b).toBeGreaterThan(a); return runtime.slice(a, b);};

const dates = ['1995-12-07', '1995-12-08', '1995-12-11', '1995-12-12', '1995-12-13', '1995-12-14', '1995-12-15'];
// The live cwin_aggressive_window NIFTY 50 trade of 8 Dec 1995 (engine JSON, 13 Nov 2026), re-indexed onto `dates`.
const cwin = {dir: -1, entry: '1995-12-08', exit: '1995-12-13', exit_i: 4, avg_px: 887.12, exit_px: 881.3978, ret: 0.00645, open: false,
  reason: '33% at 877.41 (close beyond SMA4); 33% at 879.49 (day-2 close in profit); breakeven stop 887.12 hit, out at 887.12',
  exits: [[2, 877.41, 0.33], [3, 879.49, 0.33], [4, 887.12, 0.34]], accounting: {signal_side: 'Short', pts: 5.7222, days_held: 5, net_ret: 0.00645}};
const single = {dir: 1, entry: '1995-12-14', exit: '1995-12-15', exit_i: 6, avg_px: 900, exit_px: 905, ret: 0.0056, open: false, reason: 'close above SMA5', exits: [[6, 905, 1]], accounting: {pts: 5, days_held: 1, net_ret: 0.0056}};

const deps = {
  VaultReport, XLSX, window: {XLSX}, sourceTradeHeaders: (h: string[]) => h, msg: () => undefined, loadXlsx: async () => undefined,
  unitFor: () => null, sourceAccount: () => null, sourceAdmission: () => null, reportName: () => 'trades.xlsx',
};
const make = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${part(' const tradeHeadersFor=', ' // Icons:')}${part(' const cellValue=', ' async function pristineExport(')};return {tradeExportRows,reportTable,xlsx};`);
const ui = make(deps);
const st = (trades: unknown[]) => ({d: {dates, trades}, tk: 'NIFTY 50', v: {id: 'cwin_aggressive_window'}, params: {}});
const workbook = async (trades: unknown[]) => {
  const [kind, rows, pc, s] = await ui.reportTable('trades', st(trades));
  return XLSX.read((await ui.xlsx(kind, rows, pc, s, [], false)).bytes, {type: 'array'});
};
const sheet = (book: {Sheets: Record<string, unknown>}, name: string) => XLSX.utils.sheet_to_json(book.Sheets[name], {header: 1}) as unknown[][];

it('a 3-piece cwin trade gets 3 piece rows, each with its own date, price, weight and reason', async () => {
  const book = await workbook([cwin, single]);
  expect(book.SheetNames.slice(0, 2)).toEqual(['Trades', 'Exit pieces']);
  expect(sheet(book, 'Exit pieces')).toEqual([
    ['Trade #', 'Signal', 'Entry Date', 'Piece #', 'Exit Date', 'Exit Price', 'Weight', 'Reason'],
    [1, 'SHORT', '08-12-1995', 1, '11-12-1995', 877.41, 0.33, '33% at 877.41 (close beyond SMA4)'],
    [1, 'SHORT', '08-12-1995', 2, '12-12-1995', 879.49, 0.33, '33% at 879.49 (day-2 close in profit)'],
    [1, 'SHORT', '08-12-1995', 3, '13-12-1995', 887.12, 0.34, 'breakeven stop 887.12 hit, out at 887.12'],
  ]);
  const trades = sheet(book, 'Trades');
  expect(trades[0]).toEqual([...VaultReport.tradeHeaders, 'Exit pieces', 'Exit piece detail']);
  expect(trades[1].slice(0, 17)).toEqual(VaultReport.tradeRows({trades: [cwin]}, 'NIFTY 50')[0].map((v: unknown) => v ?? undefined));
  expect(trades[1][17]).toBe(3);
  expect(trades[1][18]).toBe('1) 11-12-1995 @ 877.41 w 0.33 [33% at 877.41 (close beyond SMA4)] | 2) 12-12-1995 @ 879.49 w 0.33 [33% at 879.49 (day-2 close in profit)] | 3) 13-12-1995 @ 887.12 w 0.34 [breakeven stop 887.12 hit, out at 887.12]');
  expect(trades[2].length).toBeLessThanOrEqual(17); // the single-exit trade has no piece cells
});

it('a run of single-exit trades exports exactly as before: same columns, no pieces sheet', async () => {
  const rows = ui.tradeExportRows(st([single]));
  expect(rows).toEqual([VaultReport.tradeHeaders, ...VaultReport.tradeRows({trades: [single]}, 'NIFTY 50')]);
  expect((await workbook([single])).SheetNames).not.toContain('Exit pieces');
});

it('piece dates stay empty when the bar indices do not belong to these dates, and reasons when the counts differ', () => {
  const off = {...cwin, exits: [[0, 877.41, 0.5], [1, 887.12, 0.5]]};
  expect(VaultReport.pieceRows({dates, trades: [off]}).map((r: unknown[]) => [r[4], r[7]])).toEqual([['', ''], ['', '']]);
});
