import {describe, expect, it} from 'vitest';
import {
  parseDatabases, databaseOfTicker, databaseDisabledReason, databaseOptions, databaseSelectHTML, tickerForDatabase,
  dbTickerParts, dataNoteHTML, dbBadgeInfo, untestedNote, type YtDatabase,
} from '../src/services/yt-databases';
import {instrumentControls, instrumentTickers, krSpotMethod} from '../src/services/instrument-controls';

const db = (id: string, kind: string, extra: Record<string, unknown> = {}) => ({
  id, name: id.toUpperCase(), kind, intraday: false, prices: 'Cash', first: '2000-01-03', last: '2022-03-31',
  has_volume: true, has_aux: false, available: true, note: '', default: false, tickers: [], ...extra,
});
const BODY = {databases: [
  db('idx_new_sector', 'index', {default: true, tickers: [{ticker: 'NIFTY 50', symbol: 'NIFTY 50'}, {ticker: 'NIFTYBANK', symbol: 'NIFTYBANK'}, {ticker: 'db:idx_new_sector:NIFTYIT', symbol: 'NIFTYIT'}]}),
  db('idx_old_2022', 'index', {has_volume: false, tickers: [{ticker: 'db:idx_old_2022:NIFTY 50', symbol: 'NIFTY 50'}, {ticker: 'db:idx_old_2022:NIFTYBANK', symbol: 'NIFTYBANK'}]}),
  db('futures_bhavcopy', 'futures', {default: true, tickers: [{ticker: 'NIFTY FUT', symbol: 'NIFTY'}, {ticker: 'BANKNIFTY FUT', symbol: 'BANKNIFTY'}]}),
  db('cp_updated_2026', 'individual_stock', {default: true, has_aux: true, tickers: [{ticker: 'basket:nse500:RELIANCE', symbol: 'RELIANCE'}, {ticker: 'db:cp_updated_2026:XYZ', symbol: 'XYZ'}]}),
  db('cp_old_2022', 'individual_stock', {has_aux: true, tickers: [{ticker: 'db:cp_old_2022:SBIN', symbol: 'SBIN'}, {ticker: 'db:cp_old_2022:RELIANCE', symbol: 'RELIANCE'}]}),
  db('ieod_fo_2023', 'futures', {intraday: true, default: true, tickers: [{ticker: 'basket:ieodfo:BANKNIFTY', symbol: 'BANKNIFTY'}]}),
  db('ieod_fo_old', 'intraday_futures', {available: false, note: 'only 2 symbols copied; its 1-minute native format is not read yet'}),
  db('idx_volume_2026', 'index', {available: false, note: 'not copied to this PC'}),
]};
const LIST = parseDatabases(BODY)!;
const byId = (id: string) => LIST.find(d => d.id === id)!;

describe('database list parsing', () => {
  it('reads the contract body, normalises intraday kinds and keeps names as listed', () => {
    expect(LIST.map(d => d.id)).toEqual(BODY.databases.map(d => d.id));
    expect(byId('cp_old_2022').name).toBe('CP_OLD_2022');
    expect(byId('ieod_fo_old')).toMatchObject({kind: 'futures', intraday: true, available: false});
    expect(byId('cp_old_2022').tickers[1]).toEqual({ticker: 'db:cp_old_2022:RELIANCE', symbol: 'RELIANCE'});
  });
  it('skips malformed rows and fills missing fields safely', () => {
    const list = parseDatabases({databases: [null, {name: 'no id'}, {id: 'x', tickers: [{ticker: 'db:x:ABC'}, {symbol: 'none'}]}]})!;
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({id: 'x', name: 'x', available: true, has_volume: true, default: false});
    expect(list[0].tickers).toEqual([{ticker: 'db:x:ABC', symbol: 'ABC'}]);
  });
  it('returns null for anything that is not a database list, so the page keeps its existing controls', () => {
    for (const body of [null, undefined, 'Not Found', {error: 'unknown endpoint'}, {databases: 'x'}, {databases: []}, 42, [{id: 'strategy_row'}]])
      expect(parseDatabases(body)).toBeNull();
    expect(databaseOptions(null, [undefined], false)).toEqual([]);
    expect(databaseSelectHTML([], '', 'strategy-db')).toBe('');
    expect(databaseOfTicker('NIFTY 50', null)).toBeNull();
    expect(dbBadgeInfo('NIFTY 50', null)).toBeNull();
  });
});

describe('default selection follows the current ticker', () => {
  it('maps existing tickers to the default database of their kind', () => {
    expect(databaseOfTicker('NIFTY 50', LIST)?.id).toBe('idx_new_sector');
    expect(databaseOfTicker('NIFTYMIDCAP100', LIST)?.id).toBe('idx_new_sector'); // not listed: the contract's database for it
    expect(databaseOfTicker('BANKNIFTY FUT', LIST)?.id).toBe('futures_bhavcopy');
    expect(databaseOfTicker('basket:nse500:RELIANCE', LIST)?.id).toBe('cp_updated_2026');
    expect(databaseOfTicker('basket:ieodfo:BANKNIFTY', LIST)?.id).toBe('ieod_fo_2023');
    expect(databaseOfTicker('db:cp_old_2022:SBIN', LIST)?.id).toBe('cp_old_2022');
    expect(databaseOfTicker('ALL', LIST)).toBeNull();
    expect(databaseOfTicker('db:unknown:SBIN', LIST)).toBeNull();
  });
  it('marks the default database selected in the dropdown', () => {
    const html = databaseSelectHTML(databaseOptions(LIST, [{allowed_types: ['index']}], false), databaseOfTicker('NIFTY 50', LIST)!.id, 'strategy-db');
    const host = document.createElement('div'); host.innerHTML = html;
    const select = host.querySelector('select#strategy-db') as HTMLSelectElement;
    expect(select.value).toBe('idx_new_sector');
    expect(host.querySelector('[data-db-picker] .swl')?.textContent).toBe('Database');
  });
  it('keeps the same symbol when switching databases, else the first existing ticker', () => {
    expect(tickerForDatabase(byId('cp_old_2022'), 'basket:nse500:RELIANCE', 'RELIANCE')).toBe('db:cp_old_2022:RELIANCE');
    expect(tickerForDatabase(byId('idx_old_2022'), 'NIFTYBANK', 'NIFTYBANK')).toBe('db:idx_old_2022:NIFTYBANK');
    expect(tickerForDatabase(byId('idx_new_sector'), 'db:idx_old_2022:NIFTY 50')).toBe('NIFTY 50');
    expect(tickerForDatabase(byId('cp_updated_2026'), 'db:cp_old_2022:SBIN')).toBe('basket:nse500:RELIANCE');
    expect(tickerForDatabase(byId('ieod_fo_old'), 'NIFTY 50')).toBe('');
    expect(dbTickerParts('db:idx_old_2022:NIFTY 50')).toEqual({id: 'idx_old_2022', symbol: 'NIFTY 50'});
    expect(dbTickerParts('basket:nse500:SBIN')).toBeNull();
  });
});

describe('databases a strategy cannot use are disabled with the reason', () => {
  const names = Object.fromEntries(LIST.map(d => [d.id, d.name]));
  it('gives each blocking reason', () => {
    expect(databaseDisabledReason(byId('idx_volume_2026'), {allowed_types: ['index']}, false)).toBe('not copied to this PC');
    expect(databaseDisabledReason(byId('futures_bhavcopy'), {allowed_types: ['index']}, false)).toBe('This strategy does not run on futures data.');
    expect(databaseDisabledReason(byId('cp_updated_2026'), {allowed_types: ['individual_stock'], databases: ['cp_updated_2026', 'cp_old_2022']}, false)).toBe('');
    expect(databaseDisabledReason(byId('cp_updated_2026'), {allowed_types: ['individual_stock'], databases: ['cp_old_2022']}, false, names)).toBe('This strategy runs only on CP_OLD_2022.');
    expect(databaseDisabledReason(byId('idx_old_2022'), {allowed_types: ['index'], needs_volume: true}, false)).toBe('The rules need volume; this database has no volume.');
    expect(databaseDisabledReason(byId('idx_new_sector'), {allowed_types: ['index'], needs_volume: true}, false)).toBe('');
    expect(databaseDisabledReason(byId('ieod_fo_2023'), {allowed_types: ['futures']}, false)).toContain('intraday AFLs only');
    expect(databaseDisabledReason(byId('futures_bhavcopy'), {allowed_types: ['futures']}, true)).toContain('1-minute bars');
  });
  it('checks an old policy (no new fields) on its allowed types only, and an empty policy not at all', () => {
    expect(databaseDisabledReason(byId('cp_old_2022'), {allowed_types: ['individual_stock']}, false)).toBe('');
    expect(databaseDisabledReason(byId('idx_old_2022'), undefined, false)).toBe('');
    expect(databaseDisabledReason(byId('idx_old_2022'), {allowed_types: []}, false)).toBe('');
  });
  it('renders disabled options with the reason as the title, and an untested note on allowed ones', () => {
    const policy = {allowed_types: ['index', 'futures'], team_types: ['futures']};
    const host = document.createElement('div');
    host.innerHTML = databaseSelectHTML(databaseOptions(LIST, [policy], false), 'futures_bhavcopy', 'strategy-db');
    const opt = (id: string) => host.querySelector(`option[value="${id}"]`) as HTMLOptionElement;
    expect(opt('futures_bhavcopy').disabled).toBe(false);
    expect(opt('futures_bhavcopy').title).toBe('');
    expect(opt('idx_new_sector').disabled).toBe(false);
    expect(opt('idx_new_sector').title).toBe("Not checked against the team's runs on index data.");
    expect(opt('cp_old_2022').disabled).toBe(true);
    expect(opt('cp_old_2022').title).toBe('This strategy does not run on stock data.');
    expect(opt('ieod_fo_old').disabled).toBe(true);
    expect(opt('ieod_fo_old').title).toContain('only 2 symbols copied');
    expect(untestedNote(byId('futures_bhavcopy'), policy)).toBe('');
  });
  it('lists only intraday databases for an intraday strategy', () => {
    const opts = databaseOptions(LIST, [{allowed_types: ['futures'], requirement: 'intraday_futures', databases: ['ieod_fo_2023']}], true);
    expect(opts.map(o => o.db.id)).toEqual(['ieod_fo_2023', 'ieod_fo_old']);
    expect(opts.map(o => !!o.reason)).toEqual([false, true]);
  });
  it('enables a database for a collection when any of its strategies can use it', () => {
    const opts = databaseOptions(LIST, [{allowed_types: ['futures']}, {allowed_types: ['index']}], false);
    const reason = (id: string) => opts.find(o => o.db.id === id)!.reason;
    expect(reason('futures_bhavcopy')).toBe('');
    expect(reason('idx_new_sector')).toBe('');
    expect(reason('cp_old_2022')).toMatch(/^None of these strategies runs on this database\./);
  });
});

describe('result database badge and data note', () => {
  it('names the database of a db: ticker; existing tickers keep their own badge', () => {
    expect(dbBadgeInfo('db:cp_old_2022:SBIN', LIST)).toEqual({badge: 'CP_OLD_2022', name: 'CP_OLD_2022', share: 'CP_OLD_2022', use: 'Stock prices · Cash · 2000-01-03 to 2022-03-31'});
    expect(dbBadgeInfo('db:gone:SBIN', LIST)).toMatchObject({badge: 'gone'});
    expect(dbBadgeInfo('basket:nse500:SBIN', LIST)).toBeNull();
  });
  it('shows data_note as an escaped notice, nothing when absent', () => {
    const html = dataNoteHTML("Not checked against the team's runs on futures data.");
    const host = document.createElement('div'); host.innerHTML = html;
    expect(host.querySelector('p.rnote[data-data-note]')?.textContent).toBe("Not checked against the team's runs on futures data.");
    expect(dataNoteHTML('<b>x</b>')).toContain('&lt;b&gt;');
    for (const none of [undefined, null, '', '   ', 3]) expect(dataNoteHTML(none)).toBe('');
  });
});

describe('Kora spot special case is only for old engines', () => {
  const idx = [{ticker: 'NIFTY 50'}, {ticker: 'NIFTYBANK'}, {ticker: 'NIFTYMIDCAP100'}], fut = [{ticker: 'NIFTY FUT'}];
  it('keeps the spot restriction when the policy has no team_types', () => {
    expect(krSpotMethod('ytkr_buyshort_open_or_close', 'yt_kr', {allowed_types: ['index']})).toBe(true);
  });
  it('uses the widened allowed_types when the engine sends team_types', () => {
    const policy = {allowed_types: ['index', 'futures'], team_types: ['index'], databases: null};
    expect(krSpotMethod('ytkr_buyshort_open_or_close', 'yt_kr', policy)).toBe(false);
    const controls = instrumentControls(policy, 'ytkr_buyshort_open_or_close', 'yt_kr');
    expect([controls.index, controls.futures]).toEqual([true, true]);
    expect(instrumentTickers(policy, idx, [], 'ytkr_buyshort_open_or_close', 'yt_kr', fut).map(t => t.ticker)).toEqual(['NIFTY 50', 'NIFTYBANK', 'NIFTYMIDCAP100', 'NIFTY FUT']);
  });
});

// Type check: the parsed list is the exported shape.
const _shape: YtDatabase[] = LIST; void _shape;
