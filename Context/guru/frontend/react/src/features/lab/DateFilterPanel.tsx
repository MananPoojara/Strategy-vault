/** Custom date filter (the AFLs' Base 1 / Base 2 windows): pick a saved set for this strategy, and upload / re-upload / delete sets.
 *  Sets live in the auth service (/api/app/lab/date-sets); the strategy carries the windows, refreshed from the set on load. */
import { useState } from "react";
import { dateSets as defaultApi, type DateSetsApi } from "./client";
import { dayFirst } from "./DateField";
import { MAX_WINDOWS, pairs, parseDateLists, parseDateTable, type Parsed } from "./date-sets";
import { Segmented } from "./ui";
import type { DateFilter, DateSet, DateWindow, Spec } from "./types";
import "./rules.css";

const message = (e: unknown) => (e instanceof Error ? e.message : "Something went wrong.");
const range = (w: readonly DateWindow[]) => {
  if (!w.length) return "no windows";
  const from = w.reduce((a, x) => (x.from < a ? x.from : a), w[0].from), to = w.reduce((a, x) => (x.to > a ? x.to : a), w[0].to);
  return `${w.length} window${w.length === 1 ? "" : "s"}, ${dayFirst(from)} to ${dayFirst(to)}`;
};

export interface DateSetsState { list: DateSet[] | null; error?: unknown; reload(): Promise<DateSet[] | undefined>; canSave: boolean; canShare: boolean; api?: DateSetsApi }

export function DateFilterPanel({ spec, onChange, sets }: { spec: Spec; onChange(spec: Spec): void; sets: DateSetsState }) {
  const df = spec.date_filter, list = sets.list ?? [];
  const chosen = df?.set?.id !== undefined ? list.find((s) => s.id === df.set!.id) : undefined;
  const [managing, setManaging] = useState(false);
  const set = (patch: Partial<DateFilter>) => df && onChange({ ...spec, date_filter: { ...df, ...patch } });
  const pick = (id: string) => {
    if (!id) { const { date_filter: _, ...rest } = spec; onChange(rest as Spec); return; }
    const s = list.find((x) => String(x.id) === id);
    if (s) apply(s);
  };
  // keeps the strategy's inside/outside, exit and on/off choices when the set changes
  const apply = (s: DateSet) => onChange({ ...spec, date_filter: { ...(df?.mode === "outside" ? { mode: "outside" as const } : {}), ...(df?.exit_at_change === false ? { exit_at_change: false } : {}), ...(df?.off ? { off: true } : {}), set: { id: s.id, name: s.name }, windows: pairs(s.windows) } });
  const inside = (df?.mode ?? "inside") === "inside";
  const shared = list.filter((s) => s.shared), mine = list.filter((s) => !s.shared);
  return <div className="lab-datefilter" data-lab-date-filter>
    <p className="cap">Trade only during chosen date windows (for example the Base 1 / Base 2 bull periods), or only outside them.</p>
    <div className="lab-row">
      <label className="dfield">Date filter
        <select aria-label="Date filter set" value={df?.set?.id !== undefined ? String(df.set.id) : ""} onChange={(e) => pick(e.target.value)}>
          <option value="">No date filter (trade on every date)</option>
          {df?.set?.id !== undefined && !chosen && <option value={String(df.set.id)}>{df.set.name ?? "Saved windows"} (set not found)</option>}
          {shared.length > 0 && <optgroup label="Shared with everyone">{shared.map((s) => <option key={s.id} value={String(s.id)}>{s.name}</option>)}</optgroup>}
          {mine.length > 0 && <optgroup label="My date filters">{mine.map((s) => <option key={s.id} value={String(s.id)}>{s.name}</option>)}</optgroup>}
        </select>
      </label>
      {df && <label className="lab-check"><input type="checkbox" aria-label="Date filter on" checked={!df.off} onChange={(e) => set({ off: e.target.checked ? undefined : true })} />On</label>}
      <button type="button" className="dbtn sm" aria-expanded={managing} onClick={() => setManaging(!managing)} data-lab-date-manage>
        {managing ? "Close upload" : "Upload or manage dates…"}</button>
    </div>
    {sets.error !== undefined && <p className="rnote" role="alert">Could not load the date filters: {message(sets.error)}</p>}
    {df && <>
      <div className="lab-row">
        <Segmented label="Date filter mode" value={inside ? "inside" : "outside"} options={[{ value: "inside", label: "Trade only inside the windows" }, { value: "outside", label: "Trade only outside the windows" }]}
          onChange={(m) => set({ mode: m === "outside" ? "outside" : undefined })} />
        <label className="lab-check"><input type="checkbox" checked={df.exit_at_change !== false} onChange={(e) => set({ exit_at_change: e.target.checked ? undefined : false })} />
          {inside ? "Close an open trade when a window ends" : "Close an open trade when a window starts"}</label>
      </div>
      <details className="lab-datefilter-dates"><summary>{df.set?.name ?? "Windows"}: {range(df.windows.map(([from, to]) => ({ from, to })))}</summary>
        <WindowTable windows={chosen?.windows ?? df.windows.map(([from, to]) => ({ from, to }))} />
      </details>
      {spec.portfolio && !df.off && <p className="rnote" role="note">A date filter works on a single instrument only; switch it off for a portfolio.</p>}
    </>}
    {managing && <DateSetManager sets={sets} onSaved={apply} onDeleted={(id) => { if (df?.set?.id === id) pick(""); }} />}
  </div>;
}

function WindowTable({ windows }: { windows: readonly DateWindow[] }) {
  return <div className="lab-datefilter-table"><table className="tbl">
    <thead><tr><th>#</th><th>Start</th><th title="The first day outside the window: no entry on it, and a position closes there">Exit date</th><th>Name</th></tr></thead>
    <tbody>{windows.map((w, i) => <tr key={i}><td>{i + 1}</td><td>{dayFirst(w.from)}</td><td>{dayFirst(w.to)}</td><td>{w.label ?? ""}</td></tr>)}</tbody>
  </table></div>;
}

const EXAMPLE = "Start date, Exit date, Name\n01-01-2008, 31-01-2008, Crash\n21-11-2008, 31-12-2010, Recovery";

/** Upload (file or paste), preview, save as new or replace an existing set's dates, delete. */
function DateSetManager({ sets, onSaved, onDeleted }: { sets: DateSetsState; onSaved(s: DateSet): void; onDeleted(id: number): void }) {
  const api = sets.api ?? defaultApi;
  const [form, setForm] = useState<"table" | "lists">("table");
  const [name, setName] = useState(""), [text, setText] = useState(""), [starts, setStarts] = useState(""), [ends, setEnds] = useState("");
  const [share, setShare] = useState(false), [editing, setEditing] = useState<DateSet | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [done, setDone] = useState(""), [deleting, setDeleting] = useState<number | null>(null);
  const empty = form === "table" ? !text.trim() : !starts.trim() && !ends.trim();
  const parsed: Parsed | null = empty ? null : form === "table" ? parseDateTable(text) : parseDateLists(starts, ends);
  const ok = !!parsed && !parsed.errors.length && !!name.trim() && name.trim().length <= 60;
  const file = async (f: File | undefined) => {
    if (!f) return;
    if (f.size > 200_000) { setError("That file is too large; a date list is only a few kilobytes."); return; }
    setForm("table"); setText(await f.text()); setError("");
    if (!name.trim()) setName(f.name.replace(/\.[^.]+$/, "").slice(0, 60));
  };
  const save = async () => {
    if (!ok || !parsed) return;
    setBusy(true); setError(""); setDone("");
    try {
      const draft = { name: name.trim(), windows: parsed.windows };
      const saved = editing ? await api.update(editing.id, editing.version, draft) : await api.create({ ...draft, ...(share ? { shared: true } : {}) });
      await sets.reload();
      onSaved(saved);
      setDone(`Saved “${saved.name}” (${range(saved.windows)}) and chose it for this strategy.`);
      setEditing(null); setText(""); setStarts(""); setEnds(""); setName(""); setShare(false);
    } catch (e) { setError(message(e)); } finally { setBusy(false); }
  };
  const remove = async (s: DateSet) => {
    setBusy(true); setError(""); setDone("");
    try { await api.remove(s.id, s.version); await sets.reload(); onDeleted(s.id); setDone(`Deleted “${s.name}”.`); if (editing?.id === s.id) setEditing(null); }
    catch (e) { setError(message(e)); } finally { setBusy(false); setDeleting(null); }
  };
  const edit = (s: DateSet) => {
    setEditing(s); setName(s.name); setForm("table"); setDone(""); setError("");
    setText(["Start date, Exit date, Name", ...s.windows.map((w) => [dayFirst(w.from), dayFirst(w.to), w.label ?? ""].filter(Boolean).join(", "))].join("\n"));
  };
  const editable = (sets.list ?? []).filter((s) => s.editable);
  return <section className="lab-datesets" aria-label="Upload date filters" data-lab-date-sets>
    {!sets.canSave ? <p className="cap">Your account can use the shared date filters but cannot upload new ones.</p> : <>
      <h4>{editing ? `Replace the dates in “${editing.name}”` : "Upload a new date filter"}</h4>
      <ol className="cap lab-datesets-steps">
        <li>Give it a name, for example <i>Base 2 bull periods</i>.</li>
        <li>Choose a CSV or text file, or paste the dates: one window per line, <b>start date, exit date</b> and an optional name. The exit date is the first day <i>outside</i> the window, as in AmiBroker end-date lists: nothing is bought on it and an open trade closes there. Dates as dd-mm-yyyy (yyyy-mm-dd also works). From Excel, copy the two date columns and paste them here.</li>
        <li>Check the preview, then save. Up to {MAX_WINDOWS} windows.</li>
      </ol>
      <div className="lab-row">
        <label className="dfield">Name<input aria-label="Date filter name" maxLength={60} value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Base 2 bull periods" /></label>
        <Segmented label="How the dates are written" value={form} options={[{ value: "table", label: "Table (start, exit date per line)" }, { value: "lists", label: "AmiBroker start / end lists" }]} onChange={(v) => setForm(v === "lists" ? "lists" : "table")} />
        <label className="dbtn sm lab-datesets-file">Choose file…<input type="file" accept=".csv,.txt,.tsv,text/csv,text/plain" aria-label="Date filter file" onChange={(e) => { void file(e.target.files?.[0]); e.target.value = ""; }} /></label>
      </div>
      {form === "table"
        ? <textarea aria-label="Date windows" className="lab-datesets-text" rows={7} value={text} placeholder={EXAMPLE} onChange={(e) => setText(e.target.value)} />
        : <div className="lab-datesets-lists">
            <label className="dfield">Start dates<textarea aria-label="Start dates" rows={4} value={starts} placeholder={'"01-01-2008, 21-11-2008"'} onChange={(e) => setStarts(e.target.value)} /></label>
            <label className="dfield">Exit (end) dates<textarea aria-label="End dates" rows={4} value={ends} placeholder={'"31-01-2008, 31-12-2010"'} onChange={(e) => setEnds(e.target.value)} /></label>
          </div>}
      {parsed && (parsed.errors.length
        ? <ul className="rnote lab-datesets-errors" role="alert">{parsed.errors.slice(0, 8).map((e) => <li key={e}>{e}</li>)}{parsed.errors.length > 8 && <li>…and {parsed.errors.length - 8} more.</li>}</ul>
        : <div className="lab-datesets-preview" data-lab-date-preview><p className="cap">Preview: {range(parsed.windows)}</p><WindowTable windows={parsed.windows} /></div>)}
      <div className="lab-row">
        {sets.canShare && !editing && <label className="lab-check"><input type="checkbox" checked={share} onChange={(e) => setShare(e.target.checked)} />Share with everyone (an official set; only admins can change it)</label>}
        <button type="button" className="dbtn primary" disabled={!ok || busy} onClick={() => void save()} data-lab-date-save>
          {busy ? "Saving…" : editing ? "Save new dates" : "Save date filter"}</button>
        {editing && <button type="button" className="dbtn" onClick={() => { setEditing(null); setText(""); setName(""); }}>Cancel</button>}
        {!name.trim() && parsed && !parsed.errors.length && <span className="cap">Add a name to save.</span>}
      </div>
    </>}
    {error && <p className="rnote" role="alert">{error}</p>}
    {done && <p className="cap" role="status">{done}</p>}
    {editable.length > 0 && <>
      <h4>Date filters you can change</h4>
      <ul className="lab-datesets-list">{editable.map((s) => <li key={s.id}>
        <span><b>{s.name}</b>{s.shared && <span className="tag">shared</span>} <small>{range(s.windows)}</small></span>
        <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => edit(s)}>Re-upload dates</button>
        {deleting === s.id
          ? <><button type="button" className="dbtn sm" disabled={busy} onClick={() => void remove(s)}>Yes, delete</button><button type="button" className="dbtn sm ghost" onClick={() => setDeleting(null)}>Keep</button></>
          : <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => setDeleting(s.id)} aria-label={`Delete ${s.name}`}>Delete</button>}
      </li>)}</ul>
    </>}
  </section>;
}
