const ASSET_BASE = '/vendor/login/'
// Antique carved gilt frame (owner 13-10-2026). Mitred moulding strips carry the profile (outer gadrooned
// torus, acanthus/palmette cove, pearl row, gilt sight lip); corner ornaments and mid-side cartouches cover
// the mitres. All gold is drawn here, no image assets.

const f = n => +n.toFixed(1)
const cubic = (p, t) => {
  const u = 1 - t
  return [0, 1].map(i => u * u * u * p[0][i] + 3 * u * u * t * p[1][i] + 3 * u * t * t * p[2][i] + t * t * t * p[3][i])
}
// Lobed acanthus leaf along a cubic spine: scalloped upper edge, plainer under edge.
function leaf(p, w, lobes, under = 0.65) {
  const top = [], bot = []
  const N = 64
  for (let i = 0; i <= N; i++) {
    const t = i / N
    const a = cubic(p, Math.max(0, t - 0.01)), b = cubic(p, Math.min(1, t + 0.01))
    const dx = b[0] - a[0], dy = b[1] - a[1], L = Math.hypot(dx, dy) || 1
    const nx = -dy / L, ny = dx / L
    const [x, y] = cubic(p, t)
    const env = Math.pow(Math.sin(Math.PI * Math.min(1, t * 1.08)), 0.75) * (1 - 0.35 * t)
    const ww = w * env * (0.62 + 0.38 * Math.abs(Math.cos(lobes * Math.PI * t)))
    top.push(`${f(x + nx * ww)} ${f(y + ny * ww)}`)
    bot.push(`${f(x - nx * w * env * under)} ${f(y - ny * w * env * under)}`)
  }
  return `M${top.join(' L')} L${bot.reverse().join(' L')}Z`
}
// Spine line for a leaf (the carved midrib highlight).
const spine = p => `M${p[0].join(' ')} C${p.slice(1).map(q => q.join(' ')).join(' ')}`
function spiral(cx, cy, r0, a0, turns, dir = 1) {
  const pts = []
  const N = Math.round(turns * 40)
  for (let i = 0; i <= N; i++) {
    const th = a0 + dir * (i / N) * turns * 2 * Math.PI
    const r = r0 * (1 - 0.8 * i / N)
    pts.push(`${f(cx + r * Math.cos(th))} ${f(cy + r * Math.sin(th))}`)
  }
  return `M${pts.join(' L')}`
}
// Scallop shell: lobes fanning out from a hinge.
function shell(hx, hy, R, mid, spread, n) {
  const lobes = []
  for (let i = 0; i < n; i++) {
    const a = (mid - spread / 2 + (spread * i) / (n - 1)) * Math.PI / 180
    const d = (spread / (n - 1) / 2 + 1.5) * Math.PI / 180
    const r = R * (0.86 + 0.14 * Math.cos((i - (n - 1) / 2) / ((n - 1) / 2) * Math.PI / 2))
    const pl = [hx + r * Math.cos(a - d), hy + r * Math.sin(a - d)]
    const pr = [hx + r * Math.cos(a + d), hy + r * Math.sin(a + d)]
    const pt = [hx + (r + 10) * Math.cos(a), hy + (r + 10) * Math.sin(a)]
    lobes.push({
      d: `M${hx} ${hy} L${f(pl[0])} ${f(pl[1])} Q${f(pt[0])} ${f(pt[1])} ${f(pr[0])} ${f(pr[1])}Z`,
      rib: `M${f(hx + 12 * Math.cos(a))} ${f(hy + 12 * Math.sin(a))} L${f(hx + r * 0.95 * Math.cos(a))} ${f(hy + r * 0.95 * Math.sin(a))}`,
    })
  }
  return lobes
}

// One carved piece: dark under-shadow, gilt body, highlight ridge.
const Carve = ({ d, rib, sw = 1 }) => (
  <g>
    <path d={d} transform="translate(1.6 2.2)" fill="#2e1b02" opacity="0.5" />
    <path d={d} fill="url(#gfGold)" stroke="#4a2e05" strokeWidth={sw} strokeLinejoin="round" />
    {rib && <path d={rib} fill="none" stroke="#fff3c6" strokeOpacity="0.75" strokeWidth="1.4" strokeLinecap="round" />}
  </g>
)
const Volute = ({ d, w = 6 }) => (
  <g fill="none" strokeLinecap="round" strokeLinejoin="round">
    <path d={d} transform="translate(1.4 2)" stroke="#2e1b02" strokeOpacity="0.5" strokeWidth={w + 1} />
    <path d={d} stroke="#4a2e05" strokeWidth={w + 1.6} />
    <path d={d} stroke="url(#gfGoldH)" strokeWidth={w} />
    <path d={d} stroke="#fff5cf" strokeOpacity="0.7" strokeWidth={w * 0.25} transform="translate(-0.6 -0.8)" />
  </g>
)

// Corner art: corner of the frame at (0,0), moulding 80 units thick along both edges.
const ARM = [[62, 60], [98, 18], [150, 52], [198, 30]]
const ARM2 = [[80, 70], [112, 50], [150, 70], [178, 62]]
const CORNER = (() => {
  const s = shell(58, 58, 74, 225, 120, 11)
  return { s, arm: leaf(ARM, 17, 4.5), arm2: leaf(ARM2, 9, 3) }
})()
function CornerArt() {
  const { s, arm, arm2 } = CORNER
  const mirror = 'matrix(0 1 1 0 0 0)'
  return (
    <g>
      {[null, mirror].map((m, i) => (
        <g key={i} transform={m || undefined}>
          <Carve d={arm2} rib={spine(ARM2)} />
          <Carve d={arm} rib={spine(ARM)} />
          <Volute d={spiral(196, 22, 13, Math.PI * 0.5, 1.6, -1)} />
          <Volute d={spiral(104, 34, 9, Math.PI, 1.3, 1)} w={4.5} />
        </g>
      ))}
      {s.map((l, i) => <Carve key={i} d={l.d} rib={l.rib} sw={0.9} />)}
      <Carve d={leaf([[70, 70], [82, 78], [90, 86], [102, 102]], 10, 2)} />
      <circle cx="58" cy="58" r="16" fill="url(#gfBoss)" stroke="#4a2e05" strokeWidth="1.2" />
    </g>
  )
}

// Mid-side cartouche: outer edge at y=0, centred on x=0.
const CART_ARM = [[10, 56], [42, 22], [80, 62], [118, 40]]
const CART = { s: shell(0, 64, 56, 270, 140, 13), arm: leaf(CART_ARM, 15, 4) }
function CartoucheArt() {
  return (
    <g>
      {[1, -1].map(sx => (
        <g key={sx} transform={`scale(${sx} 1)`}>
          <Carve d={CART.arm} rib={spine(CART_ARM)} />
          <Volute d={spiral(120, 30, 12, Math.PI * 0.5, 1.6, -1)} />
          <Volute d={spiral(30, 62, 8, -Math.PI * 0.5, 1.3, 1)} w={4.5} />
        </g>
      ))}
      {CART.s.map((l, i) => <Carve key={i} d={l.d} rib={l.rib} sw={0.9} />)}
      <circle cx="0" cy="64" r="13" fill="url(#gfBoss)" stroke="#4a2e05" strokeWidth="1.2" />
    </g>
  )
}

// Repeating band tiles (data URIs, each its own tiny SVG document).
const uri = svg => `data:image/svg+xml,${encodeURIComponent(svg)}`
const TILE_GOLD = `<linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff2bf"/><stop offset=".35" stop-color="#e8c25e"/><stop offset=".7" stop-color="#b07e22"/><stop offset="1" stop-color="#6a450a"/></linearGradient>`
const PALMETTE = (() => {
  const lobe = (L, w, a) => `<path transform="translate(18 44) rotate(${a})" d="M0 0 C ${-w * 1.3} ${-L * 0.35} ${-w} ${-L * 0.8} 0 ${-L} C ${w} ${-L * 0.8} ${w * 1.3} ${-L * 0.35} 0 0Z"/>`
  const lobes = [[24, 5, -62], [32, 5.5, -31], [40, 6, 0], [32, 5.5, 31], [24, 5, 62]].map(([L, w, a]) => lobe(L, w, a)).join('')
  const scroll = `<path d="M30 44 C 40 44 44 38 44 30 C 44 22 36 22 36 28 C 36 33 42 33 42 29" fill="none" stroke-width="3.2" stroke-linecap="round"/><path d="M68 44 C 58 44 54 38 54 30 C 54 22 62 22 62 28 C 62 33 56 33 56 29" fill="none" stroke-width="3.2" stroke-linecap="round"/><path d="M30 44 L68 44" fill="none" stroke-width="2.4"/>`
  const art = lobes + scroll
  return uri(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 48" width="72" height="48"><defs>${TILE_GOLD}</defs>` +
    `<g transform="translate(1.3 1.8)" fill="#2e1b02" stroke="#2e1b02" fill-opacity=".45" stroke-opacity=".45">${art}</g>` +
    `<g fill="url(#g)" stroke="#4a2e05" stroke-width=".9">${lobes}</g><g stroke="#4a2e05">${scroll.replaceAll('stroke-width="3.2"', 'stroke-width="4.4"').replaceAll('stroke-width="2.4"', 'stroke-width="3.6"')}</g><g stroke="url(#g)">${scroll}</g>` +
    `<g stroke="#fff6d2" stroke-opacity=".7" stroke-width="1" fill="none"><path d="M18 40 L18 8"/><path d="M18 40 L4 22"/><path d="M18 40 L32 22"/></g></svg>`)
})()
const GADROON = uri(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 16" width="12" height="16"><defs><linearGradient id="h" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fff6d0" stop-opacity=".85"/><stop offset=".45" stop-color="#ffe9a8" stop-opacity=".1"/><stop offset="1" stop-color="#3a2302" stop-opacity=".6"/></linearGradient></defs><path d="M-2 16 C 1 9 6 2 10 0 L14 0 C 11 7 6 14 2 16Z" fill="url(#h)" stroke="#3a2302" stroke-opacity=".55" stroke-width=".7"/></svg>`)
const PEARL = uri(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 12" width="14" height="12"><defs><radialGradient id="p" cx=".35" cy=".3" r=".75"><stop offset="0" stop-color="#fff8d8"/><stop offset=".45" stop-color="#e2b54a"/><stop offset="1" stop-color="#5e3d06"/></radialGradient></defs><ellipse cx="7.6" cy="6.8" rx="5.4" ry="4.6" fill="#2e1b02" opacity=".45"/><ellipse cx="7" cy="6" rx="5.2" ry="4.5" fill="url(#p)"/></svg>`)


// The whole frame is one SVG in frame units (800 x 1189.4, moulding T thick). Only the RIGHT half is drawn
// (half top strip, right strip, half bottom strip, two corners, the right cartouche); the left half is the
// same group mirrored, so both sides are identical by construction (owner 13-10-2026: left side broke).
const W = 800, H = 1189.4, T = 90.72, S = T / 80
const BANDS = [
  { id: 'gfPatG', href: GADROON, y: 0.04, h: 0.15, ar: 12 / 16, c: 0.5 },
  { id: 'gfPatP', href: PALMETTE, y: 0.245, h: 0.45, ar: 72 / 48, c: 0.25 },   // palmette axis at 18/72 of the tile
  { id: 'gfPatR', href: PEARL, y: 0.73, h: 0.12, ar: 14 / 12, c: 0.5 },
]
const PROFILE = [[0, '#2a1902'], [1.6, '#6e4a0c'], [5, '#d9b155'], [8.5, '#fff0b8'], [13, '#c99632'], [18, '#6a440a'],
  [20.5, '#3a2403'], [23, '#6e4a0e'], [31, '#a97a26'], [41, '#d2a648'], [53, '#b8892f'], [65, '#7a5212'], [70.5, '#3e2704'],
  [73, '#8f6619'], [79, '#d9ab4a'], [85.5, '#6e4a0e'], [87, '#fbe8a6'], [91, '#e2b752'], [96, '#9a6a16'], [100, '#241602']]

// A moulding strip in local units: runs along x from a to b, outer edge at y=0, sight edge at y=T, mitred at b
// (and at a when both). Band tiles are centred on x=0, the middle of the side, so mirrored halves meet cleanly.
function Strip({ id, a, b, both, transform, light }) {
  const pts = `${a},0 ${b},0 ${b - T},${T} ${both ? a + T : a},${T}`
  return (
    <g transform={transform}>
      <clipPath id={id}><polygon points={pts} /></clipPath>
      <g clipPath={`url(#${id})`}>
        <rect x={a} y="0" width={b - a} height={T} fill="url(#gfProfile)" />
        {BANDS.map(bd => <rect key={bd.id} x={a} y={bd.y * T} width={b - a} height={bd.h * T} fill={`url(#${bd.id})`} />)}
        <rect x={a} y="0" width={b - a} height={T} fill={light > 0 ? '#fff6d8' : '#1a0f00'} opacity={Math.abs(light)} />
      </g>
    </g>
  )
}

// THE FRAME IS SHOWN AS A BAKED IMAGE (owner 13-10-2026, screenshot of broken strips around the frame). Chrome's GPU
// rasteriser misplaced tiles of the band patterns (patterned <image> fills under rotate/mirror + a CSS shadow), leaving
// fragments of moulding beside and inside the frame. A plain <img> cannot do that. GoldFrameArt below is the source:
// open /__lab/frame?art and run scripts/bake_frame.sh to regenerate public/landing/frame@2x.webp after changing it.
// The image is the frame box plus a margin M (frame units) on every side for the corner shells and the shadow.
export const FRAME_W = W, FRAME_H = H, FRAME_M = 120

export default function GoldFrame({ children, sightRef }) {
  return (
    <div className="gf">
      <div className="gf-sight" ref={sightRef}>
        {children}
        <div className="gf-sight-shade" />
      </div>
      <img className="gf-img" src={ASSET_BASE + "landing/frame@2x.webp"} alt="" aria-hidden="true" draggable="false" />
    </div>
  )
}

export function GoldFrameArt() {
  return (
      <svg className="gf-art" viewBox={`0 0 ${W} ${H}`} aria-hidden="true" focusable="false">
        <defs>
          <linearGradient id="gfGold" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#fff1c0" />
            <stop offset="0.26" stopColor="#e6bf5c" />
            <stop offset="0.6" stopColor="#aa771e" />
            <stop offset="1" stopColor="#583805" />
          </linearGradient>
          <linearGradient id="gfGoldH" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#fbe7a0" />
            <stop offset="0.5" stopColor="#d9aa45" />
            <stop offset="1" stopColor="#8c5f12" />
          </linearGradient>
          <radialGradient id="gfBoss" cx="0.38" cy="0.32" r="0.75">
            <stop offset="0" stopColor="#fff7d4" />
            <stop offset="0.45" stopColor="#e2b04a" />
            <stop offset="1" stopColor="#6a450a" />
          </radialGradient>
          <linearGradient id="gfProfile" x1="0" y1="0" x2="0" y2="1">
            {PROFILE.map(([o, c]) => <stop key={o} offset={o / 100} stopColor={c} />)}
          </linearGradient>
          {BANDS.map(bd => {
            const h = bd.h * T, w = h * bd.ar
            return (
              <pattern key={bd.id} id={bd.id} patternUnits="userSpaceOnUse" x={-w * bd.c} y={bd.y * T} width={w} height={h}>
                <image href={bd.href} width={w} height={h} preserveAspectRatio="none" />
              </pattern>
            )
          })}
          <g id="gfHalf">
            <Strip id="gfClipT" a={-0.6} b={W / 2} transform={`translate(${W / 2} 0)`} light={0.07} />
            <Strip id="gfClipR" a={-H / 2} b={H / 2} both transform={`translate(${W} ${H / 2}) rotate(90)`} light={-0.05} />
            <Strip id="gfClipB" a={-0.6} b={W / 2} transform={`translate(${W / 2} ${H}) scale(1 -1)`} light={-0.14} />
            <g transform={`translate(${W} ${H / 2}) rotate(90) scale(${S})`}><CartoucheArt /></g>
            <g transform={`translate(${W} 0) scale(${-S} ${S})`}><CornerArt /></g>
            <g transform={`translate(${W} ${H}) scale(${-S} ${-S})`}><CornerArt /></g>
          </g>
        </defs>
        <use href="#gfHalf" />
        <use href="#gfHalf" transform={`matrix(-1 0 0 1 ${W} 0)`} />
        <g transform={`translate(${W / 2} 0) scale(${S})`}><CartoucheArt /></g>
      </svg>
  )
}
