/** The Dates filter: trade only in the bull or the bear periods of Base 1, Base 2 (built in) or the user's own dates, uploaded
 *  as CSV / Excel or pasted in any date format. The strategy carries the chosen periods; saved sets live in the auth service. */
import { useState } from "react";
import { dateSets as defaultApi, type DateSetsApi } from "./client";
import { BUILT_IN } from "./builtin-periods";
import { chosenPhase, dmy, pairs, parseDateTable, parseSheets, phaseWindows, withBear, type Parsed, type Phase } from "./date-sets";
import { readXlsx } from "./xlsx-read";
import { Segmented } from "./ui";
import type { DateSet, DateWindow, Spec } from "./types";
import "./rules.css";

const message = (e: unknown) => (e instanceof Error ? e.message : "Something went wrong.");
const PHASES: Phase[] = ["Bull", "Bear"];
const span = (w: readonly DateWindow[]) => (w.length ? `${dmy(w.reduce((a, x) => (x.from < a ? x.from : a), w[0].from))} to ${dmy(w.reduce((a, x) => (x.to > a ? x.to : a), w[0].to))}` : "");

export interface DateSetsState { list: DateSet[] | null; error?: unknown; reload(): Promise<DateSet[] | undefined>; canSave: boolean; canShare: boolean; api?: DateSetsApi }
interface Base { key: string; name: string; id?: number; windows: DateWindow[] }

/** The Dates filter turned on with nothing chosen yet: Base 1's bull periods. */
export const defaultDates = () => ({ set: { name: BUILT_IN[0].name }, windows: pairs(phaseWindows(BUILT_IN[0].windows, "Bull")) });

export function DateFilterPanel({ spec, onChange, sets }: { spec: Spec; onChange(spec: Spec): void; sets: DateSetsState }) {
  const df = spec.date_filter;
  const [own, setOwn] = useState<Base | null>(null); // dates used in this visit that could not be saved
  const bases: Base[] = [...BUILT_IN.map((b) => ({ key: b.name, ...b })), ...(sets.list ?? []).filter((s) => !BUILT_IN.some((b) => b.name === s.name)).map((s) => ({ key: `set-${s.id}`, name: s.name, id: s.id, windows: s.windows })),
    ...(own && !(sets.list ?? []).some((s) => s.name === own.name) ? [own] : [])];
  if (!df) return null;
  const base = bases.find((b) => (df.set?.id !== undefined ? b.id === df.set.id : b.id === undefined && b.name === df.set?.name));
  const phase: Phase = df.mode === "outside" ? "Bear" : base ? chosenPhase(df.windows, base.windows) ?? "Bull" : "Bull";
  const use = (b: Base, p: Phase) => {
    const { mode: _, ...rest } = df;
    onChange({ ...spec, date_filter: { ...rest, set: b.id !== undefined ? { id: b.id, name: b.name } : { name: b.name }, windows: pairs(phaseWindows(b.windows, p)) } });
  };
  const traded = base ? phaseWindows(base.windows, phase) : df.windows.map(([from, to]) => ({ from, to }));
  return <div className="lab-datefilter" data-lab-date-filter>
    <div className="lab-param-row"><span className="lab-param-k">Dates</span><span className="lab-param-v">
      <Segmented label="Dates" value={base?.key ?? "?"} options={[...bases.map((b) => ({ value: b.key, label: b.name })), ...(base ? [] : [{ value: "?", label: df.set?.name ?? "Saved dates" }])]}
        onChange={(k) => { const b = bases.find((x) => x.key === k); if (b) use(b, phase); }} />
    </span></div>
    <div className="lab-param-row"><span className="lab-param-k">Trade in</span><span className="lab-param-v">
      <Segmented label="Trade in" value={phase} options={PHASES.map((p) => ({ value: p, label: p }))} onChange={(p) => base && use(base, p as Phase)} />
    </span></div>
    {/* an open trade always closes when its period ends (the AFLs' forced exit); the box shows only for a strategy that turned it off */}
    {df.exit_at_change === false && <label className="lab-check"><input type="checkbox" checked={false} onChange={() => { const { exit_at_change: _, ...rest } = df; onChange({ ...spec, date_filter: rest }); }} />
      Close an open trade when the period ends</label>}
    <details className="lab-datefilter-dates"><summary>See the {traded.length} {phase.toLowerCase()} periods</summary><WindowTable windows={traded} /></details>
    <OwnDates sets={sets} onUse={(b) => { setOwn(b); use(b, b.windows.some((w) => w.label === "Bull") || !b.windows.some((w) => w.label === "Bear") ? "Bull" : "Bear"); }} />
    {spec.portfolio && !df.off && <p className="rnote" role="note">Dates work on a single instrument only; turn them off for a portfolio.</p>}
  </div>;
}

/** Every period on one line through time: bull green, bear red; the traded ones solid, the rest faded. No motion: the filter
 *  is switched often, the highlight just changes. */
export function DateStrip({ windows, active }: { windows: readonly DateWindow[]; active?: Phase }) {
  if (!windows.length) return null;
  const t = (iso: string) => Date.parse(iso), lo = Math.min(...windows.map((w) => t(w.from))), hi = Math.max(...windows.map((w) => t(w.to))), width = hi - lo || 1;
  const count = (p: Phase) => windows.filter((w) => w.label === p).length, year = (ms: number) => new Date(ms).getUTCFullYear();
  return <div className="lab-datestrip" data-lab-date-strip>
    <div className="lab-datestrip-bar" role="img" aria-label={`${count("Bull")} bull and ${count("Bear")} bear periods, ${span(windows)}`}>
      {windows.map((w, i) => <span key={i} className={`lab-datestrip-w ${w.label === "Bear" ? "bear" : "bull"}`} data-on={!active || w.label === active || undefined}
        style={{ left: `${((t(w.from) - lo) / width) * 100}%`, width: `${Math.max(0.4, ((t(w.to) - t(w.from)) / width) * 100)}%` }} title={`${w.label ?? "Period"} · ${dmy(w.from)} to ${dmy(w.to)}`} />)}
    </div>
    <div className="lab-datestrip-axis">
      <span>{year(lo)}</span>
      <span className="lab-datestrip-key">{PHASES.filter(count).map((p) => <span key={p}><i className={p.toLowerCase()} />{p} · {count(p)}</span>)}</span>
      <span>{year(hi)}</span>
    </div>
  </div>;
}

function WindowTable({ windows }: { windows: readonly DateWindow[] }) {
  return <div className="lab-datefilter-table"><table className="tbl">
    <thead><tr><th>#</th><th>Start</th><th title="The first day outside the period: no entry on it, and a position closes there">Exit date</th></tr></thead>
    <tbody>{windows.map((w, i) => <tr key={i}><td>{i + 1}</td><td>{dmy(w.from)}</td><td>{dmy(w.to)}</td></tr>)}</tbody>
  </table></div>;
}

const EXAMPLE = "21/11/2008  31/12/2010\n2011-09-26, 2011-11-25\n15 Jun 2012 – 01 Mar 2013";

/** Your own dates: a CSV / Excel file or pasted text, any date format (read day first). Used for this strategy at once, and
 *  saved for reuse when the account can save. */
function OwnDates({ sets, onUse }: { sets: DateSetsState; onUse(b: Base): void }) {
  const api = sets.api ?? defaultApi;
  const [text, setText] = useState(""), [tag, setTag] = useState<Phase>("Bull"), [fromFile, setFromFile] = useState<{ name: string; parsed: Parsed } | null>(null);
  const [name, setName] = useState(""), [busy, setBusy] = useState(false), [note, setNote] = useState(""), [error, setError] = useState("");
  const parsed = fromFile?.parsed ?? (text.trim() ? parseDateTable(text, tag) : null);
  const file = async (f: File | undefined) => {
    if (!f) return;
    setError(""); setNote("");
    if (/\.xls$/i.test(f.name)) { setError("Old .xls files cannot be read: save it as .xlsx or CSV and choose it again."); return; }
    try {
      setFromFile({ name: f.name, parsed: /\.xlsx$/i.test(f.name) ? parseSheets(await readXlsx(await f.arrayBuffer())) : parseDateTable(await f.text(), tag) });
      if (!name.trim()) setName(f.name.replace(/\.[^.]+$/, "").slice(0, 60));
    } catch (e) { setError(message(e)); }
  };
  const use = async () => {
    if (!parsed || parsed.errors.length) return;
    const label = name.trim() || "My dates", windows = parsed.windows;
    setBusy(true); setError(""); setNote("");
    try {
      const saved = sets.canSave ? await api.create({ name: label, windows }) : null;
      if (saved) { await sets.reload(); onUse({ key: `set-${saved.id}`, name: saved.name, id: saved.id, windows: saved.windows }); setNote(`Saved “${saved.name}”: pick it any time under Dates.`); }
      else onUse({ key: `own-${label}`, name: label, windows });
    } catch (e) { onUse({ key: `own-${label}`, name: label, windows }); setNote(`Used for this strategy; not saved for later (${message(e)})`); }
    finally { setBusy(false); setText(""); setFromFile(null); }
  };
  return <details className="lab-owndates" data-lab-own-dates>
    <summary>Use my own dates</summary>
    <p className="cap">Any date format, read as day / month / year. One period per line: start date and exit date. A line or Excel sheet named bull or bear is tagged so.</p>
    <div className="lab-row">
      <label className="dbtn sm lab-datesets-file">Upload CSV or Excel…<input type="file" accept=".xlsx,.csv,.txt,.tsv,text/csv,text/plain,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" aria-label="Date filter file" onChange={(e) => { void file(e.target.files?.[0]); e.target.value = ""; }} /></label>
      {fromFile && <span className="cap">{fromFile.name} <button type="button" className="dbtn sm ghost" onClick={() => setFromFile(null)}>×</button></span>}
    </div>
    {!fromFile && <textarea aria-label="Date windows" className="lab-datesets-text" rows={4} value={text} placeholder={EXAMPLE} onChange={(e) => setText(e.target.value)} />}
    {!fromFile && <div className="lab-row"><span className="cap">Pasted periods are</span>
      <Segmented label="Pasted periods are" value={tag} options={PHASES.map((p) => ({ value: p, label: p }))} onChange={(p) => setTag(p as Phase)} /></div>}
    {parsed && (parsed.errors.length
      ? <ul className="rnote lab-datesets-errors" role="alert">{parsed.errors.slice(0, 6).map((e) => <li key={e}>{e}</li>)}{parsed.errors.length > 6 && <li>…and {parsed.errors.length - 6} more.</li>}</ul>
      : <div data-lab-date-preview><p className="cap">{parsed.windows.length} periods, {span(parsed.windows)}</p><DateStrip windows={withBear(parsed.windows)} /></div>)}
    <div className="lab-row">
      <label className="dfield">Name<input aria-label="Date filter name" maxLength={60} value={name} placeholder="My dates" onChange={(e) => setName(e.target.value)} /></label>
      <button type="button" className="dbtn primary" disabled={!parsed || !!parsed.errors.length || busy} onClick={() => void use()} data-lab-date-save>{busy ? "Saving…" : "Use these dates"}</button>
    </div>
    {error && <p className="rnote" role="alert">{error}</p>}
    {note && <p className="cap" role="status">{note}</p>}
  </details>;
}
