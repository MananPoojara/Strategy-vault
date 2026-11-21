/** Team databases a YT run can use (GET {api}/guru/yt/databases, DB_CONTRACT 20-11-2026).
 * Pure helpers: parsing, which database a ticker belongs to, why a strategy cannot use a database, and the
 * small notices. The runtime falls back to its existing controls when the list is missing (old engines). */
import type {InstrumentPolicy} from './instrument-controls';

export interface YtDbTicker { ticker: string; symbol: string }
export interface YtDatabase {
  id: string; name: string; kind: string; intraday: boolean; prices: string; first: string; last: string;
  has_volume: boolean; has_aux: boolean; available: boolean; note: string; default: boolean; tickers: YtDbTicker[];
}
export interface DatabasePolicy extends InstrumentPolicy {
  team_types?: string[]; databases?: string[] | null; needs_volume?: boolean;
}

const str = (v: unknown) => typeof v === 'string' ? v : '';

/** The API body as a list of databases, or null when the body is not a database list (so callers fall back). */
export function parseDatabases(body: unknown): YtDatabase[] | null {
  const list = body && typeof body === 'object' && !Array.isArray(body) ? (body as {databases?: unknown}).databases : null;
  if (!Array.isArray(list)) return null;
  const out: YtDatabase[] = [];
  for (const raw of list) {
    if (!raw || typeof raw !== 'object') continue;
    const r = raw as Record<string, unknown>, id = str(r.id);
    if (!id) continue;
    const kind = str(r.kind), intraday = r.intraday === true || kind === 'intraday_futures';
    const tickers = (Array.isArray(r.tickers) ? r.tickers : []).flatMap(t => {
      const row = t && typeof t === 'object' ? t as Record<string, unknown> : null, ticker = str(row?.ticker);
      return ticker ? [{ticker, symbol: str(row?.symbol) || ticker.split(':').at(-1) || ticker}] : [];
    });
    out.push({id, name: str(r.name) || id, kind: kind === 'intraday_futures' ? 'futures' : kind, intraday, prices: str(r.prices),
      first: str(r.first), last: str(r.last), has_volume: r.has_volume !== false, has_aux: r.has_aux === true,
      available: r.available !== false, note: str(r.note), default: r.default === true, tickers});
  }
  return out.length ? out : null;
}

/** `db:<id>:<SYMBOL>` → its parts; existing tickers → null. */
export function dbTickerParts(ticker?: string): {id: string; symbol: string} | null {
  const m = /^db:([^:]+):(.+)$/.exec(String(ticker || ''));
  return m ? {id: m[1], symbol: m[2]} : null;
}

/** Existing tickers keep their database (the contract's `default: true` rows). */
const LEGACY_BASKET_DB: Record<string, string> = {nse500: 'cp_updated_2026', allfo: 'fo_updated_2024', ieodfo: 'ieod_fo_2023'};
const LEGACY_INDICES = ['NIFTY 50', 'NIFTYBANK', 'NIFTYMIDCAP100'];
export function legacyDatabaseId(ticker?: string): string {
  const t = String(ticker || '');
  if (t.startsWith('basket:')) return LEGACY_BASKET_DB[t.split(':')[1]] || '';
  if (/ FUT$/.test(t)) return 'futures_bhavcopy';
  return LEGACY_INDICES.includes(t) ? 'idx_new_sector' : '';
}

/** The database a ticker comes from: its `db:` id, the database listing it (a default one first), or the
 * contract's database for an existing ticker. */
export function databaseOfTicker(ticker: string | undefined, list: YtDatabase[] | null): YtDatabase | null {
  if (!list || !ticker) return null;
  const parts = dbTickerParts(ticker);
  if (parts) return list.find(d => d.id === parts.id) || null;
  const listing = list.filter(d => d.tickers.some(t => t.ticker === ticker));
  const legacy = legacyDatabaseId(ticker);
  return listing.find(d => d.default) || listing.find(d => d.id === legacy) || listing[0] || list.find(d => d.id === legacy) || null;
}

export const KIND_LABEL: Record<string, string> = {index: 'index', individual_stock: 'stock', futures: 'futures', etf: 'ETF'};
const kindLabel = (k: string) => KIND_LABEL[k] || k || 'this';

/** Why this strategy cannot run on this database ('' when it can). Old policies without the new fields
 * are checked on what they have. */
export function databaseDisabledReason(db: YtDatabase, policy: DatabasePolicy | undefined, intradayStrategy: boolean, names: Record<string, string> = {}): string {
  if (!db.available) return db.note || 'This database is not available on this PC.';
  if (intradayStrategy && !db.intraday) return 'This intraday AFL needs 1-minute bars; this database has daily bars only.';
  if (!intradayStrategy && db.intraday) return 'A 1-minute intraday database: used by the intraday AFLs only.';
  const types = policy?.allowed_types || [];
  if (types.length && !types.includes(db.kind)) return `This strategy does not run on ${kindLabel(db.kind)} data.`;
  if (Array.isArray(policy?.databases) && !policy!.databases.includes(db.id))
    return `This strategy runs only on ${policy!.databases.map(id => names[id] || id).join(', ')}.`;
  if (policy?.needs_volume && !db.has_volume) return 'The rules need volume; this database has no volume.';
  return '';
}

/** Allowed but never run by the team on this kind of data (shown as the option's title). */
export function untestedNote(db: YtDatabase, policy: DatabasePolicy | undefined): string {
  const team = policy?.team_types;
  return Array.isArray(team) && team.length && !team.includes(db.kind) ? `Not checked against the team's runs on ${kindLabel(db.kind)} data.` : '';
}

export interface DatabaseOption { db: YtDatabase; reason: string; note: string }
/** The dropdown: intraday strategies list only intraday databases. `policies` holds one policy for a strategy
 * page, or every strategy of a collection (a database is usable when any of them can use it). */
export function databaseOptions(list: YtDatabase[] | null, policies: (DatabasePolicy | undefined)[], intraday: boolean): DatabaseOption[] {
  if (!list) return [];
  const names = Object.fromEntries(list.map(d => [d.id, d.name]));
  const all = policies.length ? policies : [undefined];
  return list.filter(d => !intraday || d.intraday).map(db => {
    const reasons = all.map(p => databaseDisabledReason(db, p, intraday, names)), ok = reasons.some(r => !r);
    const reason = ok ? '' : all.length > 1 ? `None of these strategies runs on this database. ${reasons[0]}` : reasons[0];
    return {db, reason, note: ok && all.length === 1 ? untestedNote(db, all[0]) : ''};
  });
}

/** The ticker to open when a database is picked: the same symbol there, else its first existing ticker, else its first. */
export function tickerForDatabase(db: YtDatabase, current?: string, currentSymbol?: string): string {
  if (db.tickers.some(t => t.ticker === current)) return current!;
  const sym = currentSymbol || dbTickerParts(current)?.symbol || String(current || '').split(':').at(-1);
  return (db.tickers.find(t => t.symbol === sym) || db.tickers.find(t => !t.ticker.startsWith('db:')) || db.tickers[0])?.ticker || '';
}

const escHtml = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;'}[c]!));

/** The Database dropdown (same field style as the Stock picker). */
export function databaseSelectHTML(options: DatabaseOption[], selected: string, id: string): string {
  if (!options.length) return '';
  return `<label class="dfield stock-picker" data-db-picker><span class="swl">Database</span><select id="${escHtml(id)}" aria-label="Database">${options.map(({db, reason, note}) =>
    `<option value="${escHtml(db.id)}" ${db.id === selected ? 'selected' : ''} ${reason ? 'disabled' : ''} ${reason || note ? `title="${escHtml(reason || note)}"` : ''}>${escHtml(db.name)}</option>`).join('')}</select></label>`;
}

/** `data_note` from readiness / run / summary ("Not checked against the team's runs on … data."). */
export function dataNoteHTML(note?: unknown): string {
  const text = typeof note === 'string' ? note.trim() : '';
  return text ? `<p class="rnote" data-data-note>${escHtml(text)}</p>` : '';
}

/** Badge / reference fields for a `db:<id>:SYM` ticker (the existing tickers keep their own badge). */
export function dbBadgeInfo(ticker: string | undefined, list: YtDatabase[] | null) {
  const parts = dbTickerParts(ticker);
  if (!parts) return null;
  const db = list?.find(d => d.id === parts.id), name = db?.name || parts.id, kind = db ? KIND_LABEL[db.kind] || db.kind : '';
  const use = [kind && `${kind[0].toUpperCase()}${kind.slice(1)} prices`, db?.prices, db?.first && db?.last ? `${db.first} to ${db.last}` : ''].filter(Boolean).join(' · ');
  return {badge: name, name, share: name, use: use || 'Team database'};
}
