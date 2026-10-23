/** A day-first date box (dd-mm-yyyy) with a calendar button. The browser's own date input shows the
 *  operating-system locale (mm/dd/yyyy on US English), so the text is formatted here; the value stays ISO. */
import { useEffect, useRef, useState } from "react";
import { isoDate } from "./date-sets";

export const dayFirst = (iso: string) => (/^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso.slice(8, 10)}-${iso.slice(5, 7)}-${iso.slice(0, 4)}` : iso);

export function DateField({ label, value, onChange, title }: { label: string; value: string; onChange: (iso: string) => void; title?: string }) {
  const [text, setText] = useState(dayFirst(value));
  const picker = useRef<HTMLInputElement>(null);
  useEffect(() => { if (isoDate(text) !== value && !(value === "" && text.trim() === "")) setText(dayFirst(value)); }, [value]); // eslint-disable-line react-hooks/exhaustive-deps
  const bad = text.trim() !== "" && !isoDate(text);
  const type = (next: string) => {
    setText(next);
    if (next.trim() === "") onChange("");
    else { const iso = isoDate(next); if (iso) onChange(iso); }
  };
  return (
    <span className="lab-datefield">
      <span className="lab-datefield-box">
      <input aria-label={label} placeholder="dd-mm-yyyy" inputMode="numeric" title={title} value={text} aria-invalid={bad || undefined}
        onChange={(e) => type(e.target.value)} onBlur={() => { if (!bad) setText(dayFirst(value)); }} />
      <button type="button" className="lab-datefield-pick" aria-label={`Pick ${label} date`} title="Open calendar"
        onClick={() => { const p = picker.current; try { p?.showPicker(); } catch { p?.focus(); } }}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
          <rect x="2" y="3" width="12" height="11" rx="1.5" /><path d="M2 6.5h12M5.5 1.5v3M10.5 1.5v3" />
        </svg>
      </button>
      <input ref={picker} type="date" tabIndex={-1} aria-hidden="true" className="lab-datefield-native" value={value} onChange={(e) => type(dayFirst(e.target.value))} />
      </span>
      {bad && <small className="lab-datefield-err" role="alert">Use dd-mm-yyyy, e.g. 01-04-2015</small>}
    </span>
  );
}
