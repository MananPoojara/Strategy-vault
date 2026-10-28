const ASSET_BASE = '/vendor/login/'
// Ganga aarti lamp (owner 13-10-2026): a hand-held cast-brass tiered lamp as waved at the Ganga ghats: three rings
// of wick cups (3, 5, 7) on a ribbed column, a dhoop kalash crown with its own flame, lotus-petal bands, beaded
// rims, chased scrolls on the arms, an inverted-lotus bowl and a long twisted handle with a knob end.
// THE BODY IS SHOWN AS A BAKED IMAGE, like the frame (Chrome's GPU raster mangles complex SVG): GangaAartiDeepakArt
// is the source; open /__lab/deepak?art and run scripts/bake_deepak.sh to regenerate
// public/landing/ganga-deepak@2x.webp after changing it. Only the 16 flames and the glow are live.

export const GD_W = 400, GD_H = 820
const CX = 200
const f = n => +n.toFixed(1)
const rad = d => d * Math.PI / 180

// Tiers: a shallow lotus dish (centre height y, rim radius R, foreshortened depth ry) with its cups (scale s) on the
// rim, the outermost at +-amax degrees round the front.
const TIERS = [
  { n: 3, y: 200, R: 56, ry: 12, s: 1.0, amax: 62 },
  { n: 5, y: 298, R: 98, ry: 19, s: 1.08, amax: 72 },
  { n: 7, y: 402, R: 142, ry: 26, s: 1.16, amax: 78 },
]
const CUPS = TIERS.flatMap((t, ti) => Array.from({ length: t.n }, (_, i) => {
  const a = rad(-t.amax + (2 * t.amax * i) / (t.n - 1))
  const ringY = t.y + t.ry * Math.cos(a)
  const x = CX + t.R * Math.sin(a)
  return { ti, x, ringY, cy: ringY - 19 * t.s, s: t.s * (0.94 + 0.06 * Math.cos(a)) }
}))
const CROWN = { x: CX, y: 101 }
// Flame bases (wick tips), front-most last; k = flame size.
export const GD_WICKS = [
  { x: CROWN.x, y: CROWN.y - 7, k: 1.18 },
  ...CUPS.map(c => ({ x: c.x, y: c.cy - 6.5 * c.s, k: c.s })),
]
// Centre of the flame cluster: the hand's pivot and where the light on the photo is aimed.
export const GD_ANCHOR = {
  x: CX,
  y: GD_WICKS.reduce((a, w) => a + w.y, 0) / GD_WICKS.length,
}
export const GD_THREE_ROW_ANCHOR = {
  x: CX,
  y: GD_WICKS.slice(0, 9).reduce((sum, wick) => sum + wick.y, 0) / 9,
}

// ---------- drawing helpers ----------
const ENG = '#3a2203'      // engraved line
const HI = '#fff1bd'       // chased-line highlight
// Lotus petal from (cx, b), half-width w, length h; dir -1 points up, +1 down.
const petal = (cx, b, w, h, dir) =>
  `M${f(cx - w)} ${f(b)} C${f(cx - w)} ${f(b + dir * h * 0.55)} ${f(cx - w * 0.35)} ${f(b + dir * h * 0.88)} ${f(cx)} ${f(b + dir * h)} ` +
  `C${f(cx + w * 0.35)} ${f(b + dir * h * 0.88)} ${f(cx + w)} ${f(b + dir * h * 0.55)} ${f(cx + w)} ${f(b)}Z`

// A band of petals round an ellipse (cx, cy, rx, ry), seen from the front: side petals narrow and foreshortened,
// front ones on top.
function PetalBand({ cx, cy, rx, ry, n, h, dir, fill = 'url(#gdBowl)', lw = 0.7, amax = 80 }) {
  const ps = Array.from({ length: n }, (_, i) => {
    const a = rad(-amax + (2 * amax * i) / (n - 1)), c = Math.cos(a)
    return { x: cx + rx * Math.sin(a), y: cy + ry * c, w: (Math.PI * rx / n) * (0.25 + 0.75 * c), h: h * (0.45 + 0.55 * c) }
  }).sort((p, q) => p.y - q.y)
  return ps.map((p, i) => (
    <g key={i}>
      <path d={petal(p.x, p.y, p.w * 1.08, p.h, dir)} fill={fill} stroke={ENG} strokeWidth={lw} strokeOpacity="0.8" />
      <path d={petal(p.x, p.y, p.w * 0.6, p.h * 0.66, dir)} fill="none" stroke={ENG} strokeWidth={lw * 0.7} strokeOpacity="0.55" />
      <path d={`M${f(p.x)} ${f(p.y)} L${f(p.x)} ${f(p.y + dir * p.h * 0.8)}`} stroke={HI} strokeWidth={lw * 0.7} strokeOpacity="0.55" />
    </g>
  ))
}
// Beads along the front half of an ellipse.
function Beads({ cx, cy, rx, ry, r, n, from = -88, to = 88 }) {
  return Array.from({ length: n }, (_, i) => {
    const a = rad(from + ((to - from) * i) / (n - 1))
    const k = 0.45 + 0.55 * Math.cos(a)
    return <circle key={i} cx={f(cx + rx * Math.sin(a))} cy={f(cy + ry * Math.cos(a))} r={f(r * (0.7 + 0.3 * k))} fill="url(#gdBead)" />
  })
}
// A ring band (moulding) drawn as a squat cylinder: front face, top highlight.
function Collar({ cx, y, rx, ry, h, beads = 0 }) {
  return (
    <g>
      <path d={`M${f(cx - rx)} ${f(y)} A${rx} ${ry} 0 0 0 ${f(cx + rx)} ${f(y)} L${f(cx + rx)} ${f(y + h)} A${rx} ${ry} 0 0 1 ${f(cx - rx)} ${f(y + h)}Z`}
        fill="url(#gdCyl)" />
      <ellipse cx={cx} cy={y} rx={rx} ry={ry} fill="url(#gdTop)" />
      <path d={`M${f(cx - rx)} ${f(y + h)} A${rx} ${ry} 0 0 0 ${f(cx + rx)} ${f(y + h)}`} fill="none" stroke={ENG} strokeWidth="0.9" strokeOpacity="0.7" />
      {beads > 0 && <Beads cx={cx} cy={y + h / 2 + 0.3} rx={rx} ry={ry} r={h * 0.42} n={beads} from={-80} to={80} />}
    </g>
  )
}
// Spherical knop on the column.
function Knop({ cx, y, rx, ry }) {
  return (
    <g>
      <ellipse cx={cx} cy={y} rx={rx} ry={ry} fill="url(#gdBall)" />
      <path d={`M${f(cx - rx)} ${f(y)} A${rx} ${ry * 0.38} 0 0 0 ${f(cx + rx)} ${f(y)}`} fill="none" stroke={ENG} strokeWidth="0.8" strokeOpacity="0.7" />
      <path d={`M${f(cx - rx * 0.92)} ${f(y + 1.2)} A${rx} ${ry * 0.38} 0 0 0 ${f(cx + rx * 0.92)} ${f(y + 1.2)}`} fill="none" stroke={HI} strokeWidth="0.6" strokeOpacity="0.5" />
    </g>
  )
}
// Small spiral curl (chased scroll) at (x, y).
function curl(x, y, r, a0, dir) {
  const pts = []
  for (let i = 0; i <= 40; i++) {
    const th = a0 + dir * (i / 40) * 1.6 * 2 * Math.PI, rr = r * (1 - 0.82 * i / 40)
    pts.push(`${f(x + rr * Math.cos(th))} ${f(y + rr * Math.sin(th))}`)
  }
  return `M${pts.join(' L')}`
}

// One wick cup: lotus bowl with petals, beaded lip, oil, cotton wick, and the warm light of its own flame.
function Cup({ x, cy, ringY, s }) {
  const rx = 15 * s, ry = 4.6 * s, bh = 15 * s
  return (
    <g>
      {/* stem from the ring to the bowl */}
      <path d={`M${f(x - 3.2 * s)} ${f(ringY)} L${f(x - 2.4 * s)} ${f(cy + bh - 2)} L${f(x + 2.4 * s)} ${f(cy + bh - 2)} L${f(x + 3.2 * s)} ${f(ringY)}Z`} fill="url(#gdCyl)" />
      <ellipse cx={f(x)} cy={f(cy + bh + 2.6 * s)} rx={f(4.4 * s)} ry={f(2.6 * s)} fill="url(#gdBall)" />
      <ellipse cx={f(x)} cy={f(ringY - 1)} rx={f(5.4 * s)} ry={f(2 * s)} fill="url(#gdBall)" />
      {/* bowl */}
      <path d={`M${f(x - rx)} ${f(cy)} C${f(x - rx)} ${f(cy + bh * 0.7)} ${f(x - rx * 0.45)} ${f(cy + bh)} ${f(x)} ${f(cy + bh)} C${f(x + rx * 0.45)} ${f(cy + bh)} ${f(x + rx)} ${f(cy + bh * 0.7)} ${f(x + rx)} ${f(cy)}Z`}
        fill="url(#gdBowl)" />
      <PetalBand cx={x} cy={cy + bh * 0.98} rx={rx * 0.5} ry={ry * 0.4} n={5} h={bh * 0.78} dir={-1} fill="url(#gdPetal)" lw={0.55} />
      <path d={`M${f(x - rx)} ${f(cy)} C${f(x - rx)} ${f(cy + bh * 0.7)} ${f(x - rx * 0.45)} ${f(cy + bh)} ${f(x)} ${f(cy + bh)} C${f(x + rx * 0.45)} ${f(cy + bh)} ${f(x + rx)} ${f(cy + bh * 0.7)} ${f(x + rx)} ${f(cy)}Z`}
        fill="url(#gdCupShade)" />
      {/* lip, oil and wick */}
      <ellipse cx={f(x)} cy={f(cy)} rx={f(rx + 0.8)} ry={f(ry + 0.5)} fill="url(#gdTop)" stroke={ENG} strokeWidth="0.6" strokeOpacity="0.6" />
      <ellipse cx={f(x)} cy={f(cy + 0.3)} rx={f(rx - 2.6 * s)} ry={f(ry - 1.4 * s)} fill="url(#gdOil)" />
      <Beads cx={x} cy={cy + 0.6} rx={rx + 0.4} ry={ry + 0.4} r={1.15 * s} n={11} from={-82} to={82} />
      <ellipse cx={f(x)} cy={f(cy - 1)} rx={f(rx * 1.5)} ry={f(ry * 3)} fill="url(#gdWarm)" />
      <path d={`M${f(x - 1.5 * s)} ${f(cy + 0.6)} C${f(x - 1.6 * s)} ${f(cy - 3 * s)} ${f(x - 0.9 * s)} ${f(cy - 5.6 * s)} ${f(x + 0.2 * s)} ${f(cy - 7.2 * s)} ` +
        `C${f(x + 1.1 * s)} ${f(cy - 5.4 * s)} ${f(x + 1.6 * s)} ${f(cy - 3 * s)} ${f(x + 1.5 * s)} ${f(cy + 0.6)}Z`} fill="url(#gdWick)" />
    </g>
  )
}

// One tier: the dish (hanging lotus petals and chased scrolls underneath, beaded rim, flame-lit inside), the lotus
// round the column, then its cups, back ones first.
function Tier({ t, ti }) {
  const { y, R, ry, s } = t, d = R * 0.22
  const cups = CUPS.filter(c => c.ti === ti).sort((a, b) => a.ringY - b.ringY)
  const under = `M${CX - R} ${y} A${R} ${f(ry + d)} 0 0 0 ${CX + R} ${y}Z`
  return (
    <g data-deepak-tier={ti + 2}>
      <path d={under} fill="url(#gdBowl)" />
      <PetalBand cx={CX} cy={y + ry * 0.3} rx={R * 0.93} ry={ry * 0.95} n={t.n * 2 + 5} h={d * 1.05} dir={1} lw={0.7} amax={74} />
      {[-52, -18, 18, 52].map(a => {
        const x = CX + R * 0.78 * Math.sin(rad(a)), yy = y + (ry + d) * 0.78 * Math.cos(rad(a)) - 2
        return <path key={a} d={curl(x, yy, 4.2 * s, -Math.PI / 2, a > 0 ? 1 : -1)} fill="none" stroke={ENG} strokeWidth="0.9" strokeOpacity="0.75" />
      })}
      <path d={under} fill="url(#gdCupShade)" />
      <ellipse cx={CX} cy={y} rx={R} ry={ry} fill="url(#gdDish)" />
      <ellipse cx={CX} cy={y} rx={R * 0.82} ry={ry * 0.82} fill="url(#gdWarm)" />
      <ellipse cx={CX} cy={y + 0.6} rx={R * 0.62} ry={ry * 0.6} fill="none" stroke={ENG} strokeWidth="0.8" strokeOpacity="0.45" />
      <ellipse cx={CX} cy={y + 1.6} rx={R * 0.62} ry={ry * 0.6} fill="none" stroke={HI} strokeWidth="0.6" strokeOpacity="0.4" />
      <ellipse cx={CX} cy={y} rx={R} ry={ry} fill="none" stroke={HI} strokeWidth="1.1" strokeOpacity="0.8" />
      <path d={`M${CX - R} ${y + 1.5} A${R} ${ry} 0 0 0 ${CX + R} ${y + 1.5}`} fill="none" stroke={ENG} strokeWidth="1" strokeOpacity="0.7" />
      <Beads cx={CX} cy={y + 2.6} rx={R} ry={ry} r={1.5 * s} n={t.n * 7} from={-88} to={88} />
      <ellipse cx={CX} cy={y + 1} rx={f(17 * s)} ry={f(5.5 * s)} fill="url(#gdBall)" />
      <PetalBand cx={CX} cy={y + 2} rx={15 * s} ry={4.5 * s} n={7} h={13 * s} dir={-1} lw={0.6} fill="url(#gdPetal)" />
      {cups.map((c, i) => <Cup key={i} {...c} />)}
    </g>
  )
}

const stops = list => list.map(([o, c, a = 1]) => <stop key={o} offset={o} stopColor={c} stopOpacity={a} />)

export function GangaAartiDeepakArt({ threeRows = true }) {
  const HTOP = 552, HBOT = 760
  // Handle outline: slight swell, waisted under the collar, flaring into the knob.
  const handle = `M${CX - 13} ${HTOP} C${CX - 16} ${HTOP + 70} ${CX - 16.5} ${HBOT - 70} ${CX - 14} ${HBOT} L${CX + 14} ${HBOT} ` +
    `C${CX + 16.5} ${HBOT - 70} ${CX + 16} ${HTOP + 70} ${CX + 13} ${HTOP}Z`
  const grooves = []
  for (let y = HTOP - 10; y < HBOT; y += 8.5) grooves.push(y)
  return (
    <svg className="gd-art" viewBox={`0 0 ${GD_W} ${GD_H}`} aria-hidden="true" focusable="false">
      <defs>
        {/* turned cylinder: dark rims, a hot highlight left of centre, warm bounce on the right */}
        <linearGradient id="gdCyl" x1="0" y1="0" x2="1" y2="0">
          {stops([[0, '#2b1702'], [0.1, '#6e4510'], [0.27, '#c8923a'], [0.38, '#ffe6a2'], [0.46, '#f0c464'], [0.62, '#a8701c'], [0.8, '#5e3b08'], [0.92, '#8a5a16'], [1, '#2e1a03']])}
        </linearGradient>
        <linearGradient id="gdTop" x1="0" y1="0" x2="0" y2="1">
          {stops([[0, '#8a5a14'], [0.35, '#e4b456'], [0.7, '#fff0bf'], [1, '#c48c2e']])}
        </linearGradient>
        <radialGradient id="gdBowl" cx="0.36" cy="0.28" r="0.85">
          {stops([[0, '#fff3c8'], [0.28, '#eab955'], [0.6, '#a66d1b'], [1, '#3d2304']])}
        </radialGradient>
        {/* inside of a dish seen from above: far wall lit by the flames, dark well, bright near lip */}
        <linearGradient id="gdDish" x1="0" y1="0" x2="0" y2="1">
          {stops([[0, '#f7d684'], [0.3, '#c18a30'], [0.62, '#6e460c'], [0.86, '#9a6a1c'], [1, '#f2cf7c']])}
        </linearGradient>
        <radialGradient id="gdPetal" cx="0.4" cy="0.2" r="0.9">
          {stops([[0, '#ffe7a0'], [0.5, '#cf9a3e'], [1, '#6b430c']])}
        </radialGradient>
        <linearGradient id="gdCupShade" x1="0" y1="0" x2="1" y2="0">
          {stops([[0, '#1e1001', 0.55], [0.22, '#1e1001', 0], [0.75, '#1e1001', 0], [1, '#1e1001', 0.6]])}
        </linearGradient>
        <radialGradient id="gdBall" cx="0.38" cy="0.3" r="0.75">
          {stops([[0, '#fff6d2'], [0.3, '#e9b552'], [0.72, '#8c5a12'], [1, '#2e1903']])}
        </radialGradient>
        <radialGradient id="gdBead" cx="0.35" cy="0.3" r="0.7">
          {stops([[0, '#fffbe2'], [0.4, '#e2b04e'], [1, '#4a2c05']])}
        </radialGradient>
        <radialGradient id="gdOil" cx="0.5" cy="0.4" r="0.6">
          {stops([[0, '#8a4e12'], [0.45, '#4a2304'], [1, '#1c0b00']])}
        </radialGradient>
        {/* the flame's own warm light on the lip and the cup's upper surfaces */}
        <radialGradient id="gdWarm" cx="0.5" cy="0.35" r="0.5">
          {stops([[0, '#ffc970', 0.55], [0.5, '#ffb24a', 0.22], [1, '#ff9a30', 0]])}
        </radialGradient>
        <linearGradient id="gdWick" x1="0" y1="1" x2="0" y2="0">
          {stops([[0, '#efe3c4'], [0.35, '#bba886'], [0.7, '#3a2716'], [1, '#120a05']])}
        </linearGradient>
        <linearGradient id="gdHandleShade" x1="0" y1="0" x2="0" y2="1">
          {stops([[0, '#ffcb75', 0.28], [0.25, '#ffcb75', 0], [1, '#000', 0.18]])}
        </linearGradient>
        <clipPath id="gdHandleClip"><path d={handle} /></clipPath>
      </defs>

      {/* ribbed central column */}
      <rect x={CX - 7.5} y="140" width="15" height={threeRows ? 216 : 320} fill="url(#gdCyl)" />
      {[-4.5, -1.5, 1.5, 4.5].map(dx => <line key={dx} x1={CX + dx} y1="140" x2={CX + dx} y2={threeRows ? 356 : 460} stroke={ENG} strokeWidth="0.6" strokeOpacity="0.45" />)}
      <Knop cx={CX} y={252} rx={13} ry={10} />
      <Collar cx={CX} y={239} rx={9} ry={2.6} h={3} />
      <Collar cx={CX} y={262} rx={9} ry={2.6} h={3} />
      {!threeRows && <>
      <Knop cx={CX} y={358} rx={14} ry={9} />
      <Collar cx={CX} y={346} rx={10} ry={3} h={3.4} />
      <Collar cx={CX} y={367} rx={10} ry={3} h={3.4} />
      </>}

      {/* inverted-lotus bowl under the lowest ring, then the collar and the twisted handle */}
      <g transform={threeRows ? 'translate(0 -104)' : undefined}>
      <path d={`M${CX - 64} 452 C${CX - 62} 486 ${CX - 30} 512 ${CX - 16} 524 L${CX + 16} 524 C${CX + 30} 512 ${CX + 62} 486 ${CX + 64} 452Z`} fill="url(#gdBowl)" />
      <PetalBand cx={CX} cy={455} rx={60} ry={12} n={11} h={36} dir={1} lw={0.8} />
      <PetalBand cx={CX} cy={478} rx={38} ry={8} n={9} h={30} dir={1} lw={0.7} fill="url(#gdPetal)" />
      <path d={`M${CX - 64} 452 C${CX - 62} 486 ${CX - 30} 512 ${CX - 16} 524 L${CX + 16} 524 C${CX + 30} 512 ${CX + 62} 486 ${CX + 64} 452Z`} fill="url(#gdCupShade)" />
      <Collar cx={CX} y={448} rx={64} ry={13} h={5} beads={31} />
      <Collar cx={CX} y={522} rx={19} ry={5} h={7} beads={11} />
      {/* leaf brackets at the neck of the handle */}
      {[-1, 1].map(d => (
        <g key={d}>
          <path d={`M${CX + d * 12} 532 C${CX + d * 30} 536 ${CX + d * 38} 552 ${CX + d * 30} 566 C${CX + d * 25} 575 ${CX + d * 15} 568 ${CX + d * 20} 560 ` +
            `C${CX + d * 24} 554 ${CX + d * 20} 546 ${CX + d * 12} 548Z`} fill="url(#gdPetal)" stroke={ENG} strokeWidth="0.8" />
          <path d={curl(CX + d * 24.5, 562, 5, d > 0 ? Math.PI : 0, d)} fill="none" stroke={ENG} strokeWidth="0.8" strokeOpacity="0.8" />
          <path d={`M${CX + d * 14} 538 C${CX + d * 26} 542 ${CX + d * 31} 551 ${CX + d * 30} 559`} fill="none" stroke={HI} strokeWidth="0.8" strokeOpacity="0.6" />
        </g>
      ))}
      <Collar cx={CX} y={530} rx={13} ry={3.6} h={20} />
      <path d={`M${CX - 13} 538 L${CX + 13} 538 M${CX - 13} 544 L${CX + 13} 544`} stroke={ENG} strokeWidth="0.8" strokeOpacity="0.6" />
      <path d={handle} fill="url(#gdCyl)" />
      <g clipPath="url(#gdHandleClip)">
        {grooves.map(y => (
          <g key={y}>
            <path d={`M${CX - 18} ${y} Q${CX} ${y + 7} ${CX + 18} ${y + 12}`} fill="none" stroke="#2a1602" strokeWidth="2.2" strokeOpacity="0.55" />
            <path d={`M${CX - 18} ${y + 2.4} Q${CX} ${y + 9.4} ${CX + 18} ${y + 14.4}`} fill="none" stroke={HI} strokeWidth="0.9" strokeOpacity="0.45" />
          </g>
        ))}
        <rect x={CX - 20} y={HTOP} width="40" height={HBOT - HTOP} fill="url(#gdHandleShade)" />
      </g>
      <Collar cx={CX} y={612} rx={16.5} ry={4} h={6} beads={9} />
      <Collar cx={CX} y={704} rx={16.5} ry={4} h={6} beads={9} />
      {/* knob end with a small bud finial */}
      <Collar cx={CX} y={HBOT - 4} rx={17} ry={4.5} h={6} />
      <ellipse cx={CX} cy={HBOT + 22} rx="21" ry="20" fill="url(#gdBall)" />
      <PetalBand cx={CX} cy={HBOT + 8} rx={17} ry={4.5} n={9} h={13} dir={1} lw={0.6} fill="url(#gdPetal)" />
      <path d={`M${CX - 7} ${HBOT + 40} C${CX - 6} ${HBOT + 46} ${CX - 2} ${HBOT + 48} ${CX} ${HBOT + 46 + 6} C${CX + 2} ${HBOT + 48} ${CX + 6} ${HBOT + 46} ${CX + 7} ${HBOT + 40}Z`} fill="url(#gdBowl)" />
      </g>

      {/* crown: dhoop kalash with lotus base and its own wick */}
      <Collar cx={CX} y={150} rx={11} ry={3} h={4} beads={9} />
      <path d={`M${CX - 12} 107 C${CX - 30} 116 ${CX - 30} 140 ${CX - 10} 150 L${CX + 10} 150 C${CX + 30} 140 ${CX + 30} 116 ${CX + 12} 107Z`} fill="url(#gdBowl)" />
      <PetalBand cx={CX} cy={148} rx={18} ry={5} n={9} h={20} dir={-1} lw={0.6} fill="url(#gdPetal)" />
      <path d={`M${CX - 12} 107 C${CX - 30} 116 ${CX - 30} 140 ${CX - 10} 150 L${CX + 10} 150 C${CX + 30} 140 ${CX + 30} 116 ${CX + 12} 107Z`} fill="url(#gdCupShade)" />
      <Beads cx={CX} cy={123} rx={23} ry={5} r={1.4} n={13} from={-80} to={80} />
      <Collar cx={CX} y={101} rx={15} ry={4.2} h={4} />
      <ellipse cx={CX} cy={101} rx={11.5} ry={3} fill="url(#gdOil)" />
      <ellipse cx={CX} cy={100} rx={24} ry={14} fill="url(#gdWarm)" />
      <path d={`M${CX - 1.8} 101.6 C${CX - 1.9} 98 ${CX - 1} 95.4 ${CX + 0.2} 93.6 C${CX + 1.2} 95.4 ${CX + 1.9} 98 ${CX + 1.8} 101.6Z`} fill="url(#gdWick)" />

      {/* tiers, top down, each over the one above's lower parts */}
      {TIERS.slice(0, threeRows ? 2 : 3).map((t, i) => <Tier key={i} t={t} ti={i} />)}
    </svg>
  )
}

// Fraction-only pseudo-random per flame, so every flame keeps its own rhythm on every load.
const fr = (i, m) => ((i + 1) * m) % 1
const FLAMES = GD_WICKS.map((w, i) => ({
  ...w,
  style: {
    left: w.x, top: w.y, '--k': w.k, '--sk': `${((fr(i, 0.447) - 0.5) * 7).toFixed(1)}deg`,
    '--t1': `${(1.31 + fr(i, 0.618) * 0.9).toFixed(2)}s`, '--d1': `${(-fr(i, 0.381) * 3).toFixed(2)}s`,
    '--t2': `${(0.53 + fr(i, 0.414) * 0.34).toFixed(2)}s`, '--d2': `${(-fr(i, 0.732) * 2).toFixed(2)}s`,
    '--t3': `${(0.97 + fr(i, 0.236) * 0.6).toFixed(2)}s`, '--d3': `${(-fr(i, 0.577) * 2).toFixed(2)}s`,
    '--t4': `${(3.7 + fr(i, 0.302) * 3.1).toFixed(2)}s`, '--d4': `${(-fr(i, 0.866) * 6).toFixed(2)}s`,
  },
})).sort((a, b) => a.y - b.y)

// Flame layers (40 x 100 box, wick at 20,93): orange envelope, yellow body, faint blue base, near-white core.
function FlameDefs() {
  return (
    <svg className="gd-defs" width="0" height="0" aria-hidden="true" focusable="false">
      <defs>
        {/* lit from inside: white-hot core low down, yellow body, orange only at the edges and the tip */}
        <linearGradient id="gdFlOut" x1="0" y1="1" x2="0" y2="0">
          {stops([[0, '#ff9a3c', 0], [0.12, '#ffab48', 0.5], [0.45, '#ff9a2e', 0.7], [0.8, '#f2731c', 0.5], [1, '#e0500c', 0]])}
        </linearGradient>
        <linearGradient id="gdFlBody" x1="0" y1="1" x2="0" y2="0">
          {stops([[0, '#ffe6a0', 0.15], [0.12, '#fff0b8', 0.9], [0.42, '#ffdc6a'], [0.75, '#ffb53c', 0.9], [1, '#ff9a2a', 0]])}
        </linearGradient>
        <radialGradient id="gdFlCore" cx="0.5" cy="0.6" r="0.55">
          {stops([[0, '#ffffff'], [0.5, '#fffdf0'], [0.8, '#fff4c4', 0.8], [1, '#ffe48a', 0]])}
        </radialGradient>
        <radialGradient id="gdFlBlue" cx="0.5" cy="0.55" r="0.5">
          {stops([[0, '#6f8cff', 0.42], [0.55, '#7a86f0', 0.22], [1, '#8a80e0', 0]])}
        </radialGradient>
        <filter id="gdBlur2" x="-80%" y="-30%" width="260%" height="160%"><feGaussianBlur stdDeviation="2.6" /></filter>
        <filter id="gdBlur1" x="-80%" y="-30%" width="260%" height="160%"><feGaussianBlur stdDeviation="1.3" /></filter>
      </defs>
    </svg>
  )
}
const FL_OUT = 'M20 0 C21.5 14 33 36 33 60 C33 80 27.5 95 20 95 C12.5 95 7 80 7 60 C7 36 18.5 14 20 0Z'
const FL_BODY = 'M20 9 C21.8 22 29.5 42 29.5 63 C29.5 80 25.5 91 20 91 C14.5 91 10.5 80 10.5 63 C10.5 42 18.2 22 20 9Z'
const FL_CORE = 'M20 32 C22 44 26.5 57 26.5 69 C26.5 80 23.8 86.5 20 86.5 C16.2 86.5 13.5 80 13.5 69 C13.5 57 18 44 20 32Z'

function Flame({ style }) {
  return (
    <div className="gd-fl" style={style}>
      <i className="gd-halo" />
      <div className="gd-fa">
        <div className="gd-fb">
          <svg viewBox="0 0 40 100" className="gd-flsvg">
            <path d={FL_OUT} fill="url(#gdFlOut)" filter="url(#gdBlur2)" />
            <path d={FL_BODY} fill="url(#gdFlBody)" filter="url(#gdBlur1)" />
            <ellipse cx="20" cy="89.5" rx="7.5" ry="6" fill="url(#gdFlBlue)" />
          </svg>
          <svg viewBox="0 0 40 100" className="gd-flsvg gd-core">
            <path d={FL_CORE} fill="url(#gdFlCore)" filter="url(#gdBlur1)" />
          </svg>
        </div>
      </div>
      <i className="gd-tip" />
    </div>
  )
}

// The lamp at width `size` (height size * GD_H / GD_W). The parent may set --lean (deg, flames lean together about
// their wicks) and --fx / --fy (flame stretch) on any ancestor.
export default function GangaAartiDeepak({ size = 200, className = '', style, threeRows = true }) {
  const anchor = threeRows ? GD_THREE_ROW_ANCHOR : GD_ANCHOR
  return (
    <div className={`gd ${className}`} data-deepak-rows={threeRows ? 3 : 4} style={{ width: size, height: size * GD_H / GD_W, '--gd-k': size / GD_W, ...style }} aria-hidden="true">
      <FlameDefs />
      <div className="gd-stage">
        <i className="gd-glow" style={{ left: anchor.x, top: anchor.y - 40 }} />
        <img className="gd-body" src={threeRows ? ASSET_BASE + 'landing/ganga-deepak-three-row@2x.png' : ASSET_BASE + 'landing/ganga-deepak@2x.webp'} alt="" draggable="false" />
        {FLAMES.slice(0, threeRows ? 9 : FLAMES.length).map((fl, i) => <Flame key={i} style={fl.style} />)}
      </div>
    </div>
  )
}
