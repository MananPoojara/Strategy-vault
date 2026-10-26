// Tradesheets of an optimization's combinations (e.g. one entry, several exits switched on and off): each combination
// is run again as a plain backtest, so a file holds exactly what Apply + Run backtest would show.
import { useEffect, useRef, useState } from "react";
import { withCombination } from "./spec-model";
import { kpiItems, tradeTable } from "./RunResults";
import { saveBytes, xlsx, zip, XLSX_TYPE, type Sheet } from "./xlsx";
import type { LabRunResult, RunResult, Spec } from "./types";
/** A grid dimension: only its spec path matters here. */
type Dim = { path: string };

/** At most this many backtests per download, so a large grid does not flood the engine. */
export const MAX_SHEETS = 50;
type Run = (spec: Spec) => Promise<LabRunResult>;

/** "Exit 2 off · RSI length 10": a rule switch reads on/off, any other setting label = value. */
export function comboName(dims: Dim[], values: number[], label: (path: string) => string): string {
  return dims.map((d, i) => d.path.endsWith(".off") ? `${label(d.path).replace(/:? ?switched off.*$/i, "")} ${values[i] ? "off" : "on"}` : `${label(d.path)} ${values[i]}`).join(" · ");
}
export const comboSpec = (ranSpec: Spec, dims: Dim[], values: number[]): Spec => withCombination(ranSpec, dims, values);
const fileSafe = (s: string) => s.replace(/[\\/:*?"<>|]+/g, " ").replace(/\s+/g, " ").trim().slice(0, 90);
const summaryRow = (no: number, name: string, r: RunResult) => [no, name, ...kpiItems(r.stats).map((k) => k.value)];
const summaryHead = (r: RunResult) => ["No.", "Combination", ...kpiItems(r.stats).map((k) => k.label)];
const sheetOf = (name: string, r: RunResult): Sheet => ({ name, rows: tradeTable(r).csv });
const single = (r: LabRunResult): RunResult => {
  if (r.kind !== "lab_simulation") throw new Error("Tradesheet downloads are for one instrument, not a portfolio.");
  return r;
};

export function TradesheetDownloads({ table, dims, ranSpec, label, onRun, ticker }: {
  table: (number | null)[][]; dims: Dim[]; ranSpec: Spec; label(path: string): string; onRun: Run; ticker: string;
}) {
  const [format, setFormat] = useState<"files" | "book">("files");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const combos = table.slice(0, MAX_SHEETS).map((r) => r.slice(0, dims.length) as number[]);
  const all = async () => {
    setError(null);
    try {
      const done: { name: string; r: RunResult }[] = [];
      for (const [i, values] of combos.entries()) {
        if (!alive.current) return;
        setBusy(`Running ${i + 1} of ${combos.length}…`);
        done.push({ name: comboName(dims, values, label), r: single(await onRun(comboSpec(ranSpec, dims, values))) });
      }
      if (!alive.current) return;
      const summary: Sheet = { name: "Summary", rows: [summaryHead(done[0].r), ...done.map((d, i) => summaryRow(i + 1, d.name, d.r))] };
      const no = (i: number) => String(i + 1).padStart(2, "0");
      if (format === "book") saveBytes(xlsx([summary, ...done.map((d, i) => sheetOf(`${no(i)} ${d.name}`, d.r))]), `tradesheets ${fileSafe(ticker)}.xlsx`, XLSX_TYPE);
      else saveBytes(zip([{ name: "00 Summary.xlsx", data: xlsx([summary]) },
        ...done.map((d, i) => ({ name: `${no(i)} ${fileSafe(d.name)}.xlsx`, data: xlsx([sheetOf("Tradesheet", d.r)]) }))]), `tradesheets ${fileSafe(ticker)}.zip`, "application/zip");
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (alive.current) setBusy(null);
    }
  };
  return <div className="lab-row" data-lab-tradesheets>
    <span className="dfield">Tradesheets</span>
    <select aria-label="Tradesheet format" value={format} onChange={(e) => setFormat(e.target.value === "book" ? "book" : "files")}>
      <option value="files">A separate Excel file for each combination (ZIP)</option>
      <option value="book">One Excel file, a sheet for each combination</option>
    </select>
    <button type="button" className="dbtn sm" disabled={!!busy || !combos.length} onClick={() => void all()}>Download all {combos.length}</button>
    {busy && <span className="cap" role="status">{busy}</span>}
    {table.length > MAX_SHEETS && <span className="cap">The first {MAX_SHEETS} of {table.length} combinations (in the table's order); narrow the ranges for the rest.</span>}
    {error && <span className="rnote" role="alert">{error}</span>}
  </div>;
}

/** One combination's tradesheet as its own Excel file. */
export function TradesheetButton({ dims, values, ranSpec, label, onRun, ticker }: {
  dims: Dim[]; values: number[]; ranSpec: Spec; label(path: string): string; onRun: Run; ticker: string;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const name = comboName(dims, values, label);
  const go = async () => {
    setBusy(true); setError(null);
    try {
      const r = single(await onRun(comboSpec(ranSpec, dims, values)));
      saveBytes(xlsx([sheetOf("Tradesheet", r), { name: "Summary", rows: [summaryHead(r), summaryRow(1, name, r)] }]), `tradesheet ${fileSafe(ticker)} ${fileSafe(name)}.xlsx`, XLSX_TYPE);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  };
  return <><button type="button" className="dbtn sm ghost" aria-label={`Excel tradesheet: ${name}`} title={`Download the tradesheet of ${name} as an Excel file`} disabled={busy} onClick={() => void go()}>{busy ? "…" : "Excel"}</button>
    {error && <span className="rnote" role="alert">{error}</span>}</>;
}
