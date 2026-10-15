export const DASH = "—";
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
/** Ratios from the engine (0.12 → "12.0%"), the same units as the book presenters. */
export const pct = (v: unknown, digits = 1) => (finite(v) ? `${(v * 100).toFixed(digits)}%` : DASH);
export const num = (v: unknown, digits = 2) =>
  finite(v) ? v.toLocaleString("en-US", { minimumFractionDigits: digits, maximumFractionDigits: digits }) : DASH;
export const int = (v: unknown) => (finite(v) ? Math.round(v).toLocaleString("en-US") : DASH);
