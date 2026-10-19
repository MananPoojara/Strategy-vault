import { useState } from "react";
import { Segmented } from "./ui";

type Cell = number | null;
/** Low → high on the theme's red and blue tokens (both themes define them). */
const shade = (t: number) => `color-mix(in oklab, var(--blue) ${Math.round(t * 100)}%, var(--red))`;
const S = 200, H = 120, C = Math.cos(Math.PI / 6); // isometric floor size and height of the surface

/** AmiBroker 3D optimization chart: one metric over two optimized settings, as an isometric surface or a heatmap.
 *  `table` rows are dim values then `cols` (the optimizer's table=1 list). */
export function OptimizeSurface({ dims, table, cols, metricLabel, show, label }: {
  dims: { path: string; values: number[] }[]; table: Cell[][]; cols: string[];
  metricLabel(key: string): string; show(key: string, v: Cell): string; label(path: string): string;
}) {
  const [metric, setMetric] = useState(cols.includes("cagr") ? "cagr" : cols[0]);
  const [view, setView] = useState<"3d" | "heat">("3d");
  const [xs, ys] = [dims[0].values, dims[1].values], k = 2 + cols.indexOf(metric);
  const at = new Map(table.map((r) => [`${r[0]}|${r[1]}`, r[k]]));
  const z: Cell[][] = xs.map((x) => ys.map((y) => { const v = at.get(`${x}|${y}`); return typeof v === "number" && Number.isFinite(v) ? v : null; }));
  const finite = z.flat().filter((v): v is number => v !== null);
  const [lo, hi] = [Math.min(...finite), Math.max(...finite)];
  const t = (v: number) => (hi > lo ? (v - lo) / (hi - lo) : 0.5);
  const flat = xs.length < 2 || ys.length < 2, mode: "3d" | "heat" = flat ? "heat" : view;
  const [nx, ny] = [label(dims[0].path), label(dims[1].path)];
  const tip = (i: number, j: number, v: Cell) => `${nx} ${xs[i]} · ${ny} ${ys[j]} · ${metricLabel(metric)} ${show(metric, v)}`;
  let chart;
  if (!finite.length) chart = <p className="cap">No values of {metricLabel(metric)} in this grid.</p>;
  else if (mode === "3d") {
    const pt = (i: number, j: number, h: number) => { const u = i / (xs.length - 1), v = j / (ys.length - 1); return [(u - v) * C * S, (u + v) * 0.5 * S - h * H] as const; };
    const xy = (p: readonly [number, number]) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`;
    const quads: { d: string; v: number; i: number; j: number }[] = [];
    for (let s = 0; s <= xs.length + ys.length - 4; s++) // back to front: the painter's order of the isometric view
      for (let i = 0; i < xs.length - 1; i++) {
        const j = s - i;
        if (j < 0 || j >= ys.length - 1) continue;
        const c = [z[i][j], z[i + 1][j], z[i + 1][j + 1], z[i][j + 1]];
        if (c.some((v) => v === null)) continue;
        const [a, b, e, f] = c as number[];
        quads.push({ d: [pt(i, j, t(a)), pt(i + 1, j, t(b)), pt(i + 1, j + 1, t(e)), pt(i, j + 1, t(f))].map(xy).join(" "), v: (a + b + e + f) / 4, i, j });
      }
    const floor = [pt(0, 0, 0), pt(xs.length - 1, 0, 0), pt(xs.length - 1, ys.length - 1, 0), pt(0, ys.length - 1, 0)];
    const [l, f, r] = [floor[3], floor[2], floor[1]]; // left, front and right floor corners
    chart = <svg className="lab-surface-svg" viewBox="-250 -150 500 400" role="img" aria-label={`3D surface of ${metricLabel(metric)} over ${nx} and ${ny}`} data-lab-surface="3d">
      <polygon className="lab-surface-floor" points={floor.map(xy).join(" ")} />
      <line className="lab-surface-axis" x1={l[0]} y1={l[1]} x2={l[0]} y2={l[1] - H} />
      <text className="lab-surface-tick" x={l[0] - 6} y={l[1] - H} textAnchor="end">{show(metric, hi)}</text>
      <text className="lab-surface-tick" x={l[0] - 6} y={l[1]} textAnchor="end">{show(metric, lo)}</text>
      {quads.map((q) => <polygon key={`${q.i}|${q.j}`} className="lab-surface-quad" points={q.d} style={{ fill: shade(t(q.v)) }}><title>{tip(q.i, q.j, q.v)}</title></polygon>)}
      <text className="lab-surface-name" x={(l[0] + f[0]) / 2 - 14} y={(l[1] + f[1]) / 2 + 30} textAnchor="middle" transform={`rotate(30 ${(l[0] + f[0]) / 2 - 14} ${(l[1] + f[1]) / 2 + 30})`}>{nx}</text>
      <text className="lab-surface-name" x={(r[0] + f[0]) / 2 + 14} y={(r[1] + f[1]) / 2 + 30} textAnchor="middle" transform={`rotate(-30 ${(r[0] + f[0]) / 2 + 14} ${(r[1] + f[1]) / 2 + 30})`}>{ny}</text>
      <text className="lab-surface-tick" x={l[0]} y={l[1] + 16} textAnchor="middle">{xs[0]}</text>
      <text className="lab-surface-tick" x={f[0]} y={f[1] + 16} textAnchor="middle">{xs[xs.length - 1]} · {ys[ys.length - 1]}</text>
      <text className="lab-surface-tick" x={r[0]} y={r[1] + 16} textAnchor="middle">{ys[0]}</text>
    </svg>;
  } else {
    const [w, h] = [400 / xs.length, 240 / ys.length];
    chart = <svg className="lab-surface-svg" viewBox="-70 -10 490 300" role="img" aria-label={`Heatmap of ${metricLabel(metric)} over ${nx} and ${ny}`} data-lab-surface="heat">
      {z.map((col, i) => col.map((v, j) => <rect key={`${i}|${j}`} x={i * w} y={240 - (j + 1) * h} width={w} height={h} className="lab-surface-cell"
        style={{ fill: v === null ? "transparent" : shade(t(v)) }}><title>{tip(i, j, v)}</title></rect>))}
      <text className="lab-surface-tick" x={0} y={256} textAnchor="start">{xs[0]}</text>
      <text className="lab-surface-tick" x={400} y={256} textAnchor="end">{xs[xs.length - 1]}</text>
      <text className="lab-surface-name" x={200} y={280} textAnchor="middle">{nx}</text>
      <text className="lab-surface-tick" x={-6} y={240} textAnchor="end">{ys[0]}</text>
      <text className="lab-surface-tick" x={-6} y={10} textAnchor="end">{ys[ys.length - 1]}</text>
      <text className="lab-surface-name" x={-40} y={120} textAnchor="middle" transform="rotate(-90 -40 120)">{ny}</text>
    </svg>;
  }
  return (
    <div className="lab-surface" data-lab-optimize-surface>
      <div className="lab-row lab-surface-controls">
        <label className="dfield">Metric
          <select aria-label="Metric" value={metric} onChange={(e) => setMetric(e.target.value)}>{cols.map((c) => <option key={c} value={c}>{metricLabel(c)}</option>)}</select>
        </label>
        <Segmented<"3d" | "heat"> label="Chart" value={mode} onChange={setView} options={[{ value: "3d", label: "3D chart", disabled: flat }, { value: "heat", label: "Heatmap" }]} />
      </div>
      {chart}
      {finite.length > 0 && <div className="lab-surface-legend" aria-label={`Colour scale: ${metricLabel(metric)} from ${show(metric, lo)} to ${show(metric, hi)}`}>
        <span>{show(metric, lo)}</span>
        <span className="lab-surface-ramp">{Array.from({ length: 10 }, (_, i) => <i key={i} style={{ background: shade(i / 9) }} />)}</span>
        <span>{show(metric, hi)}</span><span className="cap">{metricLabel(metric)}</span>
      </div>}
      <p className="cap">Values from the all-results list ({mode === "3d" ? "fixed isometric view, not rotatable as in AmiBroker; each facet coloured by its four corners' mean; facets with a missing corner are left open" : "empty cells: no value"}).</p>
    </div>
  );
}
