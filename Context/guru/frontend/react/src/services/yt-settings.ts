/**
 * YT strategy settings shown as an AmiBroker-style properties panel: settings grouped under headed sections, choice
 * settings as dropdowns. Presentation only; the engine's parameter keys, values and defaults are unchanged.
 *
 * Choices come from the label. "Name::A|B|C" lists values 1, 2, 3 (the YT Holidays event list). Otherwise a trailing
 * bracket that numbers every value from min to max, e.g. "Days exit (0 = off, 1 = on)" or
 * "Filter timeframe (0 monthly, 1 weekly, 2 daily)", becomes a dropdown; text after ":" in an item is a note.
 */

export interface YtSetting { key: string; label: string; def: number; min: number; max: number; step: number }
export interface YtOption { value: number; text: string }
export interface YtGroup { name: string; settings: YtSetting[] }

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

/** The last top-level bracket of a label: its text and where it starts, or null. */
function lastBracket(label: string): { start: number; inner: string } | null {
  if (!label.trimEnd().endsWith(")")) return null;
  const end = label.lastIndexOf(")");
  let depth = 0;
  for (let i = end; i >= 0; i--) {
    if (label[i] === ")") depth++;
    else if (label[i] === "(" && --depth === 0) return { start: i, inner: label.slice(i + 1, end) };
  }
  return null;
}

/** Split on commas and semicolons outside nested brackets. */
function topLevelItems(text: string): string[] {
  const out: string[] = [];
  let depth = 0, cur = "";
  for (const ch of text) {
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
    if ((ch === "," || ch === ";") && depth === 0) { out.push(cur); cur = ""; } else cur += ch;
  }
  out.push(cur);
  return out.map(s => s.trim()).filter(Boolean);
}

function bracketOptions(s: YtSetting): YtOption[] | null {
  const b = lastBracket(s.label);
  if (!b || !Number.isInteger(s.min) || !Number.isInteger(s.max) || s.step !== 1) return null;
  const items: [number, string][] = [];
  for (const item of topLevelItems(b.inner)) {
    const m = item.match(/^(-?\d+)\s*(?:=\s*)?(.+)$/);
    if (m) items.push([+m[1], m[2]]);
    else if (items.length) items[items.length - 1][1] += ", " + item; // "3 average of mid point, pivot and close"
  }
  const opts: YtOption[] = [];
  for (const [value, raw] of items) {
    const text = raw.split(":")[0].trim();
    if (!text || /^\.\./.test(text)) return null;
    opts.push({ value, text: cap(text) });
  }
  if (opts.length < 2 || new Set(opts.map(o => o.value)).size !== opts.length) return null;
  for (let v = s.min; v <= s.max; v++) if (!opts.some(o => o.value === v)) return null;
  return opts.filter(o => o.value >= s.min && o.value <= s.max).sort((a, b) => a.value - b.value);
}

/** The values of a choice setting with their names, or [] for a number setting. */
export function settingOptions(s: YtSetting): YtOption[] {
  const listed = s.label.split("::")[1];
  if (listed !== undefined) return listed.split("|").filter(Boolean).map((text, i) => ({ value: i + 1, text }));
  return bracketOptions(s) ?? [];
}

/** The label without its list of choices. */
export function settingName(s: YtSetting): string {
  const name = s.label.split("::")[0];
  if (s.label.includes("::")) return name;
  const b = bracketOptions(s) && lastBracket(name);
  return b ? name.slice(0, b.start).trim() : name;
}

/** The name of a value: the choice text for a choice setting, the number otherwise. */
export function settingValueText(s: YtSetting, v: number): string {
  return settingOptions(s).find(o => o.value === Math.round(v))?.text ?? String(v);
}

/** A two-way switch whose 0 means off (Off/On, No/Yes). */
export function isSwitch(s: YtSetting): boolean {
  const o = settingOptions(s).map(x => x.text.toLowerCase());
  return o.length === 2 && settingOptions(s)[0].value === 0 && ((o[0] === "off" && o[1] === "on") || (o[0] === "no" && o[1] === "yes"));
}

/** Panel sections in display order, matched on the setting name (first match wins, entry wording before exits). */
const GROUPS: [string, (n: string, k: string) => boolean][] = [
  ["Entry", n => /^entry|^event|days from|entry side|^buy |^sell |weekday(?! exit)|crossover|cross over/i.test(n)],
  ["Dual ATR exit", n => /^atr\b|(initial|favour|fresh|dual) atr|atr exit/i.test(n)],
  ["Filter", (n, k) => /filter|base ?2|regime|dead stock|penny|turnover|circuit|liquidity|\badx\b/i.test(n) || /^flt_|^mkt_/.test(k)],
  ["Stop loss", (n, k) => /stop|\bsl\b|\bhsl\b|\btsl\b|gap/i.test(n) || /^hsl|^sl_/.test(k)],
  ["Candle exit", (n, k) => /candle|up ?close|down close/i.test(n) && /exit|\bn\b|count|type/i.test(n) || /^nc_/.test(k)],
  ["Exit", n => /exit|target|profit booking|\bhold/i.test(n)],
  ["Entry", n => /entry/i.test(n)],
];
export const GROUP_ORDER = ["Entry", "Filter", "Candle exit", "Dual ATR exit", "Exit", "Stop loss", "Strategy settings"];

export function settingGroup(s: YtSetting): string {
  const n = settingName(s);
  return GROUPS.find(([, test]) => test(n, s.key))?.[0] ?? "Strategy settings";
}

/** Settings in panel sections; sections in GROUP_ORDER, settings in engine order within a section. */
export function groupSettings(ps: YtSetting[]): YtGroup[] {
  const by = new Map<string, YtSetting[]>();
  for (const s of ps) { const g = settingGroup(s); by.set(g, [...(by.get(g) ?? []), s]); }
  return GROUP_ORDER.filter(g => by.has(g)).map(name => ({ name, settings: by.get(name)! }));
}

/**
 * Keys made idle by a section's switch: when a section's first setting is an Off/On switch set to Off, the rest of
 * that section has no effect on the run. Used only to grey those rows; they stay editable.
 */
export function idleKeys(groups: YtGroup[], value: (key: string) => number): Set<string> {
  const idle = new Set<string>();
  for (const g of groups) {
    const [head, ...rest] = g.settings;
    if (head && rest.length && isSwitch(head) && Math.round(value(head.key)) === 0) rest.forEach(s => idle.add(s.key));
  }
  return idle;
}
