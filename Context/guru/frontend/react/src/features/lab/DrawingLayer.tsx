import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { LabChartApi, LabSeriesApi } from "./globals";

/** TradingView-style drawings: a horizontal price line, or a trend line between two bars ([bar date, price] ends). */
export type Drawing = { kind: "h"; price: number } | { kind: "t"; a: [string, number]; b: [string, number] };
type Pt = [string, number];
type Move = { i: number; part: "a" | "b" | "line"; x0: number; y0: number; from: Drawing };

const fmt = (p: number) => p.toLocaleString("en-IN", { maximumFractionDigits: 2, minimumFractionDigits: 2 });

/** The drawings over the price chart. Draw by pressing and dragging (or click the start, then the end) with the line
 *  following the pointer; drag a line to move it, drag an end to reshape it, click a line and press Delete to remove it. */
export function DrawingLayer({ chart, series, times, drawings, tool, color, height, onChange, onDone }: {
  chart: LabChartApi; series: LabSeriesApi; times: string[]; drawings: Drawing[]; tool: "t" | "h" | null; color: string; height: number;
  onChange(next: Drawing[]): void; onDone(): void;
}) {
  const svg = useRef<SVGSVGElement>(null);
  const [, redraw] = useState(0);
  const [draft, setDraft] = useState<Drawing | null>(null); // the line being drawn
  const [moving, setMoving] = useState<{ i: number; d: Drawing } | null>(null);
  const [sel, setSel] = useState<number | null>(null);
  const move = useRef<Move | null>(null), pressed = useRef(false);
  const index = useMemo(() => new Map(times.map((t, i) => [t, i])), [times]);

  // follow the chart: pans and zooms, any pointer move or wheel over it (price-axis drags) and resizes
  useEffect(() => {
    const tick = () => redraw((n) => n + 1), ts = chart.timeScale(), box = svg.current?.parentElement;
    ts.subscribeVisibleLogicalRangeChange?.(tick);
    box?.addEventListener("pointermove", tick); box?.addEventListener("wheel", tick, { passive: true });
    const ro = typeof ResizeObserver !== "undefined" && box ? new ResizeObserver(tick) : null;
    if (ro && box) ro.observe(box);
    return () => {
      try { ts.unsubscribeVisibleLogicalRangeChange?.(tick); } catch { /* the chart was already removed by a rebuild */ }
      box?.removeEventListener("pointermove", tick); box?.removeEventListener("wheel", tick); ro?.disconnect();
    };
  }, [chart]);
  useEffect(() => { setDraft(null); pressed.current = false; }, [tool]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { setDraft(null); setSel(null); onDone(); }
      else if ((e.key === "Delete" || e.key === "Backspace") && sel !== null && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
        e.preventDefault(); onChange(drawings.filter((_, i) => i !== sel)); setSel(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [sel, drawings, onChange, onDone]);

  // new settings rebuild the chart: for that render the old chart is already removed and its API throws, so draw nothing
  let axis = 0;
  try { axis = chart.priceScale?.("right").width?.() ?? 0; } catch { return null; }
  const ts = chart.timeScale(), last = times.length - 1;
  const bar = (l: number) => times[Math.max(0, Math.min(last, Math.round(l)))];
  const xOf = (t: string) => { const i = index.get(t); return i === undefined ? null : ts.logicalToCoordinate?.(i) ?? null; };
  const yOf = (p: number) => series.priceToCoordinate?.(p) ?? null;
  const local = (e: { clientX: number; clientY: number }) => { const r = svg.current!.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top] as const; };
  const at = (x: number, y: number): Pt | null => {
    const l = ts.coordinateToLogical?.(x), p = series.coordinateToPrice?.(y);
    return l === null || l === undefined || p === null || p === undefined || !times.length ? null : [bar(l), p];
  };
  const moved = (m: Move, x: number, y: number): Drawing | null => {
    const l0 = ts.coordinateToLogical?.(m.x0), l1 = ts.coordinateToLogical?.(x), p0 = series.coordinateToPrice?.(m.y0), p1 = series.coordinateToPrice?.(y);
    if (l0 == null || l1 == null || p0 == null || p1 == null) return null;
    const db = Math.round(l1 - l0), dp = p1 - p0, d = m.from;
    if (d.kind === "h") return { kind: "h", price: d.price + dp };
    const shift = ([t, p]: Pt): Pt => [bar((index.get(t) ?? 0) + db), p + dp];
    if (m.part === "line") return { kind: "t", a: shift(d.a), b: shift(d.b) };
    const end = at(x, y);
    return end ? { ...d, [m.part]: end } : null;
  };

  const down = (e: ReactPointerEvent<SVGSVGElement>) => {
    const [x, y] = local(e), target = e.target as Element, i = target.getAttribute("data-i");
    if (tool) {
      const pt = at(x, y);
      if (!pt) return;
      e.preventDefault();
      svg.current?.setPointerCapture?.(e.pointerId);
      pressed.current = true;
      if (tool === "h") setDraft({ kind: "h", price: pt[1] });
      else if (!draft) setDraft({ kind: "t", a: pt, b: pt }); // a second click on a started line ends it on release
      return;
    }
    if (i === null) { setSel(null); return; }
    e.preventDefault(); e.stopPropagation();
    svg.current?.setPointerCapture?.(e.pointerId);
    const n = Number(i);
    move.current = { i: n, part: (target.getAttribute("data-part") as Move["part"]) ?? "line", x0: x, y0: y, from: drawings[n] };
    setSel(n);
  };
  const over = (e: ReactPointerEvent<SVGSVGElement>) => {
    const [x, y] = local(e);
    if (move.current) { const d = moved(move.current, x, y); if (d) setMoving({ i: move.current.i, d }); return; }
    if (!tool) return;
    const pt = at(x, y);
    if (!pt) return;
    if (tool === "h") setDraft({ kind: "h", price: pt[1] }); // the line follows the pointer before the click
    else if (draft?.kind === "t") setDraft({ ...draft, b: pt });
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
    if (!tool || !pressed.current || !draft) return;
    pressed.current = false;
    const pt = at(x, y);
    const done: Drawing = !pt ? draft : draft.kind === "h" ? { kind: "h", price: pt[1] } : { ...draft, b: pt };
    if (done.kind === "t" && done.a[0] === done.b[0]) { setDraft(done); return; } // a click without a drag: the end follows the pointer until the next click
    onChange([...drawings, done]);
    setDraft(null); setSel(drawings.length);
    onDone();
  };

  const width = svg.current?.clientWidth ?? 0, plot = Math.max(0, width - axis);
  const shapes = drawings.map((d, i) => (moving?.i === i ? moving.d : d));
  const shape = (d: Drawing, i: number | null) => {
    const live = i === null || i === sel || moving?.i === i, key = i ?? "draft";
    if (d.kind === "h") {
      const y = yOf(d.price);
      if (y === null) return null;
      return <g key={key} data-lab-drawing={d.kind}>
        <line x1={0} x2={plot} y1={y} y2={y} stroke={color} strokeWidth={live ? 2 : 1.5} strokeDasharray={i === null ? "6 4" : undefined} />
        {i !== null && <line className="lab-draw-hit" data-i={i} data-part="line" x1={0} x2={plot} y1={y} y2={y} />}
        <rect x={plot} y={y - 9} width={Math.max(axis, 60)} height={18} fill={color} />
        <text x={plot + 4} y={y + 4} fill="#fff" fontSize={11}>{fmt(d.price)}</text>
      </g>;
    }
    const [xa, ya, xb, yb] = [xOf(d.a[0]), yOf(d.a[1]), xOf(d.b[0]), yOf(d.b[1])];
    if (xa === null || ya === null || xb === null || yb === null) return null;
    return <g key={key} data-lab-drawing={d.kind}>
      <line x1={xa} y1={ya} x2={xb} y2={yb} stroke={color} strokeWidth={live ? 2.5 : 2} strokeLinecap="round" />
      {i !== null && <line className="lab-draw-hit" data-i={i} data-part="line" x1={xa} y1={ya} x2={xb} y2={yb} />}
      {(live || !tool) && ([["a", xa, ya], ["b", xb, yb]] as const).map(([part, x, y]) =>
        <circle key={part} className="lab-draw-end" data-i={i ?? undefined} data-part={part} cx={x} cy={y} r={live ? 5 : 3.5} fill="#fff" stroke={color} strokeWidth={2} />)}
      {i === null && <g transform={`translate(${Math.min(xb + 8, plot - 80)} ${yb - 22})`}>
        <rect width={78} height={18} rx={3} fill={color} /><text x={5} y={13} fill="#fff" fontSize={11}>{fmt(d.b[1])}</text></g>}
    </g>;
  };
  return <>
    <svg ref={svg} className={`lab-draw-layer${tool ? " on" : ""}`} style={{ height }} data-lab-draw-layer
      onPointerDown={down} onPointerMove={over} onPointerUp={up} onPointerCancel={up}>
      {shapes.map((d, i) => shape(d, i))}
      {draft && shape(draft, null)}
    </svg>
    {!tool && sel !== null && drawings[sel] && <p className="cap lab-drawing-hint" data-lab-draw-selected>
      Drag the line to move it{drawings[sel].kind === "t" ? ", drag an end to change it" : ""} · Delete removes it · Esc deselects</p>}
  </>;
}
