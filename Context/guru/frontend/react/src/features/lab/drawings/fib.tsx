import type { RenderCtx, ToolDef } from "./model";
import { path, px } from "./model";

/** TradingView's Fib retracement levels and colours; level 0 is the second point, 1 the first. */
export const FIB_LEVELS: [number, string][] = [[0, "#787B86"], [0.236, "#F23645"], [0.382, "#FF9800"], [0.5, "#4CAF50"], [0.618, "#089981"], [0.786, "#00BCD4"],
  [1, "#787B86"], [1.618, "#2962FF"], [2.618, "#F23645"], [3.618, "#9C27B0"], [4.236, "#E91E63"]];
/** Gann box, Gann square and Fib speed resistance fan levels, in price and in time. */
const GANN_LEVELS: [number, string][] = [[0, "#787B86"], [0.25, "#FF9800"], [0.382, "#4CAF50"], [0.5, "#089981"], [0.618, "#00BCD4"], [0.75, "#2962FF"], [1, "#787B86"]];
/** Gann fan lines, price units x time units: 1x1 runs through the second point. */
const GANN_FAN: [number, number, string][] = [[1, 8, "#FF9800"], [1, 4, "#4CAF50"], [1, 3, "#089981"], [1, 2, "#00BCD4"], [1, 1, "#2962FF"],
  [2, 1, "#9C27B0"], [3, 1, "#E91E63"], [4, 1, "#F23645"], [8, 1, "#787B86"]];
const FIB_NUMBERS = [0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
const FIB_TIME = [0, 0.382, 0.5, 0.618, 1, 1.382, 1.618, 2, 2.382, 2.618, 3, 3.618, 4.236];
const RINGS = FIB_LEVELS.slice(1), WEDGE = FIB_LEVELS.slice(1, 7); // circles and arcs have no 0 ring; the wedge stops at 1
const PITCHFAN: [number, string][] = [[0.5, "#089981"], [1, "#2962FF"]];
const GREY = "#787B86", TINT = 0.12, PHI = (1 + Math.sqrt(5)) / 2;

type XY = readonly [number, number];
const colorOf = (set: [number, string][], lv: number) => set.find(([v]) => v === lv)?.[1] ?? GREY;
/** A point on the ray from o through t, past the layer's edge (the SVG clips it). */
const far = (c: RenderCtx, o: XY, t: XY): XY => {
  const len = Math.hypot(t[0] - o[0], t[1] - o[1]), k = len ? Math.max(1, 2 * (c.plot + c.height) / len) : 1;
  return [o[0] + (t[0] - o[0]) * k, o[1] + (t[1] - o[1]) * k];
};
const dashed = (q: readonly XY[]) => <path d={path(q)} stroke={GREY} strokeWidth={1} strokeDasharray="4 4" fill="none" />;
const seg = (a: XY, b: XY, col: string, w = 1) => <line x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={col} strokeWidth={w} />;
const tag = (s: string, x: number, y: number, col: string, anchor: "start" | "middle" | "end" = "start") => <text x={x} y={y} fill={col} fontSize={11} textAnchor={anchor}>{s}</text>;
/** Until the last point is placed: the dashed trend so far and its points. */
const partial = (q: XY[], c: RenderCtx) => <>{dashed(q)}{q.length > 1 && c.hit(q[0][0], q[0][1], q[1][0], q[1][1])}{c.handles()}</>;
const circle = ([x, y]: XY, r: number) => `M${x - r} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;

/** Horizontal Fib levels from l to r, tinted between levels and labelled at the left "0.618 (25,281.85)". */
const levels = (c: RenderCtx, l: number, r: number, at: (lv: number) => number) => {
  const ys = FIB_LEVELS.map(([lv, col]) => [lv, col, c.y(at(lv))] as const);
  return <>
    {ys.slice(1).map(([lv, col, y], k) => { const y0 = ys[k][2]; return y === null || y0 === null ? null
      : <rect key={`f${lv}`} x={l} y={Math.min(y, y0)} width={r - l} height={Math.abs(y - y0)} fill={col} opacity={TINT} />; })}
    {ys.map(([lv, col, y]) => y === null ? null : <g key={lv} data-lab-fib={lv}>
      <line x1={l} x2={r} y1={y} y2={y} stroke={col} strokeWidth={1} />
      <text x={l - 4} y={y + 4} fill={col} fontSize={11} textAnchor="end">{`${lv} (${c.fmt(at(lv))})`}</text></g>)}
  </>;
};
/** Rays from o through each point, tinted between neighbours (each wedge in the outer ray's colour). */
const fan = (c: RenderCtx, o: XY, rays: [string, XY, string][], tint = true) => {
  const ends = rays.map(([, t]) => far(c, o, t));
  return <>
    {tint && rays.slice(1).map(([key, , col], k) => <path key={`f${key}`} d={path([o, ends[k], ends[k + 1]], true)} fill={col} opacity={TINT} stroke="none" />)}
    {rays.map(([key, , col], k) => <g key={key} data-lab-fib={key}>{seg(o, ends[k], col)}{c.hit(o[0], o[1], ends[k][0], ends[k][1])}</g>)}
  </>;
};
/** Vertical lines over the layer's height at bar positions, labelled at the bottom. */
const verticals = (c: RenderCtx, at: [string, number | null, string][]) => at.map(([key, x, col]) =>
  x === null || (c.plot > 0 && (x < 0 || x > c.plot)) ? null
  : <g key={key} data-lab-fib={key}>{seg([x, 0], [x, c.height], col)}{tag(key, x + 4, c.height - 6, col)}{c.hit(x, 0, x, c.height)}</g>);
/** Gann box and Gann square: price and time levels across the box from corner a to corner b, labelled left and top. */
const grid = (c: RenderCtx, [x1, y1]: XY, [x2, y2]: XY, fill: boolean) => {
  const l = Math.min(x1, x2), r = Math.max(x1, x2), top = Math.min(y1, y2), bottom = Math.max(y1, y2);
  const ys = GANN_LEVELS.map(([lv, col]) => [lv, col, y1 + (y2 - y1) * lv] as const), xs = GANN_LEVELS.map(([lv, col]) => [lv, col, x1 + (x2 - x1) * lv] as const);
  return <>
    {fill && ys.slice(1).map(([lv, col, y], k) => <rect key={`py${lv}`} x={l} y={Math.min(y, ys[k][2])} width={r - l} height={Math.abs(y - ys[k][2])} fill={col} opacity={0.06} />)}
    {fill && xs.slice(1).map(([lv, col, x], k) => <rect key={`px${lv}`} x={Math.min(x, xs[k][2])} y={top} width={Math.abs(x - xs[k][2])} height={bottom - top} fill={col} opacity={0.06} />)}
    {ys.map(([lv, col, y]) => <g key={`y${lv}`} data-lab-fib={`p${lv}`}>{seg([l, y], [r, y], col)}{tag(String(lv), l - 4, y + 4, col, "end")}{c.hit(l, y, r, y)}</g>)}
    {xs.map(([lv, col, x]) => <g key={`x${lv}`} data-lab-fib={`t${lv}`}>{seg([x, top], [x, bottom], col)}{tag(String(lv), x, top - 4, col, "middle")}{c.hit(x, top, x, bottom)}</g>)}
  </>;
};
/** Gann square (and fixed): the grid, Gann fan lines from the first corner to the box's edge and arcs at half and full size. */
const square = (c: RenderCtx, a: XY, b: XY) => {
  const [w, h] = [b[0] - a[0], b[1] - a[1]], sweep = (w > 0) === (h > 0) ? 1 : 0;
  return <>
    {grid(c, a, b, false)}
    {GANN_FAN.map(([p, t, col]) => { const k = p / t, e: XY = k <= 1 ? [a[0] + w, a[1] + h * k] : [a[0] + w / k, a[1] + h];
      return <g key={`${p}x${t}`} data-lab-fib={`${p}x${t}`}>{seg(a, e, col)}</g>; })}
    {[0.5, 1].map((lv) => <path key={`a${lv}`} data-lab-fib={`a${lv}`} d={`M${a[0] + w * lv} ${a[1]}A${Math.abs(w * lv)} ${Math.abs(h * lv)} 0 0 ${sweep} ${a[0]} ${a[1] + h * lv}`}
      stroke={colorOf(GANN_LEVELS, lv === 1 ? 0.75 : 0.5)} strokeWidth={1} fill="none" />)}
  </>;
};

export const FIB: ToolDef[] = [
  { id: "fib", name: "Fib retracement", section: "FIBONACCI", points: 2, shortcut: "Alt+F",
    make: (pts) => ({ kind: "fib", a: pts[0], b: pts[1] ?? pts[0] }),
    render: (d, c) => {
      const [a, b] = d.pts, [xa, ya, xb, yb] = [c.x(a[0]), c.y(a[1]), c.x(b[0]), c.y(b[1])];
      if (xa === null || ya === null || xb === null || yb === null) return null;
      const [l, r] = [Math.min(xa, xb), Math.max(xa, xb)];
      return <>
        {levels(c, l, r, (lv) => b[1] + (a[1] - b[1]) * lv)}
        <line x1={xa} y1={ya} x2={xb} y2={yb} stroke="#787B86" strokeWidth={1} strokeDasharray="4 4" />
        {c.hit(xa, ya, xb, yb)}{c.hit(l, ya, r, ya)}{c.hit(l, yb, r, yb)}{c.handles()}
      </>;
    },
    icon: <><path d="M3 5h18M3 10h18M3 14h18M3 19h18"/><path d="m4 19 16-14" strokeDasharray="2 2"/></> },
  // levels project the 1→2 move from point 3, drawn from point 2 to point 3
  { id: "fibext", name: "Trend-based fib extension", section: "FIBONACCI", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 3) return partial(q, c);
      const [p1, p2, p3] = d.pts, l = Math.min(q[1][0], q[2][0]), r = Math.max(q[1][0], q[2][0]);
      const [L, R] = r - l >= 1 ? [l, r] : [Math.min(q[0][0], l), Math.max(q[0][0], r)];
      const at = (lv: number) => p3[1] + (p2[1] - p1[1]) * lv, y0 = c.y(at(0)), y1 = c.y(at(1));
      return <>{levels(c, L, R, at)}{dashed(q)}{c.hit(...q[0], ...q[1])}{c.hit(...q[1], ...q[2])}
        {y0 !== null && c.hit(L, y0, R, y0)}{y1 !== null && c.hit(L, y1, R, y1)}{c.handles()}</>;
    },
    icon: <><path d="M3 4h18M3 9h18M3 14h18"/><path d="m4 20 5-9 4 6" strokeDasharray="2 2"/></> },
  // points 1 and 2 set level 0, point 3 sets level 1; the levels are parallel copies of the 1→2 line
  { id: "fibchannel", name: "Fib channel", section: "FIBONACCI", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 3) return partial(q, c);
      const [p1, p2, p3] = q;
      const off: XY = p2[0] !== p1[0] ? [0, p3[1] - (p1[1] + (p2[1] - p1[1]) * (p3[0] - p1[0]) / (p2[0] - p1[0]))] : [p3[0] - p1[0], 0];
      const at = (lv: number, p: XY): XY => [p[0] + off[0] * lv, p[1] + off[1] * lv];
      const left = p1[0] <= p2[0] ? p1 : p2;
      return <>
        {FIB_LEVELS.slice(1).map(([lv, col], k) => { const lo = FIB_LEVELS[k][0];
          return <path key={`f${lv}`} d={path([at(lo, p1), at(lo, p2), at(lv, p2), at(lv, p1)], true)} fill={col} opacity={TINT} stroke="none" />; })}
        {FIB_LEVELS.map(([lv, col]) => { const [a, b, t] = [at(lv, p1), at(lv, p2), at(lv, left)];
          return <g key={lv} data-lab-fib={lv}>{seg(a, b, col)}{tag(String(lv), t[0] - 4, t[1] + 4, col, "end")}{(lv === 0 || lv === 1) && c.hit(a[0], a[1], b[0], b[1])}</g>; })}
        {c.handles()}
      </>;
    },
    icon: <><path d="m3 13 18-8M3 17l18-8M3 21l18-8"/></> },
  // vertical lines at Fibonacci multiples of the bars from point 1 to point 2
  { id: "fibtimezone", name: "Fib time zone", section: "FIBONACCI", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      const i = c.index(d.pts[0][0]), u = d.pts[1] ? c.index(d.pts[1][0]) - i : 0;
      return <>{verticals(c, (u ? FIB_NUMBERS : [0]).map((n, k) => [String(n), c.xAt(i + n * u), FIB_LEVELS[k % FIB_LEVELS.length][1]]))}
        {dashed(q)}{q.length > 1 && c.hit(...q[0], ...q[1])}{c.handles()}</>;
    },
    icon: <><path d="M3 3v18M6 3v18M9 3v18M14 3v18M21 3v18"/></> },
  // rays from point 1 through the price levels on point 2's bar and the time levels on point 2's price
  { id: "fibfan", name: "Fib speed resistance fan", section: "FIBONACCI", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 2) return partial(q, c);
      const [o, b] = q, [w, h] = [b[0] - o[0], b[1] - o[1]];
      const price = GANN_LEVELS.map(([lv, col]): [string, XY, string] => [`p${lv}`, [b[0], o[1] + h * (1 - lv)], col]);
      const time = GANN_LEVELS.map(([lv, col]): [string, XY, string] => [`t${lv}`, [b[0] - w * lv, b[1]], col]);
      return <>
        {GANN_LEVELS.map(([lv]) => <g key={`g${lv}`} opacity={0.4}>
          {seg([o[0], o[1] + h * lv], [b[0], o[1] + h * lv], GREY)}{seg([o[0] + w * lv, o[1]], [o[0] + w * lv, b[1]], GREY)}</g>)}
        {fan(c, o, price.filter(([, t]) => t[0] !== o[0] || t[1] !== o[1]))}
        {fan(c, o, time.filter(([, t]) => t[0] !== o[0] || t[1] !== o[1]))}
        {price.map(([k, t, col]) => <g key={`l${k}`}>{tag(k.slice(1), t[0] + 4, t[1] + 4, col)}</g>)}
        {time.map(([k, t, col]) => <g key={`l${k}`}>{tag(k.slice(1), t[0], t[1] + (h < 0 ? -6 : 14), col, "middle")}</g>)}
        {c.handles()}
      </>;
    },
    icon: <><path d="M3 21 21 3M3 21l18-7M3 21l18-12M3 21l7-18M3 21l12-18"/></> },
  // vertical lines at time levels from point 3, measured by the bars from point 1 to point 2
  { id: "fibtrendtime", name: "Trend-based fib time", section: "FIBONACCI", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 3) return partial(q, c);
      const i3 = c.index(d.pts[2][0]), u = c.index(d.pts[1][0]) - c.index(d.pts[0][0]);
      return <>{verticals(c, (u ? FIB_TIME : [0]).map((lv) => [String(lv), c.xAt(i3 + lv * u), colorOf(FIB_LEVELS, lv)]))}
        {dashed(q)}{c.hit(...q[0], ...q[1])}{c.hit(...q[1], ...q[2])}{c.handles()}</>;
    },
    icon: <><path d="M10 3v18M14 3v18M21 3v18"/><path d="m3 18 3-8 3 5" strokeDasharray="2 2"/></> },
  // centred on point 1; level 1 runs through point 2
  { id: "fibcircles", name: "Fib circles", section: "FIBONACCI", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 2) return partial(q, c);
      const [o, b] = q, R = Math.hypot(b[0] - o[0], b[1] - o[1]);
      return <>
        {RINGS.slice(1).map(([lv, col], k) => <path key={`f${lv}`} d={circle(o, R * lv) + circle(o, R * RINGS[k][0])} fillRule="evenodd" fill={col} opacity={TINT} stroke="none" />)}
        {RINGS.map(([lv, col]) => <g key={lv} data-lab-fib={lv}><circle cx={o[0]} cy={o[1]} r={R * lv} stroke={col} strokeWidth={1} fill="none" />
          {tag(String(lv), o[0], o[1] - R * lv - 3, col, "middle")}</g>)}
        {dashed(q)}{c.hitPath(RINGS.map(([lv]) => circle(o, R * lv)).join(""))}{c.hit(...o, ...b)}{c.handles()}
      </>;
    },
    icon: <><circle cx="12" cy="12" r="3"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="10"/></> },
  // a golden spiral around point 1 through point 2: its radius grows by φ every quarter turn
  { id: "fibspiral", name: "Fib spiral", section: "FIBONACCI", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 2) return partial(q, c);
      const [o, b] = q, R = Math.hypot(b[0] - o[0], b[1] - o[1]), a0 = Math.atan2(b[1] - o[1], b[0] - o[0]);
      const spiral = path(Array.from({ length: 241 }, (_, k): XY => { const s = (k - 160) / 20, r = R * PHI ** s, a = a0 - s * Math.PI / 2;
        return [o[0] + r * Math.cos(a), o[1] + r * Math.sin(a)]; }));
      return <><path data-lab-fib="spiral" d={spiral} stroke={c.color} strokeWidth={c.live ? 2 : 1.5} fill="none" />
        {dashed(q)}{c.hitPath(spiral)}{c.hit(...o, ...b)}{c.handles()}</>;
    },
    icon: <><path d="M12 12a2 2 0 0 1 2 2 4 4 0 0 1-4 4 6 6 0 0 1-6-6 8 8 0 0 1 8-8 10 10 0 0 1 10 10"/></> },
  // half circles around point 1 on point 2's side; level 1 runs through point 2
  { id: "fibarcs", name: "Fib speed resistance arcs", section: "FIBONACCI", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 2) return partial(q, c);
      const [o, b] = q, R = Math.hypot(b[0] - o[0], b[1] - o[1]), up = b[1] <= o[1], s = up ? 1 : 0;
      const arc = (r: number) => `M${o[0] - r} ${o[1]}A${r} ${r} 0 0 ${s} ${o[0] + r} ${o[1]}`;
      return <>
        {RINGS.slice(1).map(([lv, col], k) => { const [r1, r2] = [R * RINGS[k][0], R * lv];
          return <path key={`f${lv}`} d={`${arc(r1)}L${o[0] + r2} ${o[1]}A${r2} ${r2} 0 0 ${1 - s} ${o[0] - r2} ${o[1]}Z`} fill={col} opacity={TINT} stroke="none" />; })}
        {RINGS.map(([lv, col]) => <g key={lv} data-lab-fib={lv}><path d={arc(R * lv)} stroke={col} strokeWidth={1} fill="none" />
          {tag(String(lv), o[0], o[1] + (up ? -R * lv - 3 : R * lv + 12), col, "middle")}</g>)}
        {dashed(q)}{c.hitPath(RINGS.map(([lv]) => arc(R * lv)).join(""))}{c.hit(...o, ...b)}{c.handles()}
      </>;
    },
    icon: <><path d="M4 20a8 8 0 0 1 16 0M8 20a4 4 0 0 1 8 0"/><path d="M12 20 18 8" strokeDasharray="2 2"/></> },
  // from point 1, between the rays to points 2 and 3; level 1 is point 2's distance
  { id: "fibwedge", name: "Fib wedge", section: "FIBONACCI", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 3) return partial(q, c);
      const [o, b, e] = q, R = Math.hypot(b[0] - o[0], b[1] - o[1]), a2 = Math.atan2(b[1] - o[1], b[0] - o[0]), a3 = Math.atan2(e[1] - o[1], e[0] - o[0]);
      const sw = ((a3 - a2 + 3 * Math.PI) % (2 * Math.PI)) - Math.PI > 0 ? 1 : 0; // the shorter way round
      const on = (a: number, r: number): XY => [o[0] + r * Math.cos(a), o[1] + r * Math.sin(a)];
      const arc = (r: number, back = false) => { const [p, t] = back ? [on(a3, r), on(a2, r)] : [on(a2, r), on(a3, r)];
        return `${p[0]} ${p[1]}A${r} ${r} 0 0 ${back ? 1 - sw : sw} ${t[0]} ${t[1]}`; };
      return <>
        {WEDGE.slice(1).map(([lv, col], k) => <path key={`f${lv}`} d={`M${arc(R * WEDGE[k][0])}L${arc(R * lv, true)}Z`} fill={col} opacity={TINT} stroke="none" />)}
        {WEDGE.map(([lv, col]) => { const t = on(a3, R * lv);
          return <g key={lv} data-lab-fib={lv}><path d={`M${arc(R * lv)}`} stroke={col} strokeWidth={1} fill="none" />{tag(String(lv), t[0] + 4, t[1] + 4, col)}</g>; })}
        {seg(o, on(a2, R), GREY)}{seg(o, on(a3, R), GREY)}
        {c.hit(...o, ...on(a2, R))}{c.hit(...o, ...on(a3, R))}{c.hitPath(`M${arc(R)}`)}{c.handles()}
      </>;
    },
    icon: <><path d="M3 21 20 4M3 21h18"/><path d="M14 21a11 11 0 0 0-3-8M19 21a16 16 0 0 0-4-11"/></> },
  // rays from point 1 through the median (the middle of points 2 and 3) and the levels either side of it
  { id: "pitchfan", name: "Pitchfan", section: "FIBONACCI", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 3) return partial(q, c);
      const [o, b, e] = q, m: XY = [(b[0] + e[0]) / 2, (b[1] + e[1]) / 2], to = (p: XY, lv: number): XY => [m[0] + (p[0] - m[0]) * lv, m[1] + (p[1] - m[1]) * lv];
      const side = (p: XY, s: string) => PITCHFAN.map(([lv, col]): [string, XY, string] => [`${s}${lv}`, to(p, lv), col]);
      const median: [string, XY, string] = ["median", m, "#F23645"];
      return <>{fan(c, o, [median, ...side(b, "b")])}{fan(c, o, [median, ...side(e, "e")])}{seg(b, e, "#F23645")}{c.hit(...b, ...e)}{c.handles()}</>;
    },
    icon: <><path d="M3 21 21 3M3 21l18-9M3 21l9-18"/><path d="M12 3l9 9"/></> },
  { id: "gannbox", name: "Gann box", section: "GANN", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      return q.length < 2 ? partial(q, c) : <>{grid(c, q[0], q[1], true)}{c.handles()}</>;
    },
    icon: <><rect x="3" y="3" width="18" height="18"/><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/></> },
  // shortcut: fixed at 36 bars by 36 × the bars' average high-low range (props.bars, props.ratio override); upgrade if a size setting is added
  { id: "gannfixed", name: "Gann square fixed", section: "GANN", points: 1,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      const n = Number(d.props?.bars) || 36, { h, l } = c.bars, avg = h.reduce((s, v, k) => s + v - l[k], 0) / (h.length || 1);
      const ratio = Number(d.props?.ratio) || avg || Math.abs(d.pts[0][1]) / 100;
      const x = c.xAt(c.index(d.pts[0][0]) + n), y = c.y(d.pts[0][1] + n * ratio);
      if (x === null || y === null) return null;
      const b: XY = [x, y];
      return <>{square(c, q[0], b)}{c.handles()}</>;
    },
    icon: <><rect x="3" y="3" width="18" height="18"/><path d="M3 21 21 3M3 21l18-9M3 21l9-18"/><circle cx="3" cy="21" r="1.5"/></> },
  { id: "gannsquare", name: "Gann square", section: "GANN", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      return q.length < 2 ? partial(q, c) : <>{square(c, q[0], q[1])}{c.handles()}</>;
    },
    icon: <><rect x="3" y="3" width="18" height="18"/><path d="M3 21 21 3M3 21a18 18 0 0 1 18-18"/></> },
  // 1x1 runs from point 1 through point 2; the other lines change price p units per t units of time
  { id: "gannfan", name: "Gann fan", section: "GANN", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q) return null;
      if (q.length < 2) return partial(q, c);
      const [o, b] = q, [w, h] = [b[0] - o[0], b[1] - o[1]];
      const rays = GANN_FAN.map(([p, t, col]): [string, XY, string] => { const k = p / t; return [`${p}x${t}`, k <= 1 ? [o[0] + w, o[1] + h * k] : [o[0] + w / k, o[1] + h], col]; });
      return <>{fan(c, o, rays)}{rays.map(([k, t, col]) => <g key={`l${k}`}>{tag(k, t[0] + 4, t[1] - 4, col)}</g>)}{c.handles()}</>;
    },
    icon: <><path d="M3 21 21 3M3 21l18-6M3 21l18-12M3 21l6-18M3 21l12-18"/></> },
];
