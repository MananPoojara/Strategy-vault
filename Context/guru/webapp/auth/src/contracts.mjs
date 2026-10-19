const THEMES = new Set(['steel', 'aqua', 'lagoon', 'glacier', 'midnight', 'forest', 'sand', 'plum', 'cobalt']);
export const ROLES = new Set(['admin', 'researcher', 'viewer']);
export function validateNoteKey(key) {
  if (typeof key !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,79}$/.test(key)) throw new Error('Invalid note identifier');
  return key;
}
export function validateTheme(theme) {
  if (!THEMES.has(theme)) throw new Error('Invalid theme');
  return theme;
}
export function roleAllows(role, action) {
  if (!ROLES.has(role)) return false;
  if (action === 'read') return true;
  if (action === 'write' || action === 'optimize') return role === 'admin' || role === 'researcher';
  return action === 'admin' && role === 'admin';
}
export function safeReturnTo(path) {
  if (typeof path !== 'string' || !path.startsWith('/') || path.startsWith('//') || path.includes('\\') || /[\x00-\x1f\x7f]/.test(path) || path.startsWith('/login.html')) return '/';
  return path;
}

export function validateSavedView(value) {
  if (typeof value !== 'string' || value.length > 4096 || /[\x00-\x1f\x7f]/.test(value) || !/^#\/(?:strategies|guru\/[a-z0-9_]+|[so]\/[A-Za-z0-9_.%~-]+)?(?:\?[^#]*)?$/.test(value)) throw new Error('Invalid saved view');
  try { decodeURIComponent(value.split('?')[0]); } catch { throw new Error('Invalid saved view'); }
  return value;
}

export function validateAnnotation(body, role) {
  if (!body || !Array.isArray(body.tags) || body.tags.length > 12 || body.tags.some(t => typeof t !== 'string' || !t.trim() || t.length > 40 || /[\x00-\x1f\x7f]/.test(t))) throw new Error('Invalid annotation');
  if (body.ceo !== undefined && (typeof body.ceo !== 'boolean' || !roleAllows(role, 'admin'))) throw new Error('Invalid recommendation permission');
  return { tags: [...new Set(body.tags.map(t => t.trim()))], ...(body.ceo !== undefined ? { ceo: body.ceo } : {}) };
}

// Indicators lab saved strategies (spec §3). Full rule validation stays in the engine.
export class LabRequestError extends Error {
  constructor(message, status = 400) { super(message); this.status = status; }
}
export const LAB_MARKETS = new Set(['in', 'us', 'gl', 'crypto']);
// Global and crypto are provisional TradingView exports: internal research only until licensed data exists.
export const LAB_PUBLISHABLE_MARKETS = new Set(['in', 'us']);
// Portfolio rules use undated member lists; even malformed reserved fields stay research-only.
export function labPublishable(row) { return LAB_PUBLISHABLE_MARKETS.has(row.market) && !Object.hasOwn(row.spec ?? {}, 'portfolio'); }
export const LAB_STATUSES = new Set(['draft', 'submitted', 'published', 'rejected']);
const LAB_SPEC_BYTES = 8192;
const CONTROL = /[\x00-\x1f\x7f]/;
const NOTE_CONTROL = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/;

export function validateLabStrategy(body) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) throw new LabRequestError('Provide a strategy name, rule spec, market and instrument.');
  const name = typeof body.name === 'string' ? body.name.trim() : '';
  if (!name || name.length > 120 || CONTROL.test(name)) throw new LabRequestError('Name must be 1–120 characters.');
  const spec = body.spec;
  if (!spec || typeof spec !== 'object' || Array.isArray(spec)) throw new LabRequestError('Rule spec must be a JSON object.');
  const json = JSON.stringify(spec);
  if (Buffer.byteLength(json) > LAB_SPEC_BYTES) throw new LabRequestError('Rule spec is larger than 8 KB.');
  if (json.includes('\\u0000')) throw new LabRequestError('Rule spec cannot contain NUL characters.'); // PostgreSQL jsonb rejects \u0000
  if (spec.v !== 1) throw new LabRequestError('Rule spec version must be 1.');
  for (const part of ['entry', 'exit']) {
    if (!spec[part] || typeof spec[part] !== 'object' || Array.isArray(spec[part])) throw new LabRequestError(`Rule spec needs ${part} rules.`);
  }
  const origin = validateLabOrigin(spec.origin);
  if (!LAB_MARKETS.has(body.market)) throw new LabRequestError('Market must be in, us, gl or crypto.');
  const instrument = typeof body.instrument === 'string' ? body.instrument.trim() : '';
  if (!instrument || instrument.length > 64 || CONTROL.test(instrument)) throw new LabRequestError('Instrument must be 1–64 characters.');
  return { name, spec, market: body.market, instrument, ...(origin ? { origin } : {}) };
}

// Customize (spec 2026-10-16 §7): spec.origin names the book template a variant came from. Shape only here;
// the engine checks that the template exists and keeps the book method's instrument rule.
const ORIGIN_ID = /^[A-Za-z0-9_]{1,80}$/;
export function validateLabOrigin(origin) {
  if (origin === undefined) return null;
  const ok = origin && typeof origin === 'object' && !Array.isArray(origin) && Object.keys(origin).sort().join() === 'book,method,template_version'
    && typeof origin.book === 'string' && typeof origin.method === 'string' && ORIGIN_ID.test(origin.book) && ORIGIN_ID.test(origin.method) && Number.isSafeInteger(origin.template_version) && origin.template_version >= 1 && origin.template_version <= 1000;
  if (!ok) throw new LabRequestError('Rule spec origin must be {book, method, template_version} from a book template.');
  return { book: origin.book, method: origin.method, template_version: origin.template_version };
}
export function sameLabOrigin(a, b) {
  return (a?.book ?? null) === (b?.book ?? null) && (a?.method ?? null) === (b?.method ?? null) && (a?.template_version ?? null) === (b?.template_version ?? null);
}

// Custom indicators (spec 2026-10-19 §3): the engine parses the formula; here only sizes and characters.
export function validateLabIndicator(body) {
  const name = typeof body?.name === 'string' ? body.name.trim() : '';
  if (!name || name.length > 60 || CONTROL.test(name)) throw new LabRequestError('Name must be 1–60 characters.');
  const formula = body.formula;
  if (typeof formula !== 'string' || !formula.trim() || formula.length > 4000) throw new LabRequestError('Formula must be 1–4000 characters.');
  if (NOTE_CONTROL.test(formula)) throw new LabRequestError('Formula cannot contain control characters other than tabs and line breaks.'); // PostgreSQL text rejects NUL
  const blocks = body.blocks ?? null;
  if (blocks !== null) {
    if (typeof blocks !== 'object' || Array.isArray(blocks)) throw new LabRequestError('Blocks must be a JSON object or null.');
    const json = JSON.stringify(blocks);
    if (Buffer.byteLength(json) > 65536) throw new LabRequestError('Blocks are larger than 64 KB.');
    if (json.includes('\\u0000')) throw new LabRequestError('Blocks cannot contain NUL characters.'); // PostgreSQL jsonb rejects \u0000
  }
  return { name, formula, blocks };
}

export function validateLabVersion(version) {
  if (!Number.isSafeInteger(version) || version < 1) throw new LabRequestError('Provide the version you are changing.');
  return version;
}

export function validateLabNote(note, required) {
  if (note === undefined || note === null || (note === '' && !required)) {
    if (required) throw new LabRequestError('Give a reason for the rejection (1–1000 characters).');
    return null;
  }
  const text = typeof note === 'string' ? note.trim() : '';
  if (!text || text.length > 1000 || NOTE_CONTROL.test(text)) throw new LabRequestError('Review note must be 1–1000 characters.');
  return text;
}

export function labCanRead(row, user) {
  return Boolean(row) && (row.owner === user.id || row.status === 'published' || roleAllows(user.role, 'admin'));
}

const LAB_ACTIONS = {
  duplicate: { role: 'write', to: 'draft' },
  update: { role: 'write', owner: true, from: ['draft', 'rejected'], to: 'draft', verb: 'edited' },
  delete: { role: 'write', owner: true, from: ['draft', 'rejected'], verb: 'deleted' },
  submit: { role: 'write', owner: true, from: ['draft', 'rejected'], to: 'submitted', verb: 'submitted', publishable: true },
  withdraw: { role: 'write', owner: true, from: ['submitted'], to: 'draft', verb: 'withdrawn' },
  publish: { role: 'admin', from: ['submitted'], to: 'published', verb: 'published', publishable: true },
  reject: { role: 'admin', from: ['submitted'], to: 'rejected', verb: 'rejected' },
};

// Decides one action on a locked row; returns the next status or throws with the HTTP status to send.
export function labAction(row, user, action, version) {
  if (!Object.hasOwn(LAB_ACTIONS, action)) throw new LabRequestError('unknown action', 400);
  const rule = LAB_ACTIONS[action];
  if (!labCanRead(row, user)) throw new LabRequestError('Strategy not found.', 404);
  if (!roleAllows(user.role, rule.role)) throw new LabRequestError(rule.role === 'admin' ? 'Administrator access required' : 'Researcher access required', 403);
  if (action === 'duplicate' && row.owner !== user.id && row.status !== 'published') throw new LabRequestError('Only the owner can duplicate an unpublished strategy', 403);
  if (rule.owner && row.owner !== user.id) throw new LabRequestError('Only the owner can change this strategy.', 403);
  if (rule.from && !rule.from.includes(row.status)) throw new LabRequestError(`A ${row.status} strategy cannot be ${rule.verb}.`, 409);
  if (action !== 'duplicate' && (version === undefined || version === null)) throw new LabRequestError('version required', 400);
  if (version !== undefined && version !== null && version !== row.version) throw new LabRequestError(`This strategy changed since you opened it (now version ${row.version}). Reload before saving.`, 409);
  if (rule.publishable && !LAB_PUBLISHABLE_MARKETS.has(row.market)) throw new LabRequestError('Global and crypto data are provisional TradingView exports; strategies on them cannot go to YT Strategies until licensed data replaces them.', 400);
  if (rule.publishable && !labPublishable(row)) throw new LabRequestError("Portfolio strategies use today's basket lists, not dated index membership (survivorship bias); they stay research-only until historical membership is admitted.", 400);
  return rule.to ?? row.status;
}
