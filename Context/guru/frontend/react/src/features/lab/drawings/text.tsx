import type { ReactNode } from "react";
import type { Drawing, Pt, RenderCtx, Shape, ToolDef } from "./model";

// TradingView's TEXT AND NOTES. The text editor opens with its bottom-left corner at pts[0], so each text body is drawn from there.
const S = "TEXT AND NOTES", FS = 14, LH = FS * 1.25;
const at = (d: Shape, c: RenderCtx, k = 0) => {
  const p = d.pts[k] ?? d.pts[0], x = c.x(p[0]), y = c.y(p[1]);
  return x === null || y === null ? null : [x, y] as const;
};
const lines = (d: Shape, dflt = "") => (d.text ?? dflt).split("\n");
/** A text box's size, estimated per glyph (jsdom and the first render measure nothing). */
const size = (ls: string[], pad: number) => ({ w: Math.max(1, ...ls.map((s) => s.length)) * FS * 0.6 + pad * 2, h: ls.length * LH + pad * 2 });
const txt = (ls: string[], x: number, top: number, pad: number, fill: string) =>
  <text x={x + pad} y={top + pad} fill={fill} fontSize={FS}>{ls.map((s, k) => <tspan key={k} x={x + pad} dy={k ? LH : FS}>{s || " "}</tspan>)}</text>;
const say = (kind: string, text: string) => (pts: Pt[]): Drawing => ({ kind, pts, text });
/** A popup shown while the drawing is selected or hovered (SMIL: the layer's CSS has no hover rule for it). */
const tip = (c: RenderCtx, body: ReactNode) =>
  <g opacity={c.live ? 1 : 0} data-lab-tip>{!c.live && <set attributeName="opacity" to="1" begin={`lab-tip-${c.i}.mouseover`} end={`lab-tip-${c.i}.mouseout`} />}{body}</g>;
/** Note's and Pin's popup: white text on translucent blue, above the marker. */
const popup = (d: Shape, c: RenderCtx, x: number, bottom: number) => {
  const ls = lines(d), { w, h } = size(ls, 8);
  return tip(c, <><rect x={x - w / 2} y={bottom - h} width={w} height={h} rx={4} fill={c.color} fillOpacity={0.7} stroke={c.color} />{txt(ls, x - w / 2, bottom - h, 8, "#fff")}</>);
};
/** A plain text: the colour on no background. */
const plain = (d: Shape, c: RenderCtx, x: number, y: number) => {
  const ls = lines(d), { w, h } = size(ls, 0);
  return <>{txt(ls, x, y - h, 0, c.color)}{c.hitArea(x, y - h, w, h)}</>;
};

// Anchored text keeps its place on the screen: its position is stored as fractions of the layer.
const anchor = (pts: Pt[], c: RenderCtx): Drawing => {
  const x = c.x(pts[0][0]), y = c.y(pts[0][1]);
  return { kind: "atext", pts, text: "Text", ...(c.plot && c.height && x !== null && y !== null
    ? { props: { fx: x / c.plot, fy: y / c.height, t: pts[0][0], p: pts[0][1] } } : {}) };
};
/** Anchored text's screen position; a move or handle drag (which change pts) shifts it by the dragged pixels. */
const anchored = (d: Shape, c: RenderCtx) => {
  const q = d.props, here = at(d, c);
  if (!q || typeof q.t !== "string" || typeof q.p !== "number" || !c.plot) return here; // not laid out yet: at its point
  const x0 = c.xAt(c.index(q.t)), x1 = c.xAt(c.index(d.pts[0][0])), y0 = c.y(q.p), y1 = c.y(d.pts[0][1]);
  return [Number(q.fx) * c.plot + (x0 === null || x1 === null ? 0 : x1 - x0), Number(q.fy) * c.height + (y0 === null || y1 === null ? 0 : y1 - y0)] as const;
};

export const TEXT: ToolDef[] = [
  { id: "text", name: "Text", section: S, points: 1, text: true, make: say("text", "Text"),
    render: (d, c) => { const p = at(d, c); return p && <>{plain(d, c, p[0], p[1])}{c.handles()}</>; },
    icon: <path d="M5 5h14M12 5v14M9 19h6"/> },
  { id: "atext", name: "Anchored text", section: S, points: 1, text: true, make: anchor,
    render: (d, c) => { const p = anchored(d, c); return p && <>{plain(d, c, p[0], p[1])}{c.handles([p])}</>; },
    icon: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M8 8h8M12 8v9"/></> },
  { id: "note", name: "Note", section: S, points: 1, text: true, make: say("note", "Note"),
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p;
      return <g id={`lab-tip-${c.i}`}>
        <rect x={x - 11} y={y - 28} width={22} height={22} rx={4} fill={c.color} />
        <path d={`M${x - 5} ${y - 7}L${x} ${y}L${x + 5} ${y - 7}Z`} fill={c.color} />
        <path d={`M${x - 6} ${y - 22}h12M${x - 6} ${y - 17}h12M${x - 6} ${y - 12}h7`} stroke="#fff" strokeWidth={1.5} />
        {c.hitArea(x - 11, y - 28, 22, 28)}{c.handles()}{popup(d, c, x, y - 32)}
      </g>;
    },
    icon: <><path d="M5 4h10l4 4v12H5z"/><path d="M15 4v4h4M8 12h8M8 16h5"/></> },
  { id: "pnote", name: "Price note", section: S, points: 2,
    render: (d, c) => {
      const a = at(d, c), b = at(d, c, 1);
      if (!a || !b) return null;
      const s = c.fmt(d.pts[0][1]), w = s.length * 6.4 + 12;
      return <>
        <line x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={c.color} strokeWidth={c.live ? 2 : 1.5} />
        <circle cx={a[0]} cy={a[1]} r={3} fill={c.color} />
        {c.label([s], b[0], b[1] - 10.5, c.color)}
        {c.hit(a[0], a[1], b[0], b[1])}{c.hitArea(b[0] - w / 2, b[1] - 10.5, w, 21)}{c.handles()}
      </>;
    },
    icon: <><circle cx="5" cy="19" r="2"/><path d="m6.5 17.5 6-6"/><rect x="12" y="4" width="9" height="7" rx="1"/></> },
  { id: "pin", name: "Pin", section: S, points: 1, text: true, make: say("pin", "Pin"),
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p;
      return <g id={`lab-tip-${c.i}`}>
        <path d={`M${x} ${y}C${x - 3} ${y - 6} ${x - 9} ${y - 11} ${x - 9} ${y - 17}A9 9 0 0 1 ${x + 9} ${y - 17}C${x + 9} ${y - 11} ${x + 3} ${y - 6} ${x} ${y}Z`} fill={c.color} />
        <circle cx={x} cy={y - 17} r={3.5} fill="#fff" />
        {c.hitArea(x - 9, y - 26, 18, 26)}{c.handles()}{popup(d, c, x, y - 30)}
      </g>;
    },
    icon: <><path d="M12 21s-6-6.2-6-11a6 6 0 0 1 12 0c0 4.8-6 11-6 11z"/><circle cx="12" cy="10" r="2"/></> },
  // the core editor edits one text: rows are lines, cells are split by "|"
  { id: "table", name: "Table", section: S, points: 1, text: true, make: say("table", "Text|Text\nText|Text"),
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const rows = lines(d).map((r) => r.split("|")), n = Math.max(...rows.map((r) => r.length));
      const ws = Array.from({ length: n }, (_, k) => Math.max(40, ...rows.map((r) => (r[k] ?? "").length * FS * 0.6 + 12)));
      const rh = LH + 8, W = ws.reduce((s, w) => s + w, 0), H = rows.length * rh, [x, top] = [p[0], p[1] - H];
      const xs = ws.map((_, k) => x + ws.slice(0, k).reduce((s, w) => s + w, 0));
      return <>
        <rect x={x} y={top} width={W} height={H} fill={c.color} fillOpacity={0.1} stroke={c.color} />
        {xs.slice(1).map((cx) => <line key={`c${cx}`} x1={cx} x2={cx} y1={top} y2={top + H} stroke={c.color} />)}
        {rows.slice(1).map((_, k) => <line key={`r${k}`} x1={x} x2={x + W} y1={top + (k + 1) * rh} y2={top + (k + 1) * rh} stroke={c.color} />)}
        {rows.flatMap((r, j) => xs.map((cx, k) => <text key={`${j}.${k}`} data-lab-cell={`${j}.${k}`} x={cx + 6} y={top + j * rh + 4 + FS} fill={c.color} fontSize={FS}>{r[k] ?? ""}</text>))}
        {c.hitArea(x, top, W, H)}{c.handles()}
      </>;
    },
    icon: <><rect x="3" y="5" width="18" height="14"/><path d="M3 12h18M12 5v14"/></> },
  // pts[0] is the bubble (where the editor opens), pts[1] the point it shows
  { id: "callout", name: "Callout", section: S, points: 2, text: true, make: say("callout", "Callout"),
    render: (d, c) => {
      const a = at(d, c), b = at(d, c, 1);
      if (!a || !b) return null;
      const ls = lines(d), { w, h } = size(ls, 8), top = a[1] - h, [cx, cy] = [a[0] + w / 2, top + h / 2];
      const len = Math.hypot(b[0] - cx, b[1] - cy) || 1, k = Math.min(10, h / 3) / len, [px, py] = [-(b[1] - cy) * k, (b[0] - cx) * k];
      return <>
        <g opacity={0.7}><path d={`M${cx + px} ${cy + py}L${b[0]} ${b[1]}L${cx - px} ${cy - py}Z`} fill={c.color} /><rect x={a[0]} y={top} width={w} height={h} rx={4} fill={c.color} /></g>
        <rect x={a[0]} y={top} width={w} height={h} rx={4} fill="none" stroke={c.color} />
        {txt(ls, a[0], top, 8, "#fff")}
        {c.hitArea(a[0], top, w, h)}{c.hit(cx, cy, b[0], b[1])}{c.handles()}
      </>;
    },
    icon: <><rect x="9" y="3" width="12" height="9" rx="1"/><path d="M12 12 4 20"/></> },
  // a speech bubble whose square bottom-left corner is the point
  { id: "comment", name: "Comment", section: S, points: 1, text: true, make: say("comment", "Comment"),
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p, ls = lines(d), { w, h } = size(ls, 8), top = y - h, r = Math.min(10, h / 2);
      return <>
        <path d={`M${x} ${y}V${top + r}A${r} ${r} 0 0 1 ${x + r} ${top}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${top + r}V${y - r}A${r} ${r} 0 0 1 ${x + w - r} ${y}Z`} fill={c.color} />
        {txt(ls, x, top, 8, "#fff")}{c.hitArea(x, top, w, h)}{c.handles()}
      </>;
    },
    icon: <path d="M4 20V8a3 3 0 0 1 3-3h10a3 3 0 0 1 3 3v6a3 3 0 0 1-3 3H7z"/> },
  { id: "plabel", name: "Price label", section: S, points: 1,
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p, s = c.fmt(d.pts[0][1]), w = s.length * FS * 0.6 + 12, h = LH + 6, top = y - 7 - h;
      return <>
        <path d={`M${x} ${y}L${x - 6} ${y - 7}H${x - w / 2}V${top}H${x + w / 2}V${y - 7}H${x + 6}Z`} fill={c.color} />
        <text x={x} y={top + 3 + FS} fill="#fff" fontSize={FS} textAnchor="middle">{s}</text>
        {c.hitArea(x - w / 2, top, w, h + 7)}{c.handles()}
      </>;
    },
    icon: <path d="M3 7h14l4 5-4 5H3z"/> },
  // a post rising 60 px from the point to a plate with the text
  { id: "signpost", name: "Signpost", section: S, points: 1, text: true, make: say("signpost", "Signpost"),
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p, ls = lines(d), { w, h } = size(ls, 6), top = y - 60 - h;
      return <>
        <line x1={x} x2={x} y1={y} y2={y - 60} stroke={c.color} strokeWidth={2} />
        <circle cx={x} cy={y} r={3} fill={c.color} />
        <rect x={x - w / 2} y={top} width={w} height={h} rx={4} fill={c.color} />
        {txt(ls, x - w / 2, top, 6, "#fff")}
        {c.hit(x, y, x, y - 60)}{c.hitArea(x - w / 2, top, w, h)}{c.handles()}
      </>;
    },
    icon: <path d="M12 21V3M12 5h7l2 2.5-2 2.5h-7"/> },
  { id: "flag", name: "Flag mark", section: S, points: 1,
    render: (d, c) => {
      const p = at(d, c);
      if (!p) return null;
      const [x, y] = p;
      return <>
        <line x1={x} x2={x} y1={y} y2={y - 26} stroke={c.color} strokeWidth={2} />
        <path d={`M${x} ${y - 26}h16l-4 5 4 5H${x}Z`} fill={c.color} />
        {c.hitArea(x - 3, y - 26, 19, 26)}{c.handles()}
      </>;
    },
    icon: <path d="M6 21V3M6 4h12l-3 4 3 4H6"/> },
];
