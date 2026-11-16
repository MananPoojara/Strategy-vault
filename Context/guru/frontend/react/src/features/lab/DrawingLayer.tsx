import { useEffect, useMemo, useRef, useState, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import type { LabChartApi, LabSeriesApi } from "./globals";
import { dateSpan, fmt, priceChange, shapeOf, withPts, type Bars, type Drawing, type Pt, type RenderCtx, type Shape } from "./drawings/model";
import { TOOLS } from "./drawings/registry";

export { FIB_LEVELS } from "./drawings/fib";
export { dateSpan, priceChange, type Drawing, type Pt } from "./drawings/model";
/** A toolbar tool: a registry tool id, or the rail's Measure ("measure") and Zoom in ("zoom"). */
export type DrawTool = string;
export type Magnet = "weak" | "strong" | null;
type Move = { i: number; part: number; x0: number; y0: number; from: Drawing }; // part −1: the whole drawing, else point k

/** How many points a tool takes: Measure and Zoom in are dragged like a two-point drawing. */
const pointsOf = (tool: string | null) => tool === "measure" || tool === "zoom" ? 2 : tool ? TOOLS[tool]?.points : undefined;

/** The drawings over the price chart. Draw by pressing and dragging (or clicking each point) with the next point
 *  following the pointer; drag a drawing to move it, drag a point to reshape it, click it and press Delete to remove it. */
export function DrawingLayer({ chart, series, times, bars, drawings, tool, color, height, magnet = null, stay = false, locked = false, hidden = false, eraser = false,
  onChange, onDone, onZoom }: {
  chart: LabChartApi; series: LabSeriesApi; times: string[]; drawings: Drawing[]; tool: DrawTool | null; color: string; height: number;
  /** The bars' prices and volume by bar index, for renderers and the magnet. */
  bars?: Omit<Bars, "d">;
  magnet?: Magnet; stay?: boolean; locked?: boolean; hidden?: boolean; eraser?: boolean;
  onChange(next: Drawing[]): void; onDone(): void;
  /** Zoom in changed the visible bars; the view before it. */
  onZoom?(before: { from: number; to: number } | null): void;
}) {
  const svg = useRef<SVGSVGElement>(null);
  const [, redraw] = useState(0);
  const [draft, setDraft] = useState<Pt[] | null>(null); // the points placed so far, then the one following the pointer
  const [moving, setMoving] = useState<{ i: number; d: Drawing } | null>(null);
  const [sel, setSel] = useState<number | null>(null);
  const [ruler, setRuler] = useState<Pt[] | null>(null); // Measure's box, until the next click
  const [shift, setShift] = useState(false); // Shift+drag measures without the tool
  const [edit, setEdit] = useState<number | null>(null); // the drawing whose text is being edited
  const move = useRef<Move | null>(null), pressed = useRef(false);
  const index = useMemo(() => new Map(times.map((t, i) => [t, i])), [times]);
  const all: Bars = useMemo(() => ({ d: times, o: bars?.o ?? [], h: bars?.h ?? [], l: bars?.l ?? [], c: bars?.c ?? [], v: bars?.v ?? [] }), [times, bars]);
  const want = tool ?? (shift ? "measure" : null), need = pointsOf(want), active = need === undefined ? null : want;
  const handlers = useRef({ down: (_e: PointerEvent, _forced: string) => undefined as void, finish: () => undefined as void });

  // follow the chart: pans and zooms, any pointer move or wheel over it (price-axis drags) and resizes
  useEffect(() => {
    const tick = () => redraw((n) => n + 1), ts = chart.timeScale(), box = svg.current?.parentElement;
    ts.subscribeVisibleLogicalRangeChange?.(tick);
    box?.addEventListener("pointermove", tick); box?.addEventListener("wheel", tick, { passive: true });
    const ro = typeof ResizeObserver !== "undefined" && box ? new ResizeObserver(tick) : null;
    if (ro && box) ro.observe(box);
    tick(); // the first render had no layer width (plot 0): draw again now that it is laid out
    return () => {
      try { ts.unsubscribeVisibleLogicalRangeChange?.(tick); } catch { /* the chart was already removed by a rebuild */ }
      box?.removeEventListener("pointermove", tick); box?.removeEventListener("wheel", tick); ro?.disconnect();
    };
  }, [chart]);
  useEffect(() => { setDraft(null); pressed.current = false; }, [tool]);
  useEffect(() => {
    const typing = (e: Event) => e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { setDraft(null); setSel(null); setRuler(null); setShift(false); onDone(); }
      else if (e.key === "Enter" && need === "multi" && !typing(e)) handlers.current.finish();
      else if ((e.key === "Delete" || e.key === "Backspace") && sel !== null && !locked && !typing(e)) {
        e.preventDefault(); onChange(drawings.filter((_, i) => i !== sel)); setSel(null);
      }
    };
    // TradingView: a click anywhere else on the chart deselects the drawing and ends a measurement; Shift+drag measures
    const box = svg.current?.parentElement;
    const away = (e: PointerEvent) => {
      if (!(e.target instanceof Element && e.target.closest("[data-i]"))) setSel(null);
      if (!draft) setRuler(null);
      if (e.shiftKey && !tool && !(e.target instanceof Element && e.target.closest("[data-i]"))) {
        e.stopPropagation(); e.preventDefault(); // the chart does not pan
        handlers.current.down(e, "measure");
      }
    };
    window.addEventListener("keydown", onKey); box?.addEventListener("pointerdown", away, true);
    return () => { window.removeEventListener("keydown", onKey); box?.removeEventListener("pointerdown", away, true); };
  }, [sel, drawings, onChange, onDone, draft, tool, need, locked]);

  // new settings rebuild the chart: for that render the old chart is already removed and its API throws, so draw nothing
  let axis = 0;
  try { axis = chart.priceScale?.("right").width?.() ?? 0; } catch { return null; }
  const ts = chart.timeScale(), last = times.length - 1;
  const bar = (l: number) => times[Math.max(0, Math.min(last, Math.round(l)))];
  const xAt = (i: number) => ts.logicalToCoordinate?.(i) ?? null;
  const xOf = (t: string) => { const i = index.get(t); return i === undefined ? null : xAt(i); };
  const yOf = (p: number) => series.priceToCoordinate?.(p) ?? null;
  const local = (e: { clientX: number; clientY: number }) => { const r = svg.current!.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top] as const; };
  const at = (x: number, y: number): Pt | null => {
    const l = ts.coordinateToLogical?.(x), p = series.coordinateToPrice?.(y);
    if (l === null || l === undefined || p === null || p === undefined || !times.length) return null;
    const k = Math.max(0, Math.min(last, Math.round(l)));
    if (!magnet) return [times[k], p];
    // Weak magnet snaps to the bar's nearest open, high, low or close within 30 px, Strong magnet always
    let best = p, dist = Infinity;
    for (const q of [all.o[k], all.h[k], all.l[k], all.c[k]]) {
      const qy = Number.isFinite(q) ? yOf(q) : null;
      if (qy !== null && Math.abs(qy - y) < dist) { dist = Math.abs(qy - y); best = q; }
    }
    return [times[k], magnet === "strong" || dist <= 30 ? best : p];
  };
  const moved = (m: Move, x: number, y: number): Drawing | null => {
    const l0 = ts.coordinateToLogical?.(m.x0), l1 = ts.coordinateToLogical?.(x), p0 = series.coordinateToPrice?.(m.y0), p1 = series.coordinateToPrice?.(y);
    if (l0 == null || l1 == null || p0 == null || p1 == null) return null;
    const db = Math.round(l1 - l0), dp = p1 - p0, pts = shapeOf(m.from).pts;
    if (m.part < 0) return withPts(m.from, pts.map(([t, p]): Pt => [index.has(t) ? bar(index.get(t)! + db) : t, p + dp]));
    const end = at(x, y);
    return end ? withPts(m.from, pts.map((q, k) => (k === m.part ? end : q))) : null;
  };
  // a press and release on the last placed point is a click: the next point follows the pointer until the next click
  const near = (p: Pt, q: Pt | undefined) => !!q && p[0] === q[0] && Math.abs((yOf(p[1]) ?? 0) - (yOf(q[1]) ?? 0)) < 4;

  const finish = (pts: Pt[], what = active) => {
    setDraft(null); setShift(false);
    if (!what) return;
    if (what === "measure") { setRuler(pts); onDone(); return; }
    if (what === "zoom") {
      const [a, b] = pts.map((p) => index.get(p[0]) ?? 0);
      if (a !== b && ts.setVisibleLogicalRange) { const before = ts.getVisibleLogicalRange?.() ?? null; ts.setVisibleLogicalRange({ from: Math.min(a, b), to: Math.max(a, b) }); onZoom?.(before); }
      onDone(); return;
    }
    onChange([...drawings, makeOf(what, pts)]);
    setSel(drawings.length);
    if (TOOLS[what].text) setEdit(drawings.length);
    if (!stay) onDone();
  };
  const finishMulti = () => { if (draft && draft.length > 2) finish(draft.slice(0, -1)); };
  const start = (x: number, y: number, what: string) => {
    const pt = at(x, y);
    if (!pt) return false;
    pressed.current = true;
    const n = pointsOf(what);
    if (n === 1 || n === "free") setDraft([pt]);
    else if (!draft) setDraft([pt, pt]); // a later press on a started drawing fixes its point on release
    return true;
  };
  handlers.current = {
    down: (e, forced) => { const [x, y] = local(e); if (start(x, y, forced)) setShift(true); },
    finish: finishMulti,
  };

  const down = (e: ReactPointerEvent<SVGSVGElement>) => {
    const [x, y] = local(e), target = e.target as Element, i = target.getAttribute("data-i");
    if (active) {
      if (!start(x, y, active)) return;
      e.preventDefault();
      svg.current?.setPointerCapture?.(e.pointerId);
      return;
    }
    if (i === null) { setSel(null); return; }
    if (locked) return;
    e.preventDefault(); e.stopPropagation();
    const n = Number(i);
    if (eraser) { onChange(drawings.filter((_, k) => k !== n)); setSel(null); return; }
    svg.current?.setPointerCapture?.(e.pointerId);
    const part = target.getAttribute("data-part") ?? "line";
    move.current = { i: n, part: part === "line" ? -1 : Number(part.slice(1)), x0: x, y0: y, from: drawings[n] };
    setSel(n);
  };
  const over = (e: ReactPointerEvent<SVGSVGElement>) => {
    const [x, y] = local(e);
    if (move.current) { const d = moved(move.current, x, y); if (d) setMoving({ i: move.current.i, d }); return; }
    if (!active) return;
    const pt = at(x, y);
    if (!pt) return;
    if (need === 1) setDraft([pt]); // the drawing follows the pointer before the click
    else if (need === "free") { if (pressed.current && draft && !near(pt, draft[draft.length - 1])) setDraft([...draft, pt]); }
    else if (draft) setDraft([...draft.slice(0, -1), pt]);
  };
  const up = (e: ReactPointerEvent<SVGSVGElement>) => {
    svg.current?.releasePointerCapture?.(e.pointerId);
    const [x, y] = local(e);
    if (move.current) {
      const m = move.current, d = moved(m, x, y) ?? moving?.d; // the release point is the final position
      move.current = null;
      if (d && (x !== m.x0 || y !== m.y0)) onChange(drawings.map((old, k) => (k === m.i ? d : old)));
      setMoving(null);
      return;
    }
    if (!active || !pressed.current || !draft) return;
    pressed.current = false;
    const pt = at(x, y) ?? draft[draft.length - 1];
    if (need === 1) { finish([pt]); return; }
    if (need === "free") { if (draft.length > 1) finish(draft); else setDraft(null); return; }
    const fixed = draft.slice(0, -1);
    if (near(pt, fixed[fixed.length - 1])) { setDraft([...fixed, pt]); return; }
    if (typeof need === "number" && fixed.length + 1 >= need) finish([...fixed, pt]);
    else setDraft([...fixed, pt, pt]);
  };
  const dbl = (e: ReactMouseEvent<SVGSVGElement>) => {
    if (need === "multi") { finishMulti(); return; }
    const i = (e.target as Element).getAttribute("data-i");
    if (active || i === null || locked) return;
    if (TOOLS[shapeOf(drawings[Number(i)]).kind]?.text) setEdit(Number(i));
  };

  const width = svg.current?.clientWidth ?? 0, plot = Math.max(0, width - axis);
  const ctxOf = (s: Shape, i: number | null, live: boolean): RenderCtx => ({
    x: xOf, xAt, y: yOf, index: (t) => index.get(t) ?? -1, bars: all, plot, axis, height, color, live, i, fmt, priceChange, dateSpan,
    hit: (x1, y1, x2, y2) => i !== null && <line className="lab-draw-hit" data-i={i} data-part="line" x1={x1} y1={y1} x2={x2} y2={y2} />,
    hitArea: (x, y, w, h) => i !== null && <rect className="lab-draw-hit area" data-i={i} data-part="line" x={x} y={y} width={w} height={h} />,
    hitPath: (d, fill) => i !== null && <path className={`lab-draw-hit${fill ? " area" : ""}`} data-i={i} data-part="line" d={d} />,
    // TradingView: the point handles show while the drawing is selected or hovered, not after it is dropped
    handles: (xy) => active && !live ? null : (xy ?? s.pts.map(([t, p]) => { const hx = xOf(t), hy = yOf(p); return hx === null || hy === null ? null : [hx, hy] as const; }))
      .map((q, k) => q && <circle key={`p${k}`} className={`lab-draw-end${live ? "" : " idle"}`} data-i={i ?? undefined} data-part={`p${k}`} cx={q[0]} cy={q[1]} r={5} fill="#fff" stroke={color} strokeWidth={2} />),
    label: (lines, x, y, fill, below = true) => {
      const w = Math.max(...lines.map((s) => s.length)) * 6.4 + 12, h = lines.length * 15 + 6;
      return <g transform={`translate(${Math.max(0, Math.min(x - w / 2, plot - w))} ${below ? y : y - h})`} data-lab-label><rect width={w} height={h} rx={3} fill={fill} />
        {lines.map((s, k) => <text key={k} x={w / 2} y={15 + k * 15} fill="#fff" fontSize={11} textAnchor="middle">{s}</text>)}</g>;
    },
  });
  const makeOf = (tool: string, pts: Pt[]): Drawing => TOOLS[tool].make?.(pts, ctxOf({ kind: tool, pts }, null, true)) ?? { kind: tool, pts };
  const shape = (d: Drawing, i: number | null, key: string | number = i ?? "draft"): ReactNode => {
    const s = shapeOf(d), def = TOOLS[s.kind];
    // a damaged or older saved drawing draws nothing instead of taking the whole chart down
    let body: ReactNode = null;
    try { body = def?.render(s, ctxOf(s, i, i === null || i === sel || moving?.i === i)); } catch { body = null; }
    return body == null ? null : <g key={key} data-lab-drawing={s.kind}>{body}</g>;
  };
  const preview = () => {
    if (!draft || !active) return null;
    if (active === "measure") return shape({ kind: "dpr", a: draft[0], b: draft[draft.length - 1] }, null);
    if (active === "zoom") {
      const [xa, xb] = [xOf(draft[0][0]), xOf(draft[draft.length - 1][0])];
      return xa === null || xb === null ? null : <rect data-lab-zoom-box x={Math.min(xa, xb)} y={0} width={Math.abs(xb - xa)} height={height} fill={color} opacity={0.12} />;
    }
    return TOOLS[active] ? shape(makeOf(active, draft), null) : null;
  };
  const edited = edit !== null ? drawings[edit] : undefined, ep = edited && shapeOf(edited).pts[0];
  const commitText = (value: string) => {
    if (edit === null || !edited) return;
    setEdit(null);
    onChange(value.trim() ? drawings.map((d, k) => (k === edit ? { ...d, text: value } as Drawing : d)) : drawings.filter((_, k) => k !== edit));
  };
  return <>
    <svg ref={svg} className={`lab-draw-layer${active ? " on" : ""}${locked ? " locked" : ""}${eraser ? " erase" : ""}`} style={{ height }} data-lab-draw-layer
      onPointerDown={down} onPointerMove={over} onPointerUp={up} onPointerCancel={up} onDoubleClick={dbl}>
      {!hidden && drawings.map((d, i) => shape(moving?.i === i ? moving.d : d, i))}
      {ruler && <g data-lab-ruler>{shape({ kind: "dpr", a: ruler[0], b: ruler[ruler.length - 1] }, null, "ruler")}</g>}
      {preview()}
    </svg>
    {ep && <textarea className="lab-draw-text" data-lab-draw-text aria-label="Text" autoFocus onFocus={(e) => e.currentTarget.select()} defaultValue={"text" in edited! ? String(edited.text ?? "") : ""}
      style={{ left: xOf(ep[0]) ?? 0, top: yOf(ep[1]) ?? 0, color }}
      onKeyDown={(e) => { e.stopPropagation(); if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); commitText(e.currentTarget.value); } else if (e.key === "Escape") setEdit(null); }}
      onBlur={(e) => commitText(e.currentTarget.value)} />}
    {!active && sel !== null && drawings[sel] && !locked && !eraser && <p className="cap lab-drawing-hint" data-lab-draw-selected>
      Drag the drawing to move it{shapeOf(drawings[sel]).pts.length > 1 ? ", drag a point to change it" : ""} · Delete removes it · Esc deselects</p>}
  </>;
}
