import type { Pt, Shape, RenderCtx, ToolDef } from "./model";

type XY = readonly [number, number];
const RED = "#F23645", GREEN = "#089981", BLUE = "#2962FF";
const px = (c: RenderCtx, p: Pt | undefined): XY | null => {
  const x = p && c.x(p[0]), y = p && c.y(p[1]);
  return x == null || y == null ? null : [x, y];
};
/** Where the line from (x0,y0) through (x1,y1) meets the chart's edge on (x1,y1)'s side. */
const far = (c: RenderCtx, x0: number, y0: number, x1: number, y1: number): XY => {
  if (x1 === x0) return [x1, y1];
  const x = x1 > x0 ? c.plot : 0;
  return [x, y0 + (y1 - y0) * (x - x0) / (x1 - x0)];
};
const width = (c: RenderCtx) => (c.live ? 2 : 1.5);
const ln = (a: XY, b: XY, stroke: string, c: RenderCtx, dash?: string, key?: string | number) =>
  <line key={key} x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={stroke} strokeWidth={width(c)} strokeDasharray={dash} />;
const poly = (ps: XY[]) => `M${ps.map((p) => `${p[0]} ${p[1]}`).join("L")}Z`;
/** The price on the price axis, as Horizontal line shows it. */
const priceTag = (c: RenderCtx, y: number, p: number) => <>
  <rect x={c.plot} y={y - 9} width={Math.max(c.axis, 60)} height={18} fill={c.color} />
  <text x={c.plot + 4} y={y + 4} fill="#fff" fontSize={11}>{c.fmt(p)}</text>
</>;
/** The screen angle of a→b in degrees, as TradingView prints it: "45.00°". */
const angle = (a: XY, b: XY) => Math.atan2(a[1] - b[1], b[0] - a[0]) * 180 / Math.PI;
const deg = (v: number) => `${v < 0 ? "−" : ""}${Math.abs(v).toFixed(2)}°`;

/** A trend line; Ray extends past the second point, Extended line both ways, to the chart's edge. */
const trend = (d: Shape, c: RenderCtx) => {
  const [a, b] = d.pts, [xa, ya, xb, yb] = [c.x(a[0]), c.y(a[1]), c.x(b[0]), c.y(b[1])];
  if (xa === null || ya === null || xb === null || yb === null) return null;
  // before the layer is laid out (no width yet) the line is drawn to its points
  const [ex2, ey2] = d.ext && c.plot ? far(c, xa, ya, xb, yb) : [xb, yb];
  const [ex1, ey1] = d.ext === "both" && c.plot ? far(c, xb, yb, xa, ya) : [xa, ya];
  return <>
    <line x1={ex1} y1={ey1} x2={ex2} y2={ey2} stroke={c.color} strokeWidth={c.live ? 2.5 : 2} strokeLinecap="round" />
    {c.hit(ex1, ey1, ex2, ey2)}
    {c.handles()}
    {c.i === null && <g transform={`translate(${Math.min(xb + 8, c.plot - 80)} ${yb - 22})`}>
      <rect width={78} height={18} rx={3} fill={c.color} /><text x={5} y={13} fill="#fff" fontSize={11}>{c.fmt(b[1])}</text></g>}
  </>;
};
const two = (ext?: "right" | "both") => (pts: Pt[]) => ({ kind: "t" as const, a: pts[0], b: pts[1] ?? pts[0], ...(ext ? { ext } : {}) });

/** A plain segment between the first two points: what a 3-point tool shows after its first click. */
const segment = (c: RenderCtx, a: XY, b: XY) => <>{ln(a, b, c.color, c)}{c.hit(a[0], a[1], b[0], b[1])}{c.handles()}</>;

/** Least squares of the closes from bar i to bar j (either order), with TradingView's deviation and Pearson's R. */
const regression = (closes: number[], i: number, j: number) => {
  const [lo, hi] = [Math.min(i, j), Math.max(i, j)], ks: number[] = [];
  for (let k = lo; k <= hi; k++) if (Number.isFinite(closes[k])) ks.push(k);
  const n = ks.length;
  if (n < 2) return null;
  const mx = ks.reduce((s, k) => s + k, 0) / n, my = ks.reduce((s, k) => s + closes[k], 0) / n;
  let sxy = 0, sxx = 0, syy = 0;
  for (const k of ks) { sxy += (k - mx) * (closes[k] - my); sxx += (k - mx) ** 2; syy += (closes[k] - my) ** 2; }
  const slope = sxy / sxx, at = (k: number) => my + slope * (k - mx);
  const dev = Math.sqrt(ks.reduce((s, k) => s + (closes[k] - at(k)) ** 2, 0) / (n - 1)); // TradingView divides by n − 1
  return { lo, hi, at, dev, r: syy ? Math.abs(sxy) / Math.sqrt(sxx * syy) : 0 };
};

/** Pitchfork origins: Andrews from the first point, Schiff halfway to the second in price, Modified Schiff / Inside halfway in both. */
type Fork = "andrews" | "schiff" | "half";
/** TradingView's visible pitchfork levels (median 0) and colours. */
const FORK_LEVELS: [number, string][] = [[0.5, GREEN], [1, BLUE]];
const pitchfork = (style: Fork) => (d: Shape, c: RenderCtx) => {
  const [p0, p1, p2] = d.pts.map((p) => px(c, p));
  if (!p0 || !p1) return null;
  if (!p2) return segment(c, p0, p1);
  const o: XY = style === "andrews" ? p0 : style === "schiff" ? [p0[0], (p0[1] + p1[1]) / 2] : [(p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2];
  const m: XY = [(p1[0] + p2[0]) / 2, (p1[1] + p2[1]) / 2], [dx, dy] = [m[0] - o[0], m[1] - o[1]];
  const end = (s: XY): XY => (c.plot ? far(c, s[0], s[1], s[0] + dx, s[1] + dy) : [s[0] + dx, s[1] + dy]);
  // level L starts on the p1–p2 line, L halves of the way from its middle to p2 (−L towards p1)
  const at = (lv: number): XY => [m[0] + lv * (p2[0] - m[0]), m[1] + lv * (p2[1] - m[1])];
  const lines = [0, ...FORK_LEVELS.flatMap(([lv]) => [lv, -lv])].map((lv) => [lv, at(lv), end(at(lv))] as const);
  const band = (a: number, b: number, col: string) => {
    const [, sa, ea] = lines.find(([lv]) => lv === a)!, [, sb, eb] = lines.find(([lv]) => lv === b)!;
    return <path key={`${a}:${b}`} d={poly([sa, ea, eb, sb])} fill={col} opacity={0.2} stroke="none" />;
  };
  const col = (lv: number) => (lv ? FORK_LEVELS.find(([l]) => l === Math.abs(lv))![1] : RED);
  return <>
    {FORK_LEVELS.flatMap(([lv, cl], k) => { const prev = k ? FORK_LEVELS[k - 1][0] : 0; return [band(prev, lv, cl), band(-prev, -lv, cl)]; })}
    {style !== "andrews" && ln(p0, o, RED, c, "4 4")}
    {ln(p1, p2, RED, c)}
    {ln(o, m, RED, c)}
    {lines.map(([lv, s, e]) => <line key={lv} data-lab-level={lv} x1={s[0]} y1={s[1]} x2={e[0]} y2={e[1]} stroke={col(lv)} strokeWidth={width(c)} />)}
    {lines.filter(([lv]) => lv === 0 || Math.abs(lv) === 1).map(([lv, s, e]) => <g key={`h${lv}`}>{c.hit(s[0], s[1], e[0], e[1])}</g>)}
    {c.hit(o[0], o[1], m[0], m[1])}{c.hit(p1[0], p1[1], p2[0], p2[1])}
    {c.handles()}
  </>;
};

const forkIcon = (o: string) => <><path d={`${o}L12 12`}/><path d="M8 7l12 1M10 19l11-2M6 6l3 15"/><circle cx="7" cy="6" r="1.5"/><circle cx="9.5" cy="20" r="1.5"/></>;

// TradingView's Lines group: LINES, CHANNELS, PITCHFORKS
export const LINES: ToolDef[] = [
  { id: "t", name: "Trend line", section: "LINES", points: 2, shortcut: "Alt+T", make: two(), render: trend,
    icon: <><path d="m5 19 14-14"/><circle cx="4" cy="20" r="2"/><circle cx="20" cy="4" r="2"/></> },
  { id: "ray", name: "Ray", section: "LINES", points: 2, make: two("right"), render: trend,
    icon: <><path d="m5 19 16-16"/><circle cx="4" cy="20" r="2"/><circle cx="11" cy="13" r="2"/></> },
  { id: "infoline", name: "Info line", section: "LINES", points: 2,
    render: (d, c) => {
      const [a, b] = d.pts, pa = px(c, a), pb = px(c, b);
      if (!pa || !pb) return null;
      const bars = c.index(b[0]) - c.index(a[0]);
      return <>
        {ln(pa, pb, c.color, c)}{c.hit(pa[0], pa[1], pb[0], pb[1])}
        {c.label([c.priceChange(a[1], b[1]), c.dateSpan(bars, a[0], b[0]), deg(angle(pa, pb))], (pa[0] + pb[0]) / 2, Math.max(pa[1], pb[1]) + 8, c.color)}
        {c.handles()}
      </>;
    },
    icon: <><path d="m4 20 10-10"/><rect x="13" y="3" width="8" height="6" rx="1"/><circle cx="4" cy="20" r="2"/></> },
  { id: "x", name: "Extended line", section: "LINES", points: 2, make: two("both"), render: trend,
    icon: <><path d="m2 22 20-20"/><circle cx="8" cy="16" r="2"/><circle cx="16" cy="8" r="2"/></> },
  { id: "trendangle", name: "Trend angle", section: "LINES", points: 2,
    render: (d, c) => {
      const pa = px(c, d.pts[0]), pb = px(c, d.pts[1]);
      if (!pa || !pb) return null;
      const v = angle(pa, pb), t = v * Math.PI / 180, r = Math.min(50, Math.hypot(pb[0] - pa[0], pb[1] - pa[1])) || 1;
      return <>
        {ln(pa, pb, c.color, c)}
        {ln(pa, [pa[0] + r, pa[1]], c.color, c, "4 4")}
        <path d={`M${pa[0] + r} ${pa[1]}A${r} ${r} 0 0 ${v > 0 ? 0 : 1} ${pa[0] + r * Math.cos(t)} ${pa[1] - r * Math.sin(t)}`} fill="none" stroke={c.color} strokeWidth={1} />
        <text x={pa[0] + r + 6} y={pa[1] + (v > 0 ? -4 : 12)} fill={c.color} fontSize={11} data-lab-angle>{deg(v)}</text>
        {c.hit(pa[0], pa[1], pb[0], pb[1])}{c.handles()}
      </>;
    },
    icon: <><path d="m4 19 15-12M4 19h16"/><path d="M11 19a7 7 0 0 0-1.5-4.5"/></> },
  { id: "h", name: "Horizontal line", section: "LINES", points: 1, shortcut: "Alt+H", hint: "Move to a price and click",
    make: (pts) => ({ kind: "h", price: pts[0][1] }),
    render: (d, c) => {
      const y = c.y(d.pts[0][1]);
      if (y === null) return null;
      return <>
        <line x1={0} x2={c.plot} y1={y} y2={y} stroke={c.color} strokeWidth={c.live ? 2 : 1.5} strokeDasharray={c.i === null ? "6 4" : undefined} />
        {c.hit(0, y, c.plot, y)}
        {priceTag(c, y, d.pts[0][1])}
      </>;
    },
    icon: <><path d="M2 12h20"/><circle cx="9" cy="12" r="2"/></> },
  { id: "hray", name: "Horizontal ray", section: "LINES", points: 1, shortcut: "Alt+J", hint: "Move to a point and click",
    render: (d, c) => {
      const p = px(c, d.pts[0]);
      if (!p) return null;
      return <>
        <line x1={p[0]} x2={c.plot} y1={p[1]} y2={p[1]} stroke={c.color} strokeWidth={width(c)} />
        {c.hit(p[0], p[1], c.plot, p[1])}{priceTag(c, p[1], d.pts[0][1])}{c.handles()}
      </>;
    },
    icon: <><path d="M8 12h14"/><circle cx="6" cy="12" r="2"/></> },
  { id: "vline", name: "Vertical line", section: "LINES", points: 1, shortcut: "Alt+V", hint: "Move to a bar and click",
    render: (d, c) => {
      const x = c.x(d.pts[0][0]);
      if (x === null) return null;
      return <>
        <line x1={x} x2={x} y1={0} y2={c.height} stroke={c.color} strokeWidth={width(c)} />
        {c.hit(x, 0, x, c.height)}{c.label([d.pts[0][0]], x, c.height, c.color, false)}{c.handles([[x, c.height / 2]])}
      </>;
    },
    icon: <><path d="M12 2v20"/><circle cx="12" cy="12" r="2"/></> },
  { id: "crossline", name: "Cross line", section: "LINES", points: 1, shortcut: "Alt+C", hint: "Move to a point and click",
    render: (d, c) => {
      const p = px(c, d.pts[0]);
      if (!p) return null;
      return <>
        <line x1={0} x2={c.plot} y1={p[1]} y2={p[1]} stroke={c.color} strokeWidth={width(c)} />
        <line x1={p[0]} x2={p[0]} y1={0} y2={c.height} stroke={c.color} strokeWidth={width(c)} />
        {c.hit(0, p[1], c.plot, p[1])}{c.hit(p[0], 0, p[0], c.height)}
        {priceTag(c, p[1], d.pts[0][1])}{c.label([d.pts[0][0]], p[0], c.height, c.color, false)}{c.handles()}
      </>;
    },
    icon: <><path d="M2 12h20M12 2v20"/><circle cx="12" cy="12" r="2"/></> },

  { id: "channel", name: "Parallel channel", section: "CHANNELS", points: 3,
    render: (d, c) => {
      const [a, b, q] = d.pts, pa = px(c, a), pb = px(c, b);
      if (!pa || !pb) return null;
      if (!q) return segment(c, pa, pb);
      // the third point sets the other line's price offset from the base line at its bar
      const [ia, ib, iq] = [a, b, q].map((p) => c.index(p[0]));
      const off = q[1] - (ib === ia ? a[1] : a[1] + (b[1] - a[1]) * (iq - ia) / (ib - ia));
      const [qa, qb, ma, mb] = [px(c, [a[0], a[1] + off]), px(c, [b[0], b[1] + off]), px(c, [a[0], a[1] + off / 2]), px(c, [b[0], b[1] + off / 2])];
      if (!qa || !qb || !ma || !mb) return null;
      return <>
        <path d={poly([pa, pb, qb, qa])} fill={BLUE} opacity={0.2} stroke="none" />
        {ln(pa, pb, c.color, c)}{ln(qa, qb, c.color, c)}{ln(ma, mb, c.color, c, "4 4")}
        {c.hitPath(poly([pa, pb, qb, qa]), true)}{c.hit(pa[0], pa[1], pb[0], pb[1])}{c.hit(qa[0], qa[1], qb[0], qb[1])}
        {c.handles([pa, pb, [(qa[0] + qb[0]) / 2, (qa[1] + qb[1]) / 2]])}
      </>;
    },
    icon: <><path d="m3 15 14-9M7 20l14-9"/><path d="m5 17.5 14-9" strokeDasharray="2 2"/></> },
  { id: "regression", name: "Regression trend", section: "CHANNELS", points: 2,
    render: (d, c) => {
      const [a, b] = d.pts, pa = px(c, a), pb = px(c, b);
      if (!pa || !pb) return null;
      const g = regression(c.bars.c, c.index(a[0]), c.index(b[0]));
      if (!g) return segment(c, pa, pb); // shortcut: no closes in the span yet, upgrade if TradingView shows something else
      const pt = (k: number, z: number): XY | null => { const x = c.xAt(k), y = c.y(g.at(k) + z * g.dev); return x === null || y === null ? null : [x, y]; };
      const [b0, b1, u0, u1, l0, l1] = [pt(g.lo, 0), pt(g.hi, 0), pt(g.lo, 2), pt(g.hi, 2), pt(g.lo, -2), pt(g.hi, -2)];
      if (!b0 || !b1 || !u0 || !u1 || !l0 || !l1) return null;
      return <>
        <path d={poly([u0, u1, b1, b0])} fill={BLUE} opacity={0.15} stroke="none" />
        <path d={poly([b0, b1, l1, l0])} fill={RED} opacity={0.15} stroke="none" />
        {ln(u0, u1, BLUE, c, undefined, "u")}{ln(b0, b1, RED, c, "6 4", "b")}{ln(l0, l1, BLUE, c, undefined, "l")}
        <text x={l0[0]} y={l0[1] + 14} fill={BLUE} fontSize={11} data-lab-pearson>{g.r.toFixed(2)}</text>
        {c.hitPath(poly([u0, u1, l1, l0]), true)}{c.hit(u0[0], u0[1], u1[0], u1[1])}{c.hit(l0[0], l0[1], l1[0], l1[1])}
        {c.handles([g.lo === c.index(a[0]) ? b0 : b1, g.lo === c.index(a[0]) ? b1 : b0])}
      </>;
    },
    icon: <><path d="m3 14 18-8M3 20l18-8"/><path d="m3 17 18-8" strokeDasharray="2 2"/></> },
  { id: "flattb", name: "Flat top/bottom", section: "CHANNELS", points: 3,
    render: (d, c) => {
      const [a, b, q] = d.pts, pa = px(c, a), pb = px(c, b);
      if (!pa || !pb) return null;
      const y = q && c.y(q[1]);
      if (y == null) return segment(c, pa, pb);
      const fa: XY = [pa[0], y], fb: XY = [pb[0], y];
      return <>
        <path d={poly([pa, pb, fb, fa])} fill={BLUE} opacity={0.2} stroke="none" />
        {ln(pa, pb, c.color, c)}{ln(fa, fb, c.color, c)}
        {c.hitPath(poly([pa, pb, fb, fa]), true)}{c.hit(pa[0], pa[1], pb[0], pb[1])}{c.hit(fa[0], y, fb[0], y)}
        {c.handles([pa, pb, fb])}
      </>;
    },
    icon: <><path d="M3 6h18M3 19l18-9"/></> },
  { id: "disjoint", name: "Disjoint channel", section: "CHANNELS", points: 3,
    render: (d, c) => {
      const [a, b, q] = d.pts, pa = px(c, a), pb = px(c, b);
      if (!pa || !pb) return null;
      if (!q) return segment(c, pa, pb);
      // the second line starts at the third point and mirrors the first: same bars, opposite price change
      const end = c.xAt(c.index(q[0]) + c.index(b[0]) - c.index(a[0])), qa = px(c, q), qy = c.y(q[1] - (b[1] - a[1]));
      if (!qa || end === null || qy === null) return null;
      const qb: XY = [end, qy];
      return <>
        <path d={poly([pa, pb, qb, qa])} fill={BLUE} opacity={0.2} stroke="none" />
        {ln(pa, pb, c.color, c)}{ln(qa, qb, c.color, c)}
        {c.hitPath(poly([pa, pb, qb, qa]), true)}{c.hit(pa[0], pa[1], pb[0], pb[1])}{c.hit(qa[0], qa[1], qb[0], qb[1])}
        {c.handles()}
      </>;
    },
    icon: <><path d="m3 9 18-5M3 15l18 5"/></> },

  { id: "pitchfork", name: "Pitchfork", section: "PITCHFORKS", points: 3, render: pitchfork("andrews"), icon: forkIcon("M3 12") },
  { id: "schiff", name: "Schiff pitchfork", section: "PITCHFORKS", points: 3, render: pitchfork("schiff"), icon: forkIcon("M3 18L5 13") },
  { id: "mschiff", name: "Modified Schiff pitchfork", section: "PITCHFORKS", points: 3, render: pitchfork("half"), icon: forkIcon("M3 18L7 13") },
  // shortcut: TradingView's help gives Inside the same origin as Modified Schiff (halfway between the first two points); compare in the browser
  { id: "ipitchfork", name: "Inside pitchfork", section: "PITCHFORKS", points: 3, render: pitchfork("half"), icon: forkIcon("M8 13") },
];
