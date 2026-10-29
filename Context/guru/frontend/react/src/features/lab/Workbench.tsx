import { useEffect, useRef, useState, type ReactNode } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, zonePairs, breakLines, breakOutputs, strongWeakLines, ohlcFromSeries, rangeStart, tradeSpans } from "./bars";
import { createLabChart, syncPanes, themeColors, tint, useThemeKey, type SyncedPane } from "./charts";
import type { LabChartApi, LabSeriesApi } from "./globals";
import { shortName } from "./catalogue";
import { defaultColor, isChoice, variantOptions, ParamsDialog, plotCss, plotPaint, type LineStyle, type Look } from "./ParamsDialog";
import { COPY, TF_LABELS } from "./labels";
import { formulaQuery } from "./formula";
import type { Catalogue, ChartRange, Indicator, SavedMarket, SeriesResponse, Trade } from "./types";
import { ErrorNotice, isProvisional, ProvisionalStrip } from "./ui";
import { DrawingLayer, type Drawing } from "./DrawingLayer";
import { CHART_TYPES, drawTimeless, MAX_SLOTS, nounOf, type ChartType } from "./timeless";
import { foreignDescription, foreignAdvertised, isPair } from "./foreign-model";
import type { InstrumentOption } from "./instruments";
import { ChartPaneDivider } from "./ChartPaneDivider";
import { ChartPopup } from "./ChartPopup";
import { ChartIndicatorLabel } from "./ChartIndicatorLabel";
import "./workbench.css";

/** The chart legend includes every active input, including default calculation/source choices. */
const chipValues = (ind: Indicator, params: Record<string, number>) =>
  ind.params.filter((p) => !(ind.id === "mfi" && p.key === "smoothing" && (params.variant ?? 0) === 0))
    .map((p) => isChoice(p) ? variantOptions(p).find(([value]) => value === (params[p.key] ?? p.default))?.[1] ?? String(params[p.key] ?? p.default) : params[p.key] ?? p.default);

/** `looks`: colour and style per plotted output (AmiBroker ParamColor / ParamStyle per Plot). */
/** `tf`: the indicator's own timeframe (weekly RSI on a daily chart); absent = the chart's. */
export interface Chip { key: string; id: string; params: Record<string, number>; vs?: string; looks?: Record<string, Look>; tf?: string; levels?: number[] }
/** Default threshold lines per oscillator (TradingView / AmiBroker conventions); a chip's own `levels` replace them ([] = none).
 *  Grouped by scale, checked against each indicator's range on NIFTY 50. Price-scale, volume and pattern panes have none. */
const levelGroups: [number[], string[]][] = [
  [[70, 30], ["rsi", "rmi", "rsx", "slow_rsi", "uo", "imi", "crsi", "aroon"]],
  [[80, 20], ["stoch", "stochrsi", "mfi", "double_stoch", "hhlls", "kdj", "preferred_stoch", "projection_osc", "rss", "sve_stochrsi", "tii"]],
  [[75, 25], ["stc"]], [[90, 10], ["dmi_stoch"]], [[68, 50, 32], ["tdi"]], [[50], ["inertia", "rvi", "qqe"]],
  [[25], ["dmi", "adxr", "dx"]], [[61.8, 38.2], ["chop"]], [[-20, -80], ["willr"]],
  [[100, -100], ["cci", "woodies_cci"]], [[50, -50], ["cmo"]], [[40, -40], ["smi", "pzo", "vzo"]], [[60, -60], ["wavetrend"]], [[80, -80], ["rci", "rci_ribbon"]],
  [[150, 50, -50, -150], ["macd_v"]], [[8, 4, -4, -8], ["value_chart"]], [[7, -7], ["szo"]], [[3, -3], ["pgo"]], [[2, 0, -2], ["zscore", "rocket_rsi", "dso"]],
  [[1.5, 0, -1.5], ["fisher"]], [[0.9, 0.2, -0.2, -0.9], ["premier_rsi", "premier_stoch"]], [[0.5, -0.5], ["ift_rsi"]],
  [[0.8, 0.2], ["laguerre_rsi", "ehlers_stoch", "ibs"]], [[0.7, 0.3], ["demarker"]], [[1, 0], ["bbpct"]], [[1], ["rwi", "vortex", "ad_ratio", "trin"]],
  [[0], ["macd", "ppo", "tsi", "roc", "mom", "ao", "trix", "kst", "coppock", "bop", "dpo", "aroon_osc", "cmf", "pvo", "bbtrend", "dosc", "repulse", "roc_bands",
    "stoch_macd", "tmo", "vfi", "vpn", "rainbow_osc", "rmo", "pfe", "di_osc", "linreg_angle", "chaikin_osc", "bbp", "accel_osc", "price_osc", "pmo", "smi_ergodic",
    "rvgi", "impulse_macd", "macd_bb", "macd_leader", "madh", "myrsi_net", "polychrome_mom", "rsih", "velocity", "vw_macd", "dmh", "efi", "eom", "mfo", "twiggs",
    "klinger", "mcclellan", "trend_strength", "apo", "fosc", "disparity", "weekly_daily_macd", "iii", "vroc", "bandpass", "cyber_cycle", "decycler_osc", "ebsw",
    "ehlers_hp", "elegant_osc", "roofing", "reflex", "trendflex", "universal_osc", "voss", "correlation_cycle", "super_passband", "swak_highpass", "rmo_ehlers",
    "reverse_ema", "fourier_series", "rvgi_ext", "cycle_trend", "wvad"]],
];
export const DEFAULT_LEVELS: Record<string, number[]> = Object.fromEntries(levelGroups.flatMap(([levels, ids]) => ids.map((id) => [id, levels])));
export const levelsOf = (chip: Chip) => chip.levels ?? DEFAULT_LEVELS[chip.id] ?? [];
/** AmiBroker ParamStyle as Lightweight Charts line options (lineStyle 0 solid, 1 dotted, 2 dashed). */
const LINE_LOOK: Record<LineStyle, Record<string, unknown>> = {
  line: { lineWidth: 2, lineStyle: 0 }, thick: { lineWidth: 4, lineStyle: 0 }, dashed: { lineWidth: 2, lineStyle: 2 }, dotted: { lineWidth: 2, lineStyle: 1 }, hidden: { visible: false },
};
const loadDrawings = (key: string): Drawing[] => { try { return JSON.parse(localStorage.getItem(key) ?? "[]"); } catch { return []; } };
const outputsOf = (chip: Chip, catalogue: Catalogue) => catalogue.indicators.find((i) => i.id === chip.id)?.outputs ?? [];
/** Output k's colour: its own look, else the k-th palette entry after the chip's. */
const plotColor = (chip: Chip, index: number, out: string | undefined, k: number) => (out && chip.looks?.[out]?.color) || defaultColor(index, k);
export interface WorkbenchProps {
  library?: ReactNode;
  toolbar?(controls: ReactNode): ReactNode; onRange?(range: ChartRange): void;
  market: SavedMarket; ticker: string; tf: string; range: ChartRange; catalogue: Catalogue; chips: Chip[];
  trades?: Trade[];
  /** `formula`/`name` are set for a "My indicators" chip. */
  loadSeries(q: { id: string; tf: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }): Promise<SeriesResponse>;
  onChipChange(key: string, params: Record<string, number>): void;
  onChipRemove(key: string): void;
  onChipLook?(key: string, looks: Record<string, Look> | undefined): void;
  instrumentLabel?: string; comparisonOptions?: readonly InstrumentOption[]; onChipVsChange?(key: string, vs?: string): void; onChipTfChange?(key: string, tf?: string): void; onChipLevels?(key: string, levels?: number[]): void;
}

const TF_NAMES: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly" };
const PRICE_IDS = ["open", "high", "low", "close"] as const;
type Marker = { time: string; position: string; shape: string; color: string; text?: string; size?: number };
type Disposable = NonNullable<ReturnType<typeof createLabChart>>;

export function Workbench({ market, ticker, tf, range, catalogue, chips, trades, loadSeries, onChipChange, onChipRemove, onChipLook, instrumentLabel, comparisonOptions = [], onChipVsChange, onChipTfChange, onChipLevels, library, toolbar, onRange }: WorkbenchProps) {
  const workspaceRef = useRef<HTMLElement>(null);
  const priceRef = useRef<HTMLDivElement>(null);
  const panesRef = useRef<HTMLDivElement>(null);
  const legendRef = useRef<HTMLDivElement>(null);
  const toLatest = useRef<() => void>(undefined);
  const [behind, setBehind] = useState(false);
  // Drawings belong to this viewer's browser, per instrument and timeframe (bar dates differ by timeframe).
  const drawKey = `lab.drawings|${market}|${ticker}|${tf}`;
  const [drawings, setDrawings] = useState<Drawing[]>(() => loadDrawings(drawKey));
  useEffect(() => setDrawings(loadDrawings(drawKey)), [drawKey]);
  const saveDrawings = (next: Drawing[]) => { setDrawings(next); try { localStorage.setItem(drawKey, JSON.stringify(next)); } catch { /* kept for this visit */ } };
  const [tool, setTool] = useState<"t" | "h" | null>(null);
  const [paneHeights, setPaneHeights] = useState<Record<string, number>>({});
  const [hiddenKeys, setHiddenKeys] = useState<Set<string>>(() => new Set());
  const hiddenRef = useRef(hiddenKeys);
  hiddenRef.current = hiddenKeys;
  const valueLabels = useRef(new Map<string, HTMLSpanElement>());
  const applyVisibility = useRef<((hidden: Set<string>) => void) | null>(null);
  useEffect(() => { applyVisibility.current?.(hiddenKeys); }, [hiddenKeys]);
  useEffect(() => { setHiddenKeys((before) => {
    const keys = new Set(chips.map((c) => c.key)), next = new Set([...before].filter((key) => keys.has(key)));
    return next.size === before.size ? before : next;
  }); }, [chips]);
  const priceApi = useRef<{ chart: LabChartApi; series: LabSeriesApi } | null>(null);
  const barTimes = useRef<string[]>([]); // the price chart's bar dates, for drawings
  const keptView = useRef<{ key: string; range: { from: string; to: string } } | null>(null);
  const [chartVersion, setChartVersion] = useState(0); // the latest bar is off screen: show TradingView's "scroll to latest" button
  // AmiBroker "Ask for parameters of newly inserted indicators": a chip added after the first render opens its Parameters window.
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [openKey, setOpenKey] = useState<string | null>(null);
  const seenKeys = useRef(new Set(chips.map((c) => c.key)));
  useEffect(() => {
    const fresh = chips.filter((c) => !seenKeys.current.has(c.key));
    fresh.forEach((c) => seenKeys.current.add(c.key));
    if (fresh.length) { setOpenKey(fresh[fresh.length - 1].key); setLibraryOpen(false); }
    else if (openKey && !chips.some((c) => c.key === openKey)) setOpenKey(null);
  }, [chips]); // eslint-disable-line react-hooks/exhaustive-deps
  const themeKey = useThemeKey();
  const [ohlcError, setOhlcError] = useState<unknown>(null);
  const [chipErrors, setChipErrors] = useState<Record<string, unknown>>({});
  const [provisionalNote, setProvisionalNote] = useState<string | undefined>();
  const [foreignNotes,setForeignNotes]=useState<string[]>([]);
  const [adaptations, setAdaptations] = useState<string[]>([]);
  // Chart type: Candles keep the date axis; the others are rebuilt from their indicator's daily outputs (params per type, set in the Parameters window).
  const [chartType, setChartType] = useState<ChartType>("candles");
  const [typeParams, setTypeParams] = useState<Partial<Record<ChartType, Record<string, number>>>>({});
  const [typeOpen, setTypeOpen] = useState(false);
  const [shown, setShown] = useState<{ slots: number; total: number } | null>(null);
  const timeless = chartType !== "candles";
  const typeKey = JSON.stringify([chartType, typeParams[chartType]]);
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
  const chipsKey = JSON.stringify(chips.map((c) => [c.key, c.id, c.vs, c.tf, c.levels, c.looks, Object.entries(c.params).sort(([a], [b]) => a.localeCompare(b))]));

  useEffect(() => {
    let live = true;
    const disposers: Disposable[] = [];
    let unsync: (() => void) | undefined;
    // a rebuild for an indicator or its settings keeps the user's zoom and scroll; a new instrument, timeframe, chart type or range preset starts fresh
    const viewKey = `${market}|${ticker}|${tf}|${range}|${chartType}`;
    // the old chart is removed here: the zoom is read first, then nothing (the » latest button included) may call it again
    const cleanup = () => {
      const shown = priceApi.current?.chart.timeScale().getVisibleRange?.();
      if (shown) keptView.current = { key: viewKey, range: shown };
      live = false; priceApi.current = null; applyVisibility.current = null; toLatest.current = undefined; unsync?.(); disposers.forEach((d) => d.dispose()); legendRef.current?.replaceChildren(); panesRef.current?.querySelectorAll(".lab-legend-tv").forEach((el) => el.replaceChildren()); };
    if (timeless) {
      // daily bars only: the engine's outputs are one value per daily bar, and a resampled event would repeat over the period
      const ind = find(chartType), params = typeParams[chartType] ?? {};
      setShown(null);
      void cachedSeries({ id: chartType, tf: "D", p: params }).then((res) => {
        if (!live || !priceRef.current || !ind) return;
        setOhlcError(null); setChipErrors({}); setForeignNotes([]); setAdaptations(res.adaptations ?? []);
        const view = drawTimeless(priceRef.current, legendRef.current, { type: chartType, name: ind.name.replace(/\s*\(bar-aligned\)$/, ""), ticker, res, params, trades, range, onBehind: setBehind });
        if (!view) return;
        disposers.push(view);
        toLatest.current = view.toLatest;
        setShown({ slots: view.slots.length, total: view.total });
      }, (e) => live && setOhlcError(e));
      return cleanup;
    }
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
        Promise.allSettled(drawn.map(({ chip, ind }) => cachedSeries({ id: chip.id, tf: chip.tf ?? tf, p: chip.params, vs: chip.vs, ...formulaQuery(ind) }))),
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
      const gridColor = themeColors(priceRef.current).rule;
      const grid = { vertLines: { visible: true, color: gridColor }, horzLines: { color: gridColor } };
      const price = createLabChart(priceRef.current, 420, { grid }, undefined, true);
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
      priceApi.current = { chart, series: priceSeries }; barTimes.current = px.d;
      setChartVersion((v) => v + 1);
      const markers: Marker[] = [];
      const markerGroups = new Map<string, Marker[]>();
      const seriesViews: { key: string; series: LabSeriesApi; visible: boolean }[] = [];
      const readouts: { key: string; out: string; box: HTMLElement | null; label: string; color: string; values: Map<string, number | null> }[] = [];
      const charts: SyncedPane[] = [{ chart, el: priceRef.current, series: priceSeries }];
      // Every line carries exactly the price bars (blank where it has no value), so the panes line up bar for bar.
      const onBars = (dates: string[], values: (number | null)[]) => {
        const at = new Map(dates.map((d, k) => [d, values[k]]));
        return linePoints(px.d, px.d.map((d) => at.get(d) ?? null));
      };
      drawn.forEach(({ chip, ind, index }, i) => {
        const part = chipParts[i];
        if (part.status !== "fulfilled") return;
        const res = part.value;
        let target = ind.placement === "price" ? chart : null;
        const host = target ? undefined : [...panesRef.current!.querySelectorAll<HTMLElement>("[data-lab-pane]")].find((el) => el.dataset.labPane === chip.key);
        // each pane reads out in its own legend, the price legend keeps the overlays
        const box = target ? legendRef.current : host?.parentElement?.querySelector<HTMLElement>(".lab-legend-tv") ?? null;
        const settings = [...chipValues(ind, chip.params), ...(chip.tf ? [TF_LABELS[chip.tf] ?? chip.tf] : [])].join(", ");
        const zones = target ? zonePairs(res.outputs) : [];
        for (const z of zones) {
          const top = res.series[`${z}_top`] ?? [], bottom = res.series[`${z}_bottom`] ?? [];
          const at = new Map(res.dates.map((d, k) => [d, [top[k], bottom[k]] as const]));
          const items = px.d.map((time) => { const [t, b] = at.get(time) ?? []; return typeof t === "number" && typeof b === "number" ? { time, top: t, bottom: b } : { time }; });
          const own = chip.looks?.[z]?.color, fill = own ? plotPaint(own, colors) // TradingView: bull / discount green, bear / premium red, equilibrium grey
            : /^(bear|premium|supply)/.test(z) ? colors.red : /^(equilibrium|eq$|mid)/.test(z) ? colors.ink3 : /block/.test(chip.id) ? colors.blue : colors.green;
          let view: { bars: readonly { x: number; originalData: { top?: number; bottom?: number } }[]; barSpacing: number; visibleRange: { from: number; to: number } | null } | null = null;
          const zone = chart.addCustomSeries?.({
            renderer: () => ({ draw: (t: { useMediaCoordinateSpace(f: (s: { context: CanvasRenderingContext2D }) => void): void }, toY: (p: number) => number | null) => t.useMediaCoordinateSpace(({ context: g }) => {
              if (!view?.visibleRange) return;
              g.fillStyle = fill; g.globalAlpha = /block/.test(chip.id) && /^bull/.test(z) ? 0.28 : 0.18;
              // one solid box per run of bars holding the same zone (per-bar slices overlap into stripes)
              const half = view.barSpacing / 2;
              for (let i = view.visibleRange.from; i < view.visibleRange.to; i++) {
                const d = view.bars[i].originalData;
                if (d.top === undefined || d.bottom === undefined) continue;
                let j = i;
                while (j + 1 < view.visibleRange.to && view.bars[j + 1].originalData.top === d.top && view.bars[j + 1].originalData.bottom === d.bottom) j++;
                const y1 = toY(d.top), y2 = toY(d.bottom);
                if (y1 !== null && y2 !== null) g.fillRect(view.bars[i].x - half, Math.min(y1, y2), view.bars[j].x - view.bars[i].x + 2 * half, Math.max(1, Math.abs(y2 - y1)));
                i = j;
              }
              g.globalAlpha = 1;
            }) }),
            update: (data: typeof view) => { view = data; },
            priceValueBuilder: (d: { top: number; bottom: number }) => [d.bottom, d.top, d.top],
            isWhitespace: (d: { top?: number }) => d.top === undefined,
            defaultOptions: () => ({ color: fill }),
          }, { priceLineVisible: false, lastValueVisible: false });
          zone?.setData(items);
          if (zone) seriesViews.push({ key: chip.key, series: zone, visible: true });
        }
        // BOS / CHoCH / EQH / EQL: TradingView's line from the swing bar to the break, dashed (dotted for equal highs/lows), labelled
        const lines = target && !keep ? [...breakLines(res.outputs, res.series, res.dates, px), ...strongWeakLines(res.outputs, res.series, res.dates, px)] : [];
        const drawnByLines = new Set([...breakOutputs(lines, res.outputs), ...(lines.some((l) => l.atEnd) ? ["trailing_high", "trailing_low", "bias"] : [])]);
        if (lines.length) {
          // LuxAlgo: swing structure (length 50) solid, internal structure (a short length) dashed
          const len = Number(chip.params?.length ?? ind.params.find((q) => q.key === "length")?.default ?? 50), internal = len < 50;
          const ts = chart.timeScale();
          const struct = chart.addCustomSeries?.({
            renderer: () => ({ draw: (t: { useMediaCoordinateSpace(f: (s: { context: CanvasRenderingContext2D }) => void): void }, toY: (p: number) => number | null) => t.useMediaCoordinateSpace(({ context: g }) => {
              g.font = "10px system-ui, sans-serif"; g.textAlign = "center"; g.lineWidth = 1;
              for (const ln of lines) {
                const x1 = ts.logicalToCoordinate?.(ln.from), x2 = ts.logicalToCoordinate?.(ln.to), y = toY(ln.price);
                if (x1 == null || x2 == null || y === null || x2 < 0 || x1 > g.canvas.width) continue;
                g.strokeStyle = g.fillStyle = ln.up ? colors.green : colors.red;
                g.setLineDash(ln.solid || (!ln.dotted && !internal) ? [] : ln.dotted ? [2, 2] : [4, 3]);
                g.beginPath(); g.moveTo(x1, y); g.lineTo(x2, y); g.stroke();
                g.textAlign = ln.atEnd ? "right" : "center";
                g.fillText(ln.text, ln.atEnd ? x2 : (x1 + x2) / 2, ln.above ? y - 4 : y + 11);
              }
              g.setLineDash([]);
            }) }),
            update: () => undefined,
            priceValueBuilder: (d: { v: number }) => [d.v],
            isWhitespace: (d: { v?: number }) => d.v === undefined,
            defaultOptions: () => ({ color: colors.ink3 }),
          }, { priceLineVisible: false, lastValueVisible: false });
          struct?.setData(px.d.map((time, i) => ({ time, v: px.c[i] })));
          if (struct) seriesViews.push({ key: chip.key, series: struct, visible: true });
        }
        res.outputs.forEach((out, k) => {
          if (drawnByLines.has(out) && isBooleanOutput(res.series[out] ?? [])) return; // drawn as the labelled line above
          // a zone's own birth / mitigation events (order block bull, bull_mitigated): the box already shows them
          if (zones.some((z) => out === z || out === `${z}_mitigated`) && isBooleanOutput(res.series[out] ?? [])) return;
          const values = res.series[out] ?? [];
          const color = plotPaint(plotColor(chip, index, out, k), colors), styled = LINE_LOOK[chip.looks?.[out]?.style ?? "line"];
          // a level set on a few bars only (sweep levels, pivots): dots, since the line would join far-apart points across the gaps
          const sparse = values.filter((v) => typeof v === "number").length < values.length * 0.25;
          const look = sparse && styled.visible !== false ? { lineVisible: false, pointMarkersVisible: true, pointMarkersRadius: 2.5 } : styled;
          if (isBooleanOutput(values)) {
            markerGroups.set(chip.key, [...(markerGroups.get(chip.key) ?? []), ...booleanMarkers(res.dates, values, `${ind.name}${res.outputs.length > 1 ? "." + out : ""}`, color, keep, out)]);
            return;
          }
          readouts.push({ key: chip.key, out, box, label: `${shortName(ind)}${res.outputs.length > 1 ? "." + out : ""}${settings ? `(${settings})` : ""}`, color, values: new Map(res.dates.map((d, k) => [d, values[k]])) });
          if (zones.some((z) => out === `${z}_top` || out === `${z}_bottom`) || drawnByLines.has(out)) return; // drawn as the zone box / line above
          // a −1 / 0 / +1 state (trend direction) is no price: on the price chart it shows in the legend only
          if (target && values.every((v) => v === null || v === -1 || v === 0 || v === 1)) return;
          if (!target) {
            // A pane chart is created only when the chip has a line to draw; boolean-only chips mark the price chart.
            const pane = host && createLabChart(host, 150, { grid }, undefined, true);
            if (!pane) return;
            disposers.push(pane);
            target = pane.chart;
            const line = target.addLineSeries({ color, ...look, priceLineVisible: false, lastValueVisible: true });
            line.setData(onBars(res.dates, values));
            seriesViews.push({ key: chip.key, series: line, visible: look.visible !== false });
            // threshold lines (RSI 70 / 30 …), dashed, with their value on the axis
            for (const price of levelsOf(chip)) line.createPriceLine?.({ price, color: colors.ink3, lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: "" });
            charts.push({ chart: pane.chart, el: host, series: line });
            return;
          }
          const line = target.addLineSeries({ color, ...look, priceLineVisible: false, lastValueVisible: true });
          line.setData(onBars(res.dates, values));
          seriesViews.push({ key: chip.key, series: line, visible: look.visible !== false });
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
      applyVisibility.current = (hidden) => {
        for (const view of seriesViews) view.series.applyOptions({ visible: view.visible && !hidden.has(view.key) });
        const visibleMarkers = [...markers, ...[...markerGroups].flatMap(([key, group]) => hidden.has(key) ? [] : group)];
        priceSeries.setMarkers?.(visibleMarkers.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)));
      };
      applyVisibility.current(hiddenRef.current);
      const from = rangeStart(px.d, range) ?? px.d[0]; // "All" is an explicit range too, so panes with different warm-up start together
      const kept = keptView.current?.key === viewKey ? keptView.current.range : null;
      for (const { chart: ch } of charts) {
        if (kept) ch.timeScale().setVisibleRange(kept);
        else if (from && last) ch.timeScale().setVisibleRange({ from, to: last });
        else ch.timeScale().fitContent();
      }
      // One date axis, under the bottom chart (AmiBroker / TradingView stacked panes).
      charts.forEach(({ chart: ch }, i) => ch.applyOptions({ timeScale: { visible: i === charts.length - 1 } }));
      unsync = syncPanes(charts);
      // Same price-axis width on every pane, else "27200.00" vs "80.00" shifts each pane's bars sideways.
      // shortcut: measured once after the first paint; a much longer price label after zooming can still differ.
      requestAnimationFrame(() => {
        if (!live) return;
        const width = Math.max(...charts.map(({ chart: ch }) => ch.priceScale?.("right").width?.() ?? 0));
        if (width > 0) for (const { chart: ch } of charts) ch.priceScale?.("right").applyOptions({ minimumWidth: Math.ceil(width) });
      });
      const priceTs = chart.timeScale(), lastBar = px.d.length - 1;
      priceTs.subscribeVisibleLogicalRangeChange?.((r) => setBehind(!!r && r.to < lastBar - 0.5));
      toLatest.current = () => {
        const r = priceTs.getVisibleLogicalRange?.();
        if (!r || !priceTs.setVisibleLogicalRange) return;
        priceTs.setVisibleLogicalRange({ from: lastBar + 2 - (r.to - r.from), to: lastBar + 2 }); // same zoom, two empty bars after the latest
        const next = priceTs.getVisibleLogicalRange?.();
        if (next) for (const { chart: ch } of charts.slice(1)) ch.timeScale().setVisibleLogicalRange?.(next);
      };
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
          if (b !== target || v == null || !Number.isFinite(v)) return [];
          const row = line([[label + " = ", color], [num(v), color]]);
          row.className = "lab-legend-output";
          return [row];
        });
        box.replaceChildren(
          line([[`${ticker} · ${TF_NAMES[tf] ?? tf} · ${time.replace(/^(\d{4})-(\d{2})-(\d{2})/, "$3-$2-$1")}  `], ["O ", colors.ink2], [num(px.o[i]) + "  ", tone], ["H ", colors.ink2], [num(px.h[i]) + "  ", tone],
            ["L ", colors.ink2], [num(px.l[i]) + "  ", tone], ["C ", colors.ink2], [num(px.c[i]) + "  ", tone], [`${change >= 0 ? "+" : ""}${num(change)} (${change >= 0 ? "+" : ""}${((change / prev) * 100).toFixed(2)}%)`, tone]]),
          ...rows(box),
        );
        for (const pane of new Set(readouts.map((r) => r.box))) if (pane && pane !== box) pane.replaceChildren(...rows(pane));
        // Only replace value text: the label buttons keep focus while the crosshair moves.
        for (const [key, label] of valueLabels.current) {
          const values = readouts.filter((r) => r.key === key).flatMap((r) => {
            const value = r.values.get(time);
            if (value == null || !Number.isFinite(value)) return [];
            const span = document.createElement("span");
            span.textContent = num(value); span.style.color = r.color; span.title = r.out;
            return [span];
          });
          label.replaceChildren(...(values.length ? values : [document.createTextNode("—")]));
        }
      };
      if (last) legend(last);
      for (const { chart: ch } of charts) ch.subscribeCrosshairMove?.((param) => legend(param.time && at.has(param.time) ? param.time : last));
    })();
    return cleanup;
    // loadSeries and find are read fresh per run; chips are tracked by their canonical key.
  }, [market, ticker, tf, range, chipsKey, trades, themeKey, catalogue, comparisonsKey, typeKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const panes = timeless ? [] : chips.filter((c) => find(c.id)?.placement === "pane" && !pairProblem(c));
  // TradingView: the price chart fills the screen below the toolbar (its panes included) until its height is dragged;
  // Full screen shows the whole chart tab on the screen
  const [fit, setFit] = useState(420), [full, setFull] = useState(false);
  const panesHeight = panes.reduce((sum, c) => sum + (paneHeights[c.key] ?? 150) + 8, 0);
  useEffect(() => {
    const measure = () => {
      const el = priceRef.current, fs = !!workspaceRef.current?.closest(".lab-maximized");
      setFull(fs);
      if (!el) return;
      const top = el.getBoundingClientRect().top + (fs ? 0 : window.scrollY);
      setFit(Math.max(360, Math.round(window.innerHeight - top - panesHeight - (fs ? 48 : 72))));
    };
    measure();
    // leaving the browser's full screen (Esc) also leaves the window-filling view
    const left = () => { if (!document.fullscreenElement) maximize(false); else measure(); };
    const esc = (e: KeyboardEvent) => { if (e.key === "Escape" && !document.fullscreenElement) maximize(false); };
    window.addEventListener("resize", measure); document.addEventListener("fullscreenchange", left); window.addEventListener("keydown", esc);
    return () => { window.removeEventListener("resize", measure); document.removeEventListener("fullscreenchange", left); window.removeEventListener("keydown", esc); };
  }, [panesHeight, chips.length, ohlcError]); // eslint-disable-line react-hooks/exhaustive-deps
  // The chart tab fills the window, and the screen when the browser allows it (it needs the user's own click).
  const maximize = (on: boolean) => {
    const panel = workspaceRef.current?.closest<HTMLElement>("[role=tabpanel]") ?? workspaceRef.current;
    if (!panel || panel.classList.contains("lab-maximized") === on) return;
    panel.classList.toggle("lab-maximized", on);
    if (on) panel.requestFullscreen?.().catch(() => { /* the window-filling view stays */ });
    else if (document.fullscreenElement) void document.exitFullscreen?.().catch(() => undefined);
    window.dispatchEvent(new Event("resize"));
  };
  const toggleFull = () => maximize(!full);
  const typeName = CHART_TYPES.find(([t]) => t === chartType)?.[1] ?? chartType, typeInd = timeless ? find(chartType) : undefined;
  const panel = <>
    <div className="lab-indicator-browser" hidden={!libraryOpen}>
      <div className="lab-browser-heading"><h2>Indicators</h2><button type="button" className="dbtn sm ghost" aria-label="Close indicator browser" onClick={() => setLibraryOpen(false)}>×</button></div>
      {library}
    </div>
    {!libraryOpen && <>
      {typeInd && typeOpen && <ParamsDialog key={chartType} name={typeName} indicator={typeInd} params={typeParams[chartType] ?? {}} index={0} showComparison={false} showLooks={false}
          onParams={(p) => setTypeParams((all) => ({ ...all, [chartType]: p }))} onLook={() => undefined} onClose={() => { setTypeOpen(false); document.querySelector<HTMLButtonElement>("[data-lab-type-params]")?.focus(); }} />}
        {!(typeInd && typeOpen) && (() => {
          const index = chips.findIndex((c) => c.key === openKey), chip = chips[index], ind = chip && find(chip.id);
          if (!chip || !ind) return null;
          const close = () => {
            setOpenKey(null);
            const control = !timeless
              ? valueLabels.current.get(chip.key)?.closest('[data-lab-chart-label]')?.querySelector<HTMLElement>('button')
              : document.querySelector<HTMLElement>(`[data-lab-chip="${chip.key}"] .lab-chip-open`);
            control?.focus();
          };
          // keyed by chip: another chip's window starts fresh (section open, focus, typed drafts)
          return <ParamsDialog key={chip.key} name={ind.name} indicator={ind} params={chip.params} looks={chip.looks} index={index} vs={chip.vs} tf={chip.tf} timeframes={catalogue.timeframes} onTf={(t) => onChipTfChange?.(chip.key, t)}
            levels={ind.placement === "pane" ? levelsOf(chip) : undefined} onLevels={(l) => onChipLevels?.(chip.key, l)}
            comparisonOptions={comparisonOptions} showComparison={isPair(chip.id) && foreignAdvertised(catalogue)}
            onParams={(params) => onChipChange(chip.key, params)} onLook={(looks) => onChipLook?.(chip.key, looks)} onVs={(vs) => onChipVsChange?.(chip.key, vs)} onClose={close} />;
        })()}
    </>}
  </>;
  const panelOpen = libraryOpen || !!(typeInd && typeOpen) || chips.some((c) => c.key === openKey);
  const labelFor = (chip: Chip, index: number) => {
    const ind = find(chip.id);
    if (!ind?.computed || pairProblem(chip)) return null;
    const fullSettings = [...chipValues(ind, chip.params), ...(chip.tf ? [TF_LABELS[chip.tf] ?? chip.tf] : [])].join(" ");
    const settings = fullSettings.replace(/Wilder\/TradingView\/AmiBroker(?: after warm-up)?/g, "Wilder").replace(/Cutler\/TradeStation RSI JP/g, "Cutler") + (ind.id === "rsi" ? " close" : "");
    return <div key={chip.key} data-lab-chart-label={chip.key}><ChartIndicatorLabel name={shortName(ind)} settings={settings} settingsTitle={fullSettings}
      color={plotCss(plotColor(chip, index, ind.outputs[0], 0))} hidden={hiddenKeys.has(chip.key)}
      valuesRef={(element) => { if (element) valueLabels.current.set(chip.key, element); else valueLabels.current.delete(chip.key); }}
      onSettings={() => { closePanel(); setLibraryOpen(false); setTypeOpen(false); setOpenKey(chip.key); }}
      onToggle={() => setHiddenKeys((before) => { const next = new Set(before); if (!next.delete(chip.key)) next.add(chip.key); return next; })}
      onRemove={() => onChipRemove(chip.key)} /></div>;
  };
  const closePanel = () => {
    const cancel = workspaceRef.current?.querySelector<HTMLButtonElement>('.lab-params [aria-label="Close parameters"]');
    if (cancel) cancel.click();
    else { setLibraryOpen(false); setOpenKey(null); setTypeOpen(false); }
  };
  const fullButton = <button type="button" className="dbtn sm ghost lab-fullscreen" data-lab-fullscreen aria-pressed={full} title={full ? "Exit full screen (Esc)" : "Full screen"}
    aria-label={full ? "Exit full screen" : "Full screen"} onClick={toggleFull}><svg viewBox="0 0 24 24" aria-hidden="true">
      <path d={full ? "M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" : "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"} /></svg></button>;
  const controls = <div className="lab-chart-actions" role="toolbar" aria-label="Chart workspace">
    <label className="lab-chart-type-control"><span className="lab-control-name">Chart type</span><select aria-label="Chart type" value={chartType} onChange={(event) => { closePanel(); setChartType(event.target.value as ChartType); setTypeOpen(false); setOpenKey(null); setTool(null); }}>
      {CHART_TYPES.map(([value, label]) => <option key={value} value={value} disabled={value !== "candles" && !find(value)?.computed}>{label}</option>)}
    </select></label>
    {typeInd && <button type="button" className="dbtn sm" data-lab-type-params aria-expanded={typeOpen} onClick={() => { setLibraryOpen(false); setTypeOpen(true); }}>{typeName} settings</button>}
    {library && <button type="button" className="dbtn sm lab-add-indicator" aria-expanded={libraryOpen} onClick={() => { if (!libraryOpen) { closePanel(); setOpenKey(null); setTypeOpen(false); } setLibraryOpen(!libraryOpen); }}><svg width="19" height="19" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true"><path d="M2 17h16M4 15V9m5 6V5m5 10v-4M2 7l5-5 5 4 6-5" /></svg>Indicators</button>}
    {fullButton}
  </div>;
  return (
    <section ref={workspaceRef} className="box lab-workbench" data-indicator-display={timeless ? "chips" : "on-chart"}>
      <div className="lab-chart-caption">{timeless ? <>{ticker} · {typeName} from daily bars</> : <>{ticker} · {TF_NAMES[tf] ?? tf} bars</>}</div>
      <div className="lab-chart-topbar">{toolbar ? toolbar(controls) : controls}</div>
      {isProvisional(market) && <ProvisionalStrip note={provisionalNote} instrumentLabel={instrumentLabel} />}
      {chips.length ? (
        <div className="lab-chips">
          {chips.map((chip, index) => (
            <ChipView key={chip.key} chip={chip} index={index} catalogue={catalogue} error={chipErrors[chip.key]} market={market}
              instrumentLabel={instrumentLabel} onRemove={onChipRemove} pairProblem={pairProblem(chip)} open={!libraryOpen && !typeOpen && openKey === chip.key} onOpen={() => { setLibraryOpen(false); setTypeOpen(false); setOpenKey(chip.key); }} />
          ))}
        </div>
      ) : null}
      {ohlcError !== null && <ErrorNotice error={ohlcError} market={market} instrumentLabel={instrumentLabel} />}
      <div className="lab-chart-body">
      <div className="lab-draw lab-draw-rail" role="toolbar" aria-label="Drawing tools" hidden={ohlcError !== null || timeless}>
        <button type="button" className="dbtn sm" title="Crosshair" aria-label="Crosshair" aria-pressed={tool === null} onClick={() => setTool(null)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2v7m0 6v7M2 12h7m6 0h7" /></svg></button>
        <button type="button" className="dbtn sm" title="Trend line" aria-label="Trend line" data-lab-draw="t" aria-pressed={tool === "t"} onClick={() => setTool(tool === "t" ? null : "t")}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 19 14-14"/><circle cx="4" cy="20" r="2"/><circle cx="20" cy="4" r="2"/></svg></button>
        <button type="button" className="dbtn sm" title="Horizontal line" aria-label="Horizontal line" data-lab-draw="h" aria-pressed={tool === "h"} onClick={() => setTool(tool === "h" ? null : "h")}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2 12h20"/><circle cx="9" cy="12" r="2"/></svg></button>
        {drawings.length > 0 && <>
          <button type="button" className="dbtn sm ghost" title="Undo drawing" aria-label="Undo drawing" data-lab-draw-undo onClick={() => saveDrawings(drawings.slice(0, -1))}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="m8 5-5 5 5 5M3 10h11a6 6 0 0 1 0 12"/></svg></button>
          <button type="button" className="dbtn sm ghost" title={`Clear drawings (${drawings.length})`} aria-label={`Clear drawings (${drawings.length})`} data-lab-draw-clear onClick={() => saveDrawings([])}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7"/></svg></button>
        </>}
      </div>
        <div hidden={ohlcError !== null}>
          <div className={`lab-price${tool ? " lab-drawing" : ""}`}>
            <div className="lwc px" ref={priceRef} style={{ height: paneHeights.price ?? fit }} data-lab-chart role="img" aria-label={timeless ? `${ticker} ${typeName} chart` : `${ticker} price chart`} />
            {!timeless && priceApi.current && priceRef.current && <DrawingLayer key={chartVersion} chart={priceApi.current.chart} series={priceApi.current.series}
              times={barTimes.current} drawings={drawings} tool={tool} color={themeColors(priceRef.current).blue} height={paneHeights.price ?? fit}
              onChange={saveDrawings} onDone={() => setTool(null)} />}
            <div className="lab-legend-tv" ref={legendRef} aria-hidden="true" />
            {!timeless && <div className="lab-chart-indicators">{chips.map((chip, index) => find(chip.id)?.placement !== "pane" ? labelFor(chip, index) : null)}</div>}
            {behind && <button type="button" className="lab-latest" data-lab-latest title="Scroll to the latest bar" aria-label="Scroll to the latest bar" onClick={() => toLatest.current?.()}>»</button>}
          </div>
          <ChartPaneDivider label="Price pane height" height={paneHeights.price ?? fit} defaultHeight={fit} onChange={(height) => setPaneHeights((all) => ({ ...all, price: height }))} />
          <div ref={panesRef}>
            {panes.map((c) => (
              <div key={c.key} className="lab-pane-box">
                <button type="button" className="lab-pane-settings" aria-label={`Settings for ${find(c.id)?.name ?? c.id}`} title="Indicator settings" onClick={() => { setLibraryOpen(false); setTypeOpen(false); setOpenKey(c.key); }}>
                  <svg width="15" height="15" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="M3 5h14M3 10h14M3 15h14M7 2v6M13 7v6M8 12v6" /></svg>
                </button>
                <div className="lab-pane" style={{ height: paneHeights[c.key] ?? 150 }} data-lab-pane={c.key} role="img" aria-label={`${find(c.id)?.name ?? c.id} chart`} />
                <div className="lab-legend-tv" aria-hidden="true" />
                <div className="lab-chart-indicators">{labelFor(c, chips.findIndex((chip) => chip.key === c.key))}</div>
                <ChartPaneDivider label={`${find(c.id)?.name ?? c.id} pane height`} height={paneHeights[c.key] ?? 150} defaultHeight={150} onChange={(height) => setPaneHeights((all) => ({ ...all, [c.key]: height }))} />
              </div>
            ))}
          </div>
        </div>

      </div>
      {tool && <p className="cap lab-drawing-hint" data-lab-draw-hint>{tool === "t" ? "Press on the chart and drag to the end of the line (or click the start, then the end)" : "Move to a price and click"} · Esc cancels</p>}
      {onRange && <div className="lab-chart-footer" role="group" aria-label="Chart range">{(["1Y", "3Y", "All"] as ChartRange[]).map((value) => <button key={value} type="button" aria-pressed={range === value} onClick={() => onRange(value)}>{value}</button>)}<span>{TF_NAMES[tf] ?? tf} · {ticker}</span></div>}
      {panelOpen && <ChartPopup key={libraryOpen ? "library" : openKey ?? chartType} wide={libraryOpen} title={libraryOpen ? "Indicators" : "Indicator settings"} onClose={closePanel}>{panel}</ChartPopup>}
      {timeless ? <p className="cap" data-lab-timeless-note>
        {`${typeName} is drawn without a date axis from the engine's ${typeName} outputs on daily bars (the same values the rules use); hover a ${nounOf(chartType)} for its dates. The last ${nounOf(chartType)} can still change until the next one forms. Indicator chips, drawings and the candle size apply to Candles only.`}
        {shown && shown.total > shown.slots && ` Showing the latest ${MAX_SLOTS.toLocaleString("en-US")} of ${shown.total.toLocaleString("en-US")} ${nounOf(chartType, true)}; a larger box shows more history.`}
      </p> : <p className="cap">{COPY.chartNote}</p>}
      {foreignNotes.map(note=><p className="cap" role="note" key={note}>{note}</p>)}
      {adaptations.map((note) => <p className="rnote" role="note" data-lab-adaptations key={note}>{note}</p>)}
      {tf !== "D" && !timeless && <p className="cap">{COPY.inProgress(tf)}</p>}
      {!!trades?.length && timeless && <p className="cap"><span style={{ color: "var(--green)" }}>▲ Buy</span> · <span style={{ color: "var(--red)" }}>▼ Sell</span>. Triangles point at the real fill price, on the {nounOf(chartType)} that holds the fill date.</p>}
      {!!trades?.length && !timeless && (
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
  const values = [...(ind ? chipValues(ind, chip.params) : []), ...(chip.vs ? [`vs ${chip.vs}`] : []), ...(chip.tf ? [TF_LABELS[chip.tf] ?? chip.tf] : [])].join(", ");
  return (
    <div className="chip" data-lab-chip={chip.key}>
      <button type="button" className="lab-chip-open" aria-expanded={open} title={`Parameters of ${name}`} onClick={onOpen}>
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
