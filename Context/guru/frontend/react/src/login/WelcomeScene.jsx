const ASSET_BASE = '/vendor/login/'
// Devotional landing page, shown before the desk on every fresh page load (owner 13-10-2026).
// First click anywhere starts the aarti (the Ganga aarti lamp waved clockwise, petal shower, music); the next click
// stops it (lamp glides back to its place in front of the flowers, still lit, shower fades, music fades out); clicks
// keep toggling. The lamp stands lit in that place from the first load.
// LOGIN MODE (owner 13-10-2026, /welcome?login, also what /login shows): Login moves the whole scene to the left
// and a sign-in panel opens on the right (native CSS transition); the aarti only runs in welcome mode.
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useSceneEnvironment } from './useSceneEnvironment'
import LoginForm from './LoginForm'
import GoldFrame from './components/landing/GoldFrame'
import GangaAartiDeepak, { GD_W, GD_ANCHOR, GD_THREE_ROW_ANCHOR } from './components/landing/GangaAartiDeepak'
import IncenseStand from './components/landing/IncenseStand'
import { FlowerDefs, FlowerBed, PetalShower } from './components/landing/Flowers'
import './landing.css'

const REV_S = 5.2          // seconds per aarti circle
const FADE_MS = 1000
const HOME = { x: 0, y: 0, scale: 1 }
// Going back (owner 13-10-2026: "the shutter should start closing first, before the main page start sliding"): the
// scene waits this long after the shutter (0.55 s) begins to close before it slides back.
const CLOSE_FIRST = 0.4

// Untransformed page offset of an element (offsets ignore CSS transforms, so the moved scene never measures itself).
function offsetXY(el, stop = null) {
  let x = 0, y = 0
  for (let e = el; e && e !== stop; e = e.offsetParent) { x += e.offsetLeft; y += e.offsetTop }
  return [x, y]
}

// Login mode: the scene (glow, text, shrine) moves as one transform, fitted into the room left of the panel
// (desktop) or above it (phone). Kept in view: the lamp at rest left of the frame, the frame,
// the flower bed, the incense on the right and, on desktop, the text column once it has text.
function sceneFit(root, text, shrine, panel, phone) {
  const vw = root.clientWidth, vh = window.innerHeight
  const [sx, sy] = offsetXY(shrine, root), sw = shrine.offsetWidth, sh = shrine.offsetHeight
  let l = sx - 0.5 * sw
  let r = sx + 1.48 * sw
  const t = sy - 0.02 * sh, b = sy + 1.12 * sh
  if (!phone && text.textContent.trim()) {
    const tx=offsetXY(text,root)[0],box=text.getBoundingClientRect(),range=document.createRange()
    range.selectNodeContents(text)
    const ink=range.getBoundingClientRect(),scale=box.width/text.offsetWidth||1
    // Include real Gujarati glyph bounds, not only the section box; centered
    // text can overhang that box after the supplied font finishes loading.
    l=Math.min(l,tx+(ink.left-box.left)/scale)
    r=Math.max(r,tx+(ink.right-box.left)/scale)
  }
  const [bw, bh] = phone ? [vw, panel.offsetTop - 24] : [panel.offsetLeft - 24, vh]
  // always a little smaller on desktop, so the move reads as stepping back, not just sliding
  const scale = Math.min(phone ? 1 : 0.9, bw * 0.97 / (r - l), bh * 0.97 / (b - t))
  const x = (bw - (r - l) * scale) / 2 - l * scale
  return { x, y: (phone ? 6 : 0) + (bh - (b - t) * scale) / 2 - t * scale, scale }
}

function WelcomeFlowerGarlands({ palette = 'ivory' }) {
  return <div className="lp-flower-garlands" aria-hidden="true">
    {['left', 'right'].map(side => <div key={side} className={`lp-garland-side lp-garland-side-${side}`}>
      {[[10, 36, 80], [35, 13, 33], [60, 15, 37], [85, 10, 27]].map(([position, count, drop], strand) => <svg
        key={strand} className="lp-garland" viewBox={`0 0 64 ${count * 32 + 116}`} focusable="false"
        style={{ left: `${position}%`, height: `${drop}vh` }}>
        <path d={`M32 0V${count * 32 + 70}`} fill="none" stroke="#bb9249" strokeWidth="1.5" />
        <use href="#lf-leaf" x="12" y="15" width="23" height="40" transform="rotate(28 32 35)" />
        <use href="#lf-leaf2" x="29" y="15" width="23" height="40" transform="rotate(-28 32 35)" />
        {Array.from({ length: count }, (_, row) => {
          const flower = palette === 'rose-yellow'
            ? (row + strand * 2) % 10 >= 7 ? 'flower-yellow' : 'rose-red'
            : (row + strand * 2) % 10 >= 7 ? 'marigold' : row % 2 ? 'mogra' : 'mogra2'
          const y = 48 + row * 32
          return <use key={row} href={`#lf-${flower}`} x="4" y={y - 28} width="56" height="56" transform={`rotate(${row * 37 + strand * 19} 32 ${y})`} />
        })}
        <use href={palette === 'rose-yellow' ? '#lf-flower-yellow' : '#lf-marigold'} x="1" y={count * 32 + 8} width="62" height="62" />
        <use href={palette === 'rose-yellow' ? '#lf-rose-red' : '#lf-bud'} x="17" y={count * 32 + 57} width="30" height="44" />
      </svg>)}
    </div>)}
  </div>
}

export default function Landing({ flowerPalette = 'ivory', threeRowDeepak = true, onSignIn, audioEnabled = false }) {
  const [loginMode, setLoginMode] = useState(() => new URLSearchParams(window.location.search).has('login'))
  const {visible, reduced} = useSceneEnvironment()
  useEffect(() => {const sync=()=>setLoginMode(new URLSearchParams(window.location.search).has('login'));window.addEventListener('popstate',sync);return()=>window.removeEventListener('popstate',sync)}, [])
  const [audioError,setAudioError] = useState(false)
  const [phase, setPhase] = useState('idle')        // idle | on | rest
  const [shower, setShower] = useState({ key: 0, mounted: false })
  const audioRef = useRef(null)
  const fadeRef = useRef(0)
  const wrapRef = useRef(null)
  const diyaRef = useRef(null)
  const lightRef = useRef(null)
  const sightRef = useRef(null)
  const motionRef = useRef(null)
  const rootRef = useRef(null)
  const textRef = useRef(null)
  const panelRef = useRef(null)
  const phaseRef = useRef(phase)
  phaseRef.current = phase
  const [fw, setFw] = useState(0)

  const stopFade = () => { cancelAnimationFrame(fadeRef.current); fadeRef.current = 0 }
  const startMusic = () => {
    const a = audioRef.current
    if (!a || !audioEnabled) return
    stopFade()
    try {a.volume = 1} catch {}
    a.play().catch(error => {if(error?.name!=='AbortError')setAudioError(true)})
  }
  const fadeMusic = () => {
    const a = audioRef.current
    if (!a || a.paused) return
    stopFade()
    const v0 = a.volume, t0 = performance.now()
    const step = now => {
      const k = Math.min(1, (now - t0) / FADE_MS)
      try { a.volume = v0 * (1 - k) } catch { /* iOS: volume is read-only */ }
      if (k < 1) fadeRef.current = requestAnimationFrame(step)
      else { a.pause(); try { a.volume = 1 } catch { /* read-only */ } fadeRef.current = 0 }
    }
    fadeRef.current = requestAnimationFrame(step)
  }

  const toggle = useCallback(() => {
    if (phaseRef.current === 'on') {
      setPhase('rest')
      fadeMusic()
    } else {
      setPhase('on')
      if (!reduced) setShower(s => ({ key: s.key + 1, mounted: true }))
      startMusic()
    }
  }, [reduced])

  // Unmount the shower nodes once they have faded out.
  useEffect(() => {
    if (phase === 'on' || !shower.mounted) return
    const t = setTimeout(() => setShower(s => ({ ...s, mounted: false })), 1600)
    return () => clearTimeout(t)
  }, [phase, shower.mounted])

  // The lamp scales with the frame.
  useEffect(() => {
    const el = wrapRef.current
    if (!el || typeof ResizeObserver !== 'function') return
    const ro = new ResizeObserver(() => setFw(el.clientWidth))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // A hidden tab stops sound and motion; returning requires a fresh user gesture.
  useEffect(() => {if(!visible){stopFade();audioRef.current?.pause();setPhase(p=>p==='on'?'rest':p)}}, [visible])

  // Leaving the page stops the music.
  useEffect(() => {
    const a = audioRef.current
    return () => { stopFade(); a?.pause() }
  }, [])

  // The lamp, held by hand: one rAF loop drives a critically damped spring toward a moving target (a clockwise
  // circle while waving, the resting spot otherwise). The waving speed eases in and out and varies gently,
  // the circle breathes and bobs a little; the flames trail against the motion on an underdamped spring and
  // stretch on smoothed noise; the warm light on the photo follows the flame cluster. State survives on/rest
  // switches (motionRef), so stopping glides to rest instead of jumping. The lamp is always shown, lit: before the
  // first click and after a stop it stands at the same resting spot (owner 13-10-2026); reduced motion keeps it there.
  useEffect(() => {
    const wrap = wrapRef.current, diya = diyaRef.current
    if (!wrap || !diya || !visible) return
    const { sin, cos, PI, min, max, hypot, sqrt, random } = Math
    const m = motionRef.current || (motionRef.current = { t: 0, theta: PI / 2, spin: 0, pos: null, vel: [0, 0], lean: 0, leanV: 0, tilt: 0, flick: 1 })
    let raf = 0, last = performance.now()
    const frame = now => {
      if(now-last<1000/30 && !reduced){raf=requestAnimationFrame(frame);return}
      const dt = min(0.05, (now - last) / 1000); last = now; m.t += dt
      const t = m.t, on = phaseRef.current === 'on' && !reduced
      const fw = wrap.clientWidth, fh = wrap.clientHeight
      diya.style.visibility = 'visible'
      // pos is the flame cluster's centre (GD_ANCHOR); the lamp hangs from it by its handle
      const anchor = threeRowDeepak ? GD_THREE_ROW_ANCHOR : GD_ANCHOR
      const k = diya.offsetWidth / GD_W, ax = anchor.x * k, ay = anchor.y * k
      // resting place: on the flower bed left of the frame, balancing the incense on the right, never off-screen
      const rest = [max(-fw * 0.2, diya.offsetWidth * 0.44 - offsetXY(wrap)[0]), fh * 0.93]
      if(reduced){m.spin=0;m.lean=0;m.leanV=0;m.tilt=0;m.flick=1}
      m.spin += ((on ? 1 : 0) - m.spin) * min(1, dt * (on ? 0.9 : 2.4))                 // eased start and stop
      m.theta += dt * m.spin * (2 * PI / REV_S) * (1 + 0.14 * sin(t * 1.13) + 0.07 * sin(t * 2.71 + 1.2)) // + is clockwise
      // A bigger circle (owner 13-10-2026: "Make the circle of aarti bigger"): was fw * 0.12 wide, 0.55 tall, centred at fh * 0.93
      const R = fw * 0.2 * (1 + 0.045 * sin(t * 0.73 + 0.4) + 0.02 * sin(t * 2.3))
      const bob = fw * (0.011 * sin(t * 3.4 + 0.7) + 0.005 * sin(t * 5.9))
      const target = on ? [fw * 0.5 + R * cos(m.theta), fh * 0.89 + R * 0.66 * sin(m.theta) + bob] : rest
      if (!m.pos || phaseRef.current === 'idle' || reduced) { m.pos = [...rest]; m.vel[0] = m.vel[1] = 0 }   // before the first click: just stand there
      const pos = m.pos, vel = m.vel, K = on ? 55 : 13, C = 2 * sqrt(K)
      for (let i = 0; i < 2; i++) {
        vel[i] += ((target[i] - pos[i]) * K - vel[i] * C) * dt
        pos[i] += vel[i] * dt
      }
      // Flames lag and trail opposite to the motion; the lamp tilts a little with the hand.
      const vx = vel[0] / fw, vy = vel[1] / fw
      m.leanV += ((max(-24, min(24, -vx * 40)) - m.lean) * 46 - m.leanV * 7.5) * dt
      m.lean += m.leanV * dt
      m.tilt += (max(-5, min(5, vx * 9)) - m.tilt) * min(1, dt * 4)
      m.flick += (1 + 0.05 * sin(t * 7.3) + 0.035 * sin(t * 12.7 + 1) + (random() - 0.5) * 0.11 - m.flick) * min(1, dt * 8)
      diya.style.transformOrigin = `${ax.toFixed(1)}px ${ay.toFixed(1)}px`
      diya.style.transform = `translate3d(${(pos[0] - ax).toFixed(1)}px, ${(pos[1] - ay).toFixed(1)}px, 0) rotate(${m.tilt.toFixed(2)}deg)`
      diya.style.setProperty('--lean', `${(m.lean - m.tilt).toFixed(2)}deg`)
      diya.style.setProperty('--fy', (m.flick * (1 + min(0.22, hypot(vx, vy) * 0.35))).toFixed(3))
      diya.style.setProperty('--fx', (1 / sqrt(m.flick)).toFixed(3))
      const sight = sightRef.current, light = lightRef.current
      if (sight && light) {
        light.style.visibility = 'visible'
        const ls = light.offsetWidth
        light.style.transform = `translate3d(${(pos[0] - sight.offsetLeft - ls / 2).toFixed(1)}px, ${(pos[1] - sight.offsetTop - ls / 2).toFixed(1)}px, 0)`
        if (light.firstChild) light.firstChild.style.opacity = max(0.7, min(1, 0.86 + (m.flick - 1) * 1.6)).toFixed(3)
      }
      const settled = !on && m.spin < 0.01 && hypot(target[0] - pos[0], target[1] - pos[1]) < 0.5 && hypot(vel[0], vel[1]) < 1
      raf = settled ? 0 : requestAnimationFrame(frame)
    }
    raf = requestAnimationFrame(frame)
    const onResize = () => { if (!raf) { last = performance.now(); raf = requestAnimationFrame(frame) } }
    window.addEventListener('resize', onResize)
    return () => { cancelAnimationFrame(raf); window.removeEventListener('resize', onResize) }
  }, [phase, reduced, fw, threeRowDeepak, visible])

  const phone = typeof matchMedia === 'function' && matchMedia('(max-width: 900px)').matches
  const finePointer = typeof matchMedia === 'function' && matchMedia('(pointer: fine)').matches
  const goLogin = e => {
    e.stopPropagation()
    const url=new URL(window.location.href);url.searchParams.set('login','')
    window.history.pushState({...window.history.state,vaultLoginOpen:true},'',url)
    setLoginMode(true)
  }
  const goBack = () => {
    if(window.history.state?.vaultLoginOpen){window.history.back();return}
    const url=new URL(window.location.href);url.searchParams.delete('login');window.history.replaceState(window.history.state,'',url)
    setLoginMode(false)
  }
  const onPageClick = e => {
    if (loginMode || e.target.closest('button,a,input,aside')) return   // the aarti runs in welcome mode only
    toggle()
  }

  // Entering login mode stops a running aarti: the lamp glides back to rest and the music fades out.
  useEffect(() => {
    if (loginMode && phaseRef.current === 'on') { setPhase('rest'); fadeMusic() }
  }, [loginMode])

  // Where the scene sits in login mode; re-fitted on resize. A deep link to ?login starts there, no glide.
  const [panelMounted,setPanelMounted] = useState(loginMode)
  useEffect(()=>{if(loginMode){setPanelMounted(true);return}const timer=setTimeout(()=>setPanelMounted(false),reduced?0:550);return()=>clearTimeout(timer)},[loginMode,reduced])
  const [fit, setFit] = useState(null)
  const glideRef = useRef(!loginMode)
  useLayoutEffect(() => {
    if (!loginMode) { glideRef.current = true; return }
    let cancelled=false
    const refit = () => {
      if(cancelled)return
      const [root, text, shrine, panel] = [rootRef.current, textRef.current, wrapRef.current, panelRef.current]
      if (root && text && shrine && panel) setFit(sceneFit(root, text, shrine, panel, matchMedia('(max-width: 900px)').matches))
    }
    refit()
    document.fonts?.ready.then(refit)
    const ready=requestAnimationFrame(()=>{glideRef.current=true})
    window.addEventListener('resize', refit)
    return () => {cancelled=true;cancelAnimationFrame(ready);window.removeEventListener('resize', refit)}
  }, [loginMode])
  const scene = loginMode ? fit || HOME : HOME
  return (
    <div className={`lp-root lp-${phase}${loginMode ? ' lp-in' : ''}${!visible ? ' lp-suspended' : ''}`} data-flower-palette={flowerPalette} ref={rootRef} onClick={onPageClick}>
      <FlowerDefs palette={flowerPalette} />

      <div className="lp-vault-brand" aria-label="Strategy Vault">STRATEGY VAULT<span>Research with conviction.</span></div>
      <button type="button" className="btn lp-login" hidden={loginMode} onClick={goLogin}>Sign in <span aria-hidden="true">↗</span></button>
      {!loginMode && <button type="button" className="lp-toggle" aria-pressed={phase === 'on'} onClick={e => {e.stopPropagation();toggle()}}>{phase === 'on' ? 'Stop aarti' : 'Begin aarti'} <span aria-live="polite">{audioEnabled && !audioError ? '· music' : '· visual'}</span></button>}

      {/* the whole scene moves as one; identity transform in welcome mode, so the fixed back shower stays full-screen */}
      <div className="lp-scene" style={{transform:`translate3d(${scene.x}px,${scene.y}px,0) scale(${scene.scale})`,transformOrigin:'0 0',transition:reduced || !glideRef.current ? 'none' : loginMode ? 'transform .85s cubic-bezier(.2,.8,.2,1)' : `transform .85s ${CLOSE_FIRST}s cubic-bezier(.2,.8,.2,1)`}}>
      <div className="lp-glow" aria-hidden="true" />
      <WelcomeFlowerGarlands palette={flowerPalette} />
      <PetalShower palette={flowerPalette} layer="back" active={phase === 'on'} mounted={shower.mounted} key={`b${shower.key}`} />

      <main className="lp-stage">
        <section className="lp-text gu" lang="gu" aria-label="Blessing" ref={textRef}>
          <div className="lp-text-body">
            {/* Owner's text (13-10-2026), set as two lines: "May Dada's blessings always remain upon all of us." */}
            <p className="lp-bless">દાદાની કૃપા<br />સદાય અમારા સૌ પર રહે.</p>
          </div>
        </section>

        <div className="lp-shrine" ref={wrapRef}>
          <GoldFrame sightRef={sightRef}>
            <img className="lp-photo" src={ASSET_BASE + "landing/dadashri@2x.jpg"} alt="Param Pujya Dadashri, Founder of Akram Vignan" draggable="false" />
            <div className="lp-light" ref={lightRef}><i /></div>
          </GoldFrame>
          <FlowerBed palette={flowerPalette} />
          <IncenseStand active={visible && !loginMode} reduced={reduced} />
          <div className="lp-diya-wrap" ref={diyaRef}>
            <GangaAartiDeepak size={Math.round((fw || 480) * 0.44)} threeRows={threeRowDeepak} />
          </div>
        </div>
      </main>
      </div>

      <p className="lp-hint" aria-hidden={phase !== 'idle' || loginMode}><span className="click">Click</span><span className="tap">Tap</span> anywhere to begin aarti</p>
      <PetalShower palette={flowerPalette} layer="front" active={phase === 'on'} mounted={shower.mounted} key={`f${shower.key}`} />

      <aside className="lp-panel" ref={panelRef} aria-label="Sign in" inert={!loginMode} aria-hidden={!loginMode}>
        <button type="button" className="lp-back" onClick={goBack}><span aria-hidden="true">←</span> Back</button>
        <div className="lp-brand"><span className="lp-wordmark">STRATEGY VAULT</span><span className="lp-word">PRIVATE RESEARCH</span></div>
        <div className="lp-rule" aria-hidden="true"><i /></div>
        {(loginMode || panelMounted) && <LoginForm autoFocus={finePointer} onSignIn={onSignIn} />}
      </aside>
      <audio ref={audioRef} src={audioEnabled ? ASSET_BASE + "landing/aarti.ogg" : undefined} loop preload="none" />
    </div>
  )
}
