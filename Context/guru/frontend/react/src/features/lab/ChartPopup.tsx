import { useEffect, useRef, useState, type ReactNode } from "react";
import "./ChartPopup.css";

/** A nonmodal window: the chart remains interactive behind the existing settings or browser header. */
export function ChartPopup({ children, title, onClose, wide = false }: { children: ReactNode; title: string; onClose(): void; wide?: boolean }) {
  const root = useRef<HTMLDivElement>(null);
  const previousFocus = useRef(document.activeElement instanceof HTMLElement ? document.activeElement : null);
  const drag = useRef<{ id: number; x: number; y: number; left: number; top: number } | null>(null);
  const [position, setPosition] = useState(() => ({ left: Math.max(12, (window.innerWidth - (wide ? 800 : 460)) / 2), top: Math.max(12, Math.min(120, window.innerHeight * .14)) }));
  const bound = (left: number, top: number) => {
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
    const box = root.current?.getBoundingClientRect();
    const width = box?.width || Math.min(wide ? 800 : 460, viewportWidth - 24), height = box?.height || 200;
    return { left: Math.max(12, Math.min(left, viewportWidth - width - 12)), top: Math.max(12, Math.min(top, window.innerHeight - height - 12)) };
  };
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const fit = () => setPosition((before) => {
      const next = bound(before.left, before.top);
      return next.left === before.left && next.top === before.top ? before : next;
    });
    fit();
    if (!element.contains(document.activeElement)) (element.querySelector<HTMLElement>('input, select') ?? element.querySelector<HTMLElement>('button, [tabindex="0"]'))?.focus();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(fit);
    observer?.observe(element);
    window.addEventListener("resize", fit);
    return () => { observer?.disconnect(); window.removeEventListener("resize", fit); if (previousFocus.current?.isConnected) previousFocus.current.focus(); };
  }, []);
  return <div className={`lab-chart-popup${wide ? " lab-chart-popup-wide" : ""}`} role="dialog" aria-label={title} aria-modal="false" tabIndex={-1} ref={root}
    aria-description="Drag the window title to move it. Alt plus arrow keys also move the window. Drag the lower-right corner to resize."
    style={position}
    onPointerDownCapture={(event) => {
      const target = event.target as HTMLElement;
      if (event.button !== 0 || !target.closest('.lab-params-h, .lab-browser-heading') || target.closest('button, input, select, a, textarea')) return;
      event.preventDefault();
      drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, ...position };
      event.currentTarget.setPointerCapture?.(event.pointerId);
      event.currentTarget.focus();
    }}
    onPointerMove={(event) => {
      const start = drag.current;
      if (start?.id === event.pointerId) setPosition(bound(start.left + event.clientX - start.x, start.top + event.clientY - start.y));
    }}
    onPointerUp={(event) => {
      drag.current = null;
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onPointerCancel={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }}
    onKeyDown={(event) => {
      if (event.key === "Escape") { event.stopPropagation(); onClose(); return; }
      if (!event.altKey || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
      event.preventDefault();
      setPosition((before) => bound(before.left + (event.key === "ArrowRight" ? 24 : event.key === "ArrowLeft" ? -24 : 0), before.top + (event.key === "ArrowDown" ? 24 : event.key === "ArrowUp" ? -24 : 0)));
    }}>{children}</div>;
}
