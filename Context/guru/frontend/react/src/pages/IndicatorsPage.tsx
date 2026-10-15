import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type ReactNode } from "react";
import { getAccount } from "../services/api";
import { vaultStore } from "../state/store";
import * as lab from "../features/lab/client";
import { COPY } from "../features/lab/labels";
import { readLabQuery, writeLabQuery } from "../features/lab/url-state";
import { editorState, initialLabState, labReducer, runKey } from "../features/lab/lab-state";
import { canonicalJson, runQuery, specProblems } from "../features/lab/spec-model";
import { treeProblems } from "../features/lab/rule-tree";
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
import { Workbench } from "../features/lab/Workbench";
import { RuleBuilder, type RunSettings } from "../features/lab/RuleBuilder";
import { RunResults, RunTables } from "../features/lab/RunResults";
import { OptimizePanel } from "../features/lab/OptimizePanel";
import { CustomizePanel } from "../features/lab/CustomizePanel";
import { SavedStrategies } from "../features/lab/SavedStrategies";
import { ReviewQueue } from "../features/lab/ReviewQueue";
import type {
  Catalogue, Indicator, InstrumentsResponse, LabStrategy, Market, Role, LabRunResult, Spec, TemplateResponse, TemplatesResponse, TickerRow,
} from "../features/lab/types";

export function IndicatorsPage() {
  return <LabErrorBoundary><IndicatorsLab /></LabErrorBoundary>;
}

/** Everything loaded for one market, tagged so a late answer for another market is never used. */
type MarketData = { market: Market; cat: Catalogue; rows?: TickerRow[]; gl?: InstrumentsResponse } | { market: Market; error: unknown };

const LAB_TABS = [{ id: "strategy", label: "Strategy & backtest" }, { id: "optimization", label: "Optimization" }, { id: "customize", label: "Customize" }, { id: "saved", label: "Saved strategies" }] as const;
type LabTab = typeof LAB_TABS[number]["id"];

const Part = ({ children }: { children: ReactNode }) => <LabErrorBoundary>{children}</LabErrorBoundary>;

function IndicatorsLab() {
  const query = useMemo(() => readLabQuery(), []);
  const [state, dispatch] = useReducer(labReducer, undefined, () =>
    initialLabState(query, /[?&]region=/.test(location.hash) ? undefined : vaultStore.getSnapshot().state?.region));
  const [tab, setTab] = useState<LabTab>("strategy");
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
  const [runError, setRunError] = useState<unknown>(null);
  const [running, setRunning] = useState(false);
  const [settings, setSettings] = useState<RunSettings>({ from: "", to: "", fees: "", slippage: "" });
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
  const portfolioContext=canonicalJson([market,state.spec.portfolio??null]);
  const currentContext=useRef(portfolioContext);currentContext.current=portfolioContext;
  const issuedContext=useRef(portfolioContext);

  useEffect(() => { getAccount().then((a) => setRole(a.user.role), () => {}); }, []); // the App shell handles sign-in

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
  const options = useMemo(() => {
    if (!loaded || !("cat" in loaded)) return null;
    return loaded.gl ? optionsFromGl(loaded.gl, allowedTypes) : optionsFromTickers(loaded.rows ?? [], allowedTypes, loaded.cat.portfolio?.instruments);
  }, [loaded, allowedTypes]);
  // Signal sources are not constrained by a selected book template's execution types.
  const signalOptions = useMemo(() => !loaded || !("cat" in loaded) ? [] : loaded.gl ? optionsFromGl(loaded.gl) : optionsFromTickers(loaded.rows ?? [], undefined, loaded.cat.portfolio?.instruments), [loaded]);
  const foreignScope = useMemo<ForeignScope | null>(() => cat ? { catalogue: cat, ticker:state.spec.portfolio?"":ticker, market, options: signalOptions, spec: state.spec, portfolio:!!state.spec.portfolio } : null, [cat, ticker, market, signalOptions, state.spec]);
  const comparisonOptions = useMemo(() => foreignScope ? foreignOptions(foreignScope) : [], [foreignScope]);
  const option = options?.find((o) => o.ticker === ticker);
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

  const run = async (spec: Spec = state.spec) => {
    if (!ticker && !spec.portfolio) return; // no selectable instrument on this market
    const n = ++runSeq.current;
    const sentContext=canonicalJson([market,spec.portfolio??null]);issuedContext.current=sentContext;currentContext.current=sentContext;
    setResult(null); setRunError(null); setRunning(true);
    try {
      assertRunnable(spec);
      const r = await lab.run(market, spec, {
        ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage),
      });
      if (n !== runSeq.current || sentContext!==currentContext.current) return;
      setResult(r);
      dispatch({ type: "ran", key: runKey(spec, savedMarketValue, ticker), combos: 0 });
    } catch (e) {
      if (n === runSeq.current) setRunError(e);
    } finally {
      if (n === runSeq.current) setRunning(false);
    }
  };

  const loadSeries = useCallback((q: { id: string; tf: string; p?: Record<string, number>; vs?: string }) => lab.series(market, { ...q, ticker }), [market, ticker]);
  const problems = problemsFor(state.spec);
  const methodNames = useMemo(() => Object.fromEntries(templates?.templates.map((t) => [t.method.id, t.method.name]) ?? []), [templates]);
  const glNote = loaded && "gl" in loaded ? loaded.gl?.provisional_note : undefined;
  const hasVolume = !!option?.hasVolume;

  const backtestShown = !!(result || running || runError);

  let work: ReactNode;
  if (loaded && "error" in loaded) work = <ErrorNotice error={loaded.error} market={market} />;
  else if (!cat || !options) work = <Loading text="Loading the Indicators lab…" />;
  else work = (
    <ForeignProvider value={foreignScope!}>
    <div className={`lab-grid${tab === "strategy" ? "" : " lab-grid-focused"}`}>
      <div className="lab-library-slot" hidden={tab !== "strategy"}>
      <Part>
        <LibraryRail catalogue={cat} market={market} hasVolume={hasVolume} tf={state.tf}
          onAdd={(ind) => { if (availability(ind, { market, hasVolume, tf: state.tf, maxForeign: cat.max_foreign }).ok) dispatch({ type: "chip-add", indicator: ind }); }}
          onRead={(ind) => setReading((cur) => (cur?.id === ind.id ? null : ind))} />
      </Part>
      </div>
      <div className="lab-main">
        <div role="tabpanel" id="lab-panel-strategy" aria-labelledby="lab-tab-strategy" hidden={tab !== "strategy"} className="lab-tab-panel">
        {reading && <Part><IndicatorDetail indicator={reading} catalogue={cat} onClose={() => setReading(null)} /></Part>}
        <Part>
          {ticker ? (
            <Workbench market={savedMarketValue} ticker={ticker} tf={state.tf} range={state.range} catalogue={cat} chips={state.chips}
              trades={result?.kind==="lab_simulation"?result.trades:undefined} loadSeries={loadSeries} instrumentLabel={option?.instrumentLabel}
              onChipChange={(key, params) => dispatch({ type: "chip-set", key, params })}
              onChipRemove={(key) => dispatch({ type: "chip-remove", key })} comparisonOptions={comparisonOptions}
              onChipVsChange={(key, vs) => dispatch({ type: "chip-vs", key, vs })} />
          ) : <div className="box empty">No instrument is available on this market.</div>}
        </Part>
        {/* No backtest yet: the rules take the whole row instead of leaving the right half empty. */}
        <div className="two lab-two" data-lab-layout={backtestShown ? "stacked" : "full"}>
          <Part>
            <div className="lab-rules-col">
            {openError ? <ErrorNotice error={openError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} /> : null}
            <PortfolioPanel spec={state.spec} catalogue={cat} market={market} options={signalOptions} onChange={(spec)=>{clearRun();dispatch({type:"spec",spec});}}/>
            <RuleBuilder spec={state.spec} catalogue={cat} instrument={instrument} onChange={(spec) => dispatch({ type: "spec", spec })}
              settings={settings} onSettings={(patch) => setSettings((s) => ({ ...s, ...patch }))}
              problems={problems} running={running} onRun={() => void run()} />
            </div>
          </Part>
          {backtestShown && <Part>
            {runError ? <ErrorNotice error={runError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} />
              : result ? result.kind==="lab_portfolio_simulation" ? <PortfolioResults result={result}/> : <RunResults result={result} market={savedMarketValue} />
              : <Loading text="Running backtest…" />}
          </Part>}
        </div>
        {result?.foreign?.length && !runError ? <div className="rnote" role="note" data-lab-foreign-disclosure>{result.foreign.map((source) => <p key={source.ticker}>{foreignDescription(source)}</p>)}</div> : null}
        {result?.kind==="lab_simulation" && !runError && <Part><RunTables result={result} /></Part>}
        </div>
        <div role="tabpanel" id="lab-panel-optimization" aria-labelledby="lab-tab-optimization" hidden={tab !== "optimization"} className="lab-tab-panel">
        <p className="lab-tab-intro">Compare settings for <b>{state.spec.portfolio?"the selected portfolio":ticker}</b>. Your strategy rules stay the same until you apply a result.</p>
        <Part key={`${market}|${ticker}`}>{/* a ticker change remounts it: an optimization belongs to one instrument */}
          <OptimizePanel spec={state.spec} catalogue={cat} market={savedMarketValue} role={role} dates={result?.dates ?? []}
            onOptimize={(spec, split) => { assertRunnable(spec); return lab.optimize(market, spec, {
              ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage), split,
            }); }}
            onApply={(spec) => { setTab("strategy"); dispatch({ type: "spec", spec }); void run(spec); }}
            onTried={(n) => dispatch({ type: "tried", n })} tried={state.tried} instrumentLabel={option?.instrumentLabel} />
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
          <SavedStrategies role={role} api={lab.strategies} editor={editorState(state, savedMarketValue)} methodNames={methodNames} prefill={savePrefill}
            onOpen={openRow} onSaved={(row) => dispatch({ type: "row", row })} />
        </Part>
        {role === "admin" && <Part><ReviewQueue api={lab.strategies} onOpen={openRow} /></Part>}
        </div>
      </div>
    </div>
    </ForeignProvider>
  );

  return (
    <div className="lab-page" data-lab-page="indicators">
      <header>
        <h1 className="st">Indicators</h1>
        <p>{COPY.lead}</p>
        {cat && <ServerLabel text={cat.label} />}
      </header>
      <Toolbar market={market} onMarket={changeMarket} options={options ?? []} ticker={ticker}
        onTicker={(t) => { opened.current = null; setSwapNote(""); dispatch({ type: "ticker", ticker: t }); }} tf={state.tf} timeframes={cat?.timeframes ?? ["D", "W", "M"]}
        onTf={(tf) => dispatch({ type: "tf", tf })} range={state.range} onRange={(range) => dispatch({ type: "range", range })}
        provisionalNote={glNote} />
      <div className="lab-tabs" role="tablist" aria-label="Indicators workspace">
        {LAB_TABS.map((item, i) => <button key={item.id} id={`lab-tab-${item.id}`} type="button" role="tab" aria-selected={tab === item.id} aria-controls={`lab-panel-${item.id}`} tabIndex={tab === item.id ? 0 : -1}
          onClick={() => setTab(item.id)} onKeyDown={(event) => {
            const next = event.key === "ArrowRight" ? (i + 1) % LAB_TABS.length : event.key === "ArrowLeft" ? (i + LAB_TABS.length - 1) % LAB_TABS.length : event.key === "Home" ? 0 : event.key === "End" ? LAB_TABS.length - 1 : undefined;
            if (next !== undefined) { event.preventDefault(); setTab(LAB_TABS[next].id); document.getElementById(`lab-tab-${LAB_TABS[next].id}`)?.focus(); }
          }}>{item.label}</button>)}
      </div>
      {swapNote && <p className="rnote" data-lab-swap>{swapNote}</p>}
      {work}
    </div>
  );
}
