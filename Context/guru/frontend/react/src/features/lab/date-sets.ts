/** Custom date filters (Base 1 / Base 2 windows): parsing uploads and pastes, and keeping a strategy's windows current. */
import type { DateSet, DateWindow, Spec } from "./types";

/** rules.rs MAX_DATE_WINDOWS, auth validateLabDateSet. */
export const MAX_WINDOWS = 200;
const real = (y: number, m: number, d: number) => {
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d;
};
/** "YYYY-MM-DD" from YYYY-MM-DD, or day-first DD-MM-YYYY / DD/MM/YYYY / DD.MM.YYYY (the AmiBroker AFL lists); undefined if not a real date. */
export function isoDate(text: string): string | undefined {
  const t = text.trim().replace(/^["']|["']$/g, "");
  let m = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(t), y: number, mo: number, d: number;
  if (m) [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  else if ((m = /^(\d{1,2})[-/.](\d{1,2})[-/.](\d{4})$/.exec(t))) [d, mo, y] = [Number(m[1]), Number(m[2]), Number(m[3])];
  else return undefined;
  return real(y, mo, d) ? `${y}-${String(mo).padStart(2, "0")}-${String(d).padStart(2, "0")}` : undefined;
}

export interface Parsed { windows: DateWindow[]; errors: string[] }
const check = (windows: DateWindow[], errors: string[]): Parsed => {
  windows.forEach((w, i) => { if (!(w.from < w.to)) errors.push(`Window ${i + 1}: the start (${w.from}) must be before the exit date (${w.to}), the first day outside the window.`); });
  if (!windows.length && !errors.length) errors.push("No date windows found.");
  if (windows.length > MAX_WINDOWS) errors.push(`${windows.length} windows; the limit is ${MAX_WINDOWS}.`);
  return { windows, errors };
};
/** One window per line: "from, to[, label]", separated by commas, tabs (cells pasted from Excel) or semicolons.
 *  A first line without dates (a header) is skipped; blank lines are ignored. */
export function parseDateTable(text: string): Parsed {
  const windows: DateWindow[] = [], errors: string[] = [];
  text.split(/\r?\n/).forEach((line, i) => {
    if (!line.trim()) return;
    const cells = line.split(/[,\t;]/).map((c) => c.trim());
    const [from, to] = [isoDate(cells[0] ?? ""), isoDate(cells[1] ?? "")];
    if (i === 0 && !from && !to) return; // header
    if (!from || !to) { errors.push(`Line ${i + 1}: expected "start date, end date[, label]", got "${line.trim().slice(0, 60)}".`); return; }
    const label = cells.slice(2).join(", ").trim();
    windows.push(label ? { from, to, label: label.slice(0, 60) } : { from, to });
  });
  return check(windows, errors);
}
/** The AmiBroker AFL form: one list of start dates and one of end dates (e.g. BASE15DP_EntryDates / BASE1_ExitDates),
 *  paired in order. Quotes, `Name =` and a trailing `;` are ignored. */
export function parseDateLists(starts: string, ends: string): Parsed {
  const list = (s: string) => s.replace(/^[^"=]*=\s*/, "").replace(/[";]/g, "").split(/[,\s]+/).filter(Boolean);
  const [a, b] = [list(starts), list(ends)], errors: string[] = [];
  if (a.length !== b.length) errors.push(`${a.length} start dates but ${b.length} end dates; they are paired in order.`);
  const windows: DateWindow[] = [];
  a.slice(0, Math.min(a.length, b.length)).forEach((s, i) => {
    const [from, to] = [isoDate(s), isoDate(b[i])];
    if (!from || !to) errors.push(`Pair ${i + 1}: "${s}" / "${b[i]}" is not a date.`);
    else windows.push({ from, to });
  });
  return check(windows, errors);
}
/** Spec windows ([from, to] pairs) from a saved set's windows. */
export const pairs = (w: readonly DateWindow[]): [string, string][] => w.map(({ from, to }) => [from, to]);
export const span = (w: DateWindow[]) => (w.length ? `${w.length} window${w.length === 1 ? "" : "s"}, ${[...w].sort((x, y) => (x.from < y.from ? -1 : 1))[0].from} to ${[...w].sort((x, y) => (x.to < y.to ? 1 : -1))[0].to}` : "no windows");
const same = (a: [string, string][], b: readonly DateWindow[]) => JSON.stringify(a) === JSON.stringify(pairs(b));
/** The spec with its date filter's windows (and name) taken from the saved set it names, so a re-uploaded set reaches every
 *  strategy that uses it. The same spec object when nothing changed or the set is not loaded (e.g. deleted). */
export function withCurrentDateSet(spec: Spec, sets: readonly DateSet[]): Spec {
  const df = spec.date_filter, set = df?.set?.id !== undefined ? sets.find((s) => s.id === df.set!.id) : undefined;
  if (!df || !set || (same(df.windows, set.windows) && df.set?.name === set.name)) return spec;
  return { ...spec, date_filter: { ...df, set: { id: set.id, name: set.name }, windows: pairs(set.windows) } };
}
