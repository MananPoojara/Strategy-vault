import type { ReactNode } from "react";
import type { Bars, Drawing, Pt, Shape, RenderCtx, ToolDef } from "./model";

const BLUE = "#2962FF", GREEN = "#089981", RED = "#F23645", GREY = "#787B86", YELLOW = "#FBC02D";
const pctOf = (a: number, b: number) => (a ? Math.abs((b - a) / a * 100) : 0).toFixed(2);
const arrow = (x1: number, y1: number, x2: number, y2: number, tint: string) => { const ang = Math.atan2(y2 - y1, x2 - x1), s = 7;
  return <g><line x1={x1} y1={y1} x2={x2} y2={y2} stroke={tint} strokeWidth={1.5} />
    <path d={`M${x2} ${y2}L${x2 - s * Math.cos(ang - 0.5)} ${y2 - s * Math.sin(ang - 0.5)}M${x2} ${y2}L${x2 - s * Math.cos(ang + 0.5)} ${y2 - s * Math.sin(ang + 0.5)}`} stroke={tint} strokeWidth={1.5} fill="none" /></g>; };
const xyOf = (c: RenderCtx, p: Pt) => { const x = c.x(p[0]), y = c.y(p[1]); return x === null || y === null ? null : [x, y] as const; };

/** Price range, Date range, Date and price range: a tinted box, arrows across it and TradingView's read-out. */
const measure = (d: Shape, c: RenderCtx) => {
  const [a, b] = d.pts, [xa, ya, xb, yb] = [c.x(a[0]), c.y(a[1]), c.x(b[0]), c.y(b[1])];
  if (xa === null || ya === null || xb === null || yb === null) return null;
  const [l, r, top, bottom] = [Math.min(xa, xb), Math.max(xa, xb), Math.min(ya, yb), Math.max(ya, yb)];
  const down = d.kind !== "dr" && b[1] < a[1], tint = down ? RED : BLUE;
  const bars = c.index(b[0]) - c.index(a[0]), mx = (xa + xb) / 2, my = (ya + yb) / 2;
  const text = d.kind === "pr" ? [c.priceChange(a[1], b[1])] : d.kind === "dr" ? [c.dateSpan(bars, a[0], b[0])] : [c.priceChange(a[1], b[1]), c.dateSpan(bars, a[0], b[0])];
  // the read-out sits below a fall and above a rise, as on TradingView
  const below = d.kind === "dr" || b[1] <= a[1];
  return <>
    <rect x={l} y={top} width={r - l} height={bottom - top} fill={tint} opacity={0.15} />
    {d.kind !== "dr" && arrow(mx, ya, mx, yb, tint)}
    {d.kind !== "pr" && arrow(xa, my, xb, my, tint)}
    <g data-lab-measure>{c.label(text, mx, below ? bottom + 8 : top - 8, tint, below)}</g>
    {c.hitArea(l, top, Math.max(1, r - l), Math.max(1, bottom - top))}
    {c.handles()}
  </>;
};
const box = (kind: "pr" | "dr" | "dpr") => (pts: Pt[]) => ({ kind, a: pts[0], b: pts[1] ?? pts[0] });

/** Long / Short position: pts = entry, target (price only), stop (price only), right edge (date only). */
const POS_BARS = 20, POS_TARGET = 0.02, POS_STOP = 0.01;
const position = (kind: "long" | "short", dir: 1 | -1) => ({
  make: (pts: Pt[]): Drawing => { const [t, p] = pts[0]; return { kind, pts: [[t, p], [t, p * (1 + dir * POS_TARGET)], [t, p * (1 - dir * POS_STOP)], [t, p]] }; },
  render: (d: Shape, c: RenderCtx) => {
    const [[t, entry], [, tp], [, sl], [tw]] = d.pts, i0 = c.index(t), span = c.index(tw) - i0;
    // shortcut: the right edge is a bar date, so it cannot be dragged past the last bar; an unset or reversed edge draws the default 20 bars
    const xl = c.x(t), xr = c.xAt(i0 + (span > 0 ? span : POS_BARS)), ye = c.y(entry), yt = c.y(tp), ys = c.y(sl);
    if (xl === null || xr === null || ye === null || yt === null || ys === null) return null;
    const risk = Math.abs(entry - sl), rr = risk ? Number((Math.abs(tp - entry) / risk).toFixed(2)) : 0, mx = (xl + xr) / 2;
    const zone = (y: number, fill: string) => <rect x={xl} y={Math.min(y, ye)} width={xr - xl} height={Math.abs(y - ye)} fill={fill} opacity={0.2} />;
    const read = (name: string, p: number, y: number, fill: string) =>
      <g data-lab-position={name}>{c.label([`${name}: ${c.fmt(p)} (${pctOf(entry, p)}%) ${c.fmt(Math.abs(p - entry))}`], mx, y < ye ? y - 4 : y + 4, fill, y >= ye)}</g>;
    const top = Math.min(yt, ys), bottom = Math.max(yt, ys);
    return <>
      {zone(yt, GREEN)}{zone(ys, RED)}
      <line x1={xl} x2={xr} y1={ye} y2={ye} stroke={GREY} strokeWidth={1} />
      {read("Target", tp, yt, GREEN)}{read("Stop", sl, ys, RED)}
      <g data-lab-position="ratio">{c.label([`Risk/Reward Ratio: ${rr}`], mx, ye - 10.5, GREY)}</g>
      {c.hitArea(xl, top, Math.max(1, xr - xl), Math.max(1, bottom - top))}
      {c.handles([[xl, ye], [xl, yt], [xl, ys], [xr, ye]])}
    </>;
  },
});

/** Position forecast: from the first bar's close to the forecast price and date. */
const forecast = (d: Shape, c: RenderCtx) => {
  const [a, b] = d.pts, close = c.bars.c[c.index(a[0])], from = Number.isFinite(close) ? close : a[1];
  const xa = c.x(a[0]), ya = c.y(from), pb = xyOf(c, b);
  if (xa === null || ya === null || !pb) return null;
  const [xb, yb] = pb, up = b[1] >= from;
  return <>
    {arrow(xa, ya, xb, yb, BLUE)}
    <circle cx={xa} cy={ya} r={3} fill={BLUE} />
    <g data-lab-forecast>{c.label([c.priceChange(from, b[1]), c.dateSpan(c.index(b[0]) - c.index(a[0]), a[0], b[0])], xb, up ? yb - 8 : yb + 8, BLUE, !up)}</g>
    {c.hit(xa, ya, xb, yb)}
    {c.handles([[xa, ya], [xb, yb]])}
  </>;
};

/** Bars pattern: props keep the copied bars (from, to) and the first point's price; pts[0] is where the copy now sits. */
const barsPattern = (d: Shape, c: RenderCtx) => {
  const p = d.props ?? {}, [i1, i2] = [c.index(String(p.from)), c.index(String(p.to))], b = c.bars;
  const at = c.index(d.pts[0][0]), shift = at - i1, dp = d.pts[0][1] - Number(p.base);
  if (i1 < 0 || i2 < 0 || at < 0 || !b.h.length) return null;
  const bw = Math.max(1, ((c.xAt(1) ?? 0) - (c.xAt(0) ?? 0)) * 0.6);
  let top = Infinity, bottom = -Infinity, left = Infinity, right = -Infinity;
  const candles: ReactNode[] = [];
  for (let j = Math.min(i1, i2); j <= Math.max(i1, i2); j++) {
    const x = c.xAt(j + shift), [yo, yh, yl, yc] = [b.o[j], b.h[j], b.l[j], b.c[j]].map((v) => c.y(v + dp));
    if (x === null || yo === null || yh === null || yl === null || yc === null) continue;
    [top, bottom, left, right] = [Math.min(top, yh, yl), Math.max(bottom, yh, yl), Math.min(left, x - bw / 2), Math.max(right, x + bw / 2)];
    candles.push(<g key={j} data-lab-ghost>
      <line x1={x} x2={x} y1={yh} y2={yl} stroke={BLUE} />
      <rect x={x - bw / 2} y={Math.min(yo, yc)} width={bw} height={Math.max(1, Math.abs(yc - yo))} fill={BLUE} />
    </g>);
  }
  const anchor = xyOf(c, d.pts[0]);
  if (!candles.length) return null;
  return <>
    <g opacity={0.5}>{candles}</g>
    {c.hitArea(left, top, right - left, bottom - top)}
    {c.handles([anchor])}
  </>;
};

/** Ghost feed: candles stepping along the clicked path, one per bar, wicks a quarter of the chart's average range. */
const ghostFeed = (d: Shape, c: RenderCtx) => {
  const b = c.bars, n = Math.min(50, b.h.length), ranges = b.h.slice(-n).map((h, k) => h - b.l[b.h.length - n + k]).filter(Number.isFinite);
  const pad = ranges.length ? ranges.reduce((s, r) => s + r, 0) / ranges.length / 4 : 0;
  const bw = Math.max(1, ((c.xAt(1) ?? 0) - (c.xAt(0) ?? 0)) * 0.6);
  const candles: ReactNode[] = [], path: string[] = [];
  d.pts.forEach((p) => { const xy = xyOf(c, p); if (xy) path.push(`${path.length ? "L" : "M"}${xy[0]} ${xy[1]}`); });
  for (let k = 1; k < d.pts.length; k++) {
    const [[t0, p0], [t1, p1]] = [d.pts[k - 1], d.pts[k]], i0 = c.index(t0), steps = c.index(t1) - i0;
    for (let j = 1; j <= steps; j++) {
      const o = p0 + (p1 - p0) * (j - 1) / steps, cl = p0 + (p1 - p0) * j / steps, x = c.xAt(i0 + j);
      const [yo, yc, yh, yl] = [o, cl, Math.max(o, cl) + pad, Math.min(o, cl) - pad].map(c.y);
      if (x === null || yo === null || yc === null || yh === null || yl === null) continue;
      const tint = cl >= o ? GREEN : RED;
      candles.push(<g key={`${k}-${j}`} data-lab-ghost>
        <line x1={x} x2={x} y1={yh} y2={yl} stroke={tint} />
        <rect x={x - bw / 2} y={Math.min(yo, yc)} width={bw} height={Math.max(1, Math.abs(yc - yo))} fill={tint} />
      </g>);
    }
  }
  if (!path.length) return null;
  return <>
    {c.live && <path d={path.join("")} stroke={GREY} strokeDasharray="4 4" fill="none" />}
    <g opacity={0.5}>{candles}</g>
    {c.hitPath(path.join(""))}
    {c.handles()}
  </>;
};

/** Sector: a wedge centred on the first point from the second point's radius to the third point's angle. */
const sector = (d: Shape, c: RenderCtx) => {
  const [pa, pb, pc] = d.pts.map((p) => xyOf(c, p));
  if (!pa || !pb) return null;
  if (!pc) return <><line x1={pa[0]} y1={pa[1]} x2={pb[0]} y2={pb[1]} stroke={BLUE} strokeWidth={1} />{c.handles()}</>;
  const r = Math.hypot(pb[0] - pa[0], pb[1] - pa[1]), t1 = Math.atan2(pb[1] - pa[1], pb[0] - pa[0]), t2 = Math.atan2(pc[1] - pa[1], pc[0] - pa[0]);
  const turn = ((t2 - t1 + 3 * Math.PI) % (2 * Math.PI)) - Math.PI; // the shorter way round
  const [ex, ey] = [pa[0] + r * Math.cos(t2), pa[1] + r * Math.sin(t2)];
  const path = `M${pa[0]} ${pa[1]}L${pb[0]} ${pb[1]}A${r} ${r} 0 0 ${turn > 0 ? 1 : 0} ${ex} ${ey}Z`;
  const [a, , cc] = d.pts, up = cc[1] >= a[1];
  return <>
    <path d={path} fill={BLUE} fillOpacity={0.2} stroke={BLUE} strokeWidth={1} />
    <g data-lab-sector>{c.label([c.priceChange(a[1], cc[1]), c.dateSpan(c.index(cc[0]) - c.index(a[0]), a[0], cc[0])], pc[0], up ? pc[1] - 8 : pc[1] + 8, BLUE, !up)}</g>
    {c.hitPath(path, true)}
    {c.handles()}
  </>;
};

/** Anchored VWAP. A display-only calculation added for this drawing (not an engine indicator): from the anchor bar to the last bar,
 *  the volume-weighted typical price (h + l + c) / 3; bars without volume add nothing. */
export const anchoredVwap = (b: Bars, from: number) => {
  const out: (number | null)[] = [];
  let pv = 0, vol = 0;
  for (let i = from; i < b.d.length; i++) {
    const v = b.v[i] ?? 0, tp = (b.h[i] + b.l[i] + b.c[i]) / 3;
    if (v > 0 && Number.isFinite(tp)) { pv += tp * v; vol += v; }
    out.push(vol ? pv / vol : null);
  }
  return out;
};
const vwap = (d: Shape, c: RenderCtx) => {
  const i0 = c.index(d.pts[0][0]);
  if (i0 < 0) return null;
  const line = anchoredVwap(c.bars, i0), path: string[] = [];
  let e: [number, number, number] | null = null; // the last point: x, y, VWAP
  for (let k = 0; k < line.length; k++) {
    const v = line[k], x = c.xAt(i0 + k), y = v === null ? null : c.y(v);
    if (x !== null && y !== null && v !== null) { path.push(`${path.length ? "L" : "M"}${x} ${y}`); e = [x, y, v]; }
  }
  const x0 = c.x(d.pts[0][0]) ?? 0, y0 = c.y(line.find((v) => v !== null) ?? d.pts[0][1]) ?? 0;
  if (!e) return <><g data-lab-vwap-empty>{c.label(["No volume data"], x0, y0 + 8, GREY)}</g>{c.hitArea(x0 - 5, y0 - 5, 10, 10)}{c.handles([[x0, y0]])}</>;
  return <>
    <path data-lab-vwap d={path.join("")} stroke={BLUE} strokeWidth={c.live ? 2 : 1.5} fill="none" />
    <rect x={c.plot} y={e[1] - 9} width={Math.max(c.axis, 60)} height={18} fill={BLUE} />
    <text x={c.plot + 4} y={e[1] + 4} fill="#fff" fontSize={11}>{c.fmt(e[2])}</text>
    {c.hitPath(path.join(""))}
    {c.handles([[x0, y0]])}
  </>;
};

/** Volume profile of bars i0..i1: each bar's volume spread over the rows its high-low range covers, split up / down by its candle. */
const ROWS = 24, VALUE_AREA = 0.7, VP_WIDTH = 0.3;
export const volumeProfile = (b: Bars, i0: number, i1: number, rows = ROWS) => {
  let lo = Infinity, hi = -Infinity;
  for (let i = i0; i <= i1; i++) if (Number.isFinite(b.l[i]) && Number.isFinite(b.h[i])) { lo = Math.min(lo, b.l[i]); hi = Math.max(hi, b.h[i]); }
  if (!(hi >= lo)) return null;
  const step = (hi - lo) / rows || 1, up: number[] = Array(rows).fill(0), down: number[] = Array(rows).fill(0);
  const row = (p: number) => Math.min(rows - 1, Math.floor((p - lo) / step));
  for (let i = i0; i <= i1; i++) {
    const v = b.v[i] ?? 0, [l, h] = [b.l[i], b.h[i]], side = b.c[i] >= b.o[i] ? up : down;
    if (!(v > 0) || !Number.isFinite(l) || !Number.isFinite(h)) continue;
    if (h === l) { side[row(l)] += v; continue; }
    for (let r = row(l); r <= row(h); r++) side[r] += v * Math.max(0, Math.min(h, lo + (r + 1) * step) - Math.max(l, lo + r * step)) / (h - l);
  }
  const total = up.map((u, r) => u + down[r]), sum = total.reduce((s, t) => s + t, 0);
  const poc = total.indexOf(Math.max(...total));
  // TradingView's value area: from the POC, add the busier neighbouring row until it holds 70% of the volume
  let [va0, va1, acc] = [poc, poc, total[poc]];
  while (acc < VALUE_AREA * sum && (va0 > 0 || va1 < rows - 1)) {
    const below = va0 > 0 ? total[va0 - 1] : -1, above = va1 < rows - 1 ? total[va1 + 1] : -1;
    acc += above >= below ? total[++va1] : total[--va0];
  }
  return { lo, hi, step, up, down, total, sum, poc, va: [va0, va1] as const };
};
const profile = (d: Shape, c: RenderCtx, anchored: boolean) => {
  const last = c.bars.d.length - 1, ia = c.index(d.pts[0][0]), ib = anchored ? last : c.index((d.pts[1] ?? d.pts[0])[0]);
  if (ia < 0 || ib < 0) return null;
  const [i0, i1] = [Math.min(ia, ib), Math.max(ia, ib)], vp = volumeProfile(c.bars, i0, i1);
  const xl = c.xAt(i0), xr = c.xAt(i1), yt = vp && c.y(vp.hi), yb = vp && c.y(vp.lo);
  if (!vp || xl === null || xr === null || yt == null || yb == null) return null;
  const ym = (yt + yb) / 2, [top, h] = [Math.min(yt, yb), Math.max(1, Math.abs(yb - yt))], max = vp.total[vp.poc];
  const ends = anchored ? [[c.x(d.pts[0][0]) ?? xl, ym] as const] : d.pts.slice(0, 2).map((p) => [c.x(p[0]) ?? xl, ym] as const);
  const w = Math.max(1, xr - xl) * VP_WIDTH;
  return <>
    {c.live && <rect x={xl} y={top} width={Math.max(1, xr - xl)} height={h} fill={BLUE} opacity={0.08} />}
    {vp.sum ? vp.total.map((t, r) => {
      const [y1, y2] = [c.y(vp.lo + (r + 1) * vp.step), c.y(vp.lo + r * vp.step)];
      if (!t || y1 === null || y2 === null) return null;
      const inVa = r >= vp.va[0] && r <= vp.va[1], op = inVa ? 0.7 : 0.3, wu = vp.up[r] / max * w, wd = vp.down[r] / max * w;
      const ry = Math.min(y1, y2), rh = Math.max(0.5, Math.abs(y2 - y1) - 1);
      return <g key={r} data-lab-vp-row={r} data-va={inVa || undefined}>
        {wu > 0 && <rect x={xl} y={ry} width={wu} height={rh} fill={BLUE} opacity={op} />}
        {wd > 0 && <rect x={xl + wu} y={ry} width={wd} height={rh} fill={YELLOW} opacity={op} />}
      </g>;
    }) : <g data-lab-vp-empty>{c.label(["No volume data"], (xl + xr) / 2, ym, GREY)}</g>}
    {vp.sum > 0 && (() => { const y = c.y(vp.lo + (vp.poc + 0.5) * vp.step);
      return y === null ? null : <line data-lab-poc x1={xl} x2={xr} y1={y} y2={y} stroke={RED} strokeWidth={1} />; })()}
    {c.hitArea(xl, top, Math.max(1, xr - xl), h)}
    {c.handles(ends)}
  </>;
};

export const FORECAST: ToolDef[] = [
  { id: "long", name: "Long position", section: "FORECASTING", points: 1, ...position("long", 1),
    icon: <><rect x="4" y="3" width="16" height="9" /><rect x="4" y="12" width="16" height="9" strokeDasharray="2 2" /><path d="m9 9 3-3 3 3" /></> },
  { id: "short", name: "Short position", section: "FORECASTING", points: 1, ...position("short", -1),
    icon: <><rect x="4" y="3" width="16" height="9" strokeDasharray="2 2" /><rect x="4" y="12" width="16" height="9" /><path d="m9 15 3 3 3-3" /></> },
  { id: "forecast", name: "Position forecast", section: "FORECASTING", points: 2, render: forecast,
    icon: <><path d="M5 18 18 6" strokeDasharray="3 2" /><circle cx="4" cy="19" r="2" /><path d="M13 6h5v5" /></> },
  { id: "barspat", name: "Bars pattern", section: "FORECASTING", points: 2, render: barsPattern,
    make: (pts) => ({ kind: "barspat", pts, props: { from: pts[0][0], to: (pts[1] ?? pts[0])[0], base: pts[0][1] } }),
    icon: <><path d="M6 4v14M10 7v10M14 3v12M18 8v12" /><path d="M3 21h18" strokeDasharray="2 2" /></> },
  { id: "ghost", name: "Ghost feed", section: "FORECASTING", points: "multi", render: ghostFeed,
    icon: <><path d="M6 8v9M10 5v9M14 10v9M18 6v9" /><path d="M4 20 20 4" strokeDasharray="2 2" /></> },
  { id: "sector", name: "Sector", section: "FORECASTING", points: 3, render: sector,
    icon: <><path d="M4 20h16A16 16 0 0 0 15 8z" /><circle cx="4" cy="20" r="1.5" /></> },
  { id: "avwap", name: "Anchored VWAP", section: "VOLUME-BASED", points: 1, render: vwap,
    icon: <><path d="M4 18c4-1 5-8 9-8s5 4 8 2" /><circle cx="4" cy="18" r="2" /></> },
  { id: "frvp", name: "Fixed range volume profile", section: "VOLUME-BASED", points: 2, render: (d, c) => profile(d, c, false),
    icon: <><path d="M4 3v18M20 3v18M4 7h9M4 11h13M4 15h7M4 19h4" /></> },
  { id: "avp", name: "Anchored volume profile", section: "VOLUME-BASED", points: 1, render: (d, c) => profile(d, c, true),
    icon: <><path d="M4 3v18M4 7h8M4 11h13M4 15h6M4 19h4" /><circle cx="4" cy="12" r="2" /></> },
  { id: "pr", name: "Price range", section: "MEASURERS", points: 2, make: box("pr"), render: measure,
    icon: <><path d="M12 3v18M8 7l4-4 4 4M8 17l4 4 4-4M5 3h14M5 21h14"/></> },
  { id: "dr", name: "Date range", section: "MEASURERS", points: 2, make: box("dr"), render: measure,
    icon: <><path d="M3 12h18M7 8l-4 4 4 4M17 8l4 4-4 4M3 5v14M21 5v14"/></> },
  { id: "dpr", name: "Date and price range", section: "MEASURERS", points: 2, make: box("dpr"), render: measure,
    icon: <><rect x="3" y="3" width="18" height="18" rx="1"/><path d="M12 6v12M6 12h12"/></> },
];
