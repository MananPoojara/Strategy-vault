export interface TradeExitCheck {
  available: boolean;
  ok: boolean | null;
  text: string;
  description: string;
}

type Fields = Record<string, unknown>;
const fields = (value: unknown): Fields => value && typeof value === "object" && !Array.isArray(value) ? value as Fields : {};
const finite = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const period = (value: unknown): value is number => finite(value) && Number.isInteger(value) && value > 0 && value <= 10_000;
const date = (value: unknown): value is string => typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value);
const unavailable = (description: string): TradeExitCheck => ({available: false, ok: null, text: "Snapshot unavailable", description});
const signed = (value: number) => `${value > 0 ? "+" : ""}${value.toFixed(2)}%`;

/** Generic completed-signal-bar diagnostics, never proof that a book's source rules
 * or execution have been verified. Legacy unlabelled custom values are ignored. */
export function tradeExitCheck(value: unknown): TradeExitCheck | null {
  const trade = fields(value), reason = typeof trade.reason === "string" ? trade.reason : "";
  if (trade.open === true) return null;
  const bars = reason.match(/^(\d+) bars held/);
  if (bars) return finite(trade.bars) && Number.isInteger(trade.bars)
    ? {available: true, ok: trade.bars === Number(bars[1]), text: `${trade.bars} bars`, description: "Recorded holding bars; not a source-rule verification."}
    : unavailable("Recorded holding bars are missing.");
  const rsi = reason.match(/^RSI\((\d+)\) ([<>]) ([\d.]+)/);
  const sma = reason.match(/^Close ([<>]) (\d+)-day SMA/);
  const band = reason.match(/≥ ([\d.]+)% (below|above) (\d+)-day SMA/);
  const profit = reason.match(/^Close ≥ ([\d.]+)% profit/);
  if (!rsi && !sma && !band && !profit) return null;
  const snapshot = fields(trade.at_exit);
  if (!date(snapshot.date) || !date(trade.signal) || snapshot.date !== trade.signal) {
    return unavailable("A matching completed signal-bar date is unavailable.");
  }
  const result = (ok: boolean, text: string): TradeExitCheck => ({available: true, ok, text,
    description: `Completed signal-bar context on ${snapshot.date}; not a source-rule or execution-price verification.`});
  const smaValue = (n: number): number | null => {
    if (!period(n)) return null;
    for (const prefix of ["ma", "trend"]) {
      if (snapshot[`${prefix}_period`] === n && snapshot[`${prefix}_kind`] === "SMA" && finite(snapshot[`vs_${prefix}_p`])) {
        return snapshot[`vs_${prefix}_p`] as number;
      }
    }
    const fixed = n === 5 ? snapshot.vs_sma5 : n === 200 ? snapshot.vs_sma200 : null;
    return finite(fixed) ? fixed : null;
  };
  if (rsi) {
    const n = Number(rsi[1]), threshold = Number(rsi[3]);
    const custom = snapshot.rsi_period === n && snapshot.rsi_kind === "Wilder" ? snapshot.rsi_p : null;
    const v = finite(custom) ? custom : n === 2 ? snapshot.rsi2 : null;
    if (!period(n) || !finite(v) || !finite(threshold)) return unavailable(`A finite Wilder RSI(${n}) snapshot is unavailable.`);
    return result(rsi[2] === ">" ? v > threshold : v < threshold, `RSI(${n}) ${v.toFixed(1)}`);
  }
  if (sma) {
    const n = Number(sma[2]), v = smaValue(n);
    return v === null ? unavailable(`A finite SMA(${n}) snapshot is unavailable.`)
      : result(sma[1] === ">" ? v > 0 : v < 0, `${signed(v)} vs SMA(${n})`);
  }
  if (band) {
    const n = Number(band[3]), threshold = Number(band[1]), v = smaValue(n);
    return v === null || !finite(threshold) ? unavailable(`A finite SMA(${n}) snapshot is unavailable.`)
      : result(band[2] === "below" ? v <= -threshold + 1e-9 : v >= threshold - 1e-9, `${signed(v)} vs SMA(${n})`);
  }
  if (!finite(snapshot.close) || !finite(trade.avg_px) || trade.avg_px <= 0 || ![1, -1].includes(trade.dir as number)) {
    return unavailable("A finite signal close, entry price and direction are required.");
  }
  const gain = (snapshot.close / trade.avg_px - 1) * (trade.dir as number), threshold = Number(profit![1]);
  return finite(gain) && finite(threshold) ? result(gain >= threshold / 100 - 1e-12, `${signed(gain * 100)} vs entry`)
    : unavailable("The price comparison is unavailable.");
}

const escapeText = (value: string) => value.replace(/[&<>"']/g, char => ({"&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"}[char]!));
export function tradeExitCheckMarkup(trade: unknown): string {
  const check = tradeExitCheck(trade);
  return check ? `<br><small class="muted" data-exit-check="${check.available ? "available" : "unavailable"}" title="${escapeText(check.description)}">${escapeText(check.text)}</small>` : "";
}
