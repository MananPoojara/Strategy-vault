import { useEffect, useRef, useState } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, ohlcFromSeries, rangeStart } from "./bars";
import { createLabChart, PALETTE, useThemeKey } from "./charts";
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
type Marker = { time: string; position: string; shape: string; color: string; text: string };
type Disposable = NonNullable<ReturnType<typeof createLabChart>>;

export function Workbench({ market, ticker, tf, range, catalogue, chips, trades, loadSeries, onChipChange, onChipRemove, instrumentLabel }: WorkbenchProps) {
  const priceRef = useRef<HTMLDivElement>(null);
  const panesRef = useRef<HTMLDivElement>(null);
  const themeKey = useThemeKey();
  const [ohlcError, setOhlcError] = useState<unknown>(null);
  const [chipErrors, setChipErrors] = useState<Record<string, unknown>>({});
  const find = (id: string) => catalogue.indicators.find((i) => i.id === id);
  const chipsKey = JSON.stringify(chips.map((c) => [c.key, c.id, Object.entries(c.params).sort(([a], [b]) => a.localeCompare(b))]));

  useEffect(() => {
    let live = true;
    const disposers: Disposable[] = [];
    const drawn = chips.flatMap((chip, index) => {
      const ind = find(chip.id);
      return ind?.computed ? [{ chip, ind, index }] : [];
    });
    void (async () => {
      const [ohlcParts, chipParts] = await Promise.all([
        Promise.allSettled(PRICE_IDS.map((id) => loadSeries({ id, tf: "D" }))),
        Promise.allSettled(drawn.map(({ chip }) => loadSeries({ id: chip.id, tf, p: chip.params }))),
      ]);
      if (!live) return;
      const failedOhlc = ohlcParts.find((r) => r.status === "rejected");
      setOhlcError(failedOhlc ? (failedOhlc as PromiseRejectedResult).reason : null);
      setChipErrors(Object.fromEntries(chipParts.flatMap((r, i) => (r.status === "rejected" ? [[drawn[i].chip.key, r.reason]] : []))));
      if (failedOhlc || !priceRef.current || !panesRef.current) return;
      const [o, h, l, c] = ohlcParts.map((r) => (r as PromiseFulfilledResult<SeriesResponse>).value);
      const px = aggregate(ohlcFromSeries({ open: o, high: h, low: l, close: c }), tf);
      const keep = tf === "D" ? undefined : new Set(px.d);
      const price = createLabChart(priceRef.current, 420);
      if (!price) return;
      disposers.push(price);
      const { chart, colors } = price;
      const candleSeries = chart.addCandlestickSeries({
        upColor: colors.green, borderUpColor: colors.green, wickUpColor: colors.green,
        downColor: colors.red, borderDownColor: colors.red, wickDownColor: colors.red,
      });
      candleSeries.setData(candles(px));
      const markers: Marker[] = [];
      const charts = [chart];
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
            charts.push(pane.chart);
            target = pane.chart;
          }
          target.addLineSeries({ color, lineWidth: 2, priceLineVisible: false, lastValueVisible: false }).setData(linePoints(res.dates, values, keep));
        });
      });
      const toPeriod = (date: string) => (!keep || keep.has(date) ? date : px.d.find((d) => d >= date));
      for (const t of trades ?? []) {
        const entry = toPeriod(t.entry);
        if (entry) markers.push({ time: entry, position: "belowBar", shape: "arrowUp", color: colors.green, text: t.dir === 1 ? "Long" : "Short" });
        const exit = t.open ? undefined : toPeriod(t.exit);
        if (exit) markers.push({ time: exit, position: "aboveBar", shape: "arrowDown", color: colors.red, text: "Exit" });
      }
      const last = px.d[px.d.length - 1];
      if (keep && last) markers.push({ time: last, position: "aboveBar", shape: "square", color: colors.ink3, text: "in progress" });
      candleSeries.setMarkers?.(markers.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)));
      const from = rangeStart(px.d, range);
      for (const ch of charts) {
        if (from && last) ch.timeScale().setVisibleRange({ from, to: last });
        else ch.timeScale().fitContent();
      }
    })();
    return () => { live = false; disposers.forEach((d) => d.dispose()); };
    // loadSeries and find are read fresh per run; chips are tracked by their canonical key.
  }, [market, ticker, tf, range, chipsKey, trades, themeKey, catalogue]); // eslint-disable-line react-hooks/exhaustive-deps

  const panes = chips.filter((c) => find(c.id)?.placement === "pane");
  return (
    <section className="box lab-workbench">
      <div className="box-h">{ticker} · {TF_NAMES[tf] ?? tf} bars <small>{COPY.chartHint}</small></div>
      {isProvisional(market) && <ProvisionalStrip instrumentLabel={instrumentLabel} />}
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
        <div className="lwc px" ref={priceRef} data-lab-chart />
        <div ref={panesRef}>
          {panes.map((c) => <div key={c.key} className="lab-pane" data-lab-pane={c.key} />)}
        </div>
      </div>
      <p className="cap">{COPY.chartNote}</p>
      {tf !== "D" && <p className="cap">{COPY.inProgress(tf)}</p>}
      {!!trades?.length && (
        <p className="cap">
          Shaded markers are trades from the backtest below.{tf !== "D" && " Trades run on daily bars and are mapped onto this timeframe."}
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
    <span className="chip" data-lab-chip={chip.key}>
      <i className="lab-dot" style={{ background: `var(--${PALETTE[index % PALETTE.length]})` }} aria-hidden="true" />
      {name}
      {ind?.params.map((p) => (p.key === "variant" ? (
        <select key={p.key} aria-label={`${name} ${p.label}`} value={chip.params[p.key] ?? p.default} onChange={(e) => set(p.key, Number(e.target.value))}>
          <option value={0}>TradingView</option>
          <option value={1}>AmiBroker</option>
        </select>
      ) : (
        <ParamInput key={p.key} param={p} name={name} value={chip.params[p.key] ?? p.default} onValid={(v) => set(p.key, v)} />
      )))}
      <button type="button" aria-label={`Remove ${name}`} onClick={() => onRemove(chip.key)}>×</button>
      {error !== undefined && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel} />}
    </span>
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
