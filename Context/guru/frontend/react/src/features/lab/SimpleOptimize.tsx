import { useState, type ReactNode } from "react";
import { COPY } from "./labels";
import { DASH, int, pct } from "./format";
import { comboCount, describeSpec, specTooLong, valuesCount, withCombination, withOptimize, type NumSlot, type Phrase } from "./spec-model";
import { optimizablePaths, type OptimizableSetting } from "./optimize-paths";
import { useFormulaScope, type FormulaCheck } from "./formula";
import { foreignDescription } from "./foreign-model";
import { NumField } from "./OperandEditor";
import { ExecutionStrip } from "./RunResults";
import { Loading, ProvisionalStrip, ServerLabel, tone } from "./ui";
import type { Catalogue, LabOptimizeResult, OptimizeDim, Spec } from "./types";
import "./simple-optimize.css";

const BEST = 20;
/** Result columns after the varied numbers; one the result does not carry (e.g. max_dd) is left out. */
const METRICS: [key: string, label: string, show: (v: number | null) => string][] = [
  ["total_return", "Net profit", (v) => pct(v)], ["trades", "Trades", int], ["win_rate", "Win rate", (v) => pct(v, 0)], ["max_dd", "Worst fall", (v) => pct(v)]];
const FIELDS = [["from", "From"], ["to", "To"], ["step", "Step"]] as const;
const tidy = (v: number) => Number(v.toPrecision(12)); // 0.1 + 0.2 shows as 0.3
const count = (n: number) => n.toLocaleString("en-US");
const plural = (n: number, word: string) => `${count(n)} ${word}${n === 1 ? "" : "s"}`;

/** A number the engine can vary: its rule sentence with that number marked, a name for the column and inputs, the lowest sensible value. */
interface Line { s: OptimizableSetting; words: ReactNode; name: string; min: number }

/** Every optimizable number (optimize-paths.ts, the engine's paths) in the words of its rule (spec-model describeSpec). */
function linesOf(spec: Spec, cat: Catalogue, formulas: Record<string, FormulaCheck>): Line[] {
  const d = describeSpec(spec, cat);
  const sentences: Phrase[] = [...d.entry.map((p) => [`${d.buy} when `, ...p]), ...(d.entryOrder ? [[`${d.buy} `, ...d.entryOrder]] : []),
    ...d.exitWhen.map((p) => [`${d.sell} when `, ...p]), ...d.exitOther.map((p) => [`${d.sell} `, ...p])];
  const slots = new Map<string, { p: Phrase; x: NumSlot }>();
  for (const p of sentences) for (const x of p) if (typeof x !== "string") slots.set(x.path, { p, x });
  // a rule's on/off switch is not a number of the rules; it stays in the Advanced optimizer
  // only the numbers the rules show in words: a calculation-variant choice the sentence leaves out stays in the Advanced optimizer
  return optimizablePaths(spec, cat, formulas).filter((s) => !s.path.endsWith(".off")).flatMap((s): Line[] => {
    const hit = slots.get(s.path);
    if (!hit) return [];
    return [{
      s, min: Math.max(s.min, hit.x.min ?? -Infinity), // e.g. a candle count starts at 1, never "the 0th up close"
      words: hit.p.map((x, i) => (typeof x === "string" ? x : x.path === s.path ? <b key={i}>{x.value}</b> : x.value)),
      name: hit.p.map((x) => (typeof x === "string" ? x : x.path === s.path ? "N" : x.value)).join("").replace(/\bN(?:st|nd|rd)\b/, "Nth"),
    }];
  });
}

/** Simple view's AmiBroker-style Optimize(): a range per number, every combination tested, the best by net profit, one click to use them. */
export function SimpleOptimize({ spec, catalogue, onOptimize, onApply }: {
  spec: Spec; catalogue: Catalogue; onOptimize(spec: Spec): Promise<LabOptimizeResult>; onApply(spec: Spec): void;
}) {
  const lines = linesOf(spec, catalogue, useFormulaScope().infos);
  const [ranges, setRanges] = useState<Record<string, OptimizeDim>>({});
  const [running, setRunning] = useState<number | null>(null); // combinations being tested
  const [result, setResult] = useState<LabOptimizeResult | null>(null);
  const [error, setError] = useState<unknown>(null);

  const dims = lines.flatMap((l) => (ranges[l.s.path] ? [ranges[l.s.path]] : []));
  const combos = comboCount(dims);
  const cap = spec.portfolio ? catalogue.portfolio?.max_combos ?? 200 : catalogue.max_combos; // the Advanced panel's caps
  const tooLong = dims.length > 0 && specTooLong(withOptimize(spec, dims));
  const toggle = ({ s, min }: Line, on: boolean) => setRanges((cur) => {
    const next = { ...cur }, near = (v: number) => tidy(Math.min(s.max, Math.max(min, v)));
    if (on) next[s.path] = { path: s.path, from: near(s.current - 2 * s.step), to: near(s.current + 2 * s.step), step: s.step };
    else delete next[s.path];
    return next;
  });
  // an empty field is NaN: that range has no values, so nothing runs until it is filled in
  const edit = (path: string, key: "from" | "to" | "step", n: number | undefined) => setRanges((cur) => ({ ...cur, [path]: { ...cur[path], [key]: n ?? NaN } }));
  const run = async () => {
    setRunning(combos); setResult(null); setError(null);
    try { setResult(await onOptimize(withOptimize(spec, dims))); } catch (e) { setError(e); } finally { setRunning(null); }
  };

  return <section className="box lab-sopt" data-lab-simple-optimize>
    <h2 className="box-h">Test every combination</h2>
    <p className="cap">Every combination of the ranges you set is tested, and the {BEST} with the highest net profit are listed.</p>
    {lines.length ? <ul className="lab-sopt-list">{lines.map((l) => {
      const d = ranges[l.s.path];
      return <li key={l.s.path} className="lab-sopt-line" data-lab-sopt-line={l.s.path}>
        <div className="lab-sopt-head">
          <span className="lab-sopt-words">{l.words}</span>
          <label className="lab-check"><input type="checkbox" aria-label={`Try a range: ${l.name}`} checked={!!d} onChange={(e) => toggle(l, e.target.checked)} />Try a range</label>
        </div>
        {d && <div className="lab-sopt-range" role="group" aria-label={`Range for ${l.name}`}>
          {FIELDS.map(([k, label]) => <NumField key={k} label={label} value={Number.isNaN(d[k]) ? undefined : d[k]} step={l.s.step} onChange={(n) => edit(l.s.path, k, n)} />)}
          <span className="cap">{plural(valuesCount(d), "value")}</span>
        </div>}
      </li>;
    })}</ul> : <p className="empty">Add a rule with a number first, for example “RSI(14) is below 30”.</p>}
    <div className="lab-sopt-run">
      {dims.length > 0 && <span className="lab-sopt-count" aria-live="polite">{plural(combos, "combination")}</span>}
      <button type="button" className="dbtn primary" data-lab-action="simple-optimize" disabled={running !== null || combos === 0 || combos > cap || tooLong}
        onClick={() => void run()}>{combos ? `Test all ${plural(combos, "combination")}` : "Test all combinations"}</button>
    </div>
    {!dims.length ? <p className="cap">Turn on “Try a range” for at least one number.</p>
      : !combos ? <p className="rnote">Check each range: From must not be above To, and Step must be above 0.</p>
      : combos > cap && <p className="rnote">{count(combos)} combinations is more than the {count(cap)} one test can run. Make a range shorter or its step bigger.</p>}
    {tooLong && <p className="rnote">{COPY.tooLong}</p>}
    {running !== null && <Loading text={`Testing ${plural(running, "combination")}… a big test can take a few minutes.`} />}
    {error !== null && <p className="rnote" role="alert">The test did not run: {error instanceof Error ? error.message : String(error)}</p>}
    {result && <Best result={result} lines={lines} spec={spec} onApply={onApply} />}
  </section>;
}

/** The best combinations by net profit (the engine's every-combination table), each with "Use these numbers". */
function Best({ result, lines, spec, onApply }: { result: LabOptimizeResult; lines: Line[]; spec: Spec; onApply(spec: Spec): void }) {
  const nd = result.dims.length, cols = result.table_cols ?? [], rows = result.table ?? [];
  const at = (r: (number | null)[], k: string) => (cols.includes(k) ? r[nd + cols.indexOf(k)] ?? null : null);
  const net = (r: (number | null)[]) => at(r, "total_return") ?? -Infinity;
  const best = [...rows].sort((a, b) => net(b) - net(a) || 0).slice(0, BEST); // gaps last; ties keep the engine's order
  const metrics = METRICS.filter(([k]) => cols.includes(k));
  const name = (path: string) => lines.find((l) => l.s.path === path)?.name;
  const usable = result.dims.every((d) => name(d.path) !== undefined); // after a rule is removed a value would land elsewhere
  const show = (v: number | null) => (v === null ? DASH : String(tidy(v)));
  return <div className="lab-sopt-results" data-lab-sopt-result>
    <ServerLabel text={result.label} />
    <ExecutionStrip model={result.execution_model} money={result.money} adaptations={result.adaptations} />
    {result.money?.basis && <p className="cap">{result.money.basis}</p>}
    {result.provisional && <ProvisionalStrip note={result.provisional_note} />}
    {"membership" in result && <p className="rnote">{result.reference.ticker} reference · {result.membership.note}</p>}
    {result.foreign?.map((f) => <p className="cap" key={f.ticker}>{foreignDescription(f)}</p>)}
    <p className="cap">The best {best.length} of {plural(rows.length, "combination")}, highest net profit first.</p>
    {!usable && <p className="rnote">The rules changed after this test. Test again to use its numbers.</p>}
    <div className="lab-sopt-scroll"><table className="lab-sopt-table">
      <thead><tr>
        {result.dims.map((d) => <th key={d.path} scope="col">{name(d.path) ?? d.path}</th>)}
        {metrics.map(([k, label]) => <th key={k} scope="col" className="n">{label}</th>)}
        <th scope="col"><span className="sr-only">Use</span></th>
      </tr></thead>
      <tbody>{best.map((r, i) => <tr key={i}>
        {result.dims.map((d, j) => <td key={d.path} className="n">{show(r[j])}</td>)}
        {metrics.map(([k, , f]) => { const v = at(r, k); return <td key={k} className={k === "total_return" ? tone(v) : "n"}>{f(v)}</td>; })}
        <td>{usable && <button type="button" className="dbtn sm" aria-label={`Use these numbers: ${r.slice(0, nd).map(show).join(", ")}`}
          onClick={() => onApply(withCombination(spec, result.dims, r.slice(0, nd) as number[]))}>Use these numbers</button>}</td>
      </tr>)}</tbody>
    </table></div>
  </div>;
}
