import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { errorPosition, FORMULA_EXAMPLES, MAX_FORMULA, MAX_NAME } from "./formula";
import { generate, starterBlocks } from "./blocks-model";
import type { IndicatorsApi } from "./client";
import type { FormulaFunction, FormulaInfo, SavedIndicator } from "./types";
import "./library.css";
import "./formula.css";

/** The block builder (a separate component) edits the same draft: it reports the formula it generates plus its own layout. */
export interface BlocksEditorProps { formula: string; blocks: unknown | null; onChange(formula: string, blocks: unknown): void; showFormula?(): void }
export type CheckState =
  | { status: "idle" } | { status: "checking" } | { status: "ok"; info: FormulaInfo }
  | { status: "error"; message: string; pos?: { line: number; col: number } };
/** All editor state in one object, so the Blocks and Formula tabs drive the same draft and check result. */
export interface EditorDraft { name: string; formula: string; blocks: unknown | null; check: CheckState }

export interface FormulaEditorProps {
  row: SavedIndicator | null; functions: FormulaFunction[]; canEdit: boolean;
  check(formula: string): Promise<FormulaInfo>;
  api: Pick<IndicatorsApi, "create" | "update" | "remove">;
  onSaved(row: SavedIndicator, info: FormulaInfo): void; onDeleted(id: number): void; onClose(): void;
  blocksEditor?: (props: BlocksEditorProps) => ReactNode;
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const NEW_FORMULA = `// Param("Label", default, min, max, step) · Plot(value, "Name")\nN = Param("Periods", 14, 2, 200, 1);\nPlot(C - MA(C, N), "Osc");`;

/** AmiBroker Formula Editor: name, formula, Check (Verify syntax), Save / Save as new / Delete / Close. */
export function FormulaEditor({ row, functions, canEdit, check, api, onSaved, onDeleted, onClose, blocksEditor }: FormulaEditorProps) {
  const [saved, setSaved] = useState<SavedIndicator | null>(row); // the row this draft overwrites; Save as new switches it
  const [d, setD] = useState<EditorDraft>(() => {
    const fresh = !row && blocksEditor ? starterBlocks() : null; // a new indicator starts on Blocks (design §5)
    return { name: row?.name ?? "", formula: row?.formula ?? (fresh ? generate(fresh).text : NEW_FORMULA), blocks: row?.blocks ?? fresh, check: { status: "idle" } };
  });
  const [mode, setMode] = useState<"blocks" | "formula">(blocksEditor && (!row || row.blocks != null) ? "blocks" : "formula");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ text: string; error?: boolean } | null>(null);
  const area = useRef<HTMLTextAreaElement>(null), box = useRef<HTMLElement>(null), seq = useRef(0), tabMoves = useRef(false);
  const idBase = `lab-formula-${row?.id ?? "new"}`;

  useEffect(() => { box.current?.scrollIntoView?.({ block: "nearest" }); }, []);
  useEffect(() => { if (row) void runCheck(row.formula); }, []); // eslint-disable-line react-hooks/exhaustive-deps -- an opened indicator shows its Params at once

  // Any formula change makes the last check stale; typed text detaches the block layout (the text is the authority).
  const setFormula = (formula: string, blocks: unknown | null) => { seq.current++; setD((s) => ({ ...s, formula, blocks, check: { status: "idle" } })); setNotice(null); };

  async function runCheck(formula = d.formula): Promise<FormulaInfo | undefined> {
    const n = ++seq.current;
    setD((s) => ({ ...s, check: { status: "checking" } }));
    try {
      const info = await check(formula);
      if (n === seq.current) setD((s) => ({ ...s, check: { status: "ok", info } }));
      return info;
    } catch (e) {
      const text = message(e), pos = errorPosition(text);
      if (n === seq.current) {
        setD((s) => ({ ...s, check: { status: "error", message: text, pos } }));
        if (pos && area.current && mode === "formula") { // put the caret on the error, as AmiBroker's Verify does
          const lines = formula.split("\n"), at = lines.slice(0, pos.line - 1).reduce((k, l) => k + l.length + 1, 0) + pos.col - 1;
          area.current.focus(); area.current.setSelectionRange(at, Math.min(at + 1, formula.length));
        }
      }
      return undefined;
    }
  }

  const save = async (asNew: boolean) => {
    const name = d.name.trim();
    if (!name) { setNotice({ text: "Give the indicator a name.", error: true }); return; }
    setBusy(true); setNotice(null);
    try {
      const info = await runCheck();
      if (!info) { setNotice({ text: "Fix the formula error before saving.", error: true }); return; }
      const draft = { name, formula: d.formula, blocks: d.blocks };
      const next = !asNew && saved ? await api.update(saved.id, saved.version, draft) : await api.create(draft);
      setSaved(next);
      onSaved(next, info);
      setNotice({ text: asNew || !saved ? `Saved as "${next.name}" in My indicators.` : `Saved "${next.name}".` });
    } catch (e) {
      setNotice({ text: message(e), error: true });
    } finally { setBusy(false); }
  };
  const remove = async () => {
    if (!saved || !window.confirm(`Delete "${saved.name}"? Strategies that use it keep their own copy of the formula.`)) return;
    setBusy(true);
    try { await api.remove(saved.id, saved.version); onDeleted(saved.id); } catch (e) { setNotice({ text: message(e), error: true }); setBusy(false); }
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Escape") { tabMoves.current = true; return; }
    if (e.key !== "Tab" || e.shiftKey || tabMoves.current) { tabMoves.current = false; return; }
    e.preventDefault(); // Tab indents; Esc then Tab leaves the field
    const t = e.currentTarget, a = t.selectionStart, b = t.selectionEnd, text = d.formula.slice(0, a) + "  " + d.formula.slice(b);
    if (text.length > MAX_FORMULA) return;
    setFormula(text, null);
    requestAnimationFrame(() => t.setSelectionRange(a + 2, a + 2));
  };

  const c = d.check;
  const errorLine = c.status === "error" && c.pos ? d.formula.split("\n")[c.pos.line - 1] : undefined;
  const tab = (id: "blocks" | "formula", label: string) => (
    <button type="button" role="tab" id={`${idBase}-tab-${id}`} aria-selected={mode === id} aria-controls={`${idBase}-${id}`} tabIndex={mode === id ? 0 : -1}
      onClick={() => setMode(id)} onKeyDown={(e) => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { const next = mode === "blocks" ? "formula" : "blocks"; setMode(next); document.getElementById(`${idBase}-tab-${next}`)?.focus(); } }}>{label}</button>
  );
  return (
    <section className="box lab-detail lab-formula" ref={box} aria-labelledby={`${idBase}-h`} data-lab-formula-editor>
      <h2 className="box-h" id={`${idBase}-h`}>{saved ? `Edit indicator · ${saved.name}` : "New indicator"} <small>Formula editor · AmiBroker-style</small>
        <span className="box-tools"><button type="button" className="dbtn ghost sm" aria-label="Close formula editor" onClick={onClose}>×</button></span>
      </h2>
      <label className="dfield lab-formula-name">Name
        <input type="text" value={d.name} maxLength={MAX_NAME} placeholder="My Momentum" readOnly={!canEdit}
          onChange={(e) => { setD((s) => ({ ...s, name: e.target.value })); setNotice(null); }} />
      </label>
      <div className="lab-formula-tabs" role="tablist" aria-label="Edit the indicator as">{tab("blocks", "Blocks")}{tab("formula", "Formula")}</div>
      <div role="tabpanel" id={`${idBase}-blocks`} aria-labelledby={`${idBase}-tab-blocks`} hidden={mode !== "blocks"}>
        {blocksEditor ? blocksEditor({ formula: d.formula, blocks: d.blocks, onChange: (formula, blocks) => setFormula(formula, blocks), showFormula: () => setMode("formula") })
          : <p className="cap">The block builder is not available yet. Write the indicator on the Formula tab.</p>}
      </div>
      <div role="tabpanel" id={`${idBase}-formula`} aria-labelledby={`${idBase}-tab-formula`} hidden={mode !== "formula"}>
        <textarea ref={area} className="lab-formula-text" aria-label="Formula" value={d.formula} maxLength={MAX_FORMULA} rows={10} spellCheck={false}
          autoCapitalize="off" autoComplete="off" readOnly={!canEdit} aria-invalid={c.status === "error"} aria-describedby={`${idBase}-hint ${idBase}-result`}
          onChange={(e) => setFormula(e.target.value, null)} onKeyDown={onKey} />
        <p className="cap" id={`${idBase}-hint`}>{d.formula.length} / {MAX_FORMULA} characters · Tab indents; press Esc then Tab to leave the field.{d.blocks != null && " Typing here replaces the block layout."}</p>
      </div>
      <div id={`${idBase}-result`} className="lab-formula-result" aria-live="polite">
        {c.status === "checking" && <p className="cap">Checking…</p>}
        {c.status === "ok" && <dl>
          <dt>Parameters</dt><dd>{c.info.params.length ? c.info.params.map((p) => `${p.key} ${p.default} (${p.min}–${p.max}, step ${p.step})`).join(" · ") : "None"}</dd>
          <dt>Plots</dt><dd>{c.info.plots.join(", ") || "None"}</dd>
          <dt>Drawn</dt><dd>{c.info.placement === "price" ? "On the price chart" : "In its own pane"}</dd>
        </dl>}
        {c.status === "error" && <div className="rnote lab-formula-err" role="alert">
          <p>{c.message}</p>
          {errorLine !== undefined && c.pos && <pre aria-label={`Line ${c.pos.line}`}>{errorLine.slice(0, c.pos.col - 1)}<mark>{errorLine.slice(c.pos.col - 1, c.pos.col) || " "}</mark>{errorLine.slice(c.pos.col)}</pre>}
        </div>}
      </div>
      <div className="lab-formula-actions">
        <button type="button" className="dbtn sm" disabled={busy || c.status === "checking"} onClick={() => void runCheck()}>Check</button>
        {canEdit && <>
          <button type="button" className="dbtn sm primary" disabled={busy} onClick={() => void save(false)}>Save</button>
          {saved && <button type="button" className="dbtn sm" disabled={busy} onClick={() => void save(true)}>Save as new</button>}
          {saved && <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => void remove()}>Delete</button>}
        </>}
        <button type="button" className="dbtn sm ghost" onClick={onClose}>Close</button>
        {notice && <span className={notice.error ? "lab-formula-note err" : "lab-formula-note"} role={notice.error ? "alert" : "status"}>{notice.text}</span>}
      </div>
      <details className="lab-formula-help">
        <summary>Function reference <small>{functions.length}</small></summary>
        {functions.length ? <dl>{functions.map((f) => <div key={f.name}><dt><code>{f.name}({f.args})</code></dt><dd>{f.desc}</dd></div>)}</dl>
          : <p className="cap">The engine has not listed its functions yet.</p>}
      </details>
      <details className="lab-formula-help">
        <summary>Examples <small>{FORMULA_EXAMPLES.length}</small></summary>
        {FORMULA_EXAMPLES.map((x) => (
          <div key={x.title} className="lab-formula-example">
            <h3>{x.title}</h3>
            <pre>{x.formula}</pre>
            {canEdit && <button type="button" className="dbtn sm" onClick={() => { setFormula(x.formula, null); setMode("formula"); setD((s) => (s.name.trim() ? s : { ...s, name: x.name })); }}>Use this example</button>}
          </div>
        ))}
      </details>
    </section>
  );
}
