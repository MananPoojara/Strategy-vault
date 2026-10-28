// Incense smoke for the landing page (owner 13-10-2026). Units are the incense stand's viewBox units (the stick is
// ~310 long, so ~12 units to the cm); y grows downward, so smoke rises toward negative y.
//
// The air is a sum of slowly drifting sine waves used as a STREAM FUNCTION, and the velocity is its curl. That
// field is divergence-free (it neither creates nor swallows smoke), which is what makes strands carried through
// it fold into curls instead of just wobbling. Smoke leaves the ember laminar (a thin straight thread for ~1.5 s),
// then the swirl takes over and the strand breaks up, widens and fades.

const mulberry32 = a => () => {
  a |= 0; a = (a + 0x6D2B79F5) | 0
  let t = Math.imul(a ^ (a >>> 15), 1 | a)
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296
}

function waves(rnd, n, speed) {
  return Array.from({ length: n }, () => {
    const lambda = 55 + rnd() * 120, ang = rnd() * Math.PI * 2, k = 2 * Math.PI / lambda
    return { kx: k * Math.cos(ang), ky: k * Math.sin(ang), w: (0.18 + rnd() * 0.45) * (rnd() < 0.5 ? -1 : 1), ph: rnd() * 6.283, v: speed * (0.5 + rnd()) }
  })
}

// One field per stick (`seed`); `strand` adds a small private term so two strands from one tip drift apart.
export function smokeField(seed, strand = 0) {
  const shared = waves(mulberry32(seed), 5, 10)
  const own = waves(mulberry32(seed * 31 + strand + 1), 2, 6)
  const all = [...shared, ...own]
  return (x, y, t) => {
    let u = 0, v = 0
    for (const q of all) {
      const c = Math.cos(q.kx * x + q.ky * y + q.w * t + q.ph), k = Math.hypot(q.kx, q.ky)
      u += q.v * (q.ky / k) * c          // u =  dpsi/dy
      v -= q.v * (q.kx / k) * c          // v = -dpsi/dx
    }
    return [u, v]
  }
}

const smooth = (a, b, x) => { const t = Math.min(1, Math.max(0, (x - a) / (b - a))); return t * t * (3 - 2 * t) }
export const EMIT_HZ = 40, LIFE_S = 12

// Advance one strand { pts, field, acc } by dt seconds at time t, emitting new points at `tip` [x, y].
export function stepStrand(sd, dt, t, tip) {
  sd.acc += dt * EMIT_HZ
  while (sd.acc >= 1) { sd.acc -= 1; sd.pts.push({ x: tip[0], y: tip[1], age: 0, life: LIFE_S * (0.85 + 0.3 * ((t * 7.3) % 1)) }) }
  for (const p of sd.pts) {
    p.age += dt
    const swirl = 0.1 + 0.9 * smooth(1.1, 3.4, p.age)          // laminar thread first, then the swirl takes over
    const rise = 21 + 18 * Math.exp(-p.age / 1.6)              // hot smoke rises fast, then slows as it cools
    const [u, v] = sd.field(p.x, p.y, t)
    p.x += u * swirl * dt
    p.y += (v * swirl - rise) * dt
  }
  let n = 0
  while (n < sd.pts.length && sd.pts[n].age >= sd.pts[n].life) n++
  if (n) sd.pts.splice(0, n)
}
