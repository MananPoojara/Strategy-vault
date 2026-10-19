import { useEffect, useRef, useState } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, ohlcFromSeries, rangeStart, tradeSpans } from "./bars";
import { createLabChart, syncPanes, tint, useThemeKey, type SyncedPane } from "./charts";
import { shortName } from "./catalogue";
import { defaultColor, ParamsDialog, plotCss, plotPaint, type LineStyle, type Look } from "./ParamsDialog";
import { COPY } from "./labels";
import { formulaQuery } from "./formula";
import type { Catalogue, ChartRange, Indicator, SavedMarket, SeriesResponse, Trade } from "./types";
import { ErrorNotice, isProvisional, ProvisionalStrip } from "./ui";
import { foreignDescription, foreignAdvertised, isPair } from "./foreign-model";
import type { InstrumentOption } from "./instruments";
import "./workbench.css";

/** Platform-formula option keys (MetaTrader/AmiBroker/… variants): shown on a chip only when set away from the default, so "MACD 12, 26, 9" stays short. */
const OPTION_KEYS = new Set(["variant", "source", "basis_type", "range", "days", "signal_type", "ma_type", "exponential", "points", "horizon", "anchor_date"]);
const chipValues = (ind: Indicator, params: Record<string, number>) =>
  ind.params.filter((p) => p.key !== "variant" && !(OPTION_KEYS.has(p.key) && (params[p.key] ?? p.default) === p.default)).map((p) => params[p.key] ?? p.default);

/** `looks`: colour and style per plotted output (AmiBroker ParamColor / ParamStyle per Plot). */
export interface Chip { key: string; id: string; params: Record<string, number>; vs?: string; looks?: Record<string, Look> }
/** AmiBroker ParamStyle as Lightweight Charts line options (lineStyle 0 solid, 1 dotted, 2 dashed). */
const LINE_LOOK: Record<LineStyle, Record<string, unknown>> = {
  line: { lineWidth: 2, lineStyle: 0 }, thick: { lineWidth: 4, lineStyle: 0 }, dashed: { lineWidth: 2, lineStyle: 2 }, dotted: { lineWidth: 2, lineStyle: 1 }, hidden: { visible: false },
};
const outputsOf = (chip: Chip, catalogue: Catalogue) => catalogue.indicators.find((i) => i.id === chip.id)?.outputs ?? [];
/** Output k's colour: its own look, else the k-th palette entry after the chip's. */
const plotColor = (chip: Chip, index: number, out: string | undefined, k: number) => (out && chip.looks?.[out]?.color) || defaultColor(index, k);
export interface WorkbenchProps {
  market: SavedMarket; ticker: string; tf: string; range: ChartRange; catalogue: Catalogue; chips: Chip[];
  trades?: Trade[];
  /** `formula`/`name` are set for a "My indicators" chip. */
  loadSeries(q: { id: string; tf: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }): Promise<SeriesResponse>;
  onChipChange(key: string, params: Record<string, number>): void;
  onChipRemove(key: string): void;
  onChipLook?(key: string, looks: Record<string, Look> | undefined): void;
  instrumentLabel?: string; comparisonOptions?: readonly InstrumentOption[]; onChipVsChange?(key: string, vs?: string): void;
}

const TF_NAMES: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly" };
const PRICE_IDS = ["open", "high", "low", "close"] as const;
type Marker = { time: string; position: string; shape: string; color: string; text?: string; size?: number };
type Disposable = NonNullable<ReturnType<typeof createLabChart>>;

export function Workbench({ market, ticker, tf, range, catalogue, chips, trades, loadSeries, onChipChange, onChipRemove, onChipLook, instrumentLabel, comparisonOptions = [], onChipVsChange }: WorkbenchProps) {
  const priceRef = useRef<HTMLDivElement>(null);
  const panesRef = useRef<HTMLDivElement>(null);
  const legendRef = useRef<HTMLDivElement>(null);
  // AmiBroker "Ask for parameters of newly inserted indicators": a chip added after the first render opens its Parameters window.
  const [openKey, setOpenKey] = useState<string | null>(null);
  const seenKeys = useRef(new Set(chips.map((c) => c.key)));
  useEffect(() => {
    const fresh = chips.filter((c) => !seenKeys.current.has(c.key));
    fresh.forEach((c) => seenKeys.current.add(c.key));
    if (fresh.length) setOpenKey(fresh[fresh.length - 1].key);
    else if (openKey && !chips.some((c) => c.key === openKey)) setOpenKey(null);
  }, [chips]); // eslint-disable-line react-hooks/exhaustive-deps
  const themeKey = useThemeKey();
  const [ohlcError, setOhlcError] = useState<unknown>(null);
  const [chipErrors, setChipErrors] = useState<Record<string, unknown>>({});
  const [provisionalNote, setProvisionalNote] = useState<string | undefined>();
  const [foreignNotes,setForeignNotes]=useState<string[]>([]);
  const [adaptations, setAdaptations] = useState<string[]>([]);
  // Prices depend only on market+ticker, so chip edits reuse the in-flight or finished OHLC requests.
  const ohlcCache = useRef<{ key: string; p: Promise<PromiseSettledResult<SeriesResponse>[]> } | null>(null);
  // Indicator series by query: a look change or a value seen before redraws without a request (failed requests are not kept).
  const seriesCache = useRef<{ key: string; map: Map<string, Promise<SeriesResponse>> }>({ key: "", map: new Map() });
  const cachedSeries = (q: { id: string; tf: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }) => {
    const scope = `${market}|${ticker}`;
    if (seriesCache.current.key !== scope) seriesCache.current = { key: scope, map: new Map() };
    const map = seriesCache.current.map, k = JSON.stringify([q.id, q.formula, q.name, q.tf, q.vs, Object.entries(q.p ?? {}).sort(([a], [b]) => a.localeCompare(b))]);
    let hit = map.get(k);
    if (!hit) { hit = loadSeries(q); map.set(k, hit); hit.catch(() => map.get(k) === hit && map.delete(k)); }
    return hit;
  };
  const find = (id: string) => catalogue.indicators.find((i) => i.id === id);
  const pairProblem = (chip: Chip) => !isPair(chip.id) ? undefined
    : !foreignAdvertised(catalogue) ? "Comparison instruments are not advertised by this engine yet."
    : tf !== "D" ? "Pair indicators use daily (D) values only; switch the chart to Daily."
    : !chip.vs ? "Choose a comparison instrument to draw this pair."
    : !comparisonOptions.some((o) => o.ticker === chip.vs && !o.disabled) ? `${chip.vs} is not available in this market; choose another comparison.` : undefined;
  const comparisonsKey = JSON.stringify(comparisonOptions.map((o) => [o.ticker, o.disabled]));
  const chipsKey = JSON.stringify(chips.map((c) => [c.key, c.id, c.vs, c.looks, Object.entries(c.params).sort(([a], [b]) => a.localeCompare(b))]));

  useEffect(() => {
    let live = true;
    const disposers: Disposable[] = [];
    let unsync: (() => void) | undefined;
    const drawn = chips.flatMap((chip, index) => {
      const ind = find(chip.id);
      return ind?.computed && !pairProblem(chip) ? [{ chip, ind, index }] : [];
    });
    const cacheKey = `${market}|${ticker}`;
    if (ohlcCache.current?.key !== cacheKey) {
      const entry = { key: cacheKey, p: Promise.allSettled(PRICE_IDS.map((id) => loadSeries({ id, tf: "D" }))) };
      ohlcCache.current = entry;
      // A failed part is not kept, so the next run retries the prices.
      void entry.p.then((parts) => { if (parts.some((r) => r.status === "rejected") && ohlcCache.current === entry) ohlcCache.current = null; });
    }
    const ohlcLoad = ohlcCache.current.p;
    void (async () => {
      const [ohlcParts, chipParts] = await Promise.all([
        ohlcLoad,
        Promise.allSettled(drawn.map(({ chip, ind }) => cachedSeries({ id: chip.id, tf, p: chip.params, vs: chip.vs, ...formulaQuery(ind) }))),
      ]);
      if (!live) return;
      const failedOhlc = ohlcParts.find((r) => r.status === "rejected");
      setOhlcError(failedOhlc ? (failedOhlc as PromiseRejectedResult).reason : null);
      setProvisionalNote(ohlcParts.flatMap((r) => (r.status === "fulfilled" ? [r.value.provisional_note] : []))[0]);
      setChipErrors(Object.fromEntries(chipParts.flatMap((r, i) => (r.status === "rejected" ? [[drawn[i].chip.key, r.reason]] : []))));
      setForeignNotes([...new Set([...ohlcParts,...chipParts].flatMap(r=>r.status==="fulfilled"?r.value.foreign?.map(foreignDescription)??[]:[]))]);
      setAdaptations([...new Set([...ohlcParts, ...chipParts].flatMap((r) => r.status === "fulfilled" ? r.value.adaptations ?? [] : []))]);
      if (failedOhlc || !priceRef.current || !panesRef.current) return;
      const [o, h, l, c] = ohlcParts.map((r) => (r as PromiseFulfilledResult<SeriesResponse>).value);
      const daily = ohlcFromSeries({ open: o, high: h, low: l, close: c });
      const px = aggregate(daily, tf);
      const keep = tf === "D" ? undefined : new Set(px.d);
      const price = createLabChart(priceRef.current, 420);
      if (!price) return;
      disposers.push(price);
      const { chart, colors } = price;
      // The house renderer takes daily bar indices, including every scale-in leg.
      // Unknown dates remain unmapped: never invent a price bar for an action.
      const dateIndex = new Map(daily.d.map((date, i) => [date, i]));
      let period = 0;
      const barOf = daily.d.map((date) => {
        while (period < px.d.length - 1 && px.d[period] < date) period++;
        return period;
      });
      // Trade spans sit behind the price on their own hidden scale: green won, red lost, grey still open.
      if (trades?.length && chart.addHistogramSeries) {
        const shade = tradeSpans(trades, dateIndex, barOf, px.d.length, (t) => tint(t.open ? colors.ink3 : (t.ret ?? 0) >= 0 ? colors.green : colors.red, 0.08));
        const spans = chart.addHistogramSeries({ priceScaleId: "lab-trades", priceLineVisible: false, lastValueVisible: false });
        chart.priceScale?.("lab-trades").applyOptions({ scaleMargins: { top: 0, bottom: 0 }, visible: false });
        spans.setData(px.d.map((time, i) => (shade[i] ? { time, value: 1, color: shade[i] } : { time })));
      }
      // TradingView-style candles at every range: filled bodies, thin wicks, up green, down red.
      const priceSeries = chart.addCandlestickSeries({
        upColor: colors.green, borderUpColor: colors.green, wickUpColor: colors.green,
        downColor: colors.red, borderDownColor: colors.red, wickDownColor: colors.red, priceLineVisible: false,
      });
      priceSeries.setData(candles(px));
      const markers: Marker[] = [];
      const readouts: { box: HTMLElement | null; label: string; color: string; values: Map<string, number | null> }[] = [];
      const charts: SyncedPane[] = [{ chart, el: priceRef.current, series: priceSeries }];
      drawn.forEach(({ chip, ind, index }, i) => {
        const part = chipParts[i];
        if (part.status !== "fulfilled") return;
        const res = part.value;
        let target = ind.placement === "price" ? chart : null;
        const host = target ? undefined : [...panesRef.current!.querySelectorAll<HTMLElement>("[data-lab-pane]")].find((el) => el.dataset.labPane === chip.key);
        // each pane reads out in its own legend, the price legend keeps the overlays
        const box = target ? legendRef.current : host?.parentElement?.querySelector<HTMLElement>(".lab-legend-tv") ?? null;
        const settings = chipValues(ind, chip.params).join(", ");
        res.outputs.forEach((out, k) => {
          const values = res.series[out] ?? [];
          const color = plotPaint(plotColor(chip, index, out, k), colors), look = LINE_LOOK[chip.looks?.[out]?.style ?? "line"];
          if (isBooleanOutput(values)) {
            markers.push(...booleanMarkers(res.dates, values, `${ind.name}${res.outputs.length > 1 ? "." + out : ""}`, color, keep));
            return;
          }
          readouts.push({ box, label: `${shortName(ind)}${res.outputs.length > 1 ? "." + out : ""}${settings ? " " + settings : ""}`, color, values: new Map(res.dates.map((d, k) => [d, values[k]])) });
          if (!target) {
            // A pane chart is created only when the chip has a line to draw; boolean-only chips mark the price chart.
            const pane = host && createLabChart(host, 150);
            if (!pane) return;
            disposers.push(pane);
            target = pane.chart;
            const line = target.addLineSeries({ color, ...look, priceLineVisible: false, lastValueVisible: false });
            line.setData(linePoints(res.dates, values, keep));
            charts.push({ chart: pane.chart, el: host, series: line });
            return;
          }
          target.addLineSeries({ color, ...look, priceLineVisible: false, lastValueVisible: false }).setData(linePoints(res.dates, values, keep));
        });
      });
      const actionTrades = (trades ?? []).map((trade) => ({
        dir: trade.dir, open: trade.open, exit_i: dateIndex.get(trade.exit) ?? -1,
        legs: (trade.legs?.length ? trade.legs : [[trade.entry, trade.entry_px ?? 0, trade.weight ?? 1] as [string, number, number]])
          .map(([date, price, weight]): [number, number, number] => [dateIndex.get(date) ?? -1, price, weight]),
      }));
      const actions = window.VaultTruth?.actionMarkers?.(actionTrades, px.d, barOf, { buy: colors.green, sell: colors.red }) ?? [];
      const priceDates = new Set(px.d);
      markers.push(...actions.filter((marker) => priceDates.has(marker.time)));
      const last = px.d[px.d.length - 1];
      if (keep && last) markers.push({ time: last, position: "aboveBar", shape: "square", color: colors.ink3, text: "in progress" });
      priceSeries.setMarkers?.(markers.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)));
      const from = rangeStart(px.d, range) ?? px.d[0]; // "All" is an explicit range too, so panes with different warm-up start together
      for (const { chart: ch } of charts) {
        if (from && last) ch.timeScale().setVisibleRange({ from, to: last });
        else ch.timeScale().fitContent();
      }
      // One date axis, under the bottom chart (AmiBroker / TradingView stacked panes).
      charts.forEach(({ chart: ch }, i) => ch.applyOptions({ timeScale: { visible: i === charts.length - 1 } }));
      unsync = syncPanes(charts);
      // TradingView-style legend: fixed top-left, last bar by default, the hovered bar while the crosshair is on the chart.
      const at = new Map(px.d.map((d, i) => [d, i]));
      const num = (v: number) => v.toFixed(2);
      const legend = (time: string) => {
        const box = legendRef.current, i = at.get(time);
        if (!box || i === undefined) return;
        const prev = i > 0 ? px.c[i - 1] : px.o[i], change = px.c[i] - prev, tone = change >= 0 ? colors.green : colors.red;
        const line = (parts: [string, string?][]) => {
          const row = document.createElement("div");
          for (const [text, color] of parts) { const span = document.createElement("span"); span.textContent = text; if (color) span.style.color = color; row.append(span); }
          return row;
        };
        const rows = (target: HTMLElement | null) => readouts.flatMap(({ box: b, label, color, values }) => {
          const v = values.get(time);
          return b !== target || v == null || !Number.isFinite(v) ? [] : [line([[label + "  ", color], [num(v), color]])];
        });
        box.replaceChildren(
          line([[`${ticker} · ${TF_NAMES[tf] ?? tf} · ${time}  `], ["O ", colors.ink2], [num(px.o[i]) + "  ", tone], ["H ", colors.ink2], [num(px.h[i]) + "  ", tone],
            ["L ", colors.ink2], [num(px.l[i]) + "  ", tone], ["C ", colors.ink2], [num(px.c[i]) + "  ", tone], [`${change >= 0 ? "+" : ""}${num(change)} (${change >= 0 ? "+" : ""}${((change / prev) * 100).toFixed(2)}%)`, tone]]),
          ...rows(box),
        );
        for (const pane of new Set(readouts.map((r) => r.box))) if (pane && pane !== box) pane.replaceChildren(...rows(pane));
      };
      if (last) legend(last);
      for (const { chart: ch } of charts) ch.subscribeCrosshairMove?.((param) => legend(param.time && at.has(param.time) ? param.time : last));
    })();
    return () => { live = false; unsync?.(); disposers.forEach((d) => d.dispose()); legendRef.current?.replaceChildren(); panesRef.current?.querySelectorAll(".lab-legend-tv").forEach((el) => el.replaceChildren()); };
    // loadSeries and find are read fresh per run; chips are tracked by their canonical key.
  }, [market, ticker, tf, range, chipsKey, trades, themeKey, catalogue, comparisonsKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const panes = chips.filter((c) => find(c.id)?.placement === "pane" && !pairProblem(c));
  return (
    <section className="box lab-workbench">
      <div className="box-h">{ticker} · {TF_NAMES[tf] ?? tf} bars <small>{COPY.chartHint}</small></div>
      {isProvisional(market) && <ProvisionalStrip note={provisionalNote} instrumentLabel={instrumentLabel} />}
      {chips.length ? (
        <div className="lab-chips">
          {chips.map((chip, index) => (
            <ChipView key={chip.key} chip={chip} index={index} catalogue={catalogue} error={chipErrors[chip.key]} market={market}
              instrumentLabel={instrumentLabel} onRemove={onChipRemove} pairProblem={pairProblem(chip)} open={openKey === chip.key} onOpen={() => setOpenKey(chip.key)} />
          ))}
        </div>
      ) : <p className="cap">{COPY.noChips}</p>}
      {ohlcError !== null && <ErrorNotice error={ohlcError} market={market} instrumentLabel={instrumentLabel} />}
      <div className="lab-stage">
        <div hidden={ohlcError !== null}>
          <div className="lab-price">
            <div className="lwc px" ref={priceRef} data-lab-chart role="img" aria-label={`${ticker} price chart`} />
            <div className="lab-legend-tv" ref={legendRef} aria-hidden="true" />
          </div>
          <div ref={panesRef}>
            {panes.map((c) => (
              <div key={c.key} className="lab-pane-box">
                <div className="lab-pane" data-lab-pane={c.key} role="img" aria-label={`${find(c.id)?.name ?? c.id} chart`} />
                <div className="lab-legend-tv" aria-hidden="true" />
              </div>
            ))}
          </div>
        </div>
        {(() => {
          const index = chips.findIndex((c) => c.key === openKey), chip = chips[index], ind = chip && find(chip.id);
          if (!chip || !ind) return null;
          const close = () => { setOpenKey(null); document.querySelector<HTMLElement>(`[data-lab-chip="${chip.key}"] .lab-chip-open`)?.focus(); };
          // keyed by chip: another chip's window starts fresh (section open, focus, typed drafts)
          return <ParamsDialog key={chip.key} name={ind.name} indicator={ind} params={chip.params} looks={chip.looks} index={index} vs={chip.vs}
            comparisonOptions={comparisonOptions} showComparison={isPair(chip.id) && foreignAdvertised(catalogue)}
            onParams={(params) => onChipChange(chip.key, params)} onLook={(looks) => onChipLook?.(chip.key, looks)} onVs={(vs) => onChipVsChange?.(chip.key, vs)} onClose={close} />;
        })()}
      </div>
      <p className="cap">{COPY.chartNote}</p>
      {foreignNotes.map(note=><p className="cap" role="note" key={note}>{note}</p>)}
      {adaptations.map((note) => <p className="rnote" role="note" data-lab-adaptations key={note}>{note}</p>)}
      {tf !== "D" && <p className="cap">{COPY.inProgress(tf)}</p>}
      {!!trades?.length && (
        <p className="cap">
          <span style={{ color: "var(--green)" }}>↑ Buy</span> · <span style={{ color: "var(--red)" }}>↓ Sell</span>. Arrows show backtest actions; smaller arrows show added legs. Shaded spans are trades: green won, red lost, grey still open.{tf !== "D" && " Trades run on daily bars and are mapped onto this timeframe."}
        </p>
      )}
    </section>
  );
}

function ChipView({ chip, index, catalogue, error, market, instrumentLabel, onRemove, pairProblem, open, onOpen }: {
  chip: Chip; index: number; catalogue: Catalogue; error: unknown; market: SavedMarket; instrumentLabel?: string;
  onRemove(key: string): void; pairProblem?: string; open: boolean; onOpen(): void;
}) {
  const ind = catalogue.indicators.find((i) => i.id === chip.id);
  const name = ind?.name ?? chip.id;
  const values = [...(ind ? chipValues(ind, chip.params) : []), ...(chip.vs ? [`vs ${chip.vs}`] : [])].join(", ");
  return (
    <div className="chip" data-lab-chip={chip.key}>
      <button type="button" className="lab-chip-open" aria-haspopup="dialog" aria-expanded={open} title={`Parameters of ${name}`} onClick={onOpen}>
        <i className="lab-dot" style={{ background: plotCss(plotColor(chip, index, outputsOf(chip, catalogue)[0], 0)), opacity: chip.looks?.[outputsOf(chip, catalogue)[0]]?.style === "hidden" ? 0.35 : 1 }} aria-hidden="true" />
        <b>{ind ? shortName(ind) : chip.id}</b>{values && <span className="lab-chip-vals">{values}</span>}
      </button>
      {ind && !ind.computed && <span className="cap">{ind.note}</span>}
      {pairProblem && <span className="cap" role="note">{pairProblem}</span>}
      <button type="button" aria-label={`Remove ${name}`} onClick={() => onRemove(chip.key)}>×</button>
      {error !== undefined && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel} />}
    </div>
  );
}
