/**
 * Smooth wheel scrolling (Lenis 1.3.27), set up exactly as the lab page (192.168.4.34): the page still scrolls natively,
 * so sticky headers, scrollIntoView and find-in-page keep working; touch stays native. Nested scroll boxes (tables, the
 * optimization grid, dropdowns, popups) scroll on their own, #anchor links glide, and the smoothing pauses while a
 * popup locks the page (body overflow hidden). Off for people who ask their system for reduced motion.
 */
import Lenis from 'lenis';
import 'lenis/dist/lenis.css';

export function startSmoothScroll(): Lenis | null {
  if (typeof window === 'undefined' || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return null;
  const lenis = new Lenis({autoRaf: true, allowNestedScroll: true, anchors: true});
  const sync = () => (document.body.style.overflow === 'hidden' ? lenis.stop() : lenis.start());
  new MutationObserver(sync).observe(document.body, {attributes: true, attributeFilter: ['style']});
  sync();
  return lenis;
}
