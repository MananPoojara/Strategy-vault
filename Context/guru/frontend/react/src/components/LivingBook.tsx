import { useEffect, useMemo, useRef } from "react";
import * as k from "./iso/iso-kit";
import type { Point3, Projection } from "./iso/iso-kit";
import "./iso/iso.css";

// Home hero figure (user request 30 Oct 2026): an open book on a round stand. A highlight reads the rule lines on the
// left page, a rising equity curve draws itself on the right page, then the page turns and the cycle repeats.
const WIDTH = 560, HEIGHT = 380;
const W = 70, Y0 = -48, Y1 = 48;
const BLOCK_H = 4, COVER_H = 1.4, ZP = COVER_H + BLOCK_H;
const STAND_R = 88, STAND_H = 5;
const READ = 2.4, DRAW = 2.6, HOLD = 0.8, TURN = 1.4, CYCLE = READ + DRAW + HOLD + TURN;

const disc = { x: -STAND_R, y: -STAND_R, w: 2 * STAND_R, d: 2 * STAND_R, r: STAND_R };
const P: Projection = k.fitProjection([...k.boxCorners(disc, -STAND_H - 3, 0), [0, 0, ZP + W * 0.9]], WIDTH, HEIGHT, { pad: 10, azimuth: 62, elevation: 32 });
const at = (p: Point3) => k.iso(p, P);

function staticSvg(): string {
  const S = k.solidSvg, L = k.lineSvg;
  const o: string[] = [`<path class="iso-halo" d="${k.haloOf(disc, -STAND_H - 3, STAND_H, P)}"/>`];
  o.push(S(k.cylinder(0, 0, STAND_R - 6, -STAND_H - 3, 3, P, 96), { tone: "lo" }));
  o.push(S(k.cylinder(0, 0, STAND_R, -STAND_H, STAND_H, P, 120, 1.2), { tone: "mid", inner: L(k.sideArc(0, 0, STAND_R, -STAND_H / 2, P, 96), { tone: "faint" }) }));
  o.push(L(k.ring(0, 0, STAND_R - 6, 0, P, 120), { tone: "lo" }));
  const ticks = k.radialTicks(0, 0, STAND_R - 2.5, 72, 6, 0, [1.4, 2.6], P);
  o.push(L(ticks.minor, { tone: "faint" }), L(ticks.major, { tone: "lo" }));
  const cover = { x: -W - 4, y: Y0 - 4, w: 2 * W + 8, d: Y1 - Y0 + 8, r: 3 };
  o.push(`<path class="iso-halo" d="${k.haloOf(cover, 0, 0.1, P)}" style="filter:blur(3px)"/>`);
  o.push(S(k.slabOf(cover, 0, COVER_H, P, 3, 0.4), { tone: "mid", className: "lb-cover" }));
  for (const [x0, x1] of [[-W, -1], [1, W]]) {
    const plan = { x: x0, y: Y0, w: x1 - x0, d: Y1 - Y0, r: 1.5 };
    const inner = [1.2, 2.4, 3.4].map((dz) => L(k.sideSeam(plan, COVER_H + dz, P, 3), { tone: "faint" })).join("");
    o.push(S(k.slabOf(plan, COVER_H, BLOCK_H, P, 3, 0.4), { tone: "hi", crease: "lo", inner }));
  }
  o.push(L(k.lineOnTop([0, Y0], [0, Y1], ZP, P), { tone: "lo" }));
  o.push(L(k.onTop([[-2, Y0 + 14], [-3, Y0 + 2]], ZP + 0.1, P) + k.segment([-3, Y0 + 2, ZP], [-4, Y0 - 3, COVER_H], P), { tone: "lit" }));
  o.push(k.dotsSvg([at([-4, Y0 - 3, COVER_H])], { size: 0.7, tone: "lit" }));
  const g: string[] = [];
  for (let x = 8; x <= W - 6; x += 9) g.push(k.lineOnTop([x, Y0 + 10], [x, Y1 - 8], ZP + 0.05, P));
  for (let y = Y0 + 10; y <= Y1 - 8; y += 8.4) g.push(k.lineOnTop([8, y], [W - 6, y], ZP + 0.05, P));
  o.push(L(g.join(""), { tone: "faint" }));
  o.push(`<g data-part="left"></g><g data-part="right"></g><g data-part="sheet"></g>`);
  return k.figureSvg({ width: WIDTH, height: HEIGHT, label: "An open book on a round stand: rules are read on the left page and a rising equity curve draws itself on the right page, then the page turns.", body: o });
}

export function LivingBook() {
  const ref = useRef<HTMLDivElement>(null);
  const html = useMemo(staticSvg, []);
  useEffect(() => {
    const root = ref.current?.querySelector("svg");
    if (!root) return;
    const $ = (n: string) => root.querySelector(`[data-part="${n}"]`) as SVGGElement;
    const leftG = $("left"), rightG = $("right"), sheetG = $("sheet");
    let seed = 7;
    const rnd = () => (seed = (seed * 9301 + 49297) % 233280) / 233280;
    const rulesPage = () => Array.from({ length: 9 }, (_, i) => (i === 4 ? 0 : 0.45 + rnd() * 0.5));
    const chartPage = () => {
      let out: number[] = [], v = 0;
      for (let i = 0; i <= 40; i++) { v += 0.025 + (rnd() - 0.5) * 0.09 + (rnd() < 0.12 ? -0.08 : 0); out.push(v); }
      for (let r = 0; r < 2; r++) out = out.map((x, i) => (i && i < out.length - 1 ? (out[i - 1] + 2 * x + out[i + 1]) / 4 : x));
      const lo = Math.min(...out), hi = Math.max(...out);
      return out.map((x) => 0.08 + (0.84 * (x - lo)) / (hi - lo || 1));
    };
    const lineY = (i: number) => Y0 + 12 + i * 7.4;
    const sheetPoint = (u: number, y: number, th: number): Point3 => {
      const phi = th + 0.32 * Math.sin(th) * u, s = u * (W - 2);
      return [1 + s * Math.cos(phi), y, ZP + 0.3 + s * Math.sin(phi)];
    };
    const pts = (list: [number, number][]) => list.map((q) => `${q[0].toFixed(2)} ${q[1].toFixed(2)}`).join("L");
    let left = rulesPage(), right = chartPage(), nextLeft = rulesPage(), nextRight = chartPage();
    const textOnLeft = (lines: number[], read: number) => lines.map((f, i) => {
      if (!f) return "";
      const y = lineY(i), a: [number, number] = [-W + 7, y], b: [number, number] = [-W + 7 + f * (W - 16), y];
      let s = k.lineSvg(k.lineOnTop(a, b, ZP + 0.05, P), { tone: "lo", free: true });
      if (read > i) s += k.lineSvg(k.lineOnTop(a, [a[0] + (b[0] - a[0]) * Math.min(1, read - i), y], ZP + 0.08, P), { tone: "lit", free: true });
      return s;
    }).join("");
    const chartOnRight = (vals: number[], drawn: number) => {
      if (drawn <= 0) return "";
      const n = Math.max(1, Math.floor(drawn * (vals.length - 1)));
      const p: [number, number][] = [];
      for (let i = 0; i <= n; i++) p.push([8 + (i / (vals.length - 1)) * (W - 14), Y1 - 8 - vals[i] * (Y1 - Y0 - 18)]);
      const end = at([p[p.length - 1][0], p[p.length - 1][1], ZP + 0.1]);
      return `<path class="iso-line" data-tone="lit" data-free="both" style="stroke-width:1.5" d="${k.onTop(p, ZP + 0.1, P)}"/>` + k.dotsSvg([end], { size: 1.1, tone: "lit" });
    };
    const drawSheet = (th: number) => {
      if (th <= 0) { sheetG.innerHTML = ""; return; }
      const N = 10, top: [number, number][] = [], bot: [number, number][] = [];
      for (let i = 0; i <= N; i++) { top.push(at(sheetPoint(i / N, Y0, th))); bot.push(at(sheetPoint(i / N, Y1, th))); }
      const outline = "M" + pts(top.concat(bot.reverse())) + "Z";
      const back = th > Math.PI / 2;
      let s = `<g class="iso-solid"><path class="iso-fill" d="${outline}"/><path class="${back ? "iso-top" : "iso-shade"}" data-shade="3" d="${outline}"/>`;
      if (!back) {
        const p = right.map((v, i) => at(sheetPoint((8 + (i / (right.length - 1)) * (W - 14)) / (W - 2), Y1 - 8 - v * (Y1 - Y0 - 18), th)));
        s += `<path class="iso-line" data-tone="lit" data-free="both" style="stroke-width:1.3" d="M${pts(p)}"/>`;
      } else {
        nextLeft.forEach((f, i) => {
          if (!f) return;
          const y = lineY(i), u0 = 7 / (W - 2), u1 = (7 + f * (W - 16)) / (W - 2);
          s += `<path class="iso-line" data-tone="lo" data-free="both" d="M${pts([at(sheetPoint(1 - u0, y, th)), at(sheetPoint(1 - u1, y, th))])}"/>`;
        });
      }
      sheetG.innerHTML = s + `<path class="iso-edge" style="fill:none" d="${outline}"/></g>`;
    };
    const ease = (x: number) => (x < 0.5 ? 4 * x ** 3 : 1 - (-2 * x + 2) ** 3 / 2);
    let t = 0, last = 0, frame = 0, visible = false;
    const render = () => {
      const read = Math.min(1, t / READ) * 9, drawn = Math.max(0, Math.min(1, (t - READ) / DRAW));
      const turn = Math.max(0, Math.min(1, (t - READ - DRAW - HOLD) / TURN));
      leftG.innerHTML = textOnLeft(left, read);
      rightG.innerHTML = turn > 0 ? "" : chartOnRight(right, drawn);
      drawSheet(ease(turn) * Math.PI);
    };
    const tick = (now: number) => {
      const dt = last ? Math.min((now - last) / 1000, 1 / 30) : 1 / 60;
      last = now; t += dt;
      if (t >= CYCLE) { t = 0; left = nextLeft; right = nextRight; nextLeft = rulesPage(); nextRight = chartPage(); }
      render();
      frame = visible ? requestAnimationFrame(tick) : 0;
    };
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) { t = READ + DRAW; render(); return; }
    render();
    const io = new IntersectionObserver((e) => {
      visible = e[e.length - 1].isIntersecting;
      if (visible && !frame) { last = 0; frame = requestAnimationFrame(tick); }
    });
    io.observe(root);
    return () => { io.disconnect(); cancelAnimationFrame(frame); };
  }, []);
  return <div ref={ref} className="living-book iso" data-theme="light" dangerouslySetInnerHTML={{ __html: html }} />;
}
