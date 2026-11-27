import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { aggregate, booleanMarkers, isBooleanOutput, linePoints, zonePairs, breakLines, breakOutputs, strongWeakLines, periodLevelLines, PERIOD_LEVELS, autoPivotPeriod, ohlcFromSeries, rangeStart, tradeSpans } from "./bars";
import { createLabChart, syncPanes, themeColors, tint, useThemeKey, type SyncedPane } from "./charts";
import type { LabChartApi, LabSeriesApi } from "./globals";
import { shortName } from "./catalogue";
import { defaultColor, isChoice, variantOptions, ParamsDialog, plotCss, plotPaint, type LineStyle, type Look } from "./ParamsDialog";
import { COPY, TF_LABELS } from "./labels";
import { formulaQuery } from "./formula";
import { RANGE_BUTTONS, type Catalogue, type ChartRange, type Indicator, type SavedMarket, type SeriesResponse, type Trade } from "./types";
import { ErrorNotice, isProvisional, ProvisionalStrip } from "./ui";
import { DrawingLayer, type Drawing, type DrawTool } from "./DrawingLayer";
import { DrawRail, useDrawPrefs } from "./drawings/rail";
import { TOOLS } from "./drawings/registry";
import type { Bars } from "./drawings/model";
import { CHART_TYPES, drawTimeless, MAX_SLOTS, nounOf, type ChartType } from "./timeless";
import { PROFILE_MODES, ProfilePrimitive, profileNeedsVolume, type ProfileMode } from "./profiles";
import { addPriceSeries, CANDLE_LIKE, isPriceStyle, PRICE_STYLES, type PriceStyle } from "./price-styles";
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
  [[75, 25], ["stc"]], [[150, 75], ["normalized_volume"]], [[90, 10], ["dmi_stoch"]], [[68, 50, 32], ["tdi"]], [[50], ["inertia", "rvi", "qqe"]],
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
    "reverse_ema", "fourier_series", "rvgi_ext", "cycle_trend", "wvad", "vol_accum_pct"]],
];
export const DEFAULT_LEVELS: Record<string, number[]> = Object.fromEntries(levelGroups.flatMap(([levels, ids]) => ids.map((id) => [id, levels])));
export const levelsOf = (chip: Chip) => chip.levels ?? DEFAULT_LEVELS[chip.id] ?? [];
/** AmiBroker ParamStyle as Lightweight Charts line options (lineStyle 0 solid, 1 dotted, 2 dashed). */
const LINE_LOOK: Record<LineStyle, Record<string, unknown>> = {
  line: { lineWidth: 2, lineStyle: 0 }, thick: { lineWidth: 4, lineStyle: 0 }, dashed: { lineWidth: 2, lineStyle: 2 }, dotted: { lineWidth: 2, lineStyle: 1 }, hidden: { visible: false },
};
const POSITIONS = "lab.positions"; // the hidden-set key of the trade markers and shading
/** What to do with the chosen tool, as the hint under the chart says it. */
const drawHint = (tool: string) => {
  const def = TOOLS[tool], n = def?.points;
  return tool === "measure" ? "Press on the chart and drag to measure (Shift+drag measures any time)" : tool === "zoom" ? "Drag over the bars to zoom in"
    : def?.hint ?? (n === 1 ? "Move to the point and click" : n === 2 ? "Press on the chart and drag to the second point (or click the first point, then the second)"
    : n === "free" ? "Press on the chart and drag to draw" : n === "multi" ? "Click each point; double-click or Enter finishes" : `Click the ${n} points`);
};
const loadDrawings = (key: string): Drawing[] => { try { return JSON.parse(localStorage.getItem(key) ?? "[]"); } catch { return []; } };
const outputsOf = (chip: Chip, catalogue: Catalogue) => catalogue.indicators.find((i) => i.id === chip.id)?.outputs ?? [];
/** Output k's colour: its own look, else the k-th palette entry after the chip's. */
const plotColor = (chip: Chip, index: number, out: string | undefined, k: number) => (out && chip.looks?.[out]?.color) || defaultColor(index, k);
export interface WorkbenchProps {
  /** Chips added with their settings already confirmed (the strategy's indicators): they do not open the Parameters window. */
  quietKeys?: ReadonlySet<string>;
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
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"], DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/** TradingView's number and date look: 25,281.85 with commas, "Sep", the crosshair date "Sun 08 Nov '26". */
export const tvPrice = (p: number) => p.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
export const tvDay = (t: string | { year: number; month: number; day: number }) => {
  const d = typeof t === "string" ? new Date(`${t}T00:00:00Z`) : new Date(Date.UTC(t.year, t.month - 1, t.day));
  return `${DAYS[d.getUTCDay()]} ${String(d.getUTCDate()).padStart(2, "0")} ${MONTHS[d.getUTCMonth()]} '${String(d.getUTCFullYear()).slice(2)}`;
};
/** TradingView's volume read-out: 192.58M, 3.21K, 1.05B. */
export const tvVolume = (v: number) => { const a = Math.abs(v); return a >= 1e9 ? `${(v / 1e9).toFixed(2)}B` : a >= 1e6 ? `${(v / 1e6).toFixed(2)}M` : a >= 1e3 ? `${(v / 1e3).toFixed(2)}K` : v.toFixed(0); };
const TV_LOOK = { localization: { locale: "en-US", priceFormatter: tvPrice, timeFormatter: tvDay } };
/** TradingView's default right margin: empty bars after the latest. */
const RIGHT_MARGIN = 10;
/** Pivots legend as TradingView names it: "Pivots Traditional Auto". */
const PIVOT_TYPES = ["Traditional", "Fibonacci", "Woodie", "Classic", "DM", "Camarilla"], PIVOT_PERIODS: Record<number, string> = { 1: "Weekly", 2: "Monthly", 3: "Quarterly", 4: "Yearly" };
type Marker = { time: string; position: string; shape: string; color: string; text?: string; size?: number };
type Disposable = NonNullable<ReturnType<typeof createLabChart>>;

export function Workbench({ quietKeys, market, ticker, tf, range, catalogue, chips, trades, loadSeries, onChipChange, onChipRemove, onChipLook, instrumentLabel, comparisonOptions = [], onChipVsChange, onChipTfChange, onChipLevels, library, toolbar, onRange }: WorkbenchProps) {
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
  const [tool, setTool] = useState<DrawTool | null>(null);
  const [prefs, setPrefs] = useDrawPrefs();
  const [zooms, setZooms] = useState<{ from: number; to: number }[]>([]); // the views before each Zoom in, for Zoom out
  const [paneHeights, setPaneHeights] = useState<Record<string, number>>({});
  const [hiddenKeys, setHiddenKeys] = useState<Set<string>>(() => new Set());
  // the rail's Hide indicators / Hide positions & orders hide on top of each chip's own eye
  const hiddenNow = useMemo(() => new Set([...(prefs.hide.indicators ? chips.map((c) => c.key) : hiddenKeys), ...(prefs.hide.positions ? [POSITIONS] : [])]),
    [hiddenKeys, prefs.hide.indicators, prefs.hide.positions, chips]);
  const hiddenRef = useRef(hiddenNow);
  hiddenRef.current = hiddenNow;
  const valueLabels = useRef(new Map<string, HTMLSpanElement>());
  const applyVisibility = useRef<((hidden: Set<string>) => void) | null>(null);
  useEffect(() => { applyVisibility.current?.(hiddenNow); }, [hiddenNow]);
  useEffect(() => { setHiddenKeys((before) => {
    const keys = new Set(chips.map((c) => c.key)), next = new Set([...before].filter((key) => keys.has(key)));
    return next.size === before.size ? before : next;
  }); }, [chips]);
  const priceApi = useRef<{ chart: LabChartApi; series: LabSeriesApi } | null>(null);
  const barTimes = useRef<string[]>([]); // the price chart's bar dates, for drawings
  const barData = useRef<Omit<Bars, "d">>({ o: [], h: [], l: [], c: [], v: [] }); // their prices and volume, for drawings and the magnet
  const keptView = useRef<{ key: string; range: { from: number; to: number } } | null>(null);
  const [chartVersion, setChartVersion] = useState(0); // the latest bar is off screen: show TradingView's "scroll to latest" button
  // TradingView's Arrow cursor has no crosshair lines (set again when a rebuild makes a new chart)
  const arrowed = useRef(false);
  useEffect(() => {
    if (prefs.cursor !== "arrow" && !arrowed.current) return;
    arrowed.current = prefs.cursor === "arrow";
    const shown = { visible: !arrowed.current };
    try { priceApi.current?.chart.applyOptions({ crosshair: { vertLine: shown, horzLine: shown } }); } catch { /* the chart was already removed by a rebuild */ }
  }, [prefs.cursor, chartVersion]);
  // AmiBroker "Ask for parameters of newly inserted indicators": a chip added after the first render opens its Parameters window.
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [openKey, setOpenKey] = useState<string | null>(null);
  const seenKeys = useRef(new Set(chips.map((c) => c.key)));
  useEffect(() => {
    const fresh = chips.filter((c) => !seenKeys.current.has(c.key));
    fresh.forEach((c) => seenKeys.current.add(c.key));
    const ask = fresh.filter((c) => !quietKeys?.has(c.key)); // added with the strategy's settings already confirmed: no window
    if (ask.length) { setOpenKey(ask[ask.length - 1].key); setLibraryOpen(false); }
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
  // Volume / Market Profile drawn on the price pane (profiles.ts); rebuilt with the chart
  const [profile, setProfile] = useState<ProfileMode>("off");
  // TradingView's time-based chart types (price-styles.ts) while chartType is "candles"; Renko … Range are timeless chart types
  const [priceStyle, setPriceStyle] = useState<PriceStyle>("candles");
  const [typeParams, setTypeParams] = useState<Partial<Record<ChartType, Record<string, number>>>>({});
  const [typeOpen, setTypeOpen] = useState(false);
  const [shown, setShown] = useState<{ slots: number; total: number } | null>(null);
  const timeless = chartType !== "candles";
  const typeKey = JSON.stringify([chartType, typeParams[chartType]]);
  // Prices depend only on market+ticker, so chip edits reuse the in-flight or finished OHLC requests.
  const ohlcCache = useRef<{ key: string; p: Promise<PromiseSettledResult<SeriesResponse>[]>; vol: Promise<SeriesResponse | null> } | null>(null);
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
      const shown = priceApi.current?.chart.timeScale().getVisibleLogicalRange?.(); // bar positions keep the right margin
      if (shown) keptView.current = { key: viewKey, range: shown };
      live = false; priceApi.current = null; applyVisibility.current = null; toLatest.current = undefined; unsync?.(); disposers.forEach((d) => d.dispose()); legendRef.current?.replaceChildren(); panesRef.current?.querySelectorAll(".lab-legend-tv").forEach((el) => el.replaceChildren()); };
    if (timeless) {
      // daily bars only: the engine's outputs are one value per daily bar, and a resampled event would repeat over the period
      const ind = find(chartType), params = typeParams[chartType] ?? {};
      setShown(null);
      // Range bars are built here from the daily open/high/low/close; the other types from their engine indicator's outputs
      const load = chartType === "range"
        ? Promise.all(PRICE_IDS.map((id) => cachedSeries({ id, tf: "D" }))).then((parts): SeriesResponse => ({ ...parts[3], outputs: [...PRICE_IDS],
            series: Object.fromEntries(PRICE_IDS.map((id, k) => [id, (() => { const at = new Map(parts[k].dates.map((d, j) => [d, parts[k].series[parts[k].outputs[0]]?.[j] ?? null])); return parts[3].dates.map((d) => at.get(d) ?? null); })()])) }))
        : cachedSeries({ id: chartType, tf: "D", p: params });
      void load.then((res) => {
        if (!live || !priceRef.current || (!ind && chartType !== "range")) return;
        setOhlcError(null); setChipErrors({}); setForeignNotes([]); setAdaptations(res.adaptations ?? []);
        const view = drawTimeless(priceRef.current, legendRef.current, { type: chartType, name: ind ? ind.name.replace(/\s*\(bar-aligned\)$/, "") : "Range", ticker, res, params, trades, range, onBehind: setBehind });
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
      // volume is optional (an index may have none): its failure never blocks the prices
      const entry = { key: cacheKey, p: Promise.allSettled(PRICE_IDS.map((id) => loadSeries({ id, tf: "D" }))), vol: Promise.resolve().then(() => loadSeries({ id: "volume", tf: "D" })).catch(() => null) };
      ohlcCache.current = entry;
      // A failed part is not kept, so the next run retries the prices.
      void entry.p.then((parts) => { if (parts.some((r) => r.status === "rejected") && ohlcCache.current === entry) ohlcCache.current = null; });
    }
    const ohlcLoad = ohlcCache.current.p, volLoad = ohlcCache.current.vol;
    void (async () => {
      const [ohlcParts, chipParts, volRes] = await Promise.all([
        ohlcLoad,
        Promise.allSettled(drawn.map(({ chip, ind }) => cachedSeries({ id: chip.id, tf: PERIOD_LEVELS[chip.id] ? "D" : chip.tf ?? tf, p: chip.params, vs: chip.vs, ...formulaQuery(ind) }))),
        volLoad,
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
      const price = createLabChart(priceRef.current, 420, { grid, ...TV_LOOK }, undefined, true);
      if (!price) return;
      disposers.push(price);
      const { chart, colors } = price;
      // TradingView's right margin, and its price-scale margins (10% above, 8% below the highest and lowest values shown)
      chart.applyOptions({ timeScale: { rightOffset: RIGHT_MARGIN }, rightPriceScale: { scaleMargins: { top: 0.1, bottom: 0.08 } } });
      // The house renderer takes daily bar indices, including every scale-in leg.
      // Unknown dates remain unmapped: never invent a price bar for an action.
      const dateIndex = new Map(daily.d.map((date, i) => [date, i]));
      let period = 0;
      const barOf = daily.d.map((date) => {
        while (period < px.d.length - 1 && px.d[period] < date) period++;
        return period;
      });
      // Trade spans sit behind the price on their own hidden scale: green won, red lost, grey still open.
      let spans: LabSeriesApi | undefined;
      if (trades?.length && chart.addHistogramSeries) {
        const shade = tradeSpans(trades, dateIndex, barOf, px.d.length, (t) => tint(t.open ? colors.ink3 : (t.ret ?? 0) >= 0 ? colors.green : colors.red, 0.08));
        spans = chart.addHistogramSeries({ priceScaleId: "lab-trades", priceLineVisible: false, lastValueVisible: false });
        chart.priceScale?.("lab-trades").applyOptions({ scaleMargins: { top: 0, bottom: 0 }, visible: false });
        spans.setData(px.d.map((time, i) => (shade[i] ? { time, value: 1, color: shade[i] } : { time })));
      }
      const volume = new Array<number | null>(px.d.length).fill(null);
      if (volRes) {
        const byDate = new Map(volRes.dates.map((d, k) => [d, volRes.series[volRes.outputs[0]]?.[k] ?? null]));
        daily.d.forEach((d, k) => { const v = byDate.get(d); if (typeof v === "number" && barOf[k] !== undefined) volume[barOf[k]] = (volume[barOf[k]] ?? 0) + v; });
      }
      // the chosen TradingView chart type (candles by default: filled bodies, thin wicks, up green, down red)
      const viewStart = rangeStart(px.d, range), viewFrom = viewStart ? Math.max(0, px.d.findIndex((d) => d >= viewStart)) : 0;
      const priceSeries = addPriceSeries(chart, priceStyle, { ...px, v: volume }, colors, viewFrom);
      // TradingView's default Volume: bars along the bottom fifth of the price chart, on their own scale, half-tinted by the candle's direction
      if (volume.some((v) => v !== null && v > 0)) {
        const vol = chart.addHistogramSeries?.({ priceScaleId: "vol", priceLineVisible: false, lastValueVisible: false, priceFormat: { type: "volume" } });
        chart.priceScale?.("vol").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 }, visible: false });
        vol?.setData(px.d.map((time, i) => (volume[i] === null ? { time } : { time, value: volume[i]!, color: tint(px.c[i] >= px.o[i] ? colors.green : colors.red, 0.5) })));
      }
      priceApi.current = { chart, series: priceSeries }; barTimes.current = px.d; barData.current = { o: px.o, h: px.h, l: px.l, c: px.c, v: volume };
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
        // Pivot Points Standard / CPR: TradingView's flat line per period, labelled "P (23593.95)", the last 15 periods only
        const levels = target ? periodLevelLines(chip.id, res.outputs, res.series, res.dates, px) : [];
        if (levels.length) {
          const ts = chart.timeScale(), paint = (out: string) => (chip.looks?.[out]?.color ? plotPaint(chip.looks[out]!.color!, colors) : "#FB8C00");
          const byText = new Map(res.outputs.map((o) => [PERIOD_LEVELS[chip.id]?.[o], o]));
          const pivots = chart.addCustomSeries?.({
            renderer: () => ({ draw: (t: { useMediaCoordinateSpace(f: (s: { context: CanvasRenderingContext2D }) => void): void }, toY: (p: number) => number | null) => t.useMediaCoordinateSpace(({ context: g }) => {
              g.font = "11px system-ui, sans-serif"; g.textAlign = "right"; g.textBaseline = "middle"; g.lineWidth = 1;
              for (const ln of levels) {
                const x1 = ts.logicalToCoordinate?.(ln.from), x2 = ts.logicalToCoordinate?.(ln.to), y = toY(ln.price);
                if (x1 == null || x2 == null || y === null || x2 < 0 || x1 > g.canvas.width) continue;
                g.strokeStyle = g.fillStyle = paint(byText.get(ln.text.split(" ")[0]) ?? "");
                g.beginPath(); g.moveTo(x1, y); g.lineTo(x2, y); g.stroke();
                g.fillText(ln.text, x1 - 4, y); // TradingView: the label left of its line, level with it
              }
            }) }),
            update: () => undefined,
            // the levels set the price scale only where they are drawn (the last 15 periods), as on TradingView
            priceValueBuilder: (d: { v: number; lo: number; hi: number }) => [d.lo, d.hi, d.v],
            isWhitespace: (d: { v?: number }) => d.v === undefined,
            defaultOptions: () => ({ color: "#FB8C00" }),
          }, { priceLineVisible: false, lastValueVisible: false });
          const lo = [...px.c], hi = [...px.c];
          for (const ln of levels) for (let b = ln.from; b <= Math.min(ln.to, px.d.length - 1); b++) { lo[b] = Math.min(lo[b], ln.price); hi[b] = Math.max(hi[b], ln.price); }
          pivots?.setData(px.d.map((time, i) => ({ time, v: px.c[i], lo: lo[i], hi: hi[i] })));
          if (pivots) seriesViews.push({ key: chip.key, series: pivots, visible: true });
          res.outputs.forEach((o) => { if (PERIOD_LEVELS[chip.id]?.[o]) drawnByLines.add(o); });
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
          if (!levels.length) readouts.push({ key: chip.key, out, box, label: `${shortName(ind)}${res.outputs.length > 1 ? "." + out : ""}${settings ? `(${settings})` : ""}`, color, values: new Map(res.dates.map((d, k) => [d, values[k]])) });
          if (zones.some((z) => out === `${z}_top` || out === `${z}_bottom`) || drawnByLines.has(out)) return; // drawn as the zone box / line above
          // a −1 / 0 / +1 state (trend direction) is no price: on the price chart it shows in the legend only
          if (target && values.every((v) => v === null || v === -1 || v === 0 || v === 1)) return;
          if (!target) {
            // A pane chart is created only when the chip has a line to draw; boolean-only chips mark the price chart.
            const pane = host && createLabChart(host, 150, { grid, ...TV_LOOK }, undefined, true);
            pane?.chart.applyOptions({ timeScale: { rightOffset: RIGHT_MARGIN } });
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
      const positions = actions.filter((marker) => priceDates.has(marker.time));
      const last = px.d[px.d.length - 1];
      if (keep && last) markers.push({ time: last, position: "aboveBar", shape: "square", color: colors.ink3, text: "in progress" });
      applyVisibility.current = (hidden) => {
        for (const view of seriesViews) view.series.applyOptions({ visible: view.visible && !hidden.has(view.key) });
        const visibleMarkers = [...markers, ...(hidden.has(POSITIONS) ? [] : positions), ...[...markerGroups].flatMap(([key, group]) => hidden.has(key) ? [] : group)];
        spans?.applyOptions({ visible: !hidden.has(POSITIONS) });
        priceSeries.setMarkers?.(visibleMarkers.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)));
      };
      applyVisibility.current(hiddenRef.current);
      const from = rangeStart(px.d, range) ?? px.d[0]; // "All" is an explicit range too, so panes with different warm-up start together
      // TradingView's default zoom: 6 px a bar from the latest (as many bars as the width holds), then its right margin
      // a chart built on a hidden tab has no width yet: the window's width (less the drawing rail and price axis) stands in
      const plotWidth = (priceRef.current.clientWidth || window.innerWidth - 80) - 80;
      const auto = range === "Auto" ? Math.max(0, px.d.length - 1 - Math.round(plotWidth / 6) + RIGHT_MARGIN) : null;
      const kept = keptView.current?.key === viewKey ? keptView.current.range : null;
      for (const { chart: ch } of charts) {
        if (kept) ch.timeScale().setVisibleLogicalRange?.(kept);
        else if (auto !== null) ch.timeScale().setVisibleLogicalRange?.({ from: auto, to: px.d.length - 1 + RIGHT_MARGIN });
        else if (from && last) {
          // the preset's first bar to TradingView's right margin after the latest (bar indexes: every pane carries the price bars)
          const first = px.d.findIndex((d) => d >= from);
          const ts = ch.timeScale();
          if (ts.setVisibleLogicalRange && first >= 0) ts.setVisibleLogicalRange({ from: first, to: px.d.length - 1 + RIGHT_MARGIN });
          else ch.timeScale().setVisibleRange({ from, to: last });
        }
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
        priceTs.setVisibleLogicalRange({ from: lastBar + RIGHT_MARGIN - (r.to - r.from), to: lastBar + RIGHT_MARGIN }); // same zoom, TradingView's empty bars after the latest
        const next = priceTs.getVisibleLogicalRange?.();
        if (next) for (const { chart: ch } of charts.slice(1)) ch.timeScale().setVisibleLogicalRange?.(next);
      };
      // TradingView-style legend: fixed top-left, last bar by default, the hovered bar while the crosshair is on the chart.
      const at = new Map(px.d.map((d, i) => [d, i]));
      const num = tvPrice;
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
          ...(volume[i] !== null ? [Object.assign(line([["Vol ", colors.ink2], [tvVolume(volume[i]!), tone]]), { className: "lab-legend-vol" })] : []),
          ...rows(box),
        );
        for (const pane of new Set(readouts.map((r) => r.box))) if (pane && pane !== box) pane.replaceChildren(...rows(pane));
        // Only replace value text: the label buttons keep focus while the crosshair moves.
        for (const [key, label] of valueLabels.current) {
          if (PERIOD_LEVELS[chips.find((c) => c.key === key)?.id ?? ""]) { label.replaceChildren(); continue; } // TradingView: "Pivots Traditional Auto", no values
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
  }, [market, ticker, tf, range, chipsKey, trades, themeKey, catalogue, comparisonsKey, typeKey, priceStyle]); // eslint-disable-line react-hooks/exhaustive-deps

  const panes = timeless ? [] : chips.filter((c) => find(c.id)?.placement === "pane" && !pairProblem(c));
  // TradingView: the price chart fills the screen below the toolbar (its panes included) until its height is dragged;
  // Full screen shows the whole chart tab on the screen
  const [fit, setFit] = useState(420), [full, setFull] = useState(false);
  const panesHeight = panes.reduce((sum, c) => sum + (paneHeights[c.key] ?? 150) + 8, 0);
  useEffect(() => {
    const measure = () => {
      const el = priceRef.current, fs = !!workspaceRef.current?.closest(".lab-maximized");
      setFull(fs);
      if (!el || el.closest("[hidden]")) return; // another tab is showing: nothing to measure
      const box = el.getBoundingClientRect(), top = box.top + (fs ? 0 : window.scrollY);
      // The page header, tabs and the whole chart workspace (toolbar, price chart, panes, range row) share one screen, so nothing
      // needs scrolling on open; the notes below are reached by scrolling. Neither part depends on the chart's own height, so this settles in one pass.
      const ws = workspaceRef.current?.getBoundingClientRect();
      const above = ws ? box.top - ws.top : 48, below = ws ? ws.bottom - box.bottom : panesHeight + 72;
      const pageTop = ws ? Math.max(0, ws.top + window.scrollY) : 0; // the workspace's place on the page, not on the scrolled screen
      setFit(Math.max(360, Math.round(fs ? window.innerHeight - top - panesHeight - 48 : window.innerHeight - pageTop - above - below - 8)));
    };
    measure();
    // notes and the strip under the chart change after the data loads (trades, provisional notes): measure again
    const page = workspaceRef.current?.closest<HTMLElement>(".lab-page");
    const observer = typeof ResizeObserver === "undefined" || !page ? null : new ResizeObserver(() => measure());
    observer?.observe(page!);
    // leaving the browser's full screen (Esc) also leaves the window-filling view
    const left = () => { if (!document.fullscreenElement) maximize(false); else measure(); };
    const esc = (e: KeyboardEvent) => { if (e.key === "Escape" && !document.fullscreenElement) maximize(false); };
    window.addEventListener("resize", measure); document.addEventListener("fullscreenchange", left); window.addEventListener("keydown", esc);
    return () => { observer?.disconnect(); window.removeEventListener("resize", measure); document.removeEventListener("fullscreenchange", left); window.removeEventListener("keydown", esc); };
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
  useEffect(() => {
    const api = priceApi.current, el = priceRef.current;
    if (profile === "off" || timeless || !api?.series.attachPrimitive || !el) return;
    const b = barData.current, prim = new ProfilePrimitive({ d: barTimes.current, o: b.o, h: b.h, l: b.l, c: b.c, v: b.v }, profile, themeColors(el));
    // TradingView's TPO is a chart type: the profiles stand in for the candles, so the candles go transparent while it is on
    const tpo = profile.startsWith("tpo") && CANDLE_LIKE.has(priceStyle) && priceStyle !== "volume_candles", clear = "rgba(0,0,0,0)", colors = themeColors(el);
    const candleLook = (on: boolean) => api.series.applyOptions(on
      ? { upColor: colors.green, downColor: colors.red, borderUpColor: colors.green, borderDownColor: colors.red, wickUpColor: colors.green, wickDownColor: colors.red }
      : { upColor: clear, downColor: clear, borderUpColor: clear, borderDownColor: clear, wickUpColor: clear, wickDownColor: clear });
    // the chart repaints a primitive only on its next change: an options update asks for that paint now
    api.series.attachPrimitive(prim); if (tpo) candleLook(false); else api.series.applyOptions({});
    return () => { try { api.series.detachPrimitive?.(prim); if (tpo) candleLook(true); else api.series.applyOptions({}); } catch { /* the chart was rebuilt: its series is gone */ } };
  }, [profile, chartVersion, timeless, priceStyle]); // eslint-disable-line react-hooks/exhaustive-deps
  const noVolume = profileNeedsVolume(profile) && !timeless && !barData.current.v.some((v) => typeof v === "number" && v > 0);
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
    const pivots = ind.id === "pivots_std"; // TradingView names it "Pivots Traditional Auto"
    const shown = pivots ? `${PIVOT_TYPES[chip.params.type ?? 0] ?? ""} ${chip.params.period === autoPivotPeriod(tf) ? "Auto" : PIVOT_PERIODS[chip.params.period] ?? ""}`.trim() : settings;
    return <div key={chip.key} data-lab-chart-label={chip.key}><ChartIndicatorLabel name={pivots ? "Pivots" : shortName(ind)} settings={shown} settingsTitle={fullSettings}
      color={plotCss(plotColor(chip, index, ind.outputs[0], 0))} hidden={hiddenNow.has(chip.key)}
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
    <div className="lab-tool-group" role="group" aria-label="Chart type and indicators">
      <label className="lab-chart-type-control"><span className="lab-control-name">Chart type</span><select aria-label="Chart type" value={timeless ? chartType : priceStyle} onChange={(event) => {
        const v = event.target.value; closePanel(); setTypeOpen(false); setOpenKey(null); setTool(null);
        if (isPriceStyle(v)) { setPriceStyle(v); setChartType("candles"); } else setChartType(v as ChartType);
      }}>
        {/* TradingView's order: time-based types, then the ones built from price moves alone */}
        <optgroup label="On the date axis">{PRICE_STYLES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</optgroup>
        <optgroup label="From price moves (daily bars)">{CHART_TYPES.filter(([value]) => value !== "candles").map(([value, label]) => <option key={value} value={value} disabled={value !== "range" && !find(value)?.computed}>{label}</option>)}</optgroup>
      </select></label>
      {!timeless && <label className="lab-chart-type-control" data-lab-profile><span className="lab-control-name">Profile</span><select aria-label="Profile" value={profile} onChange={(event) => setProfile(event.target.value as ProfileMode)}>
        {PROFILE_MODES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select></label>}
      {noVolume && <span className="cap lab-profile-note" data-lab-profile-note>{ticker} has no volume: pick Market profile (TPO), which needs prices only</span>}
      {typeInd && <button type="button" className="dbtn sm" data-lab-type-params aria-expanded={typeOpen} onClick={() => { setLibraryOpen(false); setTypeOpen(true); }}>{typeName} settings</button>}
      {library && <button type="button" className="dbtn sm lab-add-indicator" title="Add or edit indicators" aria-expanded={libraryOpen} onClick={() => { if (!libraryOpen) { closePanel(); setOpenKey(null); setTypeOpen(false); } setLibraryOpen(!libraryOpen); }}><svg width="19" height="19" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true"><path d="M2 17h16M4 15V9m5 6V5m5 10v-4M2 7l5-5 5 4 6-5" /></svg>Indicators</button>}
    </div>
    <div className="lab-tool-group lab-tool-group-view" role="group" aria-label="View">{fullButton}</div>
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
      <DrawRail hidden={ohlcError !== null || timeless} tool={tool} setTool={setTool} prefs={prefs} setPrefs={setPrefs} drawings={drawings.length} indicators={chips.length}
        zoomed={zooms.length > 0} onZoomOut={() => { priceApi.current?.chart.timeScale().setVisibleLogicalRange?.(zooms[zooms.length - 1]); setZooms(zooms.slice(0, -1)); }}
        onUndo={() => saveDrawings(drawings.slice(0, -1))} onRemoveDrawings={() => saveDrawings([])} onRemoveIndicators={() => chips.forEach((c) => onChipRemove(c.key))} />
        <div hidden={ohlcError !== null}>
          <div className={`lab-price${tool ? " lab-drawing" : ""} lab-cursor-${prefs.cursor}`}>
            <div className="lwc px" ref={priceRef} style={{ height: paneHeights.price ?? fit }} data-lab-chart role="img" aria-label={timeless ? `${ticker} ${typeName} chart` : `${ticker} price chart`} />
            {!timeless && priceApi.current && priceRef.current && <DrawingLayer key={chartVersion} chart={priceApi.current.chart} series={priceApi.current.series}
              times={barTimes.current} bars={barData.current} drawings={drawings} tool={tool} color={themeColors(priceRef.current).blue} height={paneHeights.price ?? fit}
              magnet={prefs.magnet} stay={prefs.stay} locked={prefs.lock} hidden={prefs.hide.drawings} eraser={prefs.cursor === "eraser" && !tool}
              onChange={saveDrawings} onDone={() => setTool(null)} onZoom={(before) => before && setZooms((z) => [...z, before])} />}
            <div className="lab-legend-tv" ref={legendRef} aria-hidden="true" />
            {!timeless && <div className="lab-chart-indicators">{chips.map((chip, index) => find(chip.id)?.placement !== "pane" ? labelFor(chip, index) : null)}</div>}
            {tool && <p className="cap lab-drawing-hint" data-lab-draw-hint>{drawHint(tool)} · Esc cancels</p>}
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
      {onRange && <div className="lab-chart-footer" role="group" aria-label="Chart range">{RANGE_BUTTONS.map((value) => <button key={value} type="button" aria-pressed={range === value} onClick={() => onRange(value)}>{value}</button>)}</div>}
      {panelOpen && <ChartPopup key={libraryOpen ? "library" : openKey ?? chartType} wide={libraryOpen} title={libraryOpen ? "Indicators" : "Indicator settings"} onClose={closePanel}>{panel}</ChartPopup>}
      {timeless && <p className="cap" data-lab-timeless-note>
        {`${typeName} ${chartType === "range" ? "built here from the daily open, high, low and close (each day walks open → low → high → close, or the reverse on a down day; a bar spans the same price range)" : "from the engine's outputs on daily bars (the values the rules use)"}, without a date axis: hover a ${nounOf(chartType)} for its dates; the last ${nounOf(chartType)} can still change. Indicator chips, drawings and candle size apply to Candles only.`}
        {shown && shown.total > shown.slots && ` Showing the latest ${MAX_SLOTS.toLocaleString("en-US")} of ${shown.total.toLocaleString("en-US")} ${nounOf(chartType, true)}; a larger box shows more history.`}
      </p>}
      {foreignNotes.map(note=><p className="cap" role="note" key={note}>{note}</p>)}
      {adaptations.map((note) => <p className="rnote" role="note" data-lab-adaptations key={note}>{note}</p>)}
      {tf !== "D" && !timeless && <p className="cap">{COPY.inProgress(tf)}</p>}
      {!!trades?.length && timeless && <p className="cap"><span style={{ color: "var(--green)" }}>▲ Buy</span> · <span style={{ color: "var(--red)" }}>▼ Sell</span>. Triangles point at the real fill price, on the {nounOf(chartType)} that holds the fill date.</p>}
      {!!trades?.length && !timeless && (
        <p className="cap">
          <span style={{ color: "var(--green)" }}>↑ Buy</span> · <span style={{ color: "var(--red)" }}>↓ Sell</span>. Small arrows: added legs. Shaded trades: green won, red lost, grey open.{tf !== "D" && " Trades run on daily bars and are mapped onto this timeframe."}
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
