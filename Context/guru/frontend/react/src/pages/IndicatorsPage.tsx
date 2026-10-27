import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type ReactNode } from "react";
import { getAccount } from "../services/api";
import { vaultStore } from "../state/store";
import * as lab from "../features/lab/client";
import { COPY } from "../features/lab/labels";
import { readLabQuery, writeLabQuery } from "../features/lab/url-state";
import { editorState, initialLabState, labReducer, runKey } from "../features/lab/lab-state";
import { canonicalJson, runQuery, specProblems } from "../features/lab/spec-model";
import { treeProblems, walkSpec } from "../features/lab/rule-tree";
import { FormulaContext, formulaIndicator, isFormulaOperand, myId, type FormulaCheck } from "../features/lab/formula";
import { FormulaEditor } from "../features/lab/FormulaEditor";
import { FormulaBlocks } from "../features/lab/FormulaBlocks";
import { executionProblems, instrumentContext } from "../features/lab/execution-model";
import { PortfolioPanel } from "../features/lab/PortfolioPanel";
import { PortfolioResults } from "../features/lab/PortfolioResults";
import { portfolioProblems, portfolioGridProblems } from "../features/lab/portfolio-model";
import { ForeignProvider } from "../features/lab/ForeignFields";
import { foreignDescription, foreignOptions, foreignProblems, type ForeignScope } from "../features/lab/foreign-model";
import { availability } from "../features/lab/catalogue";
import { defaultTicker, optionsFromGl, optionsFromTickers, savedMarket } from "../features/lab/instruments";
import { ErrorNotice, LabErrorBoundary, Loading, ServerLabel } from "../features/lab/ui";
import { Toolbar } from "../features/lab/Toolbar";
import { LibraryRail } from "../features/lab/LibraryRail";
import { IndicatorDetail } from "../features/lab/IndicatorDetail";
import { ChartPopup } from "../features/lab/ChartPopup";
import { Workbench } from "../features/lab/Workbench";
import { RuleBuilder, SettingsTabs, type RunSettings } from "../features/lab/RuleBuilder";
import { StepGuide } from "../features/lab/StepGuide";
import { DateFilterPanel } from "../features/lab/DateFilterPanel";
import { withCurrentDateSet } from "../features/lab/date-sets";
import { RunResults, RunTables } from "../features/lab/RunResults";
import { NoTradesHelp } from "../features/lab/NoTrades";
import { SimpleOptimize } from "../features/lab/SimpleOptimize";
import { DEFAULT_FROM, defaultExitWords, firstRule, loadDefaultExit, storeDefaultExit, withDefaultExit, type DefaultExit } from "../features/lab/defaults";
import { ChartMatch, type ChartPick } from "../features/lab/PlainRules";
import { ReportPanel } from "../features/lab/Report";
import { MonteCarloPanel } from "../features/lab/MonteCarlo";
import { ExplorePanel } from "../features/lab/Explore";
import { OptimizePanel } from "../features/lab/OptimizePanel";
import { CustomizePanel } from "../features/lab/CustomizePanel";
import { SavedStrategies } from "../features/lab/SavedStrategies";
import { ReviewQueue } from "../features/lab/ReviewQueue";
import { CompactContext, LabModeContext, ModeSwitch, SimpleSummary, storedMode, storeMode, type LabMode } from "../features/lab/simple";
import type {
  Catalogue, DateSet, FormulaInfo, Indicator, InstrumentsResponse, SavedIndicator, LabStrategy, Market, Role, LabRunResult, Spec, TemplateResponse, TemplatesResponse, TickerRow,
} from "../features/lab/types";

export function IndicatorsPage() {
  return <LabErrorBoundary><IndicatorsLab /></LabErrorBoundary>;
}

/** Everything loaded for one market, tagged so a late answer for another market is never used. */
type MarketData = { market: Market; cat: Catalogue; rows?: TickerRow[]; gl?: InstrumentsResponse } | { market: Market; error: unknown };

const LAB_TABS = [{ id: "chart", label: "Chart" }, { id: "strategy", label: "Strategy & backtest" }, { id: "optimization", label: "Optimization" }, { id: "customize", label: "Customize" }, { id: "saved", label: "Saved strategies" }] as const;
type LabTab = typeof LAB_TABS[number]["id"];

/** The Strategy & backtest tab as three plain-language steps, each a tab (the chart has its own tab). */
const STEPS = [
  { id: "idea", label: "Your idea", hint: "Say when to buy. Selling is already set: the default exit, unless you add your own.", tips: [
    "No buy rule yet? Click a suggestion (indicators on your chart come first) or an example. You can change every number afterwards.",
    "Every buy row must be true on the same candle, so each extra row means fewer trades. Remove a row you do not need with ×.",
    "No sell rule? The test sells at the 1st up close after entry, or at the 4th down close. Change those two numbers in the Sell box, or add your own sell rule.",
    "“Your strategy in plain words” at the top shows exactly what will be tested, and any setting that can stop a trade." ] },
  { id: "settings", label: "Settings", hint: "The test period, trading costs and how money is counted.", tips: [
    "Leave From and To empty to test all the data. Type dates as dd-mm-yyyy, or use the calendar button.",
    "Fees and slippage are in basis points per side: 10 bps = 0.10% on every buy and every sell.",
    "“Signal day's close” trades at the close of the candle that gave the signal. “Next day's open” waits for the next candle's open.",
    "If you are unsure, keep the defaults and go to Results." ] },
  { id: "results", label: "Results", hint: "How the idea did, compared with simply holding the market.", tips: [
    "Press “Test this idea” to run the backtest.",
    "Compare your strategy with buy-and-hold: simply buying once and holding for the whole period.",
    "Check the list of trades to see each buy and sell date and its result.",
    "Results are simulated on past data; they are not a promise of future returns." ] },
] as const;
type Step = typeof STEPS[number]["id"];

const Part = ({ children }: { children: ReactNode }) => <LabErrorBoundary>{children}</LabErrorBoundary>;

function IndicatorsLab() {
  const query = useMemo(() => readLabQuery(), []);
  const [state, dispatch] = useReducer(labReducer, undefined, () =>
    initialLabState(query, /[?&]region=/.test(location.hash) ? undefined : vaultStore.getSnapshot().state?.region));
  const [tab, setTab] = useState<LabTab>("chart"); // the chart first; Strategy & backtest opens on Your idea
  const [mode, setModeState] = useState<LabMode>(storedMode);
  const simple = mode === "simple";
  const tabs = LAB_TABS.filter((t) => !simple || t.id === "chart" || t.id === "strategy" || t.id === "saved"); // Simple hides Optimization and Customize
  const setMode = (m: LabMode) => { setModeState(m); storeMode(m); if (m === "simple" && (tab === "optimization" || tab === "customize")) setTab("strategy"); };
  const [details, setDetails] = useState(false); // Simple: the full Results tabs after the summary
  const [step, setStep] = useState<Step>("idea");
  const stepIndex = STEPS.findIndex((s) => s.id === step);
  const showStrategy = (selector = ".lab-rules") => {
    setTab("strategy");
    requestAnimationFrame(() => {
      const target = document.querySelector<HTMLElement>(selector);
      target?.focus({ preventScroll: true });
      target?.scrollIntoView?.({ behavior: "smooth", block: "start" });
    });
  };
  const [role, setRole] = useState<Role | null>(null);
  const [data, setData] = useState<MarketData | null>(null);
  const [result, setResult] = useState<LabRunResult | null>(null);
  const [ranOwn, setRanOwn] = useState<Spec | null>(null); // the user's own rules behind `result` (ranSpec adds the default exit)
  const [ranSpec, setRanSpec] = useState<Spec | null>(null); // the spec behind `result`, for the report's Settings block
  const [runError, setRunError] = useState<unknown>(null);
  const [running, setRunning] = useState(false);
  const [settings, setSettings] = useState<RunSettings>({ from: DEFAULT_FROM, to: "", fees: "", slippage: "" }); // tests start in 2000 unless cleared
  // the exit a test uses when no sell rule is on (defaults.ts): the user's two numbers, kept per browser
  const [defaultExit, setDefaultExitState] = useState<DefaultExit>(loadDefaultExit);
  const setDefaultExit = (d: DefaultExit) => { setDefaultExitState(d); storeDefaultExit(d); };
  const [ranDefault, setRanDefault] = useState(""); // the default exit's words when the shown result used it
  const [reading, setReading] = useState<Indicator | null>(null);
  const [templates, setTemplates] = useState<TemplatesResponse | null>(null);
  const [templatesError, setTemplatesError] = useState<unknown>(null);
  const [customize, setCustomize] = useState(query.customize);
  const [template, setTemplate] = useState<TemplateResponse | null>(null);
  const [templateError, setTemplateError] = useState<unknown>(null);
  const [openError, setOpenError] = useState<unknown>(null);
  const runSeq = useRef(0);
  const opened = useRef<string | null>(null); // instrument of the row or snapshot just opened
  const [swapNote, setSwapNote] = useState("");
  const [savePrefill, setSavePrefill] = useState<{ name: string; seq: number } | undefined>();
  const { market, ticker } = state;
  // "My indicators": saved formulas, their chart indicators (by saved id) and the check result per formula text in the spec.
  const [mine, setMine] = useState<SavedIndicator[]>([]);
  const [dateSetList, setDateSetList] = useState<DateSet[] | null>(null);
  const [dateSetError, setDateSetError] = useState<unknown>(undefined);
  const reloadDateSets = useCallback(() => lab.dateSets.list().then((l) => { setDateSetList(l); setDateSetError(undefined); return l; }, (e) => { setDateSetError(e); return undefined; }), []);
  useEffect(() => { void reloadDateSets(); }, [reloadDateSets]);
  // A strategy that uses a saved date filter picks up the set's current dates (re-uploads, renames) once the sets load.
  useEffect(() => { if (dateSetList) { const next = withCurrentDateSet(state.spec, dateSetList); if (next !== state.spec) dispatch({ type: "spec", spec: next }); } }, [dateSetList, state.spec]);
  const [mineError, setMineError] = useState<unknown>(null);
  const [myInds, setMyInds] = useState<Record<number, Indicator>>({});
  const [infos, setInfos] = useState<Record<string, FormulaCheck>>({});
  const [editing, setEditing] = useState<SavedIndicator | "new" | null>(null);
  const portfolioContext=canonicalJson([market,state.spec.portfolio??null]);
  const currentContext=useRef(portfolioContext);currentContext.current=portfolioContext;
  const issuedContext=useRef(portfolioContext);

  useEffect(() => { getAccount().then((a) => setRole(a.user.role), () => {}); }, []); // the App shell handles sign-in
  useEffect(() => { lab.indicators.list().then(setMine, setMineError); }, []);
  // Every formula operand in the spec is checked once per text (cached in the client), for its fields and optimize paths.
  const infosRef = useRef(infos); infosRef.current = infos;
  useEffect(() => {
    const texts = new Set<string>();
    walkSpec(state.spec, (x) => { if (x.t === "operand" && isFormulaOperand(x.operand)) texts.add(x.operand.formula); });
    for (const text of texts) if (!(text in infosRef.current)) lab.formulaInfo(market, text).then(
      (info) => setInfos((s) => ({ ...s, [text]: info })),
      (e: unknown) => setInfos((s) => ({ ...s, [text]: { error: e instanceof Error ? e.message : String(e) } })));
  }, [state.spec, market]);
  const formulaScope = useMemo(() => ({ mine, infos }), [mine, infos]);

  useEffect(() => {
    let cancelled = false;
    const instruments = market === "gl" ? lab.labInstruments().then((gl) => ({ gl })) : lab.tickers(market).then((rows) => ({ rows }));
    Promise.all([lab.catalogue(market), instruments]).then(
      ([cat, inst]) => !cancelled && setData({ market, cat, ...inst }),
      (error: unknown) => !cancelled && setData({ market, error }),
    );
    return () => { cancelled = true; };
  }, [market]);

  const allowedTypes = template?.status === "available" ? template.instrument.allowed_types : undefined;
  const loaded = data?.market === market ? data : null;
  const cat = loaded && "cat" in loaded ? loaded.cat : null;
  /** What a test runs: the edited rules, with the default exit on a side that has no sell rule (an engine without the
   *  rule tree cannot read since-entry counts, so its strategies keep their own exits). */
  const runSpec = useMemo(() => cat?.execution ? withDefaultExit(state.spec, defaultExit) : state.spec, [cat, state.spec, defaultExit]);
  const options = useMemo(() => {
    if (!loaded || !("cat" in loaded)) return null;
    return loaded.gl ? optionsFromGl(loaded.gl, allowedTypes) : optionsFromTickers(loaded.rows ?? [], allowedTypes, loaded.cat.portfolio?.instruments);
  }, [loaded, allowedTypes]);
  // Signal sources are not constrained by a selected book template's execution types.
  const signalOptions = useMemo(() => !loaded || !("cat" in loaded) ? [] : loaded.gl ? optionsFromGl(loaded.gl) : optionsFromTickers(loaded.rows ?? [], undefined, loaded.cat.portfolio?.instruments), [loaded]);
  const foreignScope = useMemo<ForeignScope | null>(() => cat ? { catalogue: cat, ticker:state.spec.portfolio?"":ticker, market, options: signalOptions, spec: state.spec, portfolio:!!state.spec.portfolio } : null, [cat, ticker, market, signalOptions, state.spec]);
  const comparisonOptions = useMemo(() => foreignScope ? foreignOptions(foreignScope) : [], [foreignScope]);
  const option = options?.find((o) => o.ticker === ticker);
  // To shows the instrument's latest data date and follows the instrument until the user types a date of their own
  const latest = option?.last ?? "";
  const autoTo = useRef(true);
  useEffect(() => { if (autoTo.current) setSettings((s) => (s.to === latest ? s : { ...s, to: latest })); }, [latest]); // "" = the last bar
  const changeSettings = (patch: Partial<RunSettings>) => { if (patch.to !== undefined) autoTo.current = patch.to === latest; setSettings((s) => ({ ...s, ...patch })); };
  const runSettings = useMemo(() => ({ ...settings, latest }), [settings, latest]);
  const savedMarketValue = savedMarket(market, option);
  const instrument = useMemo(() => state.spec.portfolio?{kind:"portfolio",market}:instrumentContext(market, option), [market, option, state.spec.portfolio]);

  useEffect(() => {
    if (!options || options.some((o) => o.ticker === ticker && !o.disabled)) return;
    const next = defaultTicker(market, options);
    if (next === undefined || next === ticker) return;
    if (opened.current === ticker) setSwapNote(`${ticker} is not available here; showing ${next || "no instrument"}.`);
    opened.current = null;
    dispatch({ type: "ticker", ticker: next });
  }, [options, ticker, market]);

  useEffect(() => { // an unknown ?tf= falls back to daily once the catalogue is known
    if (cat && !cat.timeframes.includes(state.tf)) dispatch({ type: "tf", tf: "D" });
  }, [cat, state.tf]);

  useEffect(() => {
    writeLabQuery({ region: market, tk: ticker, tf: state.tf, range: state.range });
  }, [market, ticker, state.tf, state.range]);

  // A result belongs to one market and instrument: its numbers never stay on screen for another one.
  const clearRun = () => { runSeq.current++; setResult(null); setRunError(null); setRunning(false); };
  useEffect(clearRun, [market, ticker]);
  useEffect(()=>{if(issuedContext.current!==portfolioContext)clearRun();},[portfolioContext]);

  useEffect(() => {
    setTemplates(null); setTemplatesError(null);
    if (market === "gl") return;
    let cancelled = false;
    lab.templates(market).then((t) => !cancelled && setTemplates(t), (e: unknown) => !cancelled && setTemplatesError(e));
    return () => { cancelled = true; };
  }, [market]);

  useEffect(() => {
    setTemplate(null); setTemplateError(null);
    if (!customize || market === "gl") return;
    let cancelled = false;
    lab.template(market, customize).then((t) => {
      if (cancelled) return;
      setTemplate(t);
      if (t.status === "available") {
        clearRun(); dispatch({ type: "open", row: null, spec: t.template.spec });
        writeLabQuery({ strategy: "", published: "" }); // only now has the template replaced the open row
      }
    }, (e: unknown) => !cancelled && setTemplateError(e));
    return () => { cancelled = true; };
  }, [market, customize]);

  const openRow = useCallback((row: LabStrategy) => {
    setTab("strategy"); opened.current = row.spec.portfolio?null:row.instrument; setSwapNote("");
    dispatch({ type: "market", market: row.market === "crypto" ? "gl" : row.market });
    if(!row.spec.portfolio)dispatch({ type: "ticker", ticker: row.instrument });
    dispatch({ type: "open", row, spec: row.spec });
    setCustomize(""); setOpenError(null); setReading(null); clearRun();
    writeLabQuery({ strategy: String(row.id), published: "", customize: "" });
  }, []);

  useEffect(() => {
    let cancelled = false;
    if (query.strategy) {
      lab.strategies.get(Number(query.strategy)).then((row) => !cancelled && openRow(row), (e: unknown) => !cancelled && setOpenError(e));
    } else if (query.published) {
      lab.strategies.published().then((list) => {
        if (cancelled) return;
        const snap = list.find((p) => p.snapshotId === Number(query.published));
        if (!snap) { setOpenError(new Error(`Published strategy ${query.published} was not found.`)); return; }
        opened.current = snap.instrument; setSwapNote("");
        dispatch({ type: "market", market: snap.market });
        if(!snap.spec.portfolio)dispatch({ type: "ticker", ticker: snap.instrument });
        dispatch({ type: "open", row: null, spec: snap.spec });
      }, (e: unknown) => !cancelled && setOpenError(e));
    }
    return () => { cancelled = true; };
  }, [query, openRow]);

  const changeMarket = (m: Market) => {
    if (m === market) return;
    opened.current = null; setSwapNote(state.chips.some((chip) => chip.vs) ? "Chart comparisons were cleared for the new market; choose new instruments. Strategy source fields are kept for review." : "");
    dispatch({ type: "market", market: m });
    setCustomize(""); setReading(null); setOpenError(null);
    writeLabQuery({ customize: "", strategy: "", published: "" });
  };

  const problemsFor = (spec: Spec) => cat ? [...specProblems(spec, cat), ...(cat.execution ? [...treeProblems(spec, cat), ...executionProblems(spec, cat, instrument)] : []), ...(foreignScope ? foreignProblems(spec, { ...foreignScope, spec }) : []), ...portfolioProblems(spec, cat, market, signalOptions), ...portfolioGridProblems(spec, cat)].filter((p, i, all) => all.indexOf(p) === i) : ["The engine catalogue is still loading."];
  const assertRunnable = (spec: Spec) => { const issues = problemsFor(spec); if (issues.length) throw new Error(issues.join("\n")); };

  const run = async (current: Spec = state.spec) => {
    const filled = cat?.execution ? withDefaultExit(current, defaultExit) : current; // no sell rule: the default exit
    const spec = dateSetList ? withCurrentDateSet(filled, dateSetList) : filled; // a re-uploaded date filter reaches the run
    if (!ticker && !spec.portfolio) return; // no selectable instrument on this market
    const n = ++runSeq.current;
    const sentContext=canonicalJson([market,spec.portfolio??null]);issuedContext.current=sentContext;currentContext.current=sentContext;
    setResult(null); setRunError(null); setRunning(true); setTab("strategy"); setStep("results");
    try {
      assertRunnable(spec);
      const r = await lab.run(market, spec, {
        ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage),
      });
      if (n !== runSeq.current || sentContext!==currentContext.current) return;
      setResult(r); setRanSpec(spec); setRanOwn(current); setRanDefault(filled !== current ? defaultExitWords(defaultExit, current.direction === "short") : "");
      dispatch({ type: "ran", key: runKey(spec, savedMarketValue, ticker), combos: 0 });
    } catch (e) {
      if (n === runSeq.current) setRunError(e);
    } finally {
      if (n === runSeq.current) setRunning(false);
    }
  };

  const loadSeries = useCallback((q: { id: string; tf: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }) => lab.series(market, { ...q, ticker }), [market, ticker]);
  const problems = problemsFor(runSpec);
  const methodNames = useMemo(() => Object.fromEntries(templates?.templates.map((t) => [t.method.id, t.method.name]) ?? []), [templates]);
  const glNote = loaded && "gl" in loaded ? loaded.gl?.provisional_note : undefined;
  const hasVolume = !!option?.hasVolume;
  // daily indicators drawn on the chart, offered as first buy rules with the same settings
  const chartPicks = useMemo<ChartPick[]>(() => !cat ? [] : state.chips.flatMap((chip) => {
    const indicator = cat.indicators.find((i) => i.id === chip.id);
    return indicator?.computed && !chip.vs && (!chip.tf || chip.tf === "D") && availability(indicator, { market, hasVolume, tf: state.tf, maxForeign: cat.max_foreign }).ok
      ? [{ indicator, params: chip.params }] : [];
  }), [cat, state.chips, market, hasVolume, state.tf]);
  const scanSignals = useCallback((spec: Spec) => lab.scan(market, spec, { ticker, apply: "current", from: settings.from || undefined, to: settings.to || undefined }), [market, ticker, settings.from, settings.to]);
  /** A fix offered with a 0-trade result: change the edited rules and test again. */
  const fixAndRun = (change: (spec: Spec) => Spec) => { const next = change(state.spec); dispatch({ type: "spec", spec: next }); void run(next); };

  const backtestShown = !!(result || running || runError);
  const canEditMine = role === "admin" || role === "researcher";
  const chartCat = useMemo(() => cat && { ...cat, indicators: [...cat.indicators, ...Object.values(myInds)] }, [cat, myInds]);
  const addMine = (row: SavedIndicator) => lab.formulaInfo(market, row.formula).then((info) => {
    const ind = formulaIndicator(row, info);
    setMyInds((m) => ({ ...m, [row.id]: ind }));
    dispatch({ type: "chip-add", indicator: ind });
  }, () => setEditing(row)); // a formula the engine refuses opens in the editor with its error
  const mineSaved = (row: SavedIndicator, info: FormulaInfo) => {
    setMine((list) => [...list.filter((r) => r.id !== row.id), row].sort((a, b) => a.name.localeCompare(b.name)));
    setInfos((s) => ({ ...s, [row.formula]: info }));
    if (!myInds[row.id]) return;
    // chips of this indicator redraw with the saved formula; settings kept where the Param still exists
    setMyInds((m) => ({ ...m, [row.id]: formulaIndicator(row, info) }));
    for (const chip of state.chips) if (chip.id === myId(row.id))
      dispatch({ type: "chip-set", key: chip.key, params: Object.fromEntries(info.params.map((p) => [p.key, chip.params[p.key] ?? p.default])) });
  };
  const mineDeleted = (id: number) => {
    setMine((list) => list.filter((r) => r.id !== id)); setEditing(null);
    for (const chip of state.chips) if (chip.id === myId(id)) dispatch({ type: "chip-remove", key: chip.key });
  };

  // AmiBroker AFL layout: the date windows (Base 1 / Base 2) sit with the filters on the rules' Filters tab, else in Settings
  const datesPanel = <DateFilterPanel spec={state.spec} onChange={(spec) => dispatch({ type: "spec", spec })}
    sets={{ list: dateSetList, error: dateSetError, reload: reloadDateSets, canSave: canEditMine, canShare: role === "admin" }} />;
  const portfolioPanel = cat && <PortfolioPanel spec={state.spec} catalogue={cat} market={market} options={signalOptions} onChange={(spec)=>{clearRun();dispatch({type:"spec",spec});}}/>;
  const indicatorLibrary = cat && <div className="lab-library-slot"><Part><LibraryRail docked catalogue={cat} market={market} hasVolume={hasVolume} tf={state.tf}
          onAdd={(ind) => { if (availability(ind, { market, hasVolume, tf: state.tf, maxForeign: cat.max_foreign }).ok) dispatch({ type: "chip-add", indicator: ind }); }}
          onRead={(ind) => setReading((cur) => (cur?.id === ind.id ? null : ind))}
          mine={{ list: mine, error: mineError ?? undefined, canEdit: canEditMine, onAdd: (row) => void addMine(row), onEdit: (row) => setEditing(row ?? "new") }} /></Part></div>;
  const chartToolbar = (controls: ReactNode) => (<Toolbar chartWorkspace={tab === "chart"} market={market} onMarket={changeMarket} options={options ?? []} ticker={ticker}
        onTicker={(t) => { opened.current = null; setSwapNote(""); dispatch({ type: "ticker", ticker: t }); }} tf={state.tf} timeframes={cat?.timeframes ?? ["D", "W", "M", "Q", "Y"]}
        onTf={(tf) => dispatch({ type: "tf", tf })} range={state.range} onRange={(range) => dispatch({ type: "range", range })}
        provisionalNote={glNote}>{controls}</Toolbar>);
  let work: ReactNode;
  if (loaded && "error" in loaded) work = <ErrorNotice error={loaded.error} market={market} />;
  else if (!cat || !options) work = <Loading text="Loading the Indicators lab…" />;
  else work = (
    <LabModeContext.Provider value={mode}><CompactContext.Provider value={true}><ForeignProvider value={foreignScope!}><FormulaContext.Provider value={formulaScope}>
    <div className="lab-grid lab-grid-focused">
      <div className="lab-main">
        <div role="tabpanel" id="lab-panel-chart" aria-labelledby="lab-tab-chart" hidden={tab !== "chart"} className="lab-tab-panel">
        <Part>
          {ticker ? (
            <Workbench toolbar={chartToolbar} onRange={(range) => dispatch({ type: "range", range })} library={indicatorLibrary} market={savedMarketValue} ticker={ticker} tf={state.tf} range={state.range} catalogue={chartCat!} chips={state.chips}
              trades={result?.kind==="lab_simulation"?result.trades:undefined} loadSeries={loadSeries} instrumentLabel={option?.instrumentLabel}
              onChipChange={(key, params) => dispatch({ type: "chip-set", key, params })}
              onChipRemove={(key) => dispatch({ type: "chip-remove", key })} onChipLook={(key, looks) => dispatch({ type: "chip-look", key, looks })} comparisonOptions={comparisonOptions}
              onChipVsChange={(key, vs) => dispatch({ type: "chip-vs", key, vs })} onChipTfChange={(key, tf) => dispatch({ type: "chip-tf", key, tf })} onChipLevels={(key, levels) => dispatch({ type: "chip-levels", key, levels })} />
          ) : <div className="box empty">No instrument is available on this market.</div>}
        </Part>
        {/* Reference and formula windows leave the chart in place. */}
        {reading && <Part><ChartPopup wide title="Indicator reference" onClose={() => setReading(null)}><div className="lab-browser-heading"><h2>{reading.name}</h2><button type="button" className="dbtn sm ghost" aria-label="Close reference window" onClick={() => setReading(null)}>×</button></div><IndicatorDetail indicator={reading} catalogue={cat} onClose={() => setReading(null)}
          onUse={(entry, exit) => { // this indicator's default rules become the strategy (mirrored for a short one); filters stay
            const short = state.spec.direction === "short", filters = state.spec.entry.all.filter((n) => (n as Record<string, unknown>).filter === true);
            const [buy, sell] = short ? [firstRule(reading, {}, true), undefined] : [entry, exit];
            dispatch({ type: "spec", spec: { ...state.spec, entry: { ...state.spec.entry, all: [buy, ...filters] }, exit: { ...state.spec.exit, any: sell ? [sell] : [] } } });
            setReading(null); setTab("strategy"); setStep("idea");
          }} /></ChartPopup></Part>}
        {editing && <Part><ChartPopup wide title="Formula editor" onClose={() => setEditing(null)}><div className="lab-browser-heading"><h2>{editing === "new" ? "New indicator" : editing.name}</h2><button type="button" className="dbtn sm ghost" aria-label="Close formula window" onClick={() => setEditing(null)}>×</button></div><FormulaEditor key={editing === "new" ? "new" : editing.id} row={editing === "new" ? null : editing} functions={cat.formula_functions ?? []}
          canEdit={canEditMine} check={(text) => lab.formulaInfo(market, text)} api={lab.indicators}
          onSaved={mineSaved} onDeleted={mineDeleted} onClose={() => setEditing(null)}
          blocksEditor={(p) => <FormulaBlocks {...p} indicators={cat.indicators} check={(text) => lab.formulaInfo(market, text)} readOnly={!canEditMine} />} /></ChartPopup></Part>}
        <div className="lab-step-nav lab-chart-next"><span className="cap">{chartPicks.length ? "Your indicators are on the chart. Turn them into buy and sell rules and test them:" : "Add indicators to the chart, or go straight to buy and sell rules:"}</span>
          <button type="button" className="dbtn primary" data-lab-build onClick={() => { setTab("strategy"); setStep("idea"); }}>Build a strategy ›</button></div>
        </div>
        <div role="tabpanel" id="lab-panel-strategy" aria-labelledby="lab-tab-strategy" hidden={tab !== "strategy"} className="lab-tab-panel">
        <nav className="lab-stepbar" aria-label="Steps" data-lab-steps>
          {STEPS.map((s, i) => <button key={s.id} type="button" className="lab-step-btn" aria-current={step === s.id ? "step" : undefined} data-lab-step={s.id}
            onClick={() => setStep(s.id)}><span className="lab-step-n">{i + 1}</span>{s.label}</button>)}
        </nav>
        <StepGuide n={stepIndex + 1} of={STEPS.length} step={STEPS[stepIndex]} />
        <div className="lab-step" hidden={step !== "idea"}>
        <Part>
          <div className="lab-rules-col">
          {openError ? <ErrorNotice error={openError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} /> : null}
          {/* the tree builder shows the portfolio controls on its Portfolio settings tab (AmiBroker layout) */}
          {!cat.execution && portfolioPanel}
          <RuleBuilder spec={state.spec} catalogue={cat} instrument={instrument} onChange={(spec) => dispatch({ type: "spec", spec })}
            settings={runSettings} onSettings={changeSettings}
            problems={problems} running={running} onRun={() => void run()} portfolio={portfolioPanel} part="rules" dates={datesPanel}
            defaultExit={defaultExit} onDefaultExit={setDefaultExit} chartPicks={chartPicks}
            chartMatch={<ChartMatch spec={state.spec} catalogue={cat} chips={state.chips} onSpec={(spec) => dispatch({ type: "spec", spec })}
              onChartSet={(key, params) => dispatch({ type: "chip-set", key, params })}
              onChartAdd={(ind, params) => { const key = `c${state.nextChip}`; dispatch({ type: "chip-add", indicator: ind }); dispatch({ type: "chip-set", key, params }); }} />} />
          {/* AmiBroker-style Optimize in the simplest form: a range per number, every combination, one click to use the best */}
          {canEditMine && ticker && !state.spec.portfolio && cat.execution && <details className="box lab-try-ranges" data-lab-try-ranges>
            <summary>Try ranges: test every combination of your numbers (like AmiBroker's Optimize)</summary>
            <SimpleOptimize spec={runSpec} catalogue={cat}
              onOptimize={(spec) => { assertRunnable(spec); return lab.optimize(market, spec, { ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage) }); }}
              onApply={(spec) => { dispatch({ type: "spec", spec }); void run(spec); }} />
          </details>}
          </div>
        </Part>
        </div>
        <div className="lab-step" hidden={step !== "settings"}>
        <Part>
          <RuleBuilder spec={state.spec} catalogue={cat} instrument={instrument} onChange={(spec) => dispatch({ type: "spec", spec })}
            settings={runSettings} onSettings={changeSettings}
            problems={problems} running={running} onRun={() => void run()} portfolio={portfolioPanel} part="settings" dates={datesPanel} defaultExit={defaultExit} onDefaultExit={setDefaultExit} />
        </Part>
        </div>
        <div className="lab-step" hidden={step !== "results"} data-lab-step-results>
        {/* WealthLab/AmiBroker style: one Results box, every view a tab instead of a long scroll */}
        {backtestShown ? <Part>
          {runError ? <ErrorNotice error={runError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} />
            : !result ? <Loading text="Running backtest…" />
            : simple && !details && result.kind === "lab_simulation" ? <div className="lab-simple-results" data-lab-simple-results>
              <SimpleSummary result={result} />
              <RunResults result={result} market={savedMarketValue} />
              <button type="button" className="dbtn sm lab-more" data-lab-more onClick={() => setDetails(true)}>More detail ▸ performance report, trades, monthly returns, Monte Carlo</button>
            </div>
            : <section className="box lab-results-tabs" data-lab-results-tabs>
              <h2 className="box-h">Results{simple && result.kind === "lab_simulation" && <button type="button" className="dbtn sm ghost" onClick={() => setDetails(false)}>◂ Back to the summary</button>}</h2>
              {(() => {
                const tabs: [string, ReactNode][] = result.kind === "lab_portfolio_simulation"
                  ? [["Portfolio results", <PortfolioResults result={result} />]]
                  : [
                    ...(result.report ? [["Performance metrics", <ReportPanel report={result.report} trades={result.trades} spec={ranSpec ?? undefined} run={result} />] as [string, ReactNode]] : []),
                    ["Equity & drawdown", <RunResults result={result} market={savedMarketValue} />],
                    ["Monthly returns", <RunTables result={result} part="monthly" />],
                    ["Trades", <RunTables result={result} part="trades" explain={ranSpec && ranOwn && cat ? { spec: ranSpec, own: ranOwn, catalogue: cat, tf: state.tf, fees: settings.fees, slippage: settings.slippage } : undefined} />],
                    ...(result.report ? [["Monte Carlo", <MonteCarloPanel trades={result.trades} from={result.from} to={result.to} ledger={result.ledger} capital={result.money?.capital} />] as [string, ReactNode]] : []),
                  ];
                if (ticker) tabs.push(["Scan / Explore", <ExplorePanel spec={runSpec} catalogue={chartCat!} chips={state.chips} portfolio={!!state.spec.portfolio} market={savedMarketValue} loadSeries={loadSeries}
          onScan={(apply) => { assertRunnable(runSpec); return lab.scan(market, runSpec, { ticker, apply, from: settings.from || undefined, to: settings.to || undefined }); }}
          onExplore={() => { assertRunnable(runSpec); return lab.explore(market, runSpec, { ticker, from: settings.from || undefined, to: settings.to || undefined }); }} />])
                return <SettingsTabs ariaLabel="Backtest results" labels={tabs.map(([l]) => l)}>{tabs.map(([, el]) => el)}</SettingsTabs>;
              })()}
            </section>}
        </Part> : <div className="box empty" data-lab-no-result>No test yet. Set up your idea in step 2, then press <b>{simple ? "▶ Test this idea" : "Run backtest"}</b>.</div>}
        {result?.kind === "lab_simulation" && !result.trades.length && !runError && ranSpec && cat && ticker
          ? <NoTradesHelp spec={ranSpec} catalogue={cat} market={savedMarketValue} ticker={ticker} period={runSettings} scan={scanSignals} onFix={fixAndRun} /> : null}
        {result?.kind === "lab_simulation" && !runError && result.trades.length > 0 && <button type="button" className="dbtn sm" data-lab-see-chart onClick={() => setTab("chart")}>See the trades on the chart ›</button>}
        {result && !runError && ranDefault ? <p className="rnote" role="note" data-lab-default-exit-used>No sell rule was set, so this test used the default exit: {ranDefault}. Change it in “Your idea”.</p> : null}
        {result?.foreign?.length && !runError ? <div className="rnote" role="note" data-lab-foreign-disclosure>{result.foreign.map((source) => <p key={source.ticker}>{foreignDescription(source)}</p>)}</div> : null}
        {!backtestShown && !simple && ticker && <Part key={`explore|${market}|${ticker}`}><ExplorePanel spec={runSpec} catalogue={chartCat!} chips={state.chips} portfolio={!!state.spec.portfolio} market={savedMarketValue} loadSeries={loadSeries}
          onScan={(apply) => { assertRunnable(runSpec); return lab.scan(market, runSpec, { ticker, apply, from: settings.from || undefined, to: settings.to || undefined }); }}
          onExplore={() => { assertRunnable(runSpec); return lab.explore(market, runSpec, { ticker, from: settings.from || undefined, to: settings.to || undefined }); }} /></Part>}
        </div>
        <div className="lab-step-nav">
          {stepIndex > 0 && <button type="button" className="dbtn" onClick={() => setStep(STEPS[stepIndex - 1].id)}>‹ Back: {STEPS[stepIndex - 1].label}</button>}
          {stepIndex < STEPS.length - 1 && <button type="button" className="dbtn primary" data-lab-next onClick={() => setStep(STEPS[stepIndex + 1].id)}>Next: {STEPS[stepIndex + 1].label} ›</button>}
        </div>
        </div>
        <div role="tabpanel" id="lab-panel-optimization" aria-labelledby="lab-tab-optimization" hidden={tab !== "optimization"} className="lab-tab-panel">
        <p className="lab-tab-intro">Compare settings for <b>{state.spec.portfolio?"the selected portfolio":ticker}</b>. Your strategy rules stay the same until you apply a result.</p>
        <Part key={JSON.stringify([market, ticker, settings.from, settings.to, settings.fees, settings.slippage])}>{/* Results and exports belong to the same instrument, dates and costs. */}
          <OptimizePanel spec={runSpec} catalogue={cat} market={savedMarketValue} role={role} dates={result?.dates ?? []}
            onOptimize={(spec, split) => { assertRunnable(spec); return lab.optimize(market, spec, {
              ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage), split,
            }); }}
            onWalkforward={(spec, o) => { assertRunnable(spec); return lab.walkforward(market, spec, {
              ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage), ...o,
            }); }}
            onIndividual={(spec) => { assertRunnable(spec); return lab.optimizeIndividual(market, spec, { from: settings.from || undefined, to: settings.to || undefined }); }}
            onApply={(spec) => { setTab("strategy"); dispatch({ type: "spec", spec }); void run(spec); }}
            onTried={(n) => dispatch({ type: "tried", n })} tried={state.tried} instrumentLabel={option?.instrumentLabel}
            onRun={(spec) => { assertRunnable(spec); return lab.run(market, spec, {
              ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage),
            }); }} ticker={ticker} />
        </Part>
        </div>
        <div role="tabpanel" id="lab-panel-customize" aria-labelledby="lab-tab-customize" hidden={tab !== "customize"} className="lab-tab-panel">
        <Part>
          <CustomizePanel market={savedMarketValue} ticker={ticker} selected={customize} instrumentType={option?.type}
            catalogue={cat} spec={state.spec} onSpec={(spec) => dispatch({ type: "spec", spec })}
            templates={templates} templatesError={templatesError} template={template} templateError={templateError}
            onSelect={(id) => { setCustomize(id); writeLabQuery({ customize: id }); }}
            onClear={() => { setCustomize(""); writeLabQuery({ customize: "" }); }}
            loadTemplate={(id) => lab.template(market, id)}
            loadFidelity={(id, tk) => lab.fidelity(market, id, tk, settings.from || undefined, settings.to || undefined)}
            running={running} canRun={!!ticker && problems.length === 0} resultReady={!!result && !runError} onRun={() => { setTab("strategy"); void run(); }} onShowRules={() => showStrategy()} onShowResult={() => showStrategy("[data-lab-result]")}
            onSaveVariant={(name) => { setTab("saved"); setSavePrefill((p) => ({ name, seq: (p?.seq ?? 0) + 1 })); }} />
        </Part>
        </div>
        <div role="tabpanel" id="lab-panel-saved" aria-labelledby="lab-tab-saved" hidden={tab !== "saved"} className="lab-tab-panel">
        <Part>
          <SavedStrategies role={role} api={lab.strategies} editor={editorState({ ...state, spec: runSpec }, savedMarketValue)} methodNames={methodNames} prefill={savePrefill}
            onOpen={openRow} onSaved={(row) => dispatch({ type: "row", row })} />
        </Part>
        {role === "admin" && <Part><ReviewQueue api={lab.strategies} onOpen={openRow} /></Part>}
        </div>
      </div>
    </div>
    </FormulaContext.Provider></ForeignProvider></CompactContext.Provider></LabModeContext.Provider>
  );

  return (
    <div className="lab-page" data-lab-page="indicators" data-chart-workspace={tab === "chart" || undefined}>
      <header>
        <h1 className="st">Indicators</h1>
        <p>{COPY.lead}</p>
        {cat && <ServerLabel text={cat.label} />}
      </header>
      {tab !== "chart" && chartToolbar(null)}
      <div className="lab-tabs-row">
      <div className="lab-tabs" role="tablist" aria-label="Indicators workspace">
        {tabs.map((item, i) => <button key={item.id} id={`lab-tab-${item.id}`} type="button" role="tab" aria-selected={tab === item.id} aria-controls={`lab-panel-${item.id}`} tabIndex={tab === item.id ? 0 : -1}
          onClick={() => setTab(item.id)} onKeyDown={(event) => {
            const next = event.key === "ArrowRight" ? (i + 1) % tabs.length : event.key === "ArrowLeft" ? (i + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : undefined;
            if (next !== undefined) { event.preventDefault(); setTab(tabs[next].id); document.getElementById(`lab-tab-${tabs[next].id}`)?.focus(); }
          }}>{item.label}</button>)}
      </div>
      <ModeSwitch mode={mode} onChange={setMode} />
      </div>
      {swapNote && <p className="rnote" data-lab-swap>{swapNote}</p>}
      {work}
    </div>
  );
}
