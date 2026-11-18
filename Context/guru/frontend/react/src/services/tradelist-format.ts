// The one Tradelist: same 17 columns, order and display formats on every strategy report (display only; exports are separate).
export const TRADE_COLS = ['Symbol','Signal','Entry Date','Entry Price','Exit Date','Exit Price','Pts','Gross P&L','Gross P&L %','Days Held','Taxes','Slippage','Net P&L','Net P&L %','MAE','MFE','Exit Reason'] as const;
export const TS_NUM = [3,5,6,7,8,9,10,11,12,13,14,15], TS_SIGNED = [6,8,13,14,15], TS_DATE = [2,4];
export const numMinus = (t: unknown) => String(t).replace(/^-/, '−');
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
export const tsPct = (v: unknown) => finite(v) ? numMinus((v * 100).toFixed(1) + '%') : '—';
export const tsPx = (v: unknown) => finite(v) ? v.toFixed(2) : '—';
export const tsDate = (v: unknown) => String(v ?? '').replace(/^(\d{2})-(\d{2})-(\d{4})$/, '$1/$2/$3').replace(/^(\d{4})-(\d{2})-(\d{2})$/, '$3/$2/$1');
export const tsCap = (v: unknown) => { const s = String(v || '').toLowerCase(); return s ? s.charAt(0).toUpperCase() + s.slice(1) : '—'; };
/** ₹ with L / Cr (India) or $ (US); '—' unless the run actually supplied a monetary amount. */
export function tsMoney(v: unknown, us = false) {
  if (!finite(v)) return '—';
  const a = Math.abs(v), sign = v < 0 ? '−' : '';
  if (us) return sign + '$' + Math.round(a).toLocaleString('en-US');
  const u = a >= 1e7 ? [a / 1e7, ' Cr'] as const : a >= 1e5 ? [a / 1e5, ' L'] as const : null;
  return sign + '₹' + (u ? u[0].toFixed(1) + u[1] : Math.round(a).toLocaleString('en-IN'));
}
/** Display text of column i from its raw value (dates may be ISO or DD-MM-YYYY; fractions for percentages). Callers escape. */
export function tradelistCell(i: number, v: unknown, us = false): string {
  switch (i) {
    case 1: return tsCap(v);
    case 2: case 4: return v == null || v === '' ? '—' : tsDate(v);
    case 3: case 5: case 6: return tsPx(v);
    case 7: case 10: case 11: case 12: return tsMoney(v, us);
    case 8: case 13: case 14: case 15: return tsPct(v);
    case 9: return finite(v) ? v.toFixed(0) : '—';
    default: return v == null || v === '' ? '—' : String(v);
  }
}
export const tradelistClass = (i: number, v: unknown) =>
  TS_NUM.includes(i) ? 'n' + (TS_SIGNED.includes(i) && finite(v) ? (v < 0 ? ' neg' : ' pos') : '') : TS_DATE.includes(i) ? 'd' : '';
