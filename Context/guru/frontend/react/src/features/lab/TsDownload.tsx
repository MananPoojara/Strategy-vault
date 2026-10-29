import { useState } from "react";
import { createPortal } from "react-dom";
import { run, series } from "./client";
import { exitVariants, MAX_SHEETS, type ExitPlan, type StopMode } from "./exit-variants";
import { isValue, runQuery } from "./spec-model";
import { isIndicatorExit } from "./ExitChoice";
import { NumField } from "./OperandEditor";
import { tsRules, tsTabName, type RunContext } from "./trade-explain";
import { buildTsWorkbook, defaultTsFormat, loadXlsxStyle, TS_COSTS, tsFileName, type TsFormat, type TsInput } from "./ts-workbook";
import { saveBytes, XLSX_TYPE } from "./xlsx";
import { Segmented } from "./ui";
import type { RunResult } from "./types";

const pct = (x: number) => Math.round(x * 1e4) / 100;

/** The research team's tradesheet (Ritik's templates): the From 2000 / From 2008 format picked by the test start, with the
 *  template's impact and rollover costs shown and editable; the workbook keeps the template's formulas, MAE from the bars. */
/** `buttonSlot`: where Download TS sits (the Tradesheet heading, after Oldest / Newest first); the settings row stays below. */
export function TsDownload({ result, explain, buttonSlot }: { result: RunResult; explain: RunContext; buttonSlot?: HTMLElement | null }) {
  const [format, setFormat] = useState<TsFormat | null>(null);
  const [costs, setCosts] = useState({ impact: pct(TS_COSTS.impact), rollLong: pct(TS_COSTS.rollLong), rollShort: pct(TS_COSTS.rollShort) });
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [progress, setProgress] = useState("");
  const [plan, setPlan] = useState<ExitPlan>({ ncandle: "", atr: "", supertrend: "", indicator: "",
    hsl: { on: false, pct: 5, mode: "close" }, target: { on: false, pct: 10, mode: "close" } });
  let variants: ReturnType<typeof exitVariants> = [], planError = "";
  try { variants = exitVariants(explain.spec, plan); } catch (e) { planError = e instanceof Error ? e.message : String(e); }
  const auto = defaultTsFormat(result.from), fmt = format ?? auto;
  const put = (k: keyof typeof costs) => (n: number | undefined) => n !== undefined && Number.isFinite(n) && setCosts({ ...costs, [k]: n });
  const market = result.instrument.market, ticker = result.instrument.ticker;
  const tradesOf = (r: Pick<RunResult, "trades">) => r.trades.filter((t) => typeof t.entry_px === "number" && typeof t.exit_px === "number")
    .map((t) => ({ dir: (t.dir === -1 ? -1 : 1) as 1 | -1, entry: t.entry, exit: t.exit || result.to, entry_px: t.entry_px!, exit_px: t.exit_px!, open: t.open }));
  const save = async (input: TsInput) => {
    const XLSX = await loadXlsxStyle();
    const bytes = XLSX.write(buildTsWorkbook(XLSX, input), { bookType: "xlsx", type: "array", cellStyles: true, compression: true });
    saveBytes(new Uint8Array(bytes), tsFileName(input), XLSX_TYPE);
  };
  const bars = async () => {
      if (!market) throw new Error("This instrument has no market to read its daily bars from.");
      const [o, h, l, c] = await Promise.all(["open", "high", "low", "close"].map((id) => series(market, { id, ticker })));
      return c.dates.flatMap((date, i) => {
        const v = [o.series.open[i], h.series.high[i], l.series.low[i], c.series.close[i]];
        return v.every((x) => typeof x === "number") ? [{ date, open: v[0]!, high: v[1]!, low: v[2]!, close: v[3]! }] : [];
      });
  };
  const base = async () => ({ format: fmt, symbol: ticker, ohlc: await bars(), rules: tsRules(explain, result),
    costs: { impact: costs.impact / 100, rollLong: costs.rollLong / 100, rollShort: costs.rollShort / 100 } });
  const busyDo = async (job: () => Promise<void>) => { setBusy(true); setError(""); try { await job(); } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); setProgress(""); } };
  const download = () => busyDo(async () => save({ ...(await base()), sheetName: tsTabName(explain, result), trades: tradesOf(result) }));
  /** One backtest per exit variant (same entry, same period and costs), each a trade sheet; runs one at a time to keep the engine free. */
  const downloadMany = () => busyDo(async () => {
    if (!market) throw new Error("This instrument has no market to run on.");
    if (variants.length > MAX_SHEETS) throw new Error(`${variants.length} sheets: the template holds ${MAX_SHEETS}. Remove some variants.`);
    const code = tsTabName(explain, result).split("_En_")[0], sheets = [];
    for (const [i, v] of variants.entries()) {
      setProgress(`Running ${i + 1} of ${variants.length}: ${v.label}`);
      const r = await run(market, v.spec, { ticker, from: result.from, to: result.to, ...runQuery(v.spec, String(explain.fees ?? ""), String(explain.slippage ?? "")) });
      sheets.push({ sheetName: `${code}_EX_${v.tab}`, trades: tradesOf(r as RunResult), exitLabel: v.label });
    }
    setProgress("Building the workbook…");
    const b = await base();
    await save({ ...b, sheetName: `${code}_All exits`, trades: sheets[0]?.trades ?? [], sheets });
  });
  const stopRow = (k: "hsl" | "target", name: string) => <span className="lab-ts-stop">
    <label className="lab-check"><input type="checkbox" checked={plan[k].on} onChange={(e) => setPlan({ ...plan, [k]: { ...plan[k], on: e.target.checked } })} />{name}</label>
    {plan[k].on && <>
      <NumField hideLabel label={`${name} %`} value={plan[k].pct} step={0.5} onChange={(n) => n !== undefined && n > 0 && setPlan({ ...plan, [k]: { ...plan[k], pct: n } })} /><span className="cap">%</span>
      <Segmented label={`${name} checked on`} value={plan[k].mode} onChange={(m) => setPlan({ ...plan, [k]: { ...plan[k], mode: m as StopMode } })}
        options={[{ value: "live", label: "Live" }, { value: "close", label: "Close" }, { value: "both", label: "Both" }]} />
    </>}
  </span>;
  const text = (k: "ncandle" | "atr" | "supertrend" | "indicator", name: string, hint: string) => {
    const on = !plan.off?.[k];
    return <span className={`lab-ts-var${on ? "" : " lab-off"}`}>
      <label className="lab-check" title={on ? `Switch ${name} off: no sheets for it (what you typed is kept)` : `Switch ${name} on`}>
        <input type="checkbox" aria-label={`${name} on`} checked={on} onChange={(e) => setPlan({ ...plan, off: { ...plan.off, [k]: !e.target.checked } })} />{name}</label>
      <input type="text" aria-label={name} placeholder={hint} value={plan[k]} disabled={!on} onChange={(e) => setPlan({ ...plan, [k]: e.target.value })} />
    </span>;
  };
  return <div className="lab-ts" data-lab-ts>
    <span className="lab-ts-k">Tradesheet format</span>
    <Segmented label="Tradesheet format" value={fmt} onChange={(v) => setFormat(v)} options={[{ value: "2000", label: "From 2000" }, { value: "2008", label: "From 2008" }]} />
    {format === null && <span className="cap">auto, from the start date</span>}
    <label className="dfield">Impact cost %<NumField hideLabel label="Impact cost %" value={costs.impact} step={0.01} onChange={put("impact")} /></label>
    <label className="dfield">Rollover Long % / 30 days<NumField hideLabel label="Rollover Long %" value={costs.rollLong} step={0.01} onChange={put("rollLong")} /></label>
    <label className="dfield">Rollover Short % / 30 days<NumField hideLabel label="Rollover Short %" value={costs.rollShort} step={0.01} onChange={put("rollShort")} /></label>
    {(() => { const btn = <button type="button" className="dbtn sm primary" data-lab-download-ts disabled={busy} onClick={download}>{busy ? "Preparing…" : "Download TS"}</button>;
      return buttonSlot ? createPortal(btn, buttonSlot) : btn; })()}
    {error && <span className="neg" role="alert">{error}</span>}
    <details className="lab-ts-many" data-lab-ts-many>
      <summary>Download all exits <span className="cap">· same entry, one sheet per exit you type</span></summary>
      <div className="lab-ts-vars">
        {text("ncandle", "N candle", "3, 5, 7")}
        {text("atr", "ATR (period-initial,favour)", "5-1,1; 5-2,2")}
        {text("supertrend", "Supertrend (period-multiplier)", "5-1; 5-2; 5-1,2")}
        {text("indicator", "Indicator exit thresholds", "55, 70")}
        {plan.indicator.trim() && !(explain.spec.exit.any ?? []).some((r) => isIndicatorExit(r) && isValue((r as { rhs?: unknown }).rhs as never)) &&
          <span className="cap neg">Add an Indicator exit with a number in the Exit card first (e.g. RSI crosses above 60).</span>}
      </div>
      <div className="lab-ts-vars">{stopRow("hsl", "HSL")}{stopRow("target", "Profit booking")}</div>
      <div className="lab-ts-vars">
        <span className={variants.length > MAX_SHEETS ? "neg" : "cap"}>{planError || `${variants.length} sheet${variants.length === 1 ? "" : "s"}${variants.length > MAX_SHEETS ? ` — the template holds ${MAX_SHEETS}` : ""}`}</span>
        {progress && <span className="cap" role="status">{progress}</span>}
        <button type="button" className="dbtn sm primary" data-lab-download-many disabled={busy || !variants.length || variants.length > MAX_SHEETS} onClick={downloadMany}>
          {busy && progress ? "Working…" : "Download all exits"}</button>
      </div>
    </details>
  </div>;
}
