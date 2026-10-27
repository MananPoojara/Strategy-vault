import { useEffect, useRef } from "react";
import "./ChartPaneDivider.css";

const MIN_HEIGHT = 80, MAX_HEIGHT = 700;
const clamp = (height: number) => Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, height));

export function ChartPaneDivider({ label, height, onChange, defaultHeight }: {
  label: string; height: number; onChange(height: number): void; defaultHeight: number;
}) {
  const root = useRef<HTMLDivElement>(null);
  const drag = useRef<{ id: number; y: number; height: number } | null>(null);
  useEffect(() => {
    const element = root.current;
    return () => {
      if (drag.current && element?.hasPointerCapture?.(drag.current.id)) element.releasePointerCapture(drag.current.id);
      drag.current = null;
    };
  }, []);
  return <div ref={root} className="lab-pane-divider" role="separator" tabIndex={0} aria-label={label} aria-orientation="horizontal"
    aria-valuemin={MIN_HEIGHT} aria-valuemax={MAX_HEIGHT} aria-valuenow={clamp(height)}
    title="Drag to resize. Up and Down resize; Home and End choose the limits. Double-click to reset."
    onPointerDown={(event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      drag.current = { id: event.pointerId, y: event.clientY, height: clamp(height) };
      event.currentTarget.setPointerCapture?.(event.pointerId);
      event.currentTarget.focus();
    }}
    onPointerMove={(event) => {
      const start = drag.current;
      if (start?.id === event.pointerId) onChange(clamp(start.height + event.clientY - start.y));
    }}
    onPointerUp={(event) => {
      drag.current = null;
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onPointerCancel={(event) => {
      drag.current = null;
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onLostPointerCapture={() => { drag.current = null; }}
    onDoubleClick={() => onChange(clamp(defaultHeight))}
    onKeyDown={(event) => {
      const next = event.key === "ArrowUp" ? height - 24 : event.key === "ArrowDown" ? height + 24
        : event.key === "Home" ? MIN_HEIGHT : event.key === "End" ? MAX_HEIGHT : undefined;
      if (next !== undefined) { event.preventDefault(); onChange(clamp(next)); }
    }} />;
}
