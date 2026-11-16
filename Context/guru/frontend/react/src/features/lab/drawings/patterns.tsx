import type { Shape, RenderCtx, ToolDef } from "./model";
import { path, px } from "./model";

type P = [number, number];
/** TradingView's pattern colours: harmonic and Elliott tools take the chart's blue. */
const GREEN = "#089981", PURPLE = "#9C27B0", TEAL = "#159980";

/** TradingView prints a point's label above a peak and below a trough. */
const peak = (d: Shape, k: number) => { const p = d.pts[k][1], a = d.pts[k - 1]?.[1], b = d.pts[k + 1]?.[1]; return (a === undefined || p >= a) && (b === undefined || p >= b); };
const tag = (d: Shape, q: P[], k: number, s: string, col: string) => s && <text key={`t${k}`} data-lab-point={s} x={q[k][0]} y={q[k][1] + (peak(d, k) ? -8 : 18)}
  fill={col} fontSize={12} fontWeight={600} textAnchor="middle">{s}</text>;

/** A dashed line from point i to j, its middle labelled |p[n1] − p[n0]| / |p[d1] − p[d0]|. */
type Leg = [i: number, j: number, n0: number, n1: number, d0: number, d1: number];
/** Polyline patterns: filled triangles (point indexes), ratio legs and point labels. */
const pattern = (col: string | null, letters: string[], fills: number[][] = [], legs: Leg[] = []) => (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  const k = col ?? c.color, p = d.pts.map((t) => t[1]);
  return <>
    {fills.filter((f) => f.every((i) => q[i])).map((f) => <g key={`f${f}`}>
      <path d={path(f.map((i) => q[i]), true)} fill={k} opacity={0.2} />{c.hitPath(path(f.map((i) => q[i]), true), true)}</g>)}
    <path d={path(q)} stroke={k} strokeWidth={2} fill="none" strokeLinejoin="round" />
    {c.hitPath(path(q))}
    {legs.filter((l) => Math.max(...l) < q.length).map(([i, j, n0, n1, d0, d1]) => {
      const den = Math.abs(p[d1] - p[d0]), [a, b] = [q[i], q[j]];
      return <g key={`l${i}${j}`} data-lab-ratio={`${i}-${j}`}>
        <line x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={k} strokeWidth={1} strokeDasharray="4 4" />
        {den > 0 && c.label([(Math.abs(p[n1] - p[n0]) / den).toFixed(3)], (a[0] + b[0]) / 2, (a[1] + b[1]) / 2 - 10.5, k)}</g>;
    })}
    {letters.map((s, i) => q[i] && tag(d, q, i, s, k))}
    {c.handles()}
  </>;
};

/** Head and shoulders: start, left shoulder, neck, head, neck, right shoulder, end; the neckline runs through the two necks. */
const shoulders = (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  let neck = null;
  if (q.length >= 5) {
    const [[x2, y2], [x4, y4]] = [q[2], q[4]];
    const nl = (x: number) => x4 === x2 ? y2 : y2 + (y4 - y2) * (x - x2) / (x4 - x2);
    // where the outer leg (outer point a, shoulder b) crosses the neckline, else its outer point
    const cross = (a: P, b: P): P => { const fa = a[1] - nl(a[0]), fb = b[1] - nl(b[0]), t = fa === fb ? 0 : Math.max(0, Math.min(1, fa / (fa - fb)));
      return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]; };
    const L = cross(q[0], q[1]), R = q[6] ? cross(q[6], q[5]) : q[4];
    const fills = [[L, q[1], q[2]], [q[2], q[3], q[4]], ...(q[6] ? [[q[4], q[5], R]] : [])];
    neck = <>
      {fills.map((f, k) => <g key={`f${k}`}><path d={path(f, true)} fill={GREEN} opacity={0.2} />{c.hitPath(path(f, true), true)}</g>)}
      <line data-lab-neckline x1={L[0]} y1={L[1]} x2={R[0]} y2={R[1]} stroke={GREEN} strokeWidth={2} />
    </>;
  }
  const names: [number, string][] = [[1, "Left Shoulder"], [3, "Head"], [5, "Right Shoulder"]];
  return <>
    {neck}
    <path d={path(q)} stroke={GREEN} strokeWidth={2} fill="none" strokeLinejoin="round" />
    {c.hitPath(path(q))}
    {names.filter(([k]) => q[k]).map(([k, s]) => { const up = peak(d, k);
      return <g key={s} data-lab-point={s}>{c.label([s], q[k][0], q[k][1] + (up ? -8 : 8), GREEN, !up)}</g>; })}
    {c.handles()}
  </>;
};

/** Triangle pattern: A B C D, the A–C and B–D lines drawn on to their apex (to the last point when they do not meet ahead). */
const triangle = (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  let sides = null;
  const [a, b, cc, dd] = q;
  if (dd && cc[0] !== a[0] && dd[0] !== b[0]) {
    const m1 = (cc[1] - a[1]) / (cc[0] - a[0]), m2 = (dd[1] - b[1]) / (dd[0] - b[0]);
    const ac = (x: number): P => [x, a[1] + m1 * (x - a[0])], bd = (x: number): P => [x, b[1] + m2 * (x - b[0])];
    const ax = m1 === m2 ? -Infinity : (b[1] - a[1] + m1 * a[0] - m2 * b[0]) / (m1 - m2), end = Math.max(cc[0], dd[0]);
    const ends = ax > end ? [ac(ax)] : [ac(end), bd(end)];
    sides = <>
      <path data-lab-triangle d={path([a, ...ends, b], true)} fill={PURPLE} opacity={0.2} />
      {c.hitPath(path([a, ...ends, b], true), true)}
      <path d={path([a, ends[0]]) + path([b, ends[ends.length - 1]])} stroke={PURPLE} strokeWidth={2} fill="none" />
    </>;
  }
  return <>{sides}{pattern(PURPLE, ["A", "B", "C", "D"])(d, c)}</>;
};

/** The x of each cycle: from the first point, every (second − first) bars on across the plot. */
const cycles = (d: Shape, c: RenderCtx) => {
  const i0 = c.index(d.pts[0][0]), i1 = c.index((d.pts[1] ?? d.pts[0])[0]), step = i1 - i0, xs: number[] = [];
  if (i0 < 0 || i1 < 0) return xs;
  for (let k = 0; k < 2000; k++) {
    const x = c.xAt(i0 + k * step);
    if (x === null || (k > 1 && (x > c.plot || x < 0))) break;
    xs.push(x);
    if (!step) break;
  }
  return xs;
};
const cyclic = (d: Shape, c: RenderCtx) => <>
  {cycles(d, c).map((x, k) => <g key={k}><line data-lab-cycle x1={x} x2={x} y1={0} y2={c.height} stroke={TEAL} strokeWidth={1} />{c.hit(x, 0, x, c.height)}</g>)}
  {c.handles()}
</>;
/** Time cycles: a half-circle on the first point's price over each cycle, above it when the second point is higher. */
const timeCycles = (d: Shape, c: RenderCtx) => {
  const y = c.y(d.pts[0][1]), yb = c.y((d.pts[1] ?? d.pts[0])[1]), xs = cycles(d, c);
  if (y === null || yb === null) return null;
  const sweep = (yb <= y) === (xs[1] >= xs[0]) ? 1 : 0;
  const arcs = xs.slice(1).map((x, k) => `M${xs[k]} ${y}A${Math.abs(x - xs[k]) / 2} ${Math.abs(x - xs[k]) / 2} 0 0 ${sweep} ${x} ${y}`);
  return <>
    {arcs.map((a, k) => <g key={k}><path data-lab-cycle d={a} stroke={TEAL} strokeWidth={1} fill={TEAL} fillOpacity={0.15} />{c.hitPath(a)}</g>)}
    {c.handles()}
  </>;
};
/** Sine line: a peak (or trough) at the first point, the opposite one at the second, continued across the plot. */
const sine = (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  const [[xa, ya], [xb, yb]] = [q[0], q[1] ?? q[0]];
  if (xa === xb) return c.handles();
  const amp = (ya - yb) / 2, mid = (ya + yb) / 2, pts: P[] = [];
  for (let x = 0; x <= Math.max(c.plot, xa, xb); x += 2) pts.push([x, +(mid + amp * Math.cos(Math.PI * (x - xa) / (xb - xa))).toFixed(1)]);
  return <><path data-lab-sine d={path(pts)} stroke={TEAL} strokeWidth={2} fill="none" />{c.hitPath(path(pts))}{c.handles()}</>;
};

const XABCD = ["X", "A", "B", "C", "D"];
const wave = (labels: string) => ["", ...labels.split("").map((s) => `(${s})`)];

export const PATTERNS: ToolDef[] = [
  { id: "xabcd", name: "XABCD pattern", section: "CHART PATTERNS", points: 5,
    render: pattern(null, XABCD, [[0, 1, 2], [2, 3, 4]], [[0, 2, 1, 2, 0, 1], [1, 3, 2, 3, 1, 2], [2, 4, 3, 4, 2, 3], [0, 4, 1, 4, 0, 1]]),
    icon: <><path d="M3 18 7 5l4 9 5-7 5 12"/><path d="M3 18l8-4M11 14l10 5" strokeDasharray="2 2"/></> },
  // Cypher: B against XA, C as an extension of XA (XC/XA), D a retracement of XC (CD/XC)
  { id: "cypher", name: "Cypher pattern", section: "CHART PATTERNS", points: 5,
    render: pattern(null, XABCD, [[0, 1, 2], [0, 3, 4]], [[0, 2, 1, 2, 0, 1], [0, 3, 0, 3, 0, 1], [0, 4, 3, 4, 0, 3]]),
    icon: <><path d="M3 19 7 9l4 5 4-11 6 12"/><path d="M3 19l8-5M3 19 15 3" strokeDasharray="2 2"/></> },
  { id: "headshoulders", name: "Head and shoulders", section: "CHART PATTERNS", points: 7, render: shoulders,
    icon: <><path d="m2 18 3-7 3 4 4-11 4 11 3-4 3 7"/><path d="M4 15h16" strokeDasharray="2 2"/></> },
  { id: "abcd", name: "ABCD pattern", section: "CHART PATTERNS", points: 4,
    render: pattern(GREEN, ["A", "B", "C", "D"], [], [[0, 2, 1, 2, 0, 1], [1, 3, 2, 3, 1, 2]]),
    icon: <><path d="m3 19 6-14 5 8 7-9"/><path d="m3 19 11-6M9 5l12-1" strokeDasharray="2 2"/></> },
  { id: "trianglepattern", name: "Triangle pattern", section: "CHART PATTERNS", points: 4, render: triangle,
    icon: <><path d="m3 4 5 16 6-12 3 7"/><path d="m3 4 18 8-13 8" strokeDasharray="2 2"/></> },
  // drives at points 2, 4 and 6; the corrections at 3 and 5
  { id: "threedrives", name: "Three drives pattern", section: "CHART PATTERNS", points: 7,
    render: pattern(null, [], [], [[1, 3, 2, 3, 1, 2], [2, 4, 3, 4, 2, 3], [3, 5, 4, 5, 3, 4], [4, 6, 5, 6, 4, 5]]),
    icon: <><path d="m2 21 3-5 3 3 4-8 3 3 4-9 3 3"/></> },
  { id: "elliott-impulse", name: "Elliott impulse wave (12345)", section: "ELLIOTT WAVES", points: 6, render: pattern(null, wave("12345")),
    icon: <><path d="m2 20 4-9 3 4 5-11 3 6 5-6"/></> },
  { id: "elliott-correction", name: "Elliott correction wave (ABC)", section: "ELLIOTT WAVES", points: 4, render: pattern(null, wave("ABC")),
    icon: <><path d="m3 4 6 12 4-6 8 10"/></> },
  { id: "elliott-triangle", name: "Elliott triangle wave (ABCDE)", section: "ELLIOTT WAVES", points: 6, render: pattern(null, wave("ABCDE")),
    icon: <><path d="m2 4 4 16 4-12 4 9 3-6 4 4"/></> },
  { id: "elliott-double", name: "Elliott double combo wave (WXY)", section: "ELLIOTT WAVES", points: 4, render: pattern(null, wave("WXY")),
    icon: <><path d="m3 4 6 10 5-5 7 11"/></> },
  { id: "elliott-triple", name: "Elliott triple combo wave (WXYXZ)", section: "ELLIOTT WAVES", points: 6, render: pattern(null, wave("WXYXZ")),
    icon: <><path d="m2 3 4 8 3-3 4 8 3-3 6 9"/></> },
  { id: "cyclic", name: "Cyclic lines", section: "CYCLES", points: 2, render: cyclic,
    icon: <><path d="M4 3v18M10 3v18M16 3v18M22 3v18"/></> },
  { id: "timecycles", name: "Time cycles", section: "CYCLES", points: 2, render: timeCycles,
    icon: <><path d="M2 18a5 5 0 0 1 10 0 5 5 0 0 1 10 0M2 18h20"/></> },
  { id: "sine", name: "Sine line", section: "CYCLES", points: 2, render: sine,
    icon: <><path d="M2 12c2.5-8 5-8 7.5 0s5 8 7.5 0 3.5-6 5-4"/></> },
];
