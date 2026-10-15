import { useEffect, useRef, useState } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, ohlcFromSeries, rangeStart } from "./bars";
import { createLabChart, PALETTE, syncPanes, useThemeKey, type SyncedPane } from "./charts";
import { COPY } from "./labels";
import type { Catalogue, ChartRange, IndicatorParam, SavedMarket, SeriesResponse, Trade } from "./types";
import { ErrorNotice, isProvisional, ProvisionalStrip } from "./ui";
import "./workbench.css";

export interface Chip { key: string; id: string; params: Record<string, number> }
export interface WorkbenchProps {
  market: SavedMarket; ticker: string; tf: string; range: ChartRange; catalogue: Catalogue; chips: Chip[];
  trades?: Trade[];
  loadSeries(q: { id: string; tf: string; p?: Record<string, number> }): Promise<SeriesResponse>;
  onChipChange(key: string, params: Record<string, number>): void;
  onChipRemove(key: string): void;
  instrumentLabel?: string;
}

const TF_NAMES: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly" };
const PRICE_IDS = ["open", "high", "low", "close"] as const;
type Marker = { time: string; position: string; shape: string; color: string; text?: string; size?: number };
type Disposable = NonNullable<ReturnType<typeof createLabChart>>;

export function Workbench({ market, ticker, tf, range, catalogue, chips, trades, loadSeries, onChipChange, onChipRemove, instrumentLabel }: WorkbenchProps) {
  const priceRef = useRef<HTMLDivElement>(null);
  const panesRef = useRef<HTMLDivElement>(null);
  const themeKey = useThemeKey();
  const [ohlcError, setOhlcError] = useState<unknown>(null);
  const [chipErrors, setChipErrors] = useState<Record<string, unknown>>({});
  const [provisionalNote, setProvisionalNote] = useState<string | undefined>();
  const [adaptations, setAdaptations] = useState<string[]>([]);
  // Prices depend only on market+ticker, so chip edits reuse the in-flight or finished OHLC requests.
  const ohlcCache = useRef<{ key: string; p: Promise<PromiseSettledResult<SeriesResponse>[]> } | null>(null);
  const find = (id: string) => catalogue.indicators.find((i) => i.id === id);
  const chipsKey = JSON.stringify(chips.map((c) => [c.key, c.id, Object.entries(c.params).sort(([a], [b]) => a.localeCompare(b))]));

  useEffect(() => {
    let live = true;
    const disposers: Disposable[] = [];
    let unsync: (() => void) | undefined;
    const drawn = chips.flatMap((chip, index) => {
      const ind = find(chip.id);
      return ind?.computed ? [{ chip, ind, index }] : [];
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
        Promise.allSettled(drawn.map(({ chip }) => loadSeries({ id: chip.id, tf, p: chip.params }))),
      ]);
      if (!live) return;
      const failedOhlc = ohlcParts.find((r) => r.status === "rejected");
      setOhlcError(failedOhlc ? (failedOhlc as PromiseRejectedResult).reason : null);
      setProvisionalNote(ohlcParts.flatMap((r) => (r.status === "fulfilled" ? [r.value.provisional_note] : []))[0]);
      setChipErrors(Object.fromEntries(chipParts.flatMap((r, i) => (r.status === "rejected" ? [[drawn[i].chip.key, r.reason]] : []))));
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
      const candleSeries = chart.addCandlestickSeries({
        upColor: colors.green, borderUpColor: colors.green, wickUpColor: colors.green,
        downColor: colors.red, borderDownColor: colors.red, wickDownColor: colors.red, priceLineVisible: false,
      });
      candleSeries.setData(candles(px));
      const markers: Marker[] = [];
      const charts: SyncedPane[] = [{ chart, el: priceRef.current, series: candleSeries }];
      drawn.forEach(({ chip, ind, index }, i) => {
        const part = chipParts[i];
        if (part.status !== "fulfilled") return;
        const res = part.value;
        const color = colors[PALETTE[index % PALETTE.length]];
        let target = ind.placement === "price" ? chart : null;
        res.outputs.forEach((out) => {
          const values = res.series[out] ?? [];
          if (isBooleanOutput(values)) {
            markers.push(...booleanMarkers(res.dates, values, `${ind.name}${res.outputs.length > 1 ? "." + out : ""}`, color, keep));
            return;
          }
          if (!target) {
            // A pane chart is created only when the chip has a line to draw; boolean-only chips mark the price chart.
            const host = ([...panesRef.current!.children] as HTMLElement[]).find((el) => el.dataset.labPane === chip.key);
            const pane = host && createLabChart(host, 150);
            if (!pane) return;
            disposers.push(pane);
            target = pane.chart;
            const line = target.addLineSeries({ color, lineWidth: 2, priceLineVisible: false, lastValueVisible: false });
            line.setData(linePoints(res.dates, values, keep));
            charts.push({ chart: pane.chart, el: host, series: line });
            return;
          }
          target.addLineSeries({ color, lineWidth: 2, priceLineVisible: false, lastValueVisible: false }).setData(linePoints(res.dates, values, keep));
        });
      });
      // The house renderer takes daily bar indices, including every scale-in leg.
      // Unknown dates remain unmapped: never invent a price bar for an action.
      const dateIndex = new Map(daily.d.map((date, i) => [date, i]));
      let period = 0;
      const barOf = daily.d.map((date) => {
        while (period < px.d.length - 1 && px.d[period] < date) period++;
        return period;
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
      candleSeries.setMarkers?.(markers.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)));
      const from = rangeStart(px.d, range) ?? px.d[0]; // "All" is an explicit range too, so panes with different warm-up start together
      for (const { chart: ch } of charts) {
        if (from && last) ch.timeScale().setVisibleRange({ from, to: last });
        else ch.timeScale().fitContent();
      }
      unsync = syncPanes(charts);
    })();
    return () => { live = false; unsync?.(); disposers.forEach((d) => d.dispose()); };
    // loadSeries and find are read fresh per run; chips are tracked by their canonical key.
  }, [market, ticker, tf, range, chipsKey, trades, themeKey, catalogue]); // eslint-disable-line react-hooks/exhaustive-deps

  const panes = chips.filter((c) => find(c.id)?.placement === "pane");
  return (
    <section className="box lab-workbench">
      <div className="box-h">{ticker} · {TF_NAMES[tf] ?? tf} bars <small>{COPY.chartHint}</small></div>
      {isProvisional(market) && <ProvisionalStrip note={provisionalNote} instrumentLabel={instrumentLabel} />}
      {chips.length ? (
        <div className="lab-chips">
          {chips.map((chip, index) => (
            <ChipView key={chip.key} chip={chip} index={index} catalogue={catalogue} error={chipErrors[chip.key]} market={market}
              instrumentLabel={instrumentLabel} onChange={onChipChange} onRemove={onChipRemove} />
          ))}
        </div>
      ) : <p className="cap">{COPY.noChips}</p>}
      {ohlcError !== null && <ErrorNotice error={ohlcError} market={market} instrumentLabel={instrumentLabel} />}
      <div hidden={ohlcError !== null}>
        <div className="lwc px" ref={priceRef} data-lab-chart role="img" aria-label={`${ticker} price chart`} />
        <div ref={panesRef}>
          {panes.map((c) => (
            <div key={c.key} className="lab-pane" data-lab-pane={c.key} role="img" aria-label={`${find(c.id)?.name ?? c.id} chart`} />
          ))}
        </div>
      </div>
      <p className="cap">{COPY.chartNote}</p>
      {adaptations.map((note) => <p className="rnote" role="note" data-lab-adaptations key={note}>{note}</p>)}
      {tf !== "D" && <p className="cap">{COPY.inProgress(tf)}</p>}
      {!!trades?.length && (
        <p className="cap">
          <span style={{ color: "var(--green)" }}>↑ Buy</span> · <span style={{ color: "var(--red)" }}>↓ Sell</span>. Arrows show backtest actions; smaller arrows show added legs.{tf !== "D" && " Trades run on daily bars and are mapped onto this timeframe."}
        </p>
      )}
    </section>
  );
}

function ChipView({ chip, index, catalogue, error, market, instrumentLabel, onChange, onRemove }: {
  chip: Chip; index: number; catalogue: Catalogue; error: unknown; market: SavedMarket; instrumentLabel?: string;
  onChange(key: string, params: Record<string, number>): void; onRemove(key: string): void;
}) {
  const ind = catalogue.indicators.find((i) => i.id === chip.id);
  const name = ind?.name ?? chip.id;
  const set = (k: string, value: number) => onChange(chip.key, { ...chip.params, [k]: value });
  return (
    <div className="chip" data-lab-chip={chip.key}>
      <i className="lab-dot" style={{ background: `var(--${PALETTE[index % PALETTE.length]})` }} aria-hidden="true" />
      {name}
      {ind && !ind.computed && <span className="cap">{ind.note}</span>}
      {ind?.computed && ind.params.map((p) => (p.key === "variant" ? (
        <select key={p.key} aria-label={`${name} ${p.label}`} value={chip.params[p.key] ?? p.default} onChange={(e) => set(p.key, Number(e.target.value))}>
          <option value={0}>TradingView</option>
          <option value={1}>AmiBroker</option>
        </select>
      ) : (
        <ParamInput key={p.key} param={p} name={name} value={chip.params[p.key] ?? p.default} onValid={(v) => set(p.key, v)} />
      )))}
      <button type="button" aria-label={`Remove ${name}`} onClick={() => onRemove(chip.key)}>×</button>
      {error !== undefined && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel} />}
    </div>
  );
}

/** Keeps the typed text locally so an out-of-range draft does not snap back; only valid values reach the page. */
function ParamInput({ param, name, value, onValid }: { param: IndicatorParam; name: string; value: number; onValid(v: number): void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  return (
    <input type="number" aria-label={`${name} ${param.label}`} min={param.min} max={param.max} step={param.step ?? "any"} value={text}
      onChange={(e) => {
        setText(e.target.value);
        const raw = e.target.value.trim() === "" ? NaN : Number(e.target.value);
        const v = param.integer ? Math.round(raw) : raw;
        if (Number.isFinite(v) && v >= param.min && v <= param.max) onValid(v);
      }} />
  );
}
