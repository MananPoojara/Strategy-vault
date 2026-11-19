import Lenis from "lenis";
import "lenis/dist/lenis.css";

/** Charts (Lightweight Charts) zoom on the wheel: Lenis must not also scroll the page while the pointer is over one.
 * The chart calls preventDefault itself, so native scrolling stays off there; outside a chart the page scrolls. */
export const isChartWheelTarget = (node: HTMLElement): boolean => node.classList.contains("tv-lightweight-charts");

/** Page-wide smooth scrolling (Lenis). Off for reduced motion; inner scroll areas (tables, popups) keep native scrolling;
 * paused while a popup locks the page (body overflow hidden). */
export function startSmoothScroll(): Lenis | null {
  if (typeof window === "undefined" || window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return null;
  const lenis = new Lenis({ autoRaf: true, allowNestedScroll: true, anchors: true, prevent: isChartWheelTarget });
  const sync = () => (document.body.style.overflow === "hidden" ? lenis.stop() : lenis.start());
  new MutationObserver(sync).observe(document.body, { attributes: true, attributeFilter: ["style"] });
  sync();
  // A page change scrolls natively (to the top or a restored spot); drop any easing still aimed at the old page
  // so the new page is not dragged back down, and resume from where it really is.
  const onRoute = () => { lenis.stop(); sync(); }; // stop/start each reset Lenis to the actual scroll
  window.addEventListener("hashchange", onRoute);
  window.addEventListener("popstate", onRoute);
  return lenis;
}
