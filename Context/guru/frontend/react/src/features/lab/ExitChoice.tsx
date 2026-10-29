/** The Exit card: pick how a trade closes, one or more at once (the first that happens closes it). Each choice is calculated as
 *  the index AFLs do ("All Systems Final ALFs with correct Dual ATR"), with an exit the engine already has: an N-candle stop,
 *  the Dual ATR stop (ATR-Based / Dual Fresh ATR Exit: initial band, wider favour band once in profit, checked on the close), the
 *  AmiBroker Supertrend (simple-average ATR, Avg Val band), a hard stop-loss % below the buy price checked on the close or
 *  intraday, a profit target, or your own indicator conditions. No motion on the toggles: used often. */
import type { ReactNode } from "react";
import { NumField } from "./OperandEditor";
import { DUAL_PRICES } from "./OrdersPanel";
import { withExitAtStop } from "./AmiSettings";
import { candleCount, candleCountNode, getAt, setAt } from "./spec-model";
import { Segmented } from "./ui";
import type { SidePre } from "./execution-model";
import type { RuleNode, Spec } from "./types";

type Obj = Record<string, unknown>;
/** The AFLs' Supertrend exit: period 5, factor 1, AmiBroker calculation (mode 1), band centre (H+L)/2 (price 0). */
const SUPERTREND = { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 }, out: "direction" };
/** rules.rs supertrend price: the band centre, in the AFLs' "ST Avg Val" list's order of meaning. */
const ST_PRICES: [number, string][] = [[0, "(High+Low)/2"], [1, "Close"], [2, "Open"], [3, "High"], [4, "Low"], [5, "(Open+Close)/2"], [6, "(H+L+C)/3"], [7, "(O+H+L+C)/4"]];
/** The AFLs' ATR-Based Exit defaults: ATR 5, initial ×2, favour ×4, Avg Val (High+Low)/2, daily. */
const DUAL_ATR = { initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 5, mult: 4, price: "hl2" } };
type DualAtr = { initial: { period: number; mult: number; price?: string }; favour: { period: number; mult: number; price?: string }; tf?: "D" | "W" | "M"; period_end_only?: boolean };
const TFS: [string, string][] = [["D", "Daily"], ["W", "Weekly"], ["M", "Monthly"]];
/** The Supertrend exit row: a long sells when its direction turns down (crosses above 0 to +1), a short covers when it turns up
 *  (crosses below 0 to −1); direction is −1 in an uptrend (rules.rs supertrend). */
export const isSupertrendExit = (n: RuleNode) => {
  const x = n as { lhs?: Obj; op?: string; rhs?: Obj };
  return x.lhs?.s === "supertrend" && x.lhs?.out === "direction" && (x.op === "crosses_above" || x.op === "crosses_below") && x.rhs?.value === 0;
};
/** N candle: after N candles in the trade (any), or at the Nth up / down close since the entry. */
type Count = "any" | "up" | "down";
/** N candle on a Weekly / Monthly timeframe, as the AFLs' "Candle Exit TimeFrame": count the week (month) ends after the entry
 *  (Up / Down close: those whose close beat the previous week's), exit at the Nth. The engine's period_end series marks the last
 *  bar of each week / month; on that bar close.tf W offset 1 is the previous week's close. */
export type CandleTf = "D" | "W" | "M";
const PE_OUT: Record<"W" | "M", string> = { W: "w", M: "m" };
const periodEnd = (tf: "W" | "M") => ({ lhs: { s: "period_end", out: PE_OUT[tf] }, op: ">", rhs: { value: 0 } });
export function nCandleNode(kind: Count, n: number, tf: CandleTf = "D"): RuleNode {
  if (tf === "D") return (kind === "any" ? { lhs: { s: "bars_held" }, op: ">=", rhs: { value: n } } : candleCountNode(kind, n)) as RuleNode;
  const when = kind === "any" ? periodEnd(tf) : { all: [periodEnd(tf), { lhs: { s: "close" }, op: kind === "up" ? ">" : "<", rhs: { s: "close", tf, offset: 1 } }] };
  return { lhs: { since_entry: "count", when }, op: ">=", rhs: { value: n } } as unknown as RuleNode;
}
export const nCandleOf = (n: RuleNode): { kind: Count; n: number; tf: CandleTf } | undefined => {
  const x = n as { lhs?: Obj; op?: string; rhs?: { value?: unknown } };
  if (x.lhs?.s === "bars_held" && x.op === ">=" && typeof x.rhs?.value === "number") return { kind: "any", n: x.rhs.value, tf: "D" };
  const c = candleCount(n);
  if (c && (c.kind === "up" || c.kind === "down")) return { kind: c.kind, n: c.n, tf: "D" };
  // weekly / monthly: rebuild from what was read and compare, so only the shape nCandleNode writes is recognised
  if (x.lhs?.since_entry !== "count" || x.op !== ">=" || typeof x.rhs?.value !== "number") return undefined;
  for (const tf of ["W", "M"] as const) for (const kind of ["any", "up", "down"] as const)
    if (JSON.stringify(nCandleNode(kind, x.rhs.value, tf)) === JSON.stringify({ lhs: x.lhs, op: x.op, rhs: x.rhs })) return { kind, n: x.rhs.value, tf };
  return undefined;
};
const nCandleRow = (kind: Count, n: number, tf: CandleTf = "D"): RuleNode => nCandleNode(kind, n, tf);
const isNBarRow = (n: RuleNode) => (n as { lhs?: Obj }).lhs?.s === "bars_held" || !!nCandleOf(n);
/** The rows the Indicator choice shows: every sell row that is not the N-candle or the Supertrend exit. */
export const isIndicatorExit = (n: RuleNode) => !isNBarRow(n) && !isSupertrendExit(n);

export function ExitChoice({ spec, pre, onChange, indicatorList }: { spec: Spec; pre: SidePre; onChange(spec: Spec): void; indicatorList: ReactNode }) {
  const path = `${pre}exit.any` as const, rows = (getAt(spec, path) as RuleNode[]) ?? [];
  const exit = ((getAt(spec, `${pre}exit`) as Obj) ?? {}) as { trailing?: { atr?: number; atr_length?: number }; stop_loss?: { pct?: number }; target?: { pct?: number }; dual_atr?: DualAtr };
  const dual = exit.dual_atr, hslOnClose = (spec.execution?.exit_at_stop?.stop_loss ?? 1) === 0;
  const setDual = (patch: Partial<DualAtr>) => dual && setExit("dual_atr", { ...dual, ...patch });
  const setExit = (key: string, value: unknown) => {
    const next = { ...((getAt(spec, `${pre}exit`) as Obj) ?? {}) };
    if (value === undefined) delete next[key]; else next[key] = value;
    onChange(setAt(spec, `${pre}exit`, next));
  };
  // one or more Supertrends (each its own Period and Multiplier): any one turning down closes the trade
  const sts = rows.filter(isSupertrendExit) as (RuleNode & { lhs: { p?: Record<string, number> } })[], st = sts[0];
  const shortSide = pre === "short_side." || spec.direction === "short";
  const stRow = (p: Record<string, number>) => ({ lhs: { ...SUPERTREND, p }, op: shortSide ? "crosses_below" : "crosses_above", rhs: { value: 0 } }) as RuleNode;
  const putSt = (i: number, p: Record<string, number>) => onChange(setAt(spec, path, rows.map((r) => (r === sts[i] ? { ...stRow(p), lhs: { ...(stRow(p) as unknown as { lhs: Obj }).lhs, ...((r as unknown as { lhs: { tf?: string } }).lhs.tf ? { tf: (r as unknown as { lhs: { tf?: string } }).lhs.tf } : {}) } } as RuleNode : r))));
  const putStTf = (i: number, tf: string) => onChange(setAt(spec, path, rows.map((r) => {
    if (r !== sts[i]) return r;
    const { tf: _tf, ...lhs } = (r as unknown as { lhs: Obj }).lhs;
    return { ...r, lhs: tf === "D" ? lhs : { ...lhs, tf } } as RuleNode;
  })));
  const addSt = (p: Record<string, number>) => onChange(setAt(spec, path, [...rows, stRow(p)]));
  const dropSt = (i?: number) => onChange(setAt(spec, path, rows.filter((r) => !isSupertrendExit(r) || (i !== undefined && r !== sts[i]))));
  // Two multipliers (the AFLs' "ST Factor Initial / Favour", rules.rs exit.dual_line): sell on a close below the Initial line;
  // once a close is above the wider Favour line, sell on a close below the Favour line instead
  type StLine = { s: string; p?: Record<string, number>; out?: string; tf?: string };
  const dl = (exit as { dual_line?: { initial: StLine; favour: StLine } }).dual_line, stDual = dl?.initial?.s === "supertrend" ? dl : undefined;
  const stLine = (p: Record<string, number>, tf?: string): StLine => ({ s: "supertrend", p, out: "supertrend", ...(tf && tf !== "D" ? { tf } : {}) });
  const exitWith = (patch: Obj, keepRows = rows) => { const next: Obj = { ...((getAt(spec, `${pre}exit`) as Obj) ?? {}), any: keepRows, ...patch };
    for (const k of Object.keys(patch)) if (patch[k] === undefined) delete next[k]; onChange(setAt(spec, `${pre}exit`, next)); };
  const toTwo = () => { const p = { ...SUPERTREND.p, ...st?.lhs.p }, tf = (st?.lhs as { tf?: string } | undefined)?.tf;
    exitWith({ dual_line: { initial: stLine(p, tf), favour: stLine({ ...p, factor: p.factor * 2 }, tf) } }, rows.filter((r) => !isSupertrendExit(r))); };
  const toOne = () => { const p = { ...SUPERTREND.p, ...stDual!.initial.p }, tf = stDual!.initial.tf;
    exitWith({ dual_line: undefined }, [...rows, { ...stRow(p), ...(tf ? { lhs: { ...SUPERTREND, p, tf } } : {}) } as RuleNode]); };
  const putDual = (initial: Record<string, number>, favour: Record<string, number>, tf = stDual?.initial.tf) => exitWith({ dual_line: { initial: stLine(initial, tf), favour: stLine(favour, tf) } });
  const indicators = rows.filter(isIndicatorExit), ncRow = rows.find((r) => nCandleOf(r)), nc = ncRow && nCandleOf(ncRow);
  const setNc = (kind: Count, n: number, tf: CandleTf = nc?.tf ?? "D") => onChange(setAt(spec, path, ncRow ? rows.map((r) => (r === ncRow ? nCandleRow(kind, n, tf) : r)) : [...rows, nCandleRow(kind, n, tf)]));
  const on = {
    ncandle: !!nc, atr: !!dual || exit.trailing?.atr !== undefined, supertrend: !!st || !!stDual,
    hsl: exit.stop_loss?.pct !== undefined, target: exit.target?.pct !== undefined, indicator: indicators.length > 0,
  };
  const toggle: Record<keyof typeof on, () => void> = {
    ncandle: () => (on.ncandle ? onChange(setAt(spec, path, rows.filter((r) => r !== ncRow))) : setNc("any", 10)),
    atr: () => { const next = { ...((getAt(spec, `${pre}exit`) as Obj) ?? {}) }; delete next.trailing; if (on.atr) delete next.dual_atr; else next.dual_atr = structuredClone(DUAL_ATR); onChange(setAt(spec, `${pre}exit`, next)); },
    supertrend: () => (on.supertrend ? exitWith({ dual_line: stDual ? undefined : dl }, rows.filter((r) => !isSupertrendExit(r))) : addSt(SUPERTREND.p)),
    hsl: () => { const next = { ...((getAt(spec, `${pre}exit`) as Obj) ?? {}) }; if (on.hsl) delete next.stop_loss; else next.stop_loss = { pct: 5 };
      onChange(withExitAtStop(setAt(spec, `${pre}exit`, next), "stop_loss", on.hsl ? 1 : 0)); }, // the AFLs check the HSL on the close by default
    target: () => setExit("target", on.target ? undefined : { pct: 10 }),
    indicator: () => onChange(setAt(spec, path, on.indicator ? rows.filter((r) => !isIndicatorExit(r))
      : [...rows, { lhs: { s: "close" }, op: "crosses_below", rhs: { s: "ema", p: { length: 20 } } } as RuleNode])),
  };
  const CHOICES: [keyof typeof on, string][] = [["ncandle", "N candle"], ["atr", "ATR"], ["supertrend", "Supertrend"], ["hsl", "HSL"], ["target", "Profit booking"], ["indicator", "Indicator"]];
  const row = (name: string, el: ReactNode) => <div className="lab-param-row" key={name}><span className="lab-param-k">{name}</span><span className="lab-param-v">{el}</span></div>;
  /** One chosen exit: its title, a × that turns it off, and its settings. */
  const part = (k: keyof typeof on, title: string, body: ReactNode) => <section className="lab-exit-part" data-lab-exit-part={k} aria-label={title}>
    <header><b>{title}</b><button type="button" className="dbtn sm ghost" aria-label={`Remove ${title}`} onClick={toggle[k]}>×</button></header>{body}</section>;
  /** AmiBroker (the AFLs' Dual ATR, the default) or a plain trailing ATR stop (close − multiplier × Wilder ATR). */
  const atrCalc = <Segmented label="ATR: calculation" value={dual ? "ami" : "trail"} options={[{ value: "ami", label: "AmiBroker (Dual ATR)" }, { value: "trail", label: "Trailing ATR" }]}
    onChange={(v) => { const next = { ...((getAt(spec, `${pre}exit`) as Obj) ?? {}) }; delete next.trailing; delete next.dual_atr;
      if (v === "ami") next.dual_atr = structuredClone(DUAL_ATR); else next.trailing = { atr: 3, atr_length: 14 }; onChange(setAt(spec, `${pre}exit`, next)); }} />;
  const sel = (label: string, value: string, options: [string, string][], put: (v: string) => void) =>
    <select aria-label={label} value={value} onChange={(e) => put(e.target.value)}>{options.map(([v, t]) => <option key={v} value={v}>{t}</option>)}</select>;
  const num = (label: string, value: number | undefined, min: number, max: number, step: number, put: (n: number) => void) =>
    <NumField hideLabel label={label} value={value} min={min} max={max} step={step} onChange={(n) => n !== undefined && n >= min && n <= max && put(n)} />;
  return <div className="lab-exit-choice" data-lab-exit-choice>
    <div className="lab-filter-choice" role="group" aria-label="Exits">
      {CHOICES.map(([k, label]) => <button key={k} type="button" className="lab-filter-chip" aria-pressed={on[k]} data-lab-exit={k} onClick={toggle[k]}>{label}</button>)}
    </div>
    {on.ncandle && nc && part("ncandle", "N candle", <>
      {row("Count", <Segmented label="N candle: count" value={nc.kind} options={[{ value: "any", label: "Any candle" }, { value: "up", label: "Up close" }, { value: "down", label: "Down close" }]}
        onChange={(k) => setNc(k as Count, nc.n)} />)}
      {row(nc.kind === "any" ? "Candles" : nc.kind === "up" ? "Nth up close" : "Nth down close", num("N candle: candles", nc.n, 1, 1000, 1, (n) => Number.isInteger(n) && setNc(nc.kind, n)))}
      {row("Timeframe", sel("N candle: timeframe", nc.tf, TFS, (t) => setNc(nc.kind, nc.n, t as CandleTf)))}</>)}
    {on.atr && dual && part("atr", "ATR stop", <>
      {row("Calculation", atrCalc)}
      {row("Timeframe", sel("ATR: timeframe", dual.tf ?? "D", TFS, (v) => setDual({ tf: v === "D" ? undefined : v as "W" | "M" })))}
      {row("Avg Val", sel("ATR: avg val", dual.initial.price ?? "hl2", DUAL_PRICES, (v) => setDual({ initial: { ...dual.initial, price: v }, favour: { ...dual.favour, price: v } })))}
      {row("Initial period", num("ATR: initial period", dual.initial.period, 1, 100, 1, (n) => setDual({ initial: { ...dual.initial, period: n } })))}
      {row("Initial multiplier", num("ATR: initial multiplier", dual.initial.mult, 0.1, 20, 0.5, (n) => setDual({ initial: { ...dual.initial, mult: n } })))}
      {row("Favour period", num("ATR: favour period", dual.favour.period, 1, 100, 1, (n) => setDual({ favour: { ...dual.favour, period: n } })))}
      {row("Favour multiplier", num("ATR: favour multiplier", dual.favour.mult, 0.1, 20, 0.5, (n) => setDual({ favour: { ...dual.favour, mult: n } })))}
      <p className="cap">Sells when the close falls below Avg Val − Initial × ATR (the line only rises). Once the close is above Avg Val + Favour × ATR, the wider Avg Val − Favour × ATR line is used. ATR is the simple average of the true range, as in the AFLs.</p></>)}
    {on.atr && !dual && exit.trailing && part("atr", "ATR stop", <>
      {row("Calculation", atrCalc)}
      {row("ATR period", num("ATR: period", exit.trailing.atr_length ?? 14, 1, 500, 1, (n) => setExit("trailing", { ...exit.trailing, atr_length: n })))}
      {row("Multiplier", num("ATR: multiplier", exit.trailing.atr, 0.1, 50, 0.5, (n) => setExit("trailing", { ...exit.trailing, atr: n })))}</>)}
    {on.supertrend && part("supertrend", `Supertrend turns ${shortSide ? "up" : "down"}${sts.length > 1 ? " (any one)" : ""}`, <>
      {row("Multipliers", <Segmented label="Supertrend: multipliers" value={stDual ? "two" : "one"} options={[{ value: "one", label: "One" }, { value: "two", label: "Two (Initial → Favour)" }]}
        onChange={(v) => (v === "two" ? !stDual && toTwo() : stDual && toOne())} />)}
      {stDual && (() => { const a = { ...SUPERTREND.p, ...stDual.initial.p }, b = { ...SUPERTREND.p, ...stDual.favour.p };
        return <>
          {row("Period", num("Supertrend: period", a.atr_length, 1, 500, 1, (n) => putDual({ ...a, atr_length: n }, { ...b, atr_length: n })))}
          {row("Initial multiplier", num("Supertrend: initial multiplier", a.factor, 0.1, 50, 0.5, (n) => putDual({ ...a, factor: n }, b)))}
          {row("Favour multiplier", num("Supertrend: favour multiplier", b.factor, 0.1, 50, 0.5, (n) => putDual(a, { ...b, factor: n })))}
          {row("Calculation", <Segmented label="Supertrend: calculation" value={String(stDual.initial.p?.mode ?? 0)} options={[{ value: "1", label: "AmiBroker" }, { value: "0", label: "TradingView" }]}
            onChange={(v) => putDual({ ...a, mode: Number(v) }, { ...b, mode: Number(v) })} />)}
          {row("Avg Val", sel("Supertrend: avg val", String(a.price ?? 0), ST_PRICES.map(([v, t]) => [String(v), t]), (v) => putDual({ ...a, price: Number(v) }, { ...b, price: Number(v) })))}
          {row("Timeframe", sel("Supertrend: timeframe", stDual.initial.tf ?? "D", TFS, (v) => putDual(a, b, v)))}
          <p className="cap">Sells when the close falls below the Initial Supertrend. Once a close is above the wider Favour Supertrend, it sells when the close falls below the Favour line instead (the AFLs' ST Factor Initial / Favour).</p>
        </>; })()}
      {!stDual && sts.map((x, i) => { const p = { ...SUPERTREND.p, ...x.lhs.p }, name = sts.length > 1 ? `Supertrend ${i + 1}: ` : "Supertrend: ";
        return <div className={sts.length > 1 ? "lab-param-line" : undefined} key={i}>
          {sts.length > 1 && row(`Supertrend ${i + 1}`, <button type="button" className="dbtn sm ghost" aria-label={`Remove Supertrend ${i + 1}`} onClick={() => dropSt(i)}>Remove</button>)}
          {row("Period", num(`${name}period`, p.atr_length, 1, 500, 1, (n) => putSt(i, { ...p, atr_length: n })))}
          {row("Multiplier", num(`${name}multiplier`, p.factor, 0.1, 50, 0.5, (n) => putSt(i, { ...p, factor: n })))}
          {row("Calculation", <Segmented label={`${name}calculation`} value={String(x.lhs.p?.mode ?? 0)} options={[{ value: "1", label: "AmiBroker" }, { value: "0", label: "TradingView" }]}
            onChange={(v) => putSt(i, { ...p, mode: Number(v) })} />)}
          {row("Avg Val", sel(`${name}avg val`, String(p.price ?? 0), ST_PRICES.map(([v, t]) => [String(v), t]), (v) => putSt(i, { ...p, price: Number(v) })))}
          {row("Timeframe", sel(`${name}timeframe`, (x.lhs as { tf?: string }).tf ?? "D", TFS, (v) => putStTf(i, v)))}</div>; })}
      {!stDual && sts.length < 4 && <button type="button" className="dbtn sm ghost lab-param-addline" data-lab-add-supertrend onClick={() => addSt({ ...SUPERTREND.p, ...st?.lhs.p })}>+ Another Supertrend</button>}</>)}
    {on.hsl && part("hsl", "Hard stop-loss (HSL)", <>
      {row("% below buy price", num("HSL: stop-loss %", exit.stop_loss?.pct, 0.1, 99, 0.5, (n) => setExit("stop_loss", { pct: n })))}
      {row("Checked on", <Segmented label="HSL: checked on" value={hslOnClose ? "close" : "live"} options={[{ value: "close", label: "Close" }, { value: "live", label: "Live (intraday low)" }]}
        onChange={(v) => onChange(withExitAtStop(spec, "stop_loss", v === "close" ? 0 : 1))} />)}</>)}
    {on.target && part("target", "Profit booking", row("Target %", num("Profit booking: target %", exit.target?.pct, 0.1, 1000, 0.5, (n) => setExit("target", { pct: n }))))}
    {on.indicator && indicatorList}
  </div>;
}
