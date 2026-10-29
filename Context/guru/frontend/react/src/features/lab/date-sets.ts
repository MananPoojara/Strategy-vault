/** Custom date filters (Base 1 / Base 2 windows): parsing uploads and pastes, and keeping a strategy's windows current. */
import type { DateSet, DateWindow, Spec } from "./types";

/** rules.rs MAX_DATE_WINDOWS, auth validateLabDateSet. */
export const MAX_WINDOWS = 200;
const real = (y: number, m: number, d: number) => {
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d;
};
const MONTHS = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
/** 1–12 for a month name or its short form ("Nov", "Sept"), else undefined. */
const monthOf = (word: string) => {
  const w = word.toLowerCase().replace(/\.$/, ""), i = MONTHS.findIndex((m) => m.startsWith(w.slice(0, 3)));
  return w.length >= 3 && i >= 0 && (MONTHS[i].startsWith(w) || w === "sept") ? i + 1 : undefined;
};
const ymd = (y: number, m: number, d: number) => (real(y, m, d) ? `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}` : undefined);
const fullYear = (t: string) => (t.length === 2 ? (Number(t) < 70 ? 2000 : 1900) + Number(t) : Number(t));
/** Excel stores a date as days since 30-12-1899 (39773 = 21-11-2008). */
const EXCEL_DAY0 = Date.UTC(1899, 11, 30);
/** "YYYY-MM-DD" from any usual way of writing a date, undefined if it is not a real date. Numbers with the year last are read
 *  day first (dd/mm/yyyy, the AmiBroker AFL lists): 21-11-2008, 21/11/08, 21.11.2008; also 2008-11-21, 20081121, 21 Nov 2008,
 *  21-Nov-08, November 21, 2008, Fri 21 Nov 2008, a time after the date, and Excel day numbers (39773). */
export function isoDate(text: string): string | undefined {
  const t = text.trim().replace(/^["']|["']$/g, "").replace(/^(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?,?\s+/i, "")
    .replace(/(?:[T\s]+)\d{1,2}:\d{2}(?::\d{2}(?:\.\d+)?)?\s*(?:am|pm)?\s*Z?$/i, "").replace(/(\d)(?:st|nd|rd|th)\b/i, "$1").trim();
  let m: RegExpExecArray | null;
  if ((m = /^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/.exec(t))) return ymd(+m[1], +m[2], +m[3]);
  if ((m = /^(\d{1,2})[-/. ](\d{1,2})[-/. ](\d{4}|\d{2})$/.exec(t))) return ymd(fullYear(m[3]), +m[2], +m[1]);
  if ((m = /^((?:19|20)\d{2})(\d{2})(\d{2})$/.exec(t))) return ymd(+m[1], +m[2], +m[3]);
  if ((m = /^(\d{1,2})[-/. ]*([a-z]{3,9}\.?)[-/., ]*(\d{4}|\d{2})$/i.exec(t)) && monthOf(m[2])) return ymd(fullYear(m[3]), monthOf(m[2])!, +m[1]);
  if ((m = /^([a-z]{3,9}\.?)[-/. ]*(\d{1,2}),?[-/. ]*(\d{4})$/i.exec(t)) && monthOf(m[1])) return ymd(+m[3], monthOf(m[1])!, +m[2]);
  if ((m = /^(\d{4})[-/. ]*([a-z]{3,9}\.?)[-/. ]*(\d{1,2})$/i.exec(t)) && monthOf(m[2])) return ymd(+m[1], monthOf(m[2])!, +m[3]);
  if ((m = /^(\d{5})(?:\.\d+)?$/.exec(t)) && +m[1] >= 20000 && +m[1] <= 80000) {
    const d = new Date(EXCEL_DAY0 + +m[1] * 864e5);
    return ymd(d.getUTCFullYear(), d.getUTCMonth() + 1, d.getUTCDate());
  }
  return undefined;
}
/** dd/mm/yyyy, the way the lab shows a date the user gave. */
export const dmy = (iso: string) => (/^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}` : iso);

/** Base 1 / Base 2 periods are tagged by market phase: the strategy trades the bull or the bear ones. */
export type Phase = "Bull" | "Bear";
export const phaseOf = (w: DateWindow): Phase | undefined => (w.label === "Bull" || w.label === "Bear" ? w.label : undefined);
/** "Bull" or "Bear" when the text names exactly one of them (a sheet name "Base 2 Bull", a Type cell "bear"). */
export const tagOf = (text: string): Phase | undefined => {
  const bull = /bull/i.test(text), bear = /bear/i.test(text);
  return bull === bear ? undefined : bull ? "Bull" : "Bear";
};
const MON = "(?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\\.?";
/** The date-like parts of a line, in order (each is checked by isoDate). */
const FIND = new RegExp([
  "\\d{4}[-/.]\\d{1,2}[-/.]\\d{1,2}(?:[T ]\\d{1,2}:\\d{2}(?::\\d{2})?)?",
  "\\d{1,2}[-/.]\\d{1,2}[-/.](?:\\d{4}|\\d{2})",
  `\\d{1,2}(?:st|nd|rd|th)?[-/. ]*${MON}[-/., ]*(?:\\d{4}|\\d{2})`,
  `${MON}[-/. ]*\\d{1,2}(?:st|nd|rd|th)?,?[-/. ]*\\d{4}`,
  `\\d{4}[-/. ]*${MON}[-/. ]*\\d{1,2}`,
  "(?:19|20)\\d{6}",
  "\\d{5}(?:\\.\\d+)?",
].map((x) => `(?<![\\w.])${x}(?![\\w])`).join("|"), "gi");

export interface Parsed { windows: DateWindow[]; errors: string[] }
const NONE = "No dates found.";
const check = (windows: DateWindow[], errors: string[]): Parsed => {
  windows.forEach((w, i) => { if (!(w.from < w.to)) errors.push(`Period ${i + 1}: the start (${dmy(w.from)}) must be before the exit date (${dmy(w.to)}), the first day outside the period.`); });
  if (!windows.length && !errors.length) errors.push(NONE);
  if (windows.length > MAX_WINDOWS) errors.push(`${windows.length} windows; the limit is ${MAX_WINDOWS}.`);
  return { windows, errors };
};
/** One period per line: its start and exit dates in any format (see isoDate), in any columns, separated by anything (cells
 *  pasted from Excel, commas, spaces). A word bull / bear on the line tags it, else `tag` does; other text after the dates is
 *  its name. Lines without a date (headers, captions) are skipped. */
export function parseDateTable(text: string, tag?: Phase): Parsed {
  const windows: DateWindow[] = [], errors: string[] = [];
  text.split(/\r?\n/).forEach((line, i) => {
    if (!line.trim()) return;
    const found = [...line.matchAll(FIND)].flatMap((m) => { const d = isoDate(m[0]); return d ? [{ d, end: m.index! + m[0].length }] : []; });
    if (!found.length && !/\d{4}/.test(line)) return;
    if (found.length < 2) { errors.push(`Line ${i + 1}: expected a start date and an exit date, got "${line.trim().slice(0, 60)}".`); return; }
    const rest = line.slice(found[1].end).replace(/^[\s,;|]+|[\s,;|]+$/g, ""), label = tagOf(line) ?? tag ?? (rest ? rest.slice(0, 60) : undefined);
    windows.push(label ? { from: found[0].d, to: found[1].d, label } : { from: found[0].d, to: found[1].d });
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
/** The phase a strategy's windows were taken from: bull or bear, whichever of the set's periods they overlap most. */
export function chosenPhase(windows: readonly [string, string][], set: readonly DateWindow[]): Phase | undefined {
  const all = withBear(set), count = (p: Phase) => windows.filter(([f, t]) => all.some((w) => w.label === p && w.from < t && f < w.to)).length;
  const [bull, bear] = [count("Bull"), count("Bear")];
  return !bull && !bear ? undefined : bear > bull ? "Bear" : "Bull";
}
/** Every period of a set as bull or bear: an untagged period is a bull one, and a set without bear periods gets the gaps
 *  between its bull periods (as the Base 2 Bear sheet is the gaps between its Bull sheet). */
export function withBear(set: readonly DateWindow[]): DateWindow[] {
  const bull = set.filter((w) => phaseOf(w) !== "Bear").map((w) => ({ ...w, label: "Bull" })).sort((a, b) => (a.from < b.from ? -1 : 1));
  const bear = set.filter((w) => phaseOf(w) === "Bear");
  const gaps = bear.length ? bear : bull.slice(1).flatMap((w, i) => (bull[i].to < w.from ? [{ from: bull[i].to, to: w.from, label: "Bear" }] : []));
  return [...bull, ...gaps].sort((a, b) => (a.from < b.from ? -1 : 1));
}
/** The windows a strategy trades from a set: its bull or its bear periods (withBear). */
export const phaseWindows = (set: readonly DateWindow[], phase: Phase) => withBear(set).filter((w) => w.label === phase);
/** The spec with its date filter's windows (and name) taken from the saved set it names, so a re-uploaded set reaches every
 *  strategy that uses it (keeping its bull or bear choice). The same spec object when nothing changed or the set is not loaded. */
export function withCurrentDateSet(spec: Spec, sets: readonly DateSet[]): Spec {
  const df = spec.date_filter, set = df?.set?.id !== undefined ? sets.find((s) => s.id === df.set!.id) : undefined;
  if (!df || !set) return spec;
  const windows = phaseWindows(set.windows, df.mode === "outside" ? "Bear" : chosenPhase(df.windows, set.windows) ?? "Bull");
  if (same(df.windows, windows) && df.set?.name === set.name) return spec;
  return { ...spec, date_filter: { ...df, set: { id: set.id, name: set.name }, windows: pairs(windows) } };
}
/** An uploaded workbook: every sheet with dates; a sheet named …Bull / …Bear tags its periods (Ritik's BASE 2 DATES.xlsx). */
export function parseSheets(sheets: { name: string; rows: string[][] }[]): Parsed {
  const parts = sheets.map((s) => ({ s, p: parseDateTable(s.rows.map((r) => r.join("\t")).join("\n"), tagOf(s.name)) }))
    .filter(({ p }) => p.windows.length || p.errors.some((e) => e !== NONE));
  const windows = parts.flatMap(({ p }) => p.windows).sort((x, y) => (x.from < y.from ? -1 : x.from > y.from ? 1 : 0)); // in date order, not sheet by sheet
  return check(windows, parts.flatMap(({ s, p }) => p.errors.filter((e) => e !== NONE).map((e) => `${s.name}: ${e}`)));
}
