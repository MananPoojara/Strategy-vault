const ASSET_BASE = '/vendor/login/'
// Code-drawn flowers for the landing page (owner 13-10-2026): mogra, jasmine buds, champa, white rose,
// sevanti, lotus, lily, glossy leaves and loose petals. Every petal carries its own gradient (base tint to
// white tip), an edge-shading overlay and a soft shadow on the petal below; each kind is ONE <symbol> in
// FlowerDefs and the bed/shower only place <use> references. Layouts are seeded: same page on every load.

// mulberry32: small seeded PRNG.
function seeded(seed) {
  return () => {
    seed = (seed + 0x6D2B79F5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
const f1 = n => +n.toFixed(1)

// Petal outlines, base at the origin, pointing up (-y), length L, half-width w.
const rounded = (L, w) => `M0 0 C ${f1(-w * .5)} ${f1(-L * .12)} ${f1(-w * 1.08)} ${f1(-L * .45)} ${f1(-w * .98)} ${f1(-L * .72)} ` +
  `C ${f1(-w * .9)} ${f1(-L * .95)} ${f1(-w * .35)} ${f1(-L * 1.02)} 0 ${-L} C ${f1(w * .35)} ${f1(-L * 1.02)} ${f1(w * .9)} ${f1(-L * .95)} ${f1(w * .98)} ${f1(-L * .72)} ` +
  `C ${f1(w * 1.08)} ${f1(-L * .45)} ${f1(w * .5)} ${f1(-L * .12)} 0 0Z`
const pointed = (L, w) => `M0 0 C ${f1(-w * .9)} ${f1(-L * .15)} ${f1(-w * 1.1)} ${f1(-L * .5)} ${f1(-w * .7)} ${f1(-L * .75)} ` +
  `C ${f1(-w * .4)} ${f1(-L * .9)} ${f1(-w * .12)} ${f1(-L * .97)} 0 ${-L} C ${f1(w * .12)} ${f1(-L * .97)} ${f1(w * .4)} ${f1(-L * .9)} ${f1(w * .7)} ${f1(-L * .75)} ` +
  `C ${f1(w * 1.1)} ${f1(-L * .5)} ${f1(w * .9)} ${f1(-L * .15)} 0 0Z`
const spatula = (L, w) => `M0 0 C ${f1(-w * .6)} ${f1(-L * .2)} ${f1(-w)} ${f1(-L * .6)} ${f1(-w)} ${f1(-L * .88)} ` +
  `Q ${f1(-w)} ${-L} ${f1(-w * .3)} ${f1(-L * .99)} L 0 ${f1(-L * .955)} L ${f1(w * .3)} ${f1(-L * .99)} Q ${f1(w)} ${-L} ${f1(w)} ${f1(-L * .88)} ` +
  `C ${f1(w)} ${f1(-L * .6)} ${f1(w * .6)} ${f1(-L * .2)} 0 0Z`
const CHAMPA_PETAL = 'M0 0 C -13 -7 -22 -27 -12 -38 C -4 -46 14 -45 17 -33 C 21 -20 9 -7 0 0Z'

// One petal: shadow on what lies below, body, edge shading, optional crease/veins/curl.
function Petal({ d, a = 0, fill, stroke = '#d3c6a8', sh = 0.16, edge = 'url(#lpEdge)', children }) {
  return (
    <g transform={`rotate(${f1(a)})`}>
      {sh > 0 && <path d={d} transform="rotate(4) translate(.6 1.8)" fill="#5a4318" fillOpacity={sh} />}
      <path d={d} fill={fill} fillOpacity="0.96" stroke={stroke} strokeWidth="0.45" />
      {edge && <path d={d} fill={edge} />}
      {children}
    </g>
  )
}
// Soft shadow an inner ring casts on the ring outside it.
const RingShade = ({ r, o = 0.22 }) => <circle r={f1(r * 1.18)} fill="url(#lpRingShade)" opacity={o} />
const SHADOW = <ellipse cx="2" cy="8" rx="55" ry="52" fill="url(#lpShadow)" />

// --- flower builders ---------------------------------------------------------------------------------
function mogra(seed, grad) {
  const r = seeded(seed)
  const rings = [[9, 40, 13.5], [8, 31.5, 12], [7, 23, 10], [5, 14, 7.5]]
  return (
    <>
      {SHADOW}
      {rings.map(([n, L, w], k) => (
        <g key={k}>
          {k > 0 && <RingShade r={L} o={0.3} />}
          {Array.from({ length: n }, (_, i) => {
            const LL = L * (0.9 + r() * 0.18)
            return (
              <Petal key={i} d={rounded(LL, w * (0.9 + r() * 0.2))} a={(360 / n) * i + k * 19 + (r() - 0.5) * 14} fill={grad}>
                <path d={`M0 ${f1(-LL * .14)} C ${f1(r() - .5)} ${f1(-LL * .4)} ${f1(r() - .5)} ${f1(-LL * .6)} 0 ${f1(-LL * .78)}`}
                  fill="none" stroke="#bfae84" strokeOpacity="0.35" strokeWidth="0.6" />
              </Petal>
            )
          })}
        </g>
      ))}
      <circle r="5" fill="url(#lpMograEye)" />
    </>
  )
}

function rose(seed, grad) {
  const r = seeded(seed)
  const curl = (L, w) => (
    <>
      <path d={`M${f1(-w * .74)} ${f1(-L * .78)} Q 0 ${f1(-L * .99)} ${f1(w * .74)} ${f1(-L * .78)}`} fill="none" stroke="#a88f5e" strokeOpacity="0.28" strokeWidth="1.3" transform="translate(0 2.4)" />
      <path d={`M${f1(-w * .76)} ${f1(-L * .79)} Q 0 ${f1(-L * 1.0)} ${f1(w * .76)} ${f1(-L * .79)}`} fill="none" stroke="#fffefa" strokeOpacity="0.9" strokeWidth="1.5" strokeLinecap="round" />
    </>
  )
  const rings = [[6, 47, 24, 0], [5, 39, 21, 31], [5, 30, 16.5, 64], [4, 22, 12.5, 97]]
  // The cupped heart: overlapping crescent petal-edges wound in a spiral, each with its own shadow.
  const arcs = []
  for (let i = 0; i < 7; i++) {
    const rad = 13.5 * (1 - i / 8.2), a = (i * 137 + 20) * Math.PI / 180, sw = 1.05 - i * 0.04
    const p = (rr, t) => `${f1(rr * Math.cos(a + t))} ${f1(rr * Math.sin(a + t))}`
    const d = `M${p(rad, -sw)} A ${f1(rad)} ${f1(rad)} 0 0 1 ${p(rad, sw)} A ${f1(rad * 1.25)} ${f1(rad * 1.25)} 0 0 0 ${p(rad, -sw)}Z`
    arcs.push(
      <g key={i}>
        <path d={d} transform="translate(.6 1.4)" fill="#6d5326" fillOpacity="0.28" />
        <path d={d} fill="url(#lpRoseIn)" stroke="#cdbb93" strokeWidth="0.4" />
      </g>)
  }
  return (
    <>
      {SHADOW}
      {rings.map(([n, L, w, off], k) => (
        <g key={k}>
          {k > 0 && <RingShade r={L} o={0.38} />}
          {Array.from({ length: n }, (_, i) => {
            const LL = L * (0.92 + r() * 0.14), ww = w * (0.92 + r() * 0.14)
            return <Petal key={i} d={rounded(LL, ww)} a={(360 / n) * i + off + (r() - 0.5) * 16} fill={grad} sh={0.2}>{curl(LL, ww)}</Petal>
          })}
        </g>
      ))}
      <circle r="13.5" fill="url(#lpRoseCup)" />
      {arcs}
    </>
  )
}

function sevanti(seed, outer, inner) {
  const r = seeded(seed)
  const rings = [[34, 47, 3.3], [30, 41, 3.3], [26, 34, 3.2], [22, 27, 3], [18, 20, 2.7], [13, 13, 2.3], [9, 7.5, 2]]
  return (
    <>
      {SHADOW}
      {rings.map(([n, L, w], k) => (
        <g key={k}>
          {k > 0 && <RingShade r={L} o={0.32} />}
          {Array.from({ length: n }, (_, i) => (
            <Petal key={i} d={spatula(L * (0.88 + r() * 0.2), w * (0.85 + r() * 0.3))} a={(360 / n) * i + k * 7 + (r() - 0.5) * 7}
              fill={k < 3 ? outer : inner} stroke="#c9bf9c" sh={0} edge={k < 4 ? 'url(#lpEdge)' : null} />
          ))}
        </g>
      ))}
      <circle r="3.5" fill="#cfc77a" />
    </>
  )
}

function champa(seed) {
  const r = seeded(seed)
  return (
    <>
      {SHADOW}
      {Array.from({ length: 5 }, (_, i) => (
        <g key={i} transform={`scale(${f1(1.12 + r() * 0.1)})`}>
          <Petal d={CHAMPA_PETAL} a={i * 72 + (r() - 0.5) * 8} fill="url(#lpChampa)" stroke="#d9c9a2" sh={0.22}>
            <path d="M13 -38 C 18 -31 18 -21 11 -11" fill="none" stroke="#fffef8" strokeWidth="1.6" strokeLinecap="round" />
            <path d="M1 -4 C 3 -15 3 -27 2 -38 M1 -4 C -3 -14 -7 -24 -8 -33" fill="none" stroke="#d9a72a" strokeOpacity="0.28" strokeWidth="0.5" />
          </Petal>
        </g>
      ))}
      <circle r="3.6" fill="url(#lpChampaEye)" />
    </>
  )
}

function lotus(seed) {
  const r = seeded(seed)
  const veins = L => [-0.5, -0.25, 0, 0.25, 0.5].map(k => `M0 ${f1(-L * .06)} Q ${f1(k * L * .3)} ${f1(-L * .5)} ${f1(k * L * .14)} ${f1(-L * .9)}`).join(' ')
  const rings = [[8, 48, 13, 0], [8, 39, 11, 22.5], [6, 28, 9, 8]]
  return (
    <>
      {SHADOW}
      {rings.map(([n, L, w, off], k) => (
        <g key={k}>
          {k > 0 && <RingShade r={L} o={0.3} />}
          {Array.from({ length: n }, (_, i) => {
            const LL = L * (0.92 + r() * 0.12)
            return (
              <Petal key={i} d={pointed(LL, w * (0.92 + r() * 0.16))} a={(360 / n) * i + off + (r() - 0.5) * 9} fill="url(#lpLotus)" stroke="#e2b4c2" sh={0.18}>
                <path d={veins(LL)} fill="none" stroke="#e07f9e" strokeOpacity="0.3" strokeWidth="0.45" />
              </Petal>
            )
          })}
        </g>
      ))}
      {Array.from({ length: 18 }, (_, i) => (
        <path key={i} d="M0 -7 L0 -13" transform={`rotate(${i * 20 + 5})`} stroke="#e9b931" strokeWidth="1.3" strokeLinecap="round" />
      ))}
      <circle r="7.5" fill="url(#lpPod)" />
      {[[0, -3.5], [-3.2, 1.2], [3.2, 1.2], [0, 3.6], [-3.4, -2.4], [3.4, -2.4]].map(([x, y], i) => <circle key={i} cx={x} cy={y} r="0.95" fill="#8d8a2c" />)}
    </>
  )
}

function lily() {
  return (
    <>
      {SHADOW}
      {[0, 120, 240, 60, 180, 300].map((a, i) => {
        const L = i < 3 ? 48 : 44, w = i < 3 ? 12 : 10
        return (
          <Petal key={i} d={pointed(L, w)} a={a + 15} fill="url(#lpLily)" stroke="#cfc8aa" sh={0.2}>
            <path d={`M0 -4 L0 ${-L * .88}`} stroke="#b9c48c" strokeOpacity="0.6" strokeWidth="1.3" />
            <path d={`M1 -4 L1 ${-L * .86}`} stroke="#ffffff" strokeOpacity="0.7" strokeWidth="0.6" />
            <path d={`M${-w * .5} ${-L * .3} Q ${-w * .4} ${-L * .6} ${-w * .15} ${-L * .82} M${w * .5} ${-L * .3} Q ${w * .4} ${-L * .6} ${w * .15} ${-L * .82}`} fill="none" stroke="#c9c09c" strokeOpacity="0.35" strokeWidth="0.5" />
          </Petal>
        )
      })}
      {Array.from({ length: 6 }, (_, i) => (
        <g key={i} transform={`rotate(${i * 60 + 45})`}>
          <path d="M0 0 Q 2 -11 0 -21" fill="none" stroke="#c5d08e" strokeWidth="0.9" />
          <ellipse cy="-22.5" rx="1.9" ry="4.2" fill="#a2531a" />
        </g>
      ))}
      <path d="M0 0 L3 -16" stroke="#9cbc58" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="3" cy="-16.5" r="1.8" fill="#bcd36a" />
    </>
  )
}

function buds() {
  const bud = (a, s, k) => (
    <g key={k} transform={`rotate(${a}) scale(${s})`}>
      <path d="M0 46 L0 32" stroke="#5f8a3c" strokeWidth="2.2" strokeLinecap="round" />
      <path d="M0 34 C -12 22 -13 -8 0 -38 C 13 -8 12 22 0 34Z" fill="#4a3612" fillOpacity="0.16" transform="translate(1 2)" />
      <path d="M0 34 C -12 22 -13 -8 0 -38 C 13 -8 12 22 0 34Z" fill="url(#lpBud)" stroke="#cfc3a6" strokeWidth="0.5" />
      <path d="M0 34 C -12 22 -13 -8 0 -38 C 13 -8 12 22 0 34Z" fill="url(#lpEdge)" />
      <path d="M-3 -33 C 5 -22 -4 -8 3 8 M3 -35 C -4 -24 5 -12 -1 4" fill="none" stroke="#c8b7a2" strokeOpacity="0.45" strokeWidth="0.6" />
      <path d="M-4 -12 C -5 0 -4 14 -2 26" fill="none" stroke="#ffffff" strokeOpacity="0.8" strokeWidth="1.4" strokeLinecap="round" />
      <path d="M-7 26 C -6 33 6 33 7 26 C 5 31 -5 31 -7 26Z M-7 26 L -9 18 L -4 27Z M7 26 L 9 18 L 4 27Z M-1 27 L 0 19 L 1 27Z" fill="#6f9a44" />
      <path d="M-6 30 C -3 34 3 34 6 30 L 2 37 L -2 37Z" fill="#5c8636" />
    </g>
  )
  return (
    <>
      <ellipse cx="2" cy="8" rx="34" ry="44" fill="url(#lpShadow)" />
      {bud(-30, 0.82, 0)}{bud(28, 0.86, 1)}{bud(-8, 1, 2)}{bud(12, 0.62, 3)}
    </>
  )
}

function leaf(l, r, line) {
  return (
    <>
      <path d="M0 46 C -22 22 -20 -20 0 -48 C 20 -20 22 22 0 46Z" fill="#14360f" opacity="0.35" transform="translate(1.5 3)" />
      <path d="M0 46 C -22 22 -20 -20 0 -48 L0 46Z" fill={l} />
      <path d="M0 46 C 22 22 20 -20 0 -48 L0 46Z" fill={r} />
      <path d="M-4 30 C -14 14 -14 -14 -3 -36 C -8 -14 -8 12 -4 30Z" fill="#ffffff" opacity="0.17" />
      <path d="M0 46 C -22 22 -20 -20 0 -48 C 20 -20 22 22 0 46Z" fill="none" stroke={line} strokeWidth="0.7" />
      {[-24, -12, 0, 12, 24].map(y => (
        <path key={y} d={`M0 ${y + 8} Q -7 ${y + 1} -13 ${y - 8} M0 ${y + 8} Q 7 ${y + 1} 13 ${y - 8}`} fill="none" stroke="#a8d184" strokeOpacity="0.3" strokeWidth="0.7" />
      ))}
      <path d="M0 44 C 1 12 1 -20 0 -45" fill="none" stroke="#c4e09a" strokeOpacity="0.75" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M-0.6 40 C 0.4 10 0.4 -20 -0.6 -42" fill="none" stroke="#f3ffd8" strokeOpacity="0.4" strokeWidth="0.5" />
    </>
  )
}

const LG = (id, stops, extra = {}) => (
  <linearGradient id={id} x1="0.5" y1="1" x2="0.5" y2="0" {...extra}>
    {stops.map(([o, c, op = 1]) => <stop key={o} offset={o} stopColor={c} stopOpacity={op} />)}
  </linearGradient>
)
const RG = (id, stops, extra = {}) => (
  <radialGradient id={id} {...extra}>
    {stops.map(([o, c, op = 1]) => <stop key={o} offset={o} stopColor={c} stopOpacity={op} />)}
  </radialGradient>
)
const US = { gradientUnits: 'userSpaceOnUse', cx: 0, cy: 0, r: 48 }
const SYM = { viewBox: '-50 -50 100 100', overflow: 'visible' }

export function FlowerDefs({ palette = 'ivory' }) {
  return (
    <svg className="lp-defs" aria-hidden="true" width="0" height="0" focusable="false">
      <defs>
        {RG('lpShadow', [[0.4, '#3e2a0c', 0.5], [0.72, '#3e2a0c', 0.24], [1, '#4a3210', 0]])}
        {RG('lpRingShade', [[0.62, '#6b5326', 0], [0.82, '#6b5326', 0.55], [1, '#6b5326', 0]])}
        {/* across the petal: edges shaded, middle clear (bbox, so it turns with every petal) */}
        {LG('lpEdge', [[0, '#8f7746', 0.3], [0.24, '#8f7746', 0], [0.76, '#8f7746', 0], [1, '#8f7746', 0.3]], { x1: 0, y1: 0.5, x2: 1, y2: 0.5 })}
        {RG('lpMogra', [[0, '#dfdcb6'], [0.2, '#eeecd6'], [0.45, '#fbfaf2'], [0.8, '#ffffff'], [1, '#fbf8ef']], US)}
        {RG('lpMogra2', [[0, '#e3d7aa'], [0.22, '#f2ead2'], [0.5, '#fdf9ee'], [1, '#fffcf4']], US)}
        {RG('lpMograEye', [[0, '#cfc697'], [1, '#ece6c8']])}
        {RG('lpRose', [[0, '#d2c197'], [0.32, '#ebe1c8'], [0.62, '#faf6ec'], [1, '#fffefa']], US)}
        {RG('lpRose2', [[0, '#d8b9a4'], [0.35, '#f1dfd3'], [0.65, '#fcf3ec'], [1, '#fffbf7']], US)}
        {RG('lpRoseCup', [[0, '#b9a274'], [0.55, '#ddd0b0'], [1, '#f3ecdc']])}
        {RG('lpRoseIn', [[0, '#d9c9a2'], [0.7, '#f4ecda'], [1, '#fffdf6']], { ...US, r: 19 })}
        {LG('lpSev', [[0, '#c9cf8f'], [0.3, '#ecedd2'], [0.65, '#fbfbf3'], [1, '#ffffff']])}
        {LG('lpSevIn', [[0, '#c4bd5e'], [0.5, '#efeac2'], [1, '#fdfbf0']])}
        {LG('lpSev2', [[0, '#d8c27c'], [0.3, '#f0e6c4'], [0.7, '#fdf9ee'], [1, '#fffdf6']])}
        {LG('lpSev2In', [[0, '#d2b44e'], [0.5, '#f3e3a6'], [1, '#fdf6df']])}
        {RG('lpMarigold', [[0, '#b9560c'], [0.35, '#e88b19'], [0.72, '#f9bb40'], [1, '#ffe090']], US)}
        {RG('lpMarigoldIn', [[0, '#ad4a09'], [0.4, '#dd7510'], [0.8, '#f5a92c'], [1, '#ffd36a']], US)}
        {RG('lpChampa', [[0, '#e49b0c'], [0.14, '#f4c138'], [0.3, '#fce59a'], [0.48, '#fff9e6'], [0.78, '#fffefa'], [1, '#f6efdc']], US)}
        {RG('lpChampaEye', [[0, '#b97a06'], [1, '#e9a817']])}
        {LG('lpLotus', [[0, '#ee93ae'], [0.32, '#f7bfd0'], [0.68, '#fde7ee'], [1, '#ffffff']])}
        {LG('lpLily', [[0, '#b7d184'], [0.26, '#ecf3d6'], [0.55, '#fbfcf5'], [1, '#ffffff']])}
        {LG('lpBud', [[0, '#c9dc9f'], [0.22, '#f1f4e2'], [0.66, '#ffffff'], [0.9, '#fbedf1'], [1, '#efcfdc']])}
        {RG('lpPod', [[0, '#eee88f'], [1, '#b3ad3a']])}
        {LG('lpLeafL', [[0, '#2c6427'], [1, '#5f9f48']], { x1: 0, y1: 0.5, x2: 1, y2: 0.5 })}
        {LG('lpLeafR', [[0, '#3f7c33'], [1, '#1c461b']], { x1: 0, y1: 0.5, x2: 1, y2: 0.5 })}
        {LG('lpLeafL2', [[0, '#1f4f1f'], [1, '#4b8a3b']], { x1: 0, y1: 0.5, x2: 1, y2: 0.5 })}
        {LG('lpLeafR2', [[0, '#2f6a2b'], [1, '#143814']], { x1: 0, y1: 0.5, x2: 1, y2: 0.5 })}

        <symbol id="lf-mogra" {...SYM}>{mogra(11, 'url(#lpMogra)')}</symbol>
        <symbol id="lf-mogra2" {...SYM}>{mogra(23, 'url(#lpMogra2)')}</symbol>
        <symbol id="lf-bud" {...SYM}>{buds()}</symbol>
        <symbol id="lf-champa" {...SYM}>{champa(5)}</symbol>
        <symbol id="lf-rose" {...SYM}>{rose(7, 'url(#lpRose)')}</symbol>
        <symbol id="lf-rose2" {...SYM}>{rose(19, 'url(#lpRose2)')}</symbol>
        <symbol id="lf-sevanti" {...SYM}>{sevanti(3, 'url(#lpSev)', 'url(#lpSevIn)')}</symbol>
        <symbol id="lf-sevanti2" {...SYM}>{sevanti(31, 'url(#lpSev2)', 'url(#lpSev2In)')}</symbol>
        <symbol id="lf-marigold" {...SYM}>{sevanti(43, 'url(#lpMarigold)', 'url(#lpMarigoldIn)')}</symbol>
        <symbol id="lf-lotus" {...SYM}>{lotus(13)}</symbol>
        <symbol id="lf-lily" {...SYM}>{lily()}</symbol>
        <symbol id="lf-leaf" {...SYM}>{leaf('url(#lpLeafL)', 'url(#lpLeafR)', '#173d17')}</symbol>
        <symbol id="lf-leaf2" {...SYM}>{leaf('url(#lpLeafL2)', 'url(#lpLeafR2)', '#0f2e10')}</symbol>

        {palette === 'rose-yellow' && <>
          {RG('lpRoseRed', [[0, '#590d21'], [.3, '#971733'], [.65, '#c52543'], [1, '#e64a5a']], US)}
          {LG('lpRoseRedEdge', [[0, '#500b1b', .35], [.24, '#500b1b', 0], [.76, '#500b1b', 0], [1, '#500b1b', .35]], { x1: 0, y1: .5, x2: 1, y2: .5 })}
          {LG('lpYellow', [[0, '#c6b72f'], [.25, '#e7d83b'], [.65, '#f7e75c'], [1, '#fff4a0']])}
          {LG('lpYellowEdge', [[0, '#9c9129', .2], [.24, '#9c9129', 0], [.76, '#9c9129', 0], [1, '#9c9129', .2]], { x1: 0, y1: .5, x2: 1, y2: .5 })}
          <symbol id="lf-rose-red" {...SYM}><image href={ASSET_BASE + "landing/rose-red-realistic.png"} x="-50" y="-50" width="100" height="100" /></symbol>
          <symbol id="lf-flower-yellow" {...SYM}><image href={ASSET_BASE + "landing/flower-yellow-realistic.png"} x="-50" y="-50" width="100" height="100" /></symbol>
          <symbol id="lf-p-rose-red" {...SYM}><g transform="translate(0 44)">
            <Petal d={rounded(88, 40)} fill="url(#lpRoseRed)" sh={0} stroke="#90152c" edge="url(#lpRoseRedEdge)">
              <path d="M-30 -70 Q0 -94 30 -70" fill="none" stroke="#ef6671" strokeWidth="2.2" strokeLinecap="round" />
              <path d="M-28 -66 Q0 -88 28 -66 M0 -6 Q-4 -40 -12 -70 M0 -6 Q4 -40 12 -70" fill="none" stroke="#650c23" strokeOpacity=".3" strokeWidth="1" />
            </Petal>
          </g></symbol>
          <symbol id="lf-p-yellow" {...SYM}><g transform="translate(0 43)">
            <Petal d={rounded(86, 22)} fill="url(#lpYellow)" sh={0} stroke="#c8b34d" edge="url(#lpYellowEdge)">
              <path d="M0 -8 Q-2 -43 0 -76" fill="none" stroke="#fff7ad" strokeWidth="1.1" />
            </Petal>
          </g></symbol>
        </>}

        {/* Loose petals for the shower. */}
        <symbol id="lf-p-rose" {...SYM}>
          <g transform="translate(0 44)">
            <Petal d={rounded(88, 40)} fill="url(#lpRose)" sh={0} stroke="#d1c3a4">
              <path d="M-30 -70 Q 0 -94 30 -70" fill="none" stroke="#fffef9" strokeOpacity="0.85" strokeWidth="2.2" strokeLinecap="round" />
              <path d="M-28 -66 Q 0 -88 28 -66" fill="none" stroke="#a88f5e" strokeOpacity="0.25" strokeWidth="1.6" />
              <path d="M0 -6 Q -4 -40 -12 -70 M0 -6 Q 4 -40 12 -70 M0 -6 L0 -76" fill="none" stroke="#c7b48a" strokeOpacity="0.3" strokeWidth="0.8" />
            </Petal>
          </g>
        </symbol>
        <symbol id="lf-p-pink" {...SYM}>
          <g transform="translate(0 46)">
            <Petal d={pointed(92, 24)} fill="url(#lpLotus)" sh={0} stroke="#e1b3c1">
              <path d="M0 -6 Q -12 -46 -5 -84 M0 -6 Q 12 -46 5 -84 M0 -6 L0 -88 M0 -6 Q -20 -40 -12 -76 M0 -6 Q 20 -40 12 -76" fill="none" stroke="#e07f9e" strokeOpacity="0.3" strokeWidth="0.8" />
            </Petal>
          </g>
        </symbol>
        <symbol id="lf-p-jas" {...SYM}>
          <g transform="translate(0 42)">
            <Petal d={rounded(84, 22)} fill="url(#lpMogra)" sh={0} stroke="#d3c6a8">
              <path d="M0 -10 C 1 -30 -1 -50 0 -70" fill="none" stroke="#bfae84" strokeOpacity="0.35" strokeWidth="1" />
            </Petal>
          </g>
        </symbol>
        <symbol id="lf-p-champa" {...SYM}>
          <g transform="translate(-4 46) scale(2.2)">
            <Petal d={CHAMPA_PETAL} fill="url(#lpChampaP)" sh={0} stroke="#d9c9a2" />
          </g>
        </symbol>
        {LG('lpChampaP', [[0, '#eeb21e'], [0.14, '#f8d465'], [0.42, '#fff6dc'], [0.72, '#fffefa'], [1, '#ffffff']])}
      </defs>
    </svg>
  )
}

// ---------------------------------------------------------------------------------------------------
// The bed. Coordinates are FRAME UNITS: the gold frame is 800 wide and 1189.4 tall (photo bottom 1098.7),
// so the bed lines up at every size. It stays entirely along the bottom, keeping the folded hands and
// both sides of the frame clear, and spills wide at the base under the incense bowl.
export const BED_VB = { x: -420, y: 580, w: 1640, h: 770 }

const KINDS = [
  ['mogra', 0.13], ['mogra2', 0.1], ['sevanti', 0.1], ['sevanti2', 0.08], ['rose', 0.1], ['rose2', 0.06],
  ['champa', 0.16], ['bud', 0.09], ['lotus', 0.09], ['lily', 0.05], ['leaf', 0.04],
]
function pickKind(r) {
  let acc = 0
  for (const [k, w] of KINDS) { acc += w; if (r < acc) return k }
  return 'mogra'
}
const HALF = 735
const yTop = x => {
  const u = Math.abs(x - 400) / HALF
  return 1220 + 7 * u * u + 2 * Math.sin(x * .045) + Math.sin(x * .11 + 1.3)
}
const yBot = x => { const u = Math.abs(x - 400) / HALF; return 1290 - 12 * u * u }

function buildBed() {
  const rnd = seeded(1308)
  const back = [], front = []
  // Dense mat of leaves first, so nothing between flowers ever shows the wall.
  for (let y = 640; y <= 1320; y += 30) {
    for (let x = -370; x <= 1170; x += 30) {
      const jx = x + (rnd() - 0.5) * 18, jy = y + (rnd() - 0.5) * 18
      if (jy < yTop(jx) - 6 || jy > yBot(jx) + 4) continue
      const out = Math.atan2(jy - 1250, jx - 400) * 180 / Math.PI + 90
      back.push({ k: rnd() < 0.5 ? 'leaf' : 'leaf2', x: jx, y: jy, s: 64 + rnd() * 22, r: out + (rnd() - 0.5) * 70, sy: 0.8 + rnd() * 0.2 })
    }
  }
  // A fringe of leaves along the outline, pointing outward, so the heap ends like a garland, not a cut edge.
  for (let x = -350; x <= 1150; x += 24 + rnd() * 10) {
    const y = yTop(x) + 14, slope = (yTop(x + 4) - yTop(x - 4)) / 8
    const out = Math.atan2(slope, 1) * 180 / Math.PI           // outward normal (slope, -1), leaf tip points along it
    back.push({ k: rnd() < 0.6 ? 'leaf' : 'leaf2', x, y, s: 78 + rnd() * 26, r: out + (rnd() - 0.5) * 50, sy: 0.75 + rnd() * 0.25 })
  }
  // Flowers: staggered rows well under a flower's size, so neighbours always overlap.
  let row = 0
  for (let y = 650; y <= 1326; y += 25, row++) {
    for (let x = -372 + (row % 2) * 16; x <= 1172; x += 30 + rnd() * 7) {
      const jx = x + (rnd() - 0.5) * 10, jy = y + (rnd() - 0.5) * 12
      if (jy < yTop(jx) + 10 || jy > yBot(jx)) continue
      const depth = Math.max(0, (jy - 1140) / 170)          // 0 at the frame edge, 1 at the front
      const s = Math.max(48, 66 + 22 * depth + rnd() * 22)
      const k = pickKind(rnd())
      front.push({ k, x: jx, y: jy, s: k === 'bud' ? s * 0.9 : s, r: rnd() * 360, sy: 0.78 + rnd() * 0.22, t: (rnd() - 0.5) * 60 })
    }
  }
  front.sort((a, b) => a.y - b.y)
  return [...back, ...front]
}
const BED = buildBed()

const Use = ({ k, x, y, s, r, sy = 1, t = 0 }) => (
  <use href={`#lf-${k}`} x={f1(-s / 2)} y={f1(-s / 2)} width={f1(s)} height={f1(s)}
    transform={`translate(${f1(x)} ${f1(y)}) rotate(${f1(t)}) scale(1 ${sy.toFixed(2)}) rotate(${f1(r - t)})`} />
)

export function FlowerBed({ palette = 'ivory' }) {
  return (
    <svg className="lp-bed" viewBox={`${BED_VB.x} ${BED_VB.y} ${BED_VB.w} ${BED_VB.h}`} aria-hidden="true" focusable="false">
      <ellipse cx="400" cy="1298" rx="770" ry="38" fill="url(#lpShadow)" />
      {BED.map((f, i) => <Use key={i} {...f} k={palette === 'rose-yellow' && !f.k.startsWith('leaf') ? (i % 5 < 3 ? 'rose-red' : 'flower-yellow') : f.k} />)}
    </svg>
  )
}

// ---------------------------------------------------------------------------------------------------
// Petal shower: a fixed pool of falling nodes, BACK (behind the frame: small, soft, dim) and FRONT. Each node
// nests fall (with drift) > sway > tumble (rotateX/Y) > spin (rotateZ), every period different, so the
// motion never visibly repeats; the tumble foreshortens the petal like a real flutter.
const SHOWER = (() => {
  const rnd = seeded(4242)
  const kinds = ['p-rose', 'p-rose', 'p-jas', 'p-jas', 'p-pink', 'p-champa', 'p-rose', 'mogra', 'p-pink', 'p-jas', 'bud', 'p-champa']
  return ['back', 'front'].map((layer, li) => Array.from({ length: li ? 26 : 18 }, () => {
    const central = rnd() < 0.7
    const left = central ? 24 + rnd() * 52 : rnd() * 100
    const k = kinds[Math.floor(rnd() * kinds.length)]
    const whole = !k.startsWith('p-')
    const near = li && rnd() < 0.18                          // a few large, soft petals close to the eye
    const base = li ? (near ? 40 + rnd() * 14 : 20 + rnd() * 14) : 10 + rnd() * 8
    return {
      k, left, size: whole ? base * 1.35 : base, near,
      fall: (li ? (near ? 7 : 9.5) : 14) + rnd() * 6, delay: rnd() * 8,
      drift: (rnd() - 0.4) * 16,
      sway: 1.9 + rnd() * 2.6, amp: 14 + rnd() * 40,
      tumble: 1.3 + rnd() * 1.9, tilt: 45 + rnd() * 35,
      spin: 5 + rnd() * 9, dir: rnd() < 0.5 ? -1 : 1,
      phase: rnd(),
    }
  }))
})()

const ROSE_YELLOW_SHOWER_KINDS = {
  'p-rose': 'p-rose-red', 'p-pink': 'p-rose-red', 'p-jas': 'p-yellow', 'p-champa': 'p-yellow',
  mogra: 'flower-yellow', bud: 'rose-red',
}

export function PetalShower({ layer, active, mounted, palette = 'ivory' }) {
  const items = SHOWER[layer === 'front' ? 1 : 0]
  return (
    <div className={`lp-shower lp-shower-${layer}${active ? ' on' : ''}`} aria-hidden="true">
      {mounted && items.map((p, i) => (
        <div key={i} className={`lp-fall${p.near ? ' near' : ''}`} style={{ left: `${p.left}%`, '--drift': `${p.drift}vw`, animationDuration: `${p.fall}s`, animationDelay: `${p.delay}s` }}>
          <div className="lp-sway" style={{ '--amp': `${p.amp}px`, animationDuration: `${p.sway}s`, animationDelay: `${-p.phase * p.sway}s` }}>
            <div className="lp-tumble" style={{ '--tilt': `${p.tilt}deg`, animationDuration: `${p.tumble}s`, animationDelay: `${-p.phase * p.tumble * 1.7}s` }}>
              <svg className="lp-spin" width={p.size} height={p.size} viewBox="-50 -50 100 100"
                style={{ animationDuration: `${p.spin}s`, animationDirection: p.dir < 0 ? 'reverse' : 'normal', animationDelay: `${-p.phase * p.spin}s` }}>
                <use href={`#lf-${palette === 'rose-yellow' ? ROSE_YELLOW_SHOWER_KINDS[p.k] : p.k}`} x="-50" y="-50" width="100" height="100" />
              </svg>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}
