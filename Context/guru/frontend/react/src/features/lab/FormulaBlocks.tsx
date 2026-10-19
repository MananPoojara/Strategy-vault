import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  MAX_AGO, MAX_PLOTS, PALETTE_GROUPS, TYPE_WORDS, blockLabel, childKey, emptyBlocks, emptyKeys, generate, getAt, guard, keyAt, paletteItems,
  parseFormula, place, problems, readBlocks, removeAt, renameLet, searchPalette, setAt, slotInfo, uniqueName, walk, fnDef,
  type BlockNode, type Blocks, type Edit, type PaletteItem,
} from "./blocks-model";
import { errorPosition } from "./formula";
import type { BlocksEditorProps } from "./FormulaEditor";
import type { FormulaInfo, Indicator } from "./types";
import "./blocks.css";

export interface FormulaBlocksProps extends BlocksEditorProps {
  /** The engine catalogue's indicators (Indicators group and indicator blocks). */
  indicators: Indicator[];
  /** Live Check (debounced): the same engine check as the editor's Check button. */
  check?(formula: string): Promise<FormulaInfo>;
  readOnly?: boolean;
  debounceMs?: number;
}
type Src = { from: "palette"; item: PaletteItem } | { from: "canvas"; key: string };
interface Ghost { label: string; x: number; y: number; target: string | null; ok: boolean; reason?: string; src?: string }
type CheckView = { status: "idle" } | { status: "checking" } | { status: "ok"; info: FormulaInfo } | { status: "error"; message: string; key?: string };
const OPEN_BY_DEFAULT = new Set(["Price", "Number", "Parameter", "My results"]);
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const kindClass = (n: BlockNode) => (n.k === "fn" ? fnDef(n.f).group : n.k).toLowerCase().replace(/[^a-z]+/g, "-");

/** Text box for a number: keeps what is typed while it is not yet a number; `onCommit` returns false when the value is refused. */
function NumInput({ value, label, onCommit }: { value: number; label: string; onCommit(v: number): boolean }) {
  const [text, setText] = useState(String(value));
  const [bad, setBad] = useState(false);
  useEffect(() => { setText((t) => (Number(t) === value && t.trim() !== "" ? t : String(value))); }, [value]);
  return (
    <input type="text" inputMode="decimal" className="fb-num" aria-label={label} value={text} size={Math.max(2, text.length)} aria-invalid={bad || undefined}
      onChange={(e) => { const t = e.target.value, v = Number(t); setText(t); setBad(!(t.trim() !== "" && Number.isFinite(v) && onCommit(v))); }}
      onBlur={() => { setText(String(value)); setBad(false); }} />
  );
}

/** Drag-and-drop block builder (design §5): palette → typed slots in "Name this result" rows and Line rows; it reports the generated formula. */
export function FormulaBlocks({ formula, blocks, onChange, showFormula, indicators, check, readOnly = false, debounceMs = 500 }: FormulaBlocksProps) {
  const model = useMemo(() => { const m = readBlocks(blocks); return m && generate(m).text === formula ? m : null; }, [blocks, formula]);
  const gen = useMemo(() => (model ? generate(model) : null), [model]);
  const probs = useMemo(() => (model ? problems(model, indicators) : []), [model, indicators]);
  const items = useMemo(() => (model ? paletteItems(model, indicators) : []), [model, indicators]);
  const [hist, setHist] = useState<{ past: Blocks[]; future: Blocks[]; tag?: string }>({ past: [], future: [] });
  const [sel, setSel] = useState<string | null>(null);
  const [status, setStatus] = useState<{ text: string; error?: boolean } | null>(null);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const [drawer, setDrawer] = useState(false);
  const [ghost, setGhost] = useState<Ghost | null>(null);
  const [paramOpen, setParamOpen] = useState<string | null>(null);
  const [checked, setChecked] = useState<CheckView>({ status: "idle" });
  const root = useRef<HTMLDivElement>(null), seq = useRef(0), focusNext = useRef<string | null>(null), suppressClick = useRef(false);
  const drag = useRef<{ src: Src; label: string; x0: number; y0: number; active: boolean } | null>(null);
  const handlers = useRef({ move: (_e: PointerEvent) => {}, up: (_e: PointerEvent) => {} });
  const stopDrag = useRef<() => void>(() => {});
  useEffect(() => () => stopDrag.current(), []);

  useEffect(() => { // after an edit, put the keyboard where the next step is
    const key = focusNext.current;
    if (!key || !root.current) return;
    focusNext.current = null;
    focusKey(key);
  });
  const focusKey = (key: string) => {
    const r = root.current, k = key.replace(/:row$/, "");
    const el = r?.querySelector<HTMLElement>(`[data-fb-block="${k}"]`) ?? r?.querySelector<HTMLElement>(`button[data-fb-slot="${k}"]`) ?? r?.querySelector<HTMLElement>(`[data-fb-name="${k.replace(/:name$/, "")}"]`);
    el?.focus();
  };

  // Live Check, debounced; only once the blocks are complete (empty slots are reported here, not by the engine).
  const complete = !!model && probs.length === 0, checkRef = useRef(check);
  checkRef.current = check; // the page passes a fresh closure each render; only the formula should restart the check
  useEffect(() => {
    const check = checkRef.current;
    if (!complete || !check || readOnly) { setChecked({ status: "idle" }); return; }
    const n = ++seq.current;
    setChecked({ status: "checking" });
    const t = setTimeout(() => {
      check(formula).then((info) => { if (n === seq.current) setChecked({ status: "ok", info }); }, (e) => {
        if (n !== seq.current) return;
        const text = message(e), pos = errorPosition(text);
        setChecked({ status: "error", message: text, key: pos && gen ? keyAt(gen, pos) : undefined });
      });
    }, debounceMs);
    return () => clearTimeout(t);
  }, [formula, complete, !check, readOnly, debounceMs]); // eslint-disable-line react-hooks/exhaustive-deps -- gen follows formula

  if (!model) return <Convert formula={formula} indicators={indicators} readOnly={readOnly} showFormula={showFormula}
    onConvert={(m) => { setHist({ past: [], future: [] }); onChange(generate(m).text, m); }} />;

  const say = (text: string, error = false) => setStatus({ text, error });
  const commit = (next: Blocks, tag?: string) => {
    setHist((h) => ({ past: tag && h.tag === tag ? h.past : [...h.past, model].slice(-200), future: [], tag }));
    onChange(generate(next).text, next);
  };
  const apply = (e: Edit, ok?: string, tag?: string): boolean => {
    if (!e.ok) { say(e.reason, true); return false; }
    commit(e.model, tag);
    if (ok) say(ok); else if (status?.error) setStatus(null);
    return true;
  };
  const undo = () => {
    const prev = hist.past[hist.past.length - 1];
    if (!prev) return;
    setHist({ past: hist.past.slice(0, -1), future: [model, ...hist.future] });
    onChange(generate(prev).text, prev); say("Undone.");
  };
  const redo = () => {
    const [next, ...rest] = hist.future;
    if (!next) return;
    setHist({ past: [...hist.past, model], future: rest });
    onChange(generate(next).text, next); say("Redone.");
  };
  const exists = (key: string | null): key is string => !!key && walk(model).some((x) => x.key === key);
  const where = (key: string) => { const r = key[0] === "l" ? model.lets[Number(key.slice(1).split(".")[0])]?.name : `Line ${Number(key.slice(1).split(".")[0]) + 1}`; return r || "the row"; };
  const afterPlace = (next: Blocks, key: string) => {
    const empty = emptyKeys(next, indicators), order = walk(next).map((x) => x.key);
    return empty.find((k) => k.startsWith(`${key}.`)) ?? empty.find((k) => order.indexOf(k) > order.indexOf(key)) ?? empty[0] ?? key;
  };
  const putAt = (key: string, node: BlockNode, from?: string, focus = false) => {
    const e = place(model, key, node, indicators, from);
    if (!apply(e, `${from ? "Moved" : "Placed"} ${blockLabel(node, indicators)} in ${where(key)}.`)) return;
    const nextSel = afterPlace((e as { model: Blocks }).model, key);
    setSel(nextSel);
    if (focus) focusNext.current = nextSel;
  };
  const remove = (key: string) => {
    const n = getAt(model, key);
    if (!n) return;
    if (apply(guard(model, removeAt(model, key), indicators), `Removed ${blockLabel(n, indicators)}.`)) { setSel(key); focusNext.current = key; }
  };
  const setNode = (key: string, node: BlockNode, tag: string) => apply(guard(model, setAt(model, key, node), indicators), undefined, tag);
  const choose = (item: PaletteItem) => {
    if (suppressClick.current || readOnly) return;
    const target = exists(sel) ? sel : emptyKeys(model, indicators)[0];
    if (!target) { say("Select a slot first: click it or Tab to it, then choose a block.", true); return; }
    putAt(target, item.make());
    setDrawer(false);
  };

  // ---- pointer drag (mouse, touch, pen): window listeners while a drag is live, hit-testing under the pointer ----
  const hit = (x: number, y: number): string | null => {
    const el = document.elementFromPoint?.(x, y), t = el?.closest?.("[data-fb-slot],[data-fb-bin],[data-fb-palette]");
    if (!t || !root.current?.contains(t)) return null;
    return t.getAttribute("data-fb-slot") ?? "bin";
  };
  const verdict = (src: Src, target: string | null): { ok: boolean; reason?: string } => {
    if (!target) return { ok: false };
    if (target === "bin") return src.from === "canvas" ? { ok: true, reason: "Release to delete" } : { ok: false, reason: "Drop it on a slot" };
    if (src.from === "canvas" && src.key === target) return { ok: false };
    const node = src.from === "palette" ? src.item.make() : getAt(model, src.key);
    if (!node) return { ok: false };
    const e = place(model, target, node, indicators, src.from === "canvas" ? src.key : undefined);
    return e.ok ? { ok: true } : { ok: false, reason: e.reason };
  };
  handlers.current.move = (ev) => {
    const d = drag.current;
    if (!d) return;
    if (!d.active) {
      if (Math.hypot(ev.clientX - d.x0, ev.clientY - d.y0) < 5) return;
      d.active = true;
      setDrawer(false); // a drag out of the narrow-screen drawer needs the canvas visible
    }
    if (ev.cancelable) ev.preventDefault();
    const target = hit(ev.clientX, ev.clientY), v = verdict(d.src, target);
    setGhost({ label: d.label, x: ev.clientX, y: ev.clientY, target, ...v, src: d.src.from === "canvas" ? d.src.key : undefined });
  };
  handlers.current.up = (ev) => {
    const d = drag.current;
    drag.current = null;
    setGhost(null);
    if (!d?.active) return;
    suppressClick.current = true;
    setTimeout(() => { suppressClick.current = false; }, 0);
    const target = hit(ev.clientX, ev.clientY);
    if (!target) { say("Dropped outside a slot; nothing changed."); return; }
    if (target === "bin") { if (d.src.from === "canvas") remove(d.src.key); return; }
    if (d.src.from === "canvas" && d.src.key === target) return;
    const node = d.src.from === "palette" ? d.src.item.make() : getAt(model, d.src.key);
    if (node) putAt(target, node, d.src.from === "canvas" ? d.src.key : undefined);
  };
  const startDrag = (e: ReactPointerEvent, src: Src, label: string) => {
    if (readOnly || (e.pointerType === "mouse" && e.button !== 0)) return;
    if ((e.target as Element).closest("input, select, textarea, [data-fb-nodrag]")) return;
    e.stopPropagation();
    stopDrag.current();
    drag.current = { src, label, x0: e.clientX, y0: e.clientY, active: false };
    const move = (ev: PointerEvent) => handlers.current.move(ev);
    const up = (ev: PointerEvent) => { stopDrag.current(); handlers.current.up(ev); };
    const cancel = () => { stopDrag.current(); drag.current = null; setGhost(null); };
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    stopDrag.current = () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); window.removeEventListener("pointercancel", cancel); stopDrag.current = () => {}; };
  };

  const onKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const t = e.target as HTMLElement, k = e.key.toLowerCase();
    if ((e.ctrlKey || e.metaKey) && (k === "z" || k === "y") && !t.matches("input, textarea, select")) {
      e.preventDefault();
      if (k === "y" || e.shiftKey) redo(); else undo();
      return;
    }
    const nav = t.hasAttribute("data-fb-nav") ? ".fb-canvas [data-fb-nav]" : t.hasAttribute("data-fb-pal") ? "[data-fb-pal]" : null;
    if (nav && e.key.startsWith("Arrow")) {
      const all = [...(root.current?.querySelectorAll<HTMLElement>(nav) ?? [])], i = all.indexOf(t);
      all[e.key === "ArrowLeft" || e.key === "ArrowUp" ? i - 1 : i + 1]?.focus();
      e.preventDefault();
    }
  };

  // ---- rows ----
  const lets = model.lets, plots = model.plots;
  const editRows = (next: Blocks, ok?: string, tag?: string) => apply(guard(model, next, indicators), ok, tag);
  const renameRow = (i: number, name: string) => {
    if (lets.some((r, j) => j !== i && r.name.toLowerCase() === name.toLowerCase())) { say(`Another result is already called “${name}”.`, true); return; }
    commit(renameLet(model, i, name), `name:l${i}`);
  };
  const removeLet = (i: number) => {
    const e = guard(model, { ...model, lets: lets.filter((_, j) => j !== i) }, indicators);
    if (!e.ok) { say(`“${lets[i].name}” is still used: ${e.reason} Remove those uses first.`, true); return; }
    apply(e, `Removed result “${lets[i].name}”.`); setSel(null);
  };
  const moveLet = (i: number, d: -1 | 1) => {
    const next = [...lets];
    [next[i], next[i + d]] = [next[i + d], next[i]];
    editRows({ ...model, lets: next });
  };
  const addLet = () => {
    const name = uniqueName(lets.map((r) => r.name), "Value");
    commit({ ...model, lets: [...lets, { name, expr: null }] });
    setSel(`l${lets.length}`); focusNext.current = `l${lets.length}`;
  };
  const addPlot = () => {
    if (plots.length >= MAX_PLOTS) return;
    commit({ ...model, plots: [...plots, { name: uniqueName(plots.map((r) => r.name), `Line ${plots.length + 1}`), onPrice: false, expr: null }] });
    setSel(`p${plots.length}`); focusNext.current = `p${plots.length}`;
  };
  const setPlot = (i: number, patch: Partial<Blocks["plots"][number]>, tag?: string) => commit({ ...model, plots: plots.map((r, j) => (j === i ? { ...r, ...patch } : r)) }, tag);

  // ---- rendering ----
  const probAt = new Map<string, string>();
  for (const p of probs) if (!probAt.has(p.key)) probAt.set(p.key, p.message);
  const errKey = checked.status === "error" ? checked.key : undefined;
  const slot = (key: string): ReactNode => {
    const n = getAt(model, key), info = slotInfo(model, key, indicators);
    const over = ghost?.target === key ? (ghost.ok ? "ok" : "no") : undefined;
    if (!n) return (
      <button type="button" className="fb-slot" data-fb-slot={key} data-fb-nav data-type={info.type} data-over={over} aria-current={sel === key || undefined}
        aria-invalid={errKey === key || undefined} aria-label={`Empty ${info.label} slot in ${where(key)}: needs ${TYPE_WORDS[info.type]}${info.optional ? ", optional" : ""}`}
        onFocus={() => setSel(key)} onClick={() => setSel(key)}>{info.label}{info.optional ? "?" : ""}</button>
    );
    return <span className="fb-slot fb-filled" data-fb-slot={key} data-over={over}>{block(key, n)}</span>;
  };
  const block = (key: string, n: BlockNode): ReactNode => {
    const label = blockLabel(n, indicators), problem = probAt.get(key) ?? (errKey === key && checked.status === "error" ? checked.message : undefined);
    let body: ReactNode;
    switch (n.k) {
      case "num": body = <NumInput value={n.v} label={`Number in ${where(key)}`} onCommit={(v) => setNode(key, { ...n, v }, `num:${key}`)} />; break;
      case "bool": case "price": case "ref": body = <span className="fb-word strong">{label}</span>; break;
      case "param": body = <>
        <span className="fb-word">Param</span> <span className="fb-word strong">“{n.label}”</span> <span className="fb-word">{n.def}</span>
        <button type="button" className="fb-mini" data-fb-nodrag aria-expanded={paramOpen === key} aria-label={`Edit Parameter ${n.label}`}
          onClick={() => setParamOpen(paramOpen === key ? null : key)}>⚙</button>
        {paramOpen === key && <span className="fb-param" role="group" aria-label={`Parameter ${n.label} settings`}>
          <label>Label <input type="text" value={n.label} maxLength={40} onChange={(e) => setNode(key, { ...n, label: e.target.value }, `plabel:${key}`)} /></label>
          {(["def", "min", "max", "step"] as const).map((f) => (
            <label key={f}>{{ def: "Default", min: "Min", max: "Max", step: "Step" }[f]}
              <NumInput value={n[f]} label={`Parameter ${n.label} ${f === "def" ? "default" : f}`} onCommit={(v) => setNode(key, { ...n, [f]: v }, `p${f}:${key}`)} /></label>
          ))}
        </span>}
      </>; break;
      case "ago": body = <>{slot(childKey(key, 0))}<NumInput value={n.n} label={`Bars ago in ${where(key)}`} onCommit={(v) => {
        if (!Number.isInteger(v) || v < 0 || v > MAX_AGO) { say(`Bars ago must be a whole number from 0 to ${MAX_AGO}.`, true); return false; }
        return setNode(key, { ...n, n: v }, `ago:${key}`);
      }} /><span className="fb-word">bars ago</span></>; break;
      case "ind": {
        const ind = indicators.find((i) => i.id === n.id);
        body = <>
          <span className="fb-word strong">{label}</span><span className="fb-word">(</span>
          {n.a.map((_, i) => <span key={i} className="fb-arg"><small>{ind?.params[i]?.label ?? `#${i + 1}`}</small>{slot(childKey(key, i))}</span>)}
          <span className="fb-word">)</span>
          {ind && ind.outputs.length > 1 && <select aria-label={`${label} output`} value={n.out ?? ind.outputs[0]}
            onChange={(e) => setNode(key, { k: "ind", id: n.id, a: n.a, ...(e.target.value !== ind.outputs[0] && { out: e.target.value }) }, `out:${key}`)}>
            {ind.outputs.map((o) => <option key={o} value={o}>{o}</option>)}
          </select>}
        </>;
        break;
      }
      case "fn": {
        const d = fnDef(n.f), show = d.show ?? [`${d.label}(`, ...d.args.flatMap((_, i) => (i ? [",", i] : [i])), ")"];
        body = show.map((x, i) => (typeof x === "number" ? <span key={i}>{slot(childKey(key, x))}</span> : <span key={i} className={i === 0 && !d.show ? "fb-word strong" : "fb-word"}>{x}</span>));
      }
    }
    return (
      <span className={`fb-block fb-k-${kindClass(n)}`} role="group" tabIndex={0} data-fb-block={key} data-fb-nav aria-label={`${label} block in ${where(key)}${problem ? `: ${problem}` : ""}`}
        aria-current={sel === key || undefined} aria-invalid={!!problem || undefined} title={problem} data-dragging={ghost?.src === key || undefined}
        onFocus={(e) => { if (e.target === e.currentTarget) setSel(key); }}
        onKeyDown={(e) => { if (!readOnly && e.target === e.currentTarget && (e.key === "Delete" || e.key === "Backspace")) { e.preventDefault(); remove(key); } }}
        onPointerDown={(e) => startDrag(e, { from: "canvas", key }, label)}>{body}</span>
    );
  };

  const shown = searchPalette(items, query), searching = query.trim() !== "";
  const isOpen = (g: string) => searching || (open[g] ?? OPEN_BY_DEFAULT.has(g));
  const toggle = (g: string) => setOpen((o) => ({ ...o, [g]: !isOpen(g) }));
  const palItem = (it: PaletteItem) => (
    <li key={it.id}><button type="button" className={`fb-pal-item fb-t-${it.type}`} data-fb-pal={it.id} title={`${it.hint} · gives ${TYPE_WORDS[it.type]}`}
      onPointerDown={(e) => startDrag(e, { from: "palette", item: it }, it.label)} onClick={() => choose(it)}>
      <b>{it.label}</b> <small>{it.hint}</small></button></li>
  );
  const groupHead = (g: string, count: number, sub = false) => (
    <button type="button" className={sub ? "fb-pal-h sub" : "fb-pal-h"} aria-expanded={isOpen(g)} onClick={() => toggle(g)}>{g.replace(/^Indicators\//, "")} <small>{count}</small></button>
  );
  const palette = (
    <aside className="fb-palette" id="fb-palette" data-fb-palette aria-label="Blocks palette">
      <div className="fb-pal-top">
        <input type="search" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search blocks" aria-label="Search blocks" />
        <button type="button" className="dbtn sm ghost fb-drawer-close" onClick={() => setDrawer(false)} aria-label="Close blocks palette">×</button>
      </div>
      <p className="cap fb-pal-tip">Drag a block onto a slot, or select a slot and press a block. Drop a placed block here to delete it.</p>
      {PALETTE_GROUPS.map((g) => {
        const list = shown.filter((it) => it.group === g);
        if (!list.length) return null;
        const subs = g === "Indicators" ? [...new Set(list.map((it) => it.sub ?? ""))] : [];
        return (
          <div key={g} className="fb-pal-group" role="group" aria-label={g}>
            {groupHead(g, list.length)}
            {isOpen(g) && (subs.length ? subs.map((s) => {
              const sl = list.filter((it) => it.sub === s), id = `Indicators/${s}`;
              return <div key={s} className="fb-pal-sub">{groupHead(id, sl.length, true)}{isOpen(id) && <ul>{sl.map(palItem)}</ul>}</div>;
            }) : <ul>{list.map(palItem)}</ul>)}
          </div>
        );
      })}
      {!shown.length && <p className="cap">No block matches “{query}”.</p>}
    </aside>
  );

  const rowErr = (k: string) => (errKey === `${k}:row` ? "true" : undefined);
  return (
    <div className="fb" ref={root} data-drawer={drawer ? "open" : "closed"} onKeyDown={onKey} data-lab-blocks>
      <div className="fb-bar" role="toolbar" aria-label="Block builder">
        {!readOnly && <button type="button" className="dbtn sm fb-drawer-open" aria-expanded={drawer} aria-controls="fb-palette" onClick={() => setDrawer(!drawer)}>Blocks</button>}
        <button type="button" className="dbtn sm" onClick={undo} disabled={readOnly || !hist.past.length} title="Ctrl+Z">Undo</button>
        <button type="button" className="dbtn sm" onClick={redo} disabled={readOnly || !hist.future.length} title="Ctrl+Shift+Z">Redo</button>
        {!readOnly && <span className="fb-bin" data-fb-bin data-over={ghost?.target === "bin" ? (ghost.ok ? "ok" : "no") : undefined}
          aria-label="Bin: drop a block here to delete it">Drop here to delete</span>}
        <span className={status?.error ? "fb-status err" : "fb-status"} role="status" aria-live="polite">{status?.text}</span>
      </div>
      <div className="fb-main">
        {!readOnly && palette}
        {drawer && <div className="fb-scrim" onClick={() => setDrawer(false)} aria-hidden="true" />}
        <fieldset className="fb-canvas" disabled={readOnly} aria-label="Indicator blocks">
          <h3>Named results <small>optional · use them in later rows from “My results”</small></h3>
          {lets.map((r, i) => (
            <div key={`l${i}`} className="fb-row" data-row-error={rowErr(`l${i}`)}>
              <input type="text" className="fb-name" data-fb-name={`l${i}`} aria-label={`Name of result ${i + 1}`} value={r.name} maxLength={40}
                aria-invalid={probAt.has(`l${i}:name`) || undefined} title={probAt.get(`l${i}:name`)} onChange={(e) => renameRow(i, e.target.value)} />
              <span className="fb-word">=</span>
              <span className="fb-row-expr">{slot(`l${i}`)}</span>
              <span className="fb-row-tools">
                <button type="button" className="fb-mini" aria-label={`Move ${r.name || `result ${i + 1}`} up`} disabled={i === 0} onClick={() => moveLet(i, -1)}>↑</button>
                <button type="button" className="fb-mini" aria-label={`Move ${r.name || `result ${i + 1}`} down`} disabled={i === lets.length - 1} onClick={() => moveLet(i, 1)}>↓</button>
                <button type="button" className="fb-mini" aria-label={`Remove result ${r.name || i + 1}`} onClick={() => removeLet(i)}>×</button>
              </span>
            </div>
          ))}
          <button type="button" className="dbtn sm ghost" onClick={addLet}>+ Name a result</button>
          <h3>Lines <small>{plots.length} / {MAX_PLOTS}</small></h3>
          {plots.map((r, i) => (
            <div key={`p${i}`} className="fb-row" data-row-error={rowErr(`p${i}`)}>
              <span className="fb-line">Line {i + 1}</span>
              <input type="text" className="fb-name" data-fb-name={`p${i}`} aria-label={`Name of line ${i + 1}`} value={r.name} maxLength={40}
                aria-invalid={probAt.has(`p${i}:name`) || undefined} title={probAt.get(`p${i}:name`)} onChange={(e) => setPlot(i, { name: e.target.value }, `pname:${i}`)} />
              <label className="fb-onprice"><input type="checkbox" checked={r.onPrice} onChange={(e) => setPlot(i, { onPrice: e.target.checked })} /> On price</label>
              <span className="fb-row-expr">{slot(`p${i}`)}</span>
              <span className="fb-row-tools">
                <button type="button" className="fb-mini" aria-label={`Remove line ${i + 1}`} disabled={plots.length <= 1}
                  onClick={() => { commit({ ...model, plots: plots.filter((_, j) => j !== i) }); setSel(null); say(`Removed line ${i + 1}.`); }}>×</button>
              </span>
            </div>
          ))}
          <button type="button" className="dbtn sm ghost" onClick={addPlot} disabled={plots.length >= MAX_PLOTS}>+ Add line</button>
          {plots.some((p) => p.onPrice) && <p className="cap">A line set to “On price” draws the whole indicator on the price chart.</p>}
        </fieldset>
      </div>
      <div className="fb-foot">
        {probs.length > 0 && <div className="fb-problems">
          <p className="cap">{probs.length === 1 ? "1 thing to finish" : `${probs.length} things to finish`} before the engine can check it:</p>
          <ul>{probs.slice(0, 6).map((p, i) => <li key={i}>{p.key === "model" ? p.message : <button type="button" className="fb-link" onClick={() => focusKey(p.key)}>{p.message}</button>}</li>)}</ul>
        </div>}
        {checked.status === "checking" && <p className="cap">Checking…</p>}
        {checked.status === "ok" && <p className="cap fb-ok">The engine accepts these blocks · {checked.info.params.length} parameter{checked.info.params.length === 1 ? "" : "s"} · lines {checked.info.plots.join(", ")}</p>}
        {checked.status === "error" && <div className="rnote lab-formula-err" role="alert"><p>{checked.message}</p>
          {checked.key && <button type="button" className="fb-link" onClick={() => focusKey(checked.key!)}>Show the block</button>}</div>}
        <details className="fb-preview" open>
          <summary>Formula <small>{formula.length} characters · generated, read-only here</small></summary>
          <pre aria-label="Generated formula">{formula}</pre>
        </details>
      </div>
      {ghost && createPortal(
        <div className="fb-ghost" data-ok={ghost.ok} style={{ left: ghost.x + 12, top: ghost.y + 12 }} aria-hidden="true">
          {ghost.label}{ghost.reason && <small>{ghost.reason}</small>}
        </div>, document.body)}
    </div>
  );
}

/** Blocks for a formula that was typed: convert it when the parser can read it, otherwise say why and point back to the Formula tab. */
function Convert({ formula, indicators, readOnly, showFormula, onConvert }: {
  formula: string; indicators: Indicator[]; readOnly: boolean; showFormula?: () => void; onConvert(m: Blocks): void;
}) {
  const parsed = useMemo(() => { try { return { model: parseFormula(formula, indicators) }; } catch (e) { return { error: message(e) }; } }, [formula, indicators]);
  const comments = /\/\//.test(formula);
  return (
    <div className="fb fb-convert" data-lab-blocks>
      {parsed.model ? <>
        <p>This indicator was written on the Formula tab. {readOnly ? "" : `Convert it to blocks to edit it by drag and drop${comments ? " (// comments are not kept)" : ""}.`}</p>
        {!readOnly && <button type="button" className="dbtn sm primary" onClick={() => onConvert(parsed.model!)}>Convert to blocks</button>}
      </> : <>
        <div className="rnote lab-formula-err" role="alert"><p>These blocks cannot show this formula: {parsed.error}</p></div>
        <p className="cap">Keep editing it on the Formula tab{readOnly ? "." : ", or start again with empty blocks (this replaces the formula)."}</p>
        <div className="lab-formula-actions">
          {showFormula && <button type="button" className="dbtn sm primary" onClick={showFormula}>Edit on the Formula tab</button>}
          {!readOnly && <button type="button" className="dbtn sm ghost" onClick={() => { if (window.confirm("Replace the formula with empty blocks?")) onConvert(emptyBlocks()); }}>Start with empty blocks</button>}
        </div>
      </>}
    </div>
  );
}
