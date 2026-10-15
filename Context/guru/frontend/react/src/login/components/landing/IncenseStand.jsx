import { useEffect, useRef } from 'react'
import { smokeField, stepStrand } from '../../lib/incenseSmoke'

// Two black incense sticks in a brass bowl (owner 13-10-2026). Each stick burns at a tiny ember under a
// short grey ash tip. SMOKE (owner 13-10-2026: "Make the smoke a bit realistic too") is simulated, not drawn:
// a stream of points leaves each ash tip, rises as a thin laminar thread, then is carried by a slowly changing
// swirling (divergence-free) air field, so it breaks into curling wisps that widen and fade the way real
// incense smoke does. Drawn on a canvas behind the sticks; each stick has its own field, never in lockstep.
// (It replaced SVG ribbons under an animated feTurbulence filter: repetitive, and a GPU-raster risk.)

const STICKS = [
  { base: [93, 610], tip: [66, 300], id: 0 },
  { base: [107, 610], tip: [136, 318], id: 1 },
]
// The canvas covers this part of the stand's 200 x 720 viewBox (smoke rises above and drifts beside the box).
const VB = { x: -100, y: -260, w: 400, h: 980 }
const at = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]

export default function IncenseStand({ className = '', active = true, reduced = false }) {
  const canvasRef = useRef(null)
  useEffect(() => {
    const cv = canvasRef.current
    if (!cv) return
    const still = reduced || !active
    const ctx = cv.getContext('2d')
    if(!ctx)return
    // two strands per stick, one shared field per stick: they leave the tip together and drift apart higher up
    const strands = STICKS.flatMap(st => [0, 1].map(k => ({ st, k, pts: [], field: smokeField(st.id * 7 + 3, k), acc: 0 })))
    let raf = 0, t = 0, last = performance.now()
    const draw = () => {
      const dpr = Math.min(1.5, window.devicePixelRatio || 1), cw = cv.clientWidth, ch = cv.clientHeight
      if (cv.width !== Math.round(cw * dpr) || cv.height !== Math.round(ch * dpr)) { cv.width = Math.round(cw * dpr); cv.height = Math.round(ch * dpr) }
      const k = cv.width / VB.w
      ctx.setTransform(k, 0, 0, k, -VB.x * k, -VB.y * k)
      ctx.clearRect(VB.x, VB.y, VB.w, VB.h)
      ctx.lineCap = 'butt'; ctx.lineJoin = 'round'   // butt: runs meet end to end, no darker overlap beads
      for (const sd of strands) {
        const p = sd.pts
        // runs of 6 points share one width/alpha: ~60 strokes a strand instead of one per segment
        for (let i = 0; i + 1 < p.length; i += 6) {
          const j = Math.min(p.length - 1, i + 6), q = p[(i + j) >> 1]
          const life = Math.min(1, q.age / q.life)
          const a = Math.min(1, q.age / 0.25) * (1 - life) ** 1.15 * (sd.k ? 0.3 : 0.42)
          if (a < 0.01) continue
          const w = 0.7 + 9 * life ** 1.25
          ctx.beginPath(); ctx.moveTo(p[i].x, p[i].y)
          for (let n = i + 1; n <= j; n++) ctx.lineTo(p[n].x, p[n].y)
          // higher up the smoke diffuses: the soft halo grows while the core thins out
          ctx.strokeStyle = `rgba(118,121,131,${(a * (0.16 + 0.2 * life)).toFixed(3)})`; ctx.lineWidth = w * (2.6 + 2 * life); ctx.stroke()
          ctx.strokeStyle = `rgba(104,107,118,${(a * (1 - 0.6 * life)).toFixed(3)})`; ctx.lineWidth = w * (1 - 0.35 * life); ctx.stroke()
        }
      }
    }
    const step = dt => {
      t += dt
      for (const sd of strands) stepStrand(sd, dt, t, [sd.st.tip[0] + (sd.k ? 0.5 : -0.5), sd.st.tip[1] - 3])
    }
    if (still) { for (let i = 0; i < 360; i++) step(1 / 40); draw(); return }   // one settled, unmoving plume
    for (let i = 0; i < 280; i++) step(1 / 40)                                   // start with smoke already risen
    const frame = now => {
      if(now-last<1000/30){raf=requestAnimationFrame(frame);return}
      step(Math.min(0.05, (now - last) / 1000)); last = now
      draw(); raf = requestAnimationFrame(frame)
    }
    raf = requestAnimationFrame(frame)
    return () => cancelAnimationFrame(raf)
  }, [active, reduced])
  return (
    <div className={`lp-incense ${className}`} aria-hidden="true">
    <canvas ref={canvasRef} className="in-smoke"
      style={{ left: `${VB.x / 2}%`, top: `${(VB.y / 720) * 100}%`, width: `${VB.w / 2}%`, height: `${(VB.h / 720) * 100}%` }} />
    <svg className="in-svg" viewBox="0 0 200 720" focusable="false">
      <defs>
        <linearGradient id="inBrass" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#4f3305" />
          <stop offset="0.18" stopColor="#b9862a" />
          <stop offset="0.33" stopColor="#fff0b4" />
          <stop offset="0.5" stopColor="#d4a03c" />
          <stop offset="0.78" stopColor="#8a5a10" />
          <stop offset="1" stopColor="#3f2804" />
        </linearGradient>
        <radialGradient id="inBloom">
          <stop offset="0" stopColor="#ffb347" stopOpacity="0.75" />
          <stop offset="0.35" stopColor="#ff6a1a" stopOpacity="0.32" />
          <stop offset="1" stopColor="#ff4500" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="inAsh" x1="0" y1="1" x2="0" y2="0">
          <stop offset="0" stopColor="#5e5852" />
          <stop offset="0.3" stopColor="#8d8780" />
          <stop offset="1" stopColor="#bcb6ae" />
        </linearGradient>
        <radialGradient id="inCore" cx="0.5" cy="0.45" r="0.6">
          <stop offset="0" stopColor="#fff2b8" />
          <stop offset="0.3" stopColor="#ffa22e" />
          <stop offset="0.65" stopColor="#e2380c" />
          <stop offset="1" stopColor="#6a1604" />
        </radialGradient>
      </defs>

      {/* Sticks: bare bamboo at the foot, black masala, the ember, then a short grey ash tip. */}
      {STICKS.map(s => {
        const bamboo = at(s.base, s.tip, 0.22)
        const ember = at(s.base, s.tip, 0.952)
        const ashFoot = at(s.base, s.tip, 0.962)
        const ang = Math.atan2(s.tip[1] - s.base[1], s.tip[0] - s.base[0]) * 180 / Math.PI + 90
        const fl = { animationDuration: `${(3.7 + s.id * 1.3).toFixed(1)}s`, animationDelay: `${-s.id * 1.1}s` }
        const fb = { animationDuration: `${(5.3 + s.id * 0.9).toFixed(1)}s`, animationDelay: `${-s.id * 2.6}s` }
        return (
          <g key={s.id} strokeLinecap="round">
            <line x1={s.base[0]} y1={s.base[1]} x2={bamboo[0]} y2={bamboo[1]} stroke="#c49a5e" strokeWidth="3" />
            <line x1={bamboo[0]} y1={bamboo[1]} x2={ember[0]} y2={ember[1]} stroke="#16110d" strokeWidth="5.6" strokeLinecap="butt" />
            <line x1={bamboo[0] - 1.2} y1={bamboo[1]} x2={ember[0] - 1.2} y2={ember[1]} stroke="#4a3d33" strokeWidth="1" strokeOpacity="0.6" />
            <circle className="in-bloom" style={fb} cx={ember[0]} cy={ember[1]} r="9" fill="url(#inBloom)" />
            <g transform={`translate(${ashFoot[0]} ${ashFoot[1]}) rotate(${ang.toFixed(1)})`}>
              {/* the ember: an orange-red band under the ash */}
              <rect className="in-core" style={fl} x="-3.1" y="-1.5" width="6.2" height="6.5" rx="2.2" fill="url(#inCore)" />
              {/* ash: porous grey column, drooping a hair, rounded crown */}
              <path d="M-2.9 -0.5 C -3.1 -5 -2.8 -9 -2.2 -12.2 C -1.4 -13.6 1.2 -13.8 2.2 -12.4 C 3 -9 3.2 -5 2.9 -0.5 Z" fill="url(#inAsh)" />
              <path d="M-1.6 -1 C -1.8 -5 -1.6 -9 -1 -11.8" fill="none" stroke="#c4bfb8" strokeWidth="0.9" />
              <circle cx="1" cy="-4" r="0.55" fill="#5f5a54" /><circle cx="-0.8" cy="-8" r="0.45" fill="#5f5a54" /><circle cx="1.3" cy="-10" r="0.4" fill="#6b665f" />
              <rect x="-3" y="-1.6" width="6" height="1.6" fill="#ff7a24" opacity="0.55" className="in-core" style={fl} />
            </g>
          </g>
        )
      })}

      {/* Brass holder: a small round urli on a foot, filled with sand. */}
      <path d="M78 682 L122 682 L128 698 L72 698 Z" fill="url(#inBrass)" stroke="#3f2804" strokeWidth="0.8" />
      <path d="M40 610 C 42 662 70 686 100 686 C 130 686 158 662 160 610 Z" fill="url(#inBrass)" stroke="#3f2804" strokeWidth="0.9" />
      <path d="M52 628 C 70 640 130 640 148 628" fill="none" stroke="#fff3c6" strokeOpacity="0.45" strokeWidth="1.5" />
      <ellipse cx="100" cy="610" rx="60" ry="12" fill="url(#inBrass)" stroke="#3f2804" strokeWidth="0.9" />
      <ellipse cx="100" cy="611" rx="52" ry="8.5" fill="#837a70" />
      <ellipse cx="96" cy="609" rx="34" ry="4" fill="#a39b90" opacity="0.7" />
    </svg>
    </div>
  )
}
