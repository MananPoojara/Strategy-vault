import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {expect, it} from 'vitest';

// Lane E6: a run over the engine's trade cap arrives summary-only (trades_page) and its tradesheet loads one page at a time.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const VaultReport = createRequire(import.meta.url)('../../report-data.js');
const part = (from: string, to: string) => runtime.slice(runtime.indexOf(from), runtime.indexOf(to));
const rows = runtime.slice(runtime.indexOf(' const tradeHeadersFor='), runtime.indexOf('\n', runtime.indexOf(' const tradeRowsFor=')));
const presenter = part('// Tradesheet number columns', ' function equityNavigator(');
const deps = {
  VaultReport, sourceTradeHeaders: (h: string[]) => h, sourceAccountCopy: () => ({tradeNote: ''}), tradeExitCheckMarkup: () => '',
  escText: (v: unknown) => String(v ?? ''), label: (n: number) => String(n), csvSafeRows: (r: unknown[][]) => r,
};
const ui = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${rows}${presenter};return {tradeHTML,tradeTableHTML};`)(deps);
const trade = (i: number) => ({entry: '2020-01-01', exit: '2020-01-02', avg_px: 100 + i, exit_px: 101 + i, ret: 0.01, dir: 1, accounting: {pts: 1}});
const page = (offset: number, n: number, total: number) => ({trades: Array.from({length: n}, (_, i) => trade(offset + i)), trades_page: {offset, limit: n, total, cap: 20000}});

it('a small run keeps its tradesheet exactly as before: every row, no pager, no page note', () => {
  const html = ui.tradeHTML({trades: Array.from({length: 12}, (_, i) => trade(i))});
  expect(html).toContain('12 positions · newest first');
  expect(html).not.toMatch(/data-trade-page|trades_page|loads 1,000/);
});

it('a summary-only run states the true total and says what the page covers', () => {
  const html = ui.tradeHTML({trades: [], trades_page: {offset: 0, limit: 0, total: 79123, cap: 20000}});
  expect(html).toContain('79123 positions · loading the newest page');
  expect(html).toContain('This run has 79,123 trades, more than the 20,000 one response carries');
  expect(html).toContain('Statistics, equity, drawdown and calendar returns use every trade');
  expect(html).not.toContain('data-trade-page="');
});

it('the newest page shows its range newest first; Older steps back one page, Newer is off at the end', () => {
  const d = page(78123, 1000, 79123), html = ui.tradeHTML(d);
  expect(html).toContain('showing 78124–79123');
  expect(html).toMatch(/data-trade-page="78123" disabled>Newer/);
  expect(html).toContain('data-trade-page="77123" >Older');
  const table = ui.tradeTableHTML(d, 'SPY');
  expect(table.match(/<tr>/g)).toHaveLength(1000 + 1);
  expect(table.indexOf('79222.00')).toBeLessThan(table.indexOf('78223.00'));
});

it('the oldest page turns Older off and Newer on', () => {
  const html = ui.tradeHTML(page(0, 1000, 79123));
  expect(html).toContain('showing 1–1000');
  expect(html).toContain('data-trade-page="1000" >Newer');
  expect(html).toMatch(/data-trade-page="0" disabled>Older/);
});

it('a page asks the engine for exactly the run the card shows, and the chart draws no markers from one page', () => {
  expect(runtime).toContain("if(j.trades_page)j._run_url=enginePath(id,'run',api)+'?' + new URLSearchParams({ id, ticker: tk, from, to, cost, ...execution,");
  expect(runtime).toContain("st.d._run_url+'&'+new URLSearchParams({trades_offset:at,trades_limit:VaultUI.TRADE_PAGE})");
  expect(runtime).toContain('const d = STRAT.d.trades_page?{...STRAT.d,trades:[]}:STRAT.d;');
});
