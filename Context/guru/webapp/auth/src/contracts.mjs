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
  if (typeof value !== 'string' || value.length > 4096 || /[\x00-\x1f\x7f]/.test(value) || !/^#\/(?:strategies|research|guru\/[a-z0-9_]+|[so]\/[A-Za-z0-9_.%~-]+)?(?:\?[^#]*)?$/.test(value)) throw new Error('Invalid saved view');
  try { decodeURIComponent(value.split('?')[0]); } catch { throw new Error('Invalid saved view'); }
  return value;
}

export function validateAnnotation(body, role) {
  if (!body || !Array.isArray(body.tags) || body.tags.length > 12 || body.tags.some(t => typeof t !== 'string' || !t.trim() || t.length > 40 || /[\x00-\x1f\x7f]/.test(t))) throw new Error('Invalid annotation');
  if (body.ceo !== undefined && (typeof body.ceo !== 'boolean' || !roleAllows(role, 'admin'))) throw new Error('Invalid recommendation permission');
  return { tags: [...new Set(body.tags.map(t => t.trim()))], ...(body.ceo !== undefined ? { ceo: body.ceo } : {}) };
}
