import type { ReactNode } from "react";
import type { Shape, RenderCtx, ToolDef } from "./model";
import { path as poly, px } from "./model";

type P = [number, number];
/** A path through the midpoints, so bar-stepped brush points read as a smooth stroke. */
const smooth = (q: P[]) => q.length < 3 ? poly(q)
  : `M${q[0][0]} ${q[0][1]}` + q.slice(1, -1).map(([x, y], k) => `Q${x} ${y} ${(x + q[k + 2][0]) / 2} ${(y + q[k + 2][1]) / 2}`).join("") + `L${q[q.length - 1][0]} ${q[q.length - 1][1]}`;
/** An open arrow head at `to`. */
const head = (from: P, to: P, s = 10) => {
  const a = Math.atan2(to[1] - from[1], to[0] - from[0]);
  return `M${to[0] - s * Math.cos(a - 0.45)} ${to[1] - s * Math.sin(a - 0.45)}L${to[0]} ${to[1]}L${to[0] - s * Math.cos(a + 0.45)} ${to[1] - s * Math.sin(a + 0.45)}`;
};
/** The unit vector a→b, its normal and length. */
const axis = (a: P, b: P) => { const L = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1, u: P = [(b[0] - a[0]) / L, (b[1] - a[1]) / L]; return { u, n: [-u[1], u[0]] as P, L }; };
const add = (a: P, v: P, k = 1): P => [a[0] + v[0] * k, a[1] + v[1] * k];
const dot = (a: P, b: P, v: P) => (b[0] - a[0]) * v[0] + (b[1] - a[1]) * v[1];

/** A stroked path; while it has too few points, the line drawn so far. */
const line = (path: string, c: RenderCtx, stroke = c.color, width = c.live ? 2.5 : 2, extra?: ReactNode) => <>
  <path d={path} fill="none" stroke={stroke} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" />{extra}{c.hitPath(path)}{c.handles()}
</>;
/** A TradingView shape: its colour as border with a 20% fill, selectable inside. */
const area = (path: string, color: string, c: RenderCtx, xy?: (P | null)[]) => <>
  <path d={path} fill={color} fillOpacity={0.2} stroke={color} strokeWidth={c.live ? 2 : 1} strokeLinejoin="round" />{c.hitPath(path, true)}{c.handles(xy)}
</>;
const brush = (color: string | null, width: number, opacity: number) => (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  const path = smooth(q);
  return <>
    <path d={path} fill="none" stroke={color ?? c.color} strokeOpacity={opacity} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" />
    {c.hitPath(path)}{c.handles(q.map((p, k) => (k === 0 || k === q.length - 1 ? p : null)))}
  </>;
};
/** Arrow mark up (green, below the point) and down (red, above it); the arrow's tip is the point. */
const mark = (dir: 1 | -1, color: string) => (d: Shape, c: RenderCtx) => {
  const q = px(d, c);
  if (!q) return null;
  const [x, y] = q[0], path = poly([[x, y], [x + 9, y + 10 * dir], [x + 4, y + 10 * dir], [x + 4, y + 24 * dir], [x - 4, y + 24 * dir], [x - 4, y + 10 * dir], [x - 9, y + 10 * dir]], true);
  return <><path d={path} fill={color} />{c.hitPath(path, true)}{c.handles()}</>;
};
/** A circular arc from the first point to the second through the third, its chord filled. */
const arcPath = ([a, b, p]: P[]) => {
  const turn = (p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0]);
  if (Math.abs(turn) < 1e-6) return poly([a, b]);
  const r = Math.hypot(b[0] - a[0], b[1] - a[1]) * Math.hypot(p[0] - b[0], p[1] - b[1]) * Math.hypot(a[0] - p[0], a[1] - p[1]) / (2 * Math.abs(turn));
  // the arc through p is the larger one when the angle at p is acute; on screen (y down) a positive turn is clockwise
  const large = (a[0] - p[0]) * (b[0] - p[0]) + (a[1] - p[1]) * (b[1] - p[1]) > 0 ? 1 : 0;
  return `M${a[0]} ${a[1]}A${r} ${r} 0 ${large} ${turn > 0 ? 1 : 0} ${b[0]} ${b[1]}`;
};

const PURPLE = "#9C27B0", ORANGE = "#FF9800", DEEP_ORANGE = "#F57C00", RED = "#F23645", GREEN = "#089981", PINK = "#E91E63";

export const SHAPES: ToolDef[] = [
  { id: "brush", name: "Brush", section: "BRUSHES", points: "free", hint: "Press and drag to draw", render: brush(null, 2, 1),
    icon: <path d="M4 18c3 0 3-4 6-4s2 4 5 2 3-8 5-10" /> },
  { id: "highlighter", name: "Highlighter", section: "BRUSHES", points: "free", hint: "Press and drag to highlight", render: brush(RED, 20, 0.2),
    icon: <><path d="m14 4 6 6-8 8H6v-6z" /><path d="M4 21h16" /></> },
  { id: "arrowmarker", name: "Arrow marker", section: "ARROWS", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      const [a, b] = q, { u, n, L } = axis(a, b), hl = Math.min(L, Math.max(12, L * 0.35)), hw = hl * 0.7, sw = hw * 0.45, base = add(b, u, -hl);
      const path = poly([add(a, n, sw), add(base, n, sw), add(base, n, hw), b, add(base, n, -hw), add(base, n, -sw), add(a, n, -sw)], true);
      return <><path d={path} fill={c.color} />{c.hitPath(path, true)}{c.handles()}</>;
    },
    icon: <path d="M4 18 14 8l-3-3h8v8l-3-3L6 20z" /> },
  { id: "arrow", name: "Arrow", section: "ARROWS", points: 2,
    render: (d, c) => { const q = px(d, c); return q && q.length > 1 ? line(poly(q) + head(q[0], q[1]), c) : null; },
    icon: <path d="M4 20 20 4M12 4h8v8" /> },
  { id: "arrowup", name: "Arrow mark up", section: "ARROWS", points: 1, render: mark(1, GREEN),
    icon: <path d="m12 4 6 7h-4v9h-4v-9H6z" /> },
  { id: "arrowdown", name: "Arrow mark down", section: "ARROWS", points: 1, render: mark(-1, RED),
    icon: <path d="m12 20 6-7h-4V4h-4v9H6z" /> },
  { id: "rect", name: "Rectangle", section: "SHAPES", points: 2, shortcut: "Alt+Shift+R",
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      const [[xa, ya], [xb, yb]] = q, [l, t, w, h] = [Math.min(xa, xb), Math.min(ya, yb), Math.abs(xb - xa), Math.abs(yb - ya)];
      return <>
        <rect x={l} y={t} width={w} height={h} fill={PURPLE} fillOpacity={0.2} stroke={PURPLE} strokeWidth={c.live ? 2 : 1} />
        {c.hitArea(l, t, Math.max(1, w), Math.max(1, h))}{c.handles()}
      </>;
    },
    icon: <rect x="4" y="6" width="16" height="12" /> },
  // the first two points are one side, the third sets the width (its handle sits on the opposite side)
  { id: "rotrect", name: "Rotated rectangle", section: "SHAPES", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      if (q.length < 3) return line(poly(q), c, PURPLE, c.live ? 2 : 1);
      const [a, b, p] = q, { n } = axis(a, b), w = dot(a, p, n), [a2, b2] = [add(a, n, w), add(b, n, w)];
      return area(poly([a, b, b2, a2], true), PURPLE, c, [a, b, [(a2[0] + b2[0]) / 2, (a2[1] + b2[1]) / 2]]);
    },
    icon: <path d="M3 14 12 3l9 7-9 11z" /> },
  { id: "path", name: "Path", section: "SHAPES", points: "multi",
    render: (d, c) => { const q = px(d, c); return q && q.length > 1 ? line(poly(q) + head(q[q.length - 2], q[q.length - 1]), c) : null; },
    icon: <path d="m3 19 6-10 6 6 6-10M17 5h4v4" /> },
  { id: "circle", name: "Circle", section: "SHAPES", points: 2,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      const [[x, y], [xr, yr]] = q, r = Math.hypot(xr - x, yr - y);
      return area(`M${x - r} ${y}A${r} ${r} 0 1 0 ${x + r} ${y}A${r} ${r} 0 1 0 ${x - r} ${y}Z`, ORANGE, c);
    },
    icon: <circle cx="12" cy="12" r="8" /> },
  // the first two points are one axis, the third sets the other (a circle until it is placed)
  { id: "ellipse", name: "Ellipse", section: "SHAPES", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      const [a, b, p] = q, { u, n, L } = axis(a, b), m = add(a, u, L / 2), ry = p ? dot(m, p, n) : L / 2, rx = L / 2;
      const rot = Math.atan2(u[1], u[0]) * 180 / Math.PI, r = `${rx} ${Math.abs(ry)} ${rot} 1 0`;
      return area(`M${a[0]} ${a[1]}A${r} ${b[0]} ${b[1]}A${r} ${a[0]} ${a[1]}Z`, RED, c, [a, b, p ? add(m, n, ry) : null]);
    },
    icon: <ellipse cx="12" cy="12" rx="9" ry="6" /> },
  { id: "polyline", name: "Polyline", section: "SHAPES", points: "multi",
    render: (d, c) => { const q = px(d, c); return q && q.length > 1 ? area(poly(q, true), c.color, c) : null; },
    icon: <path d="M4 18 6 6l8 4 6-4-2 14z" /> },
  { id: "triangle", name: "Triangle", section: "SHAPES", points: 3,
    render: (d, c) => { const q = px(d, c); return q && q.length > 1 ? area(poly(q, q.length > 2), DEEP_ORANGE, c) : null; },
    icon: <path d="M12 4 21 20H3z" /> },
  { id: "arc", name: "Arc", section: "SHAPES", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      if (q.length < 3) return line(poly(q), c, PINK, c.live ? 2 : 1);
      const path = arcPath(q);
      return <>
        <path d={path + "Z"} fill={PINK} fillOpacity={0.2} stroke="none" />
        <path d={path} fill="none" stroke={PINK} strokeWidth={c.live ? 2 : 1} />
        {c.hitPath(path + "Z", true)}{c.handles()}
      </>;
    },
    icon: <path d="M4 18a8 8 0 0 1 16 0" /> },
  // start, end, then the control point
  { id: "curve", name: "Curve", section: "SHAPES", points: 3,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      return line(q.length < 3 ? poly(q) : `M${q[0][0]} ${q[0][1]}Q${q[2][0]} ${q[2][1]} ${q[1][0]} ${q[1][1]}`, c);
    },
    icon: <path d="M3 19Q6 4 21 8" /> },
  // start, end, then the two control points
  { id: "dcurve", name: "Double curve", section: "SHAPES", points: 4,
    render: (d, c) => {
      const q = px(d, c);
      if (!q || q.length < 2) return null;
      const [a, b, c1, c2] = q;
      return line(!c1 ? poly(q) : !c2 ? `M${a[0]} ${a[1]}Q${c1[0]} ${c1[1]} ${b[0]} ${b[1]}` : `M${a[0]} ${a[1]}C${c1[0]} ${c1[1]} ${c2[0]} ${c2[1]} ${b[0]} ${b[1]}`, c);
    },
    icon: <path d="M3 18C8 2 14 22 21 6" /> },
];
