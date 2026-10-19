export const DASH = "—";
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
/** A value that rounds to zero prints unsigned ("-0.0" → "0.0"). */
const unsign = (s: string) => (/^-[0.,]+$/.test(s) ? s.slice(1) : s);
/** Ratios from the engine (0.12 → "12.0%"), the same units as the book presenters. */
export const pct = (v: unknown, digits = 1) => (finite(v) ? `${unsign((v * 100).toFixed(digits))}%` : DASH);
export const num = (v: unknown, digits = 2) =>
  finite(v) ? unsign(v.toLocaleString("en-US", { minimumFractionDigits: digits, maximumFractionDigits: digits })) : DASH;
export const int = (v: unknown) => (finite(v) ? unsign(Math.round(v).toLocaleString("en-US")) : DASH);
