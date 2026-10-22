/** Wire-consistency bounds, not a Rust Display formatter or strategy calculator.
 * Endpoints are exact dyadics in units 2^-1075, including half-subnormal steps.
 * See the independently frozen parser-bound proof before candidate observation.
 */
export type RoundingCell = readonly [bigint, bigint];
const buffer = new ArrayBuffer(8), view = new DataView(buffer);
const SCALE = 1n << 1075n, MAX = 0x7fefffffffffffffn;
const bits = (value: number) => { view.setFloat64(0, value, false); return view.getBigUint64(0, false); };
const number = (value: bigint) => { view.setBigUint64(0, value, false); return view.getFloat64(0, false); };
const finite = (value: number) => Number.isFinite(value);
const canonical = (value: number) => finite(value) && !Object.is(value, -0);
function scaledPositive(value: number): bigint {
  const raw = bits(value), exponent = Number((raw >> 52n) & 0x7ffn), fraction = raw & ((1n << 52n) - 1n);
  return (exponent ? (1n << 52n) | fraction : fraction) << BigInt((exponent ? exponent - 1023 - 52 : -1074) + 1075);
}
export function roundingCell(value: number): RoundingCell {
  if (!finite(value)) throw new Error('Finite binary64 scalar required');
  if (value === 0) return [0n, 0n]; // Nonzero model scalars rounding to zero are forbidden.
  const magnitude = Math.abs(value), raw = bits(magnitude), center = scaledPositive(magnitude);
  const previous = scaledPositive(number(raw - 1n));
  const next = raw === MAX ? 1n << 2099n : scaledPositive(number(raw + 1n));
  const low = center - (center - previous) / 2n, high = center + (next - center) / 2n;
  return value < 0 ? [-high, -low] : [low, high];
}
const intersects = (a: RoundingCell, b: RoundingCell) => a[0] <= b[1] && b[0] <= a[1];
export function unitReturnConsistent(gross: number, net: number, fee: number, slippage: number) {
  if (![gross, net, fee, slippage].every(canonical) || fee < 0 || slippage < 0) return false;
  const g = roundingCell(gross), r = roundingCell(net), f = roundingCell(fee), s = roundingCell(slippage);
  return intersects(r, [g[0] - f[1] - s[1], g[1] - f[0] - s[0]]);
}
export function unitPriceReturnConsistent(entry: number, exit: number, value: number) {
  if (!finite(entry) || !finite(exit) || entry <= 0 || exit <= 0 || !canonical(value)) return false;
  if (entry === exit ? value !== 0 : exit > entry ? value <= 0 : value >= 0) return false;
  const e = roundingCell(entry), x = roundingCell(exit), r = roundingCell(value);
  return r[1] * e[1] >= (x[0] - e[1]) * SCALE && r[0] * e[0] <= (x[1] - e[0]) * SCALE;
}
export function unitFactorConsistent(entry: number, exit: number, factor: number) {
  if (![entry, exit, factor].every(v => finite(v) && v > 0)) return false;
  const e = roundingCell(entry), x = roundingCell(exit), f = roundingCell(factor);
  return f[1] * e[1] >= x[0] * SCALE && f[0] * e[0] <= x[1] * SCALE;
}
export function unitFeeConsistent(componentBps: number, value: number) {
  if (!finite(componentBps) || componentBps < 0 || !canonical(value) || value < 0) return false;
  if (componentBps === 0) return value === 0;
  if (value === 0) return false;
  const c = roundingCell(componentBps), r = roundingCell(value);
  return intersects(c, [5000n * r[0], 5000n * r[1]]);
}
export function unitProductConsistent(allocation: number, fraction: number, contribution: number) {
  if (!finite(allocation) || allocation <= 0 || !canonical(fraction) || !canonical(contribution)) return false;
  if (fraction === 0) return contribution === 0;
  const product = allocation * fraction;
  return product !== 0 && finite(product) && Object.is(product, contribution);
}
/** Exact preimage of one rounded multiplication; at most 128 factor evaluations. */
function productFactorCells(allocation: number, equity: number): RoundingCell | null {
  if (![allocation, equity].every(v => finite(v) && v > 0)) return null;
  let low = 1n, high = MAX + 1n;
  while (low < high) {
    const mid = (low + high) >> 1n;
    if (allocation * number(mid) < equity) low = mid + 1n; else high = mid;
  }
  const first = low;
  if (first > MAX || allocation * number(first) !== equity) return null;
  high = MAX + 1n;
  while (low < high) {
    const mid = (low + high) >> 1n;
    if (allocation * number(mid) <= equity) low = mid + 1n; else high = mid;
  }
  return [roundingCell(number(first))[0], roundingCell(number(low - 1n))[1]];
}
export function unitCurveConsistent(allocation: number, equity: number, auditedReturn: number) {
  if (!canonical(auditedReturn)) return false;
  const factors = productFactorCells(allocation, equity), r = roundingCell(auditedReturn);
  return factors !== null && SCALE + r[1] > 0n && intersects(factors, [SCALE + r[0], SCALE + r[1]]);
}
export function unitEntryCurveConsistent(allocation: number, equity: number, twoSideFee: number, twoSideSlippage: number) {
  if (![twoSideFee, twoSideSlippage].every(v => canonical(v) && v >= 0)) return false;
  const factors = productFactorCells(allocation, equity), f = roundingCell(twoSideFee), s = roundingCell(twoSideSlippage);
  return factors !== null && intersects([2n * factors[0], 2n * factors[1]], [2n * SCALE - f[1] - s[1], 2n * SCALE - f[0] - s[0]]);
}
