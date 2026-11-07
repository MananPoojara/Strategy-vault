/**
 * Rule lines with the strategy's current parameter values filled in (YT BTST rules from the engine).
 * Line text (after the `F|`/`E|`/`X|` section) may start with a guard `?<key><op><num>|` (op `>`, `=`, `!=`):
 * the line is shown only when the guard holds. Placeholders: `{key}` the number, `{key:t}` HHMM as HH:MM,
 * `{key:opt:A/B/C}` the option named by the 1-based value. Lines without guards or placeholders pass unchanged.
 */
export type ParamValues = Record<string, number>;

const GUARD = /^\?([a-z_][a-z0-9_]*)(>|=|!=)(-?\d+(?:\.\d+)?)\|/i;
const TOKEN = /\{([a-z_][a-z0-9_]*)(?::(t|opt:[^}]*))?\}/gi;

const num = (x: number) => Number.isInteger(x) ? String(x) : String(+x.toFixed(4));
const hhmm = (x: number) => { const v = Math.round(x); return `${String(Math.floor(v / 100)).padStart(2, '0')}:${String(v % 100).padStart(2, '0')}`; };

/** The rule text with values filled in, or null when its guard is false. Unknown keys are left as written. */
export function renderRule(text: string, values: ParamValues): string | null {
  let body = text;
  const g = GUARD.exec(body);
  if (g) {
    const [, key, op, raw] = g, v = values[key], n = +raw;
    if (v === undefined || !Number.isFinite(v)) return body.slice(g[0].length);
    const ok = op === '>' ? v > n : op === '=' ? v === n : v !== n;
    if (!ok) return null;
    body = body.slice(g[0].length);
  }
  return body.replace(TOKEN, (all, key: string, fmt?: string) => {
    const v = values[key];
    if (v === undefined || !Number.isFinite(v)) return all;
    if (fmt === 't') return hhmm(v);
    if (fmt?.startsWith('opt:')) return fmt.slice(4).split('/')[Math.round(v) - 1] ?? num(v);
    return num(v);
  });
}

/** Defaults from a parameter schema, overridden by the page's values. */
export function paramValues(schema: {key: string; def: number}[] | undefined, overrides: ParamValues): ParamValues {
  const out: ParamValues = {};
  for (const p of schema || []) out[p.key] = p.def;
  return {...out, ...overrides};
}
