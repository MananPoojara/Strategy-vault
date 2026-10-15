import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type ReactNode } from "react";
import { getAccount } from "../services/api";
import { vaultStore } from "../state/store";
import * as lab from "../features/lab/client";
import { COPY } from "../features/lab/labels";
import { readLabQuery, writeLabQuery } from "../features/lab/url-state";
import { editorState, initialLabState, labReducer, runKey } from "../features/lab/lab-state";
import { runQuery, specProblems } from "../features/lab/spec-model";
import { availability } from "../features/lab/catalogue";
import { defaultTicker, optionsFromGl, optionsFromTickers, savedMarket } from "../features/lab/instruments";
import { ErrorNotice, LabErrorBoundary, Loading, ServerLabel } from "../features/lab/ui";
import { Toolbar } from "../features/lab/Toolbar";
import { LibraryRail } from "../features/lab/LibraryRail";
import { IndicatorDetail } from "../features/lab/IndicatorDetail";
import { Workbench } from "../features/lab/Workbench";
import { RuleBuilder, type RunSettings } from "../features/lab/RuleBuilder";
import { RunResults } from "../features/lab/RunResults";
import { OptimizePanel } from "../features/lab/OptimizePanel";
import { CustomizePanel } from "../features/lab/CustomizePanel";
import { SavedStrategies } from "../features/lab/SavedStrategies";
import { ReviewQueue } from "../features/lab/ReviewQueue";
import type {
  Catalogue, Indicator, InstrumentsResponse, LabStrategy, Market, Role, RunResult, Spec, TemplateResponse, TemplatesResponse, TickerRow,
} from "../features/lab/types";

export function IndicatorsPage() {
  return <LabErrorBoundary><IndicatorsLab /></LabErrorBoundary>;
}

/** Everything loaded for one market, tagged so a late answer for another market is never used. */
type MarketData = { market: Market; cat: Catalogue; rows?: TickerRow[]; gl?: InstrumentsResponse } | { market: Market; error: unknown };

const Part = ({ children }: { children: ReactNode }) => <LabErrorBoundary>{children}</LabErrorBoundary>;

function IndicatorsLab() {
  const query = useMemo(() => readLabQuery(), []);
  const [state, dispatch] = useReducer(labReducer, undefined, () =>
    initialLabState(query, /[?&]region=/.test(location.hash) ? undefined : vaultStore.getSnapshot().state?.region));
  const [role, setRole] = useState<Role | null>(null);
  const [data, setData] = useState<MarketData | null>(null);
  const [result, setResult] = useState<RunResult | null>(null);
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
  const { market, ticker } = state;

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
    return loaded.gl ? optionsFromGl(loaded.gl, allowedTypes) : optionsFromTickers(loaded.rows ?? [], allowedTypes);
  }, [loaded, allowedTypes]);
  const option = options?.find((o) => o.ticker === ticker);
  const savedMarketValue = savedMarket(market, option);

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
    opened.current = row.instrument; setSwapNote("");
    dispatch({ type: "market", market: row.market === "crypto" ? "gl" : row.market });
    dispatch({ type: "ticker", ticker: row.instrument });
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
        dispatch({ type: "ticker", ticker: snap.instrument });
        dispatch({ type: "open", row: null, spec: snap.spec });
      }, (e: unknown) => !cancelled && setOpenError(e));
    }
    return () => { cancelled = true; };
  }, [query, openRow]);

  const changeMarket = (m: Market) => {
    if (m === market) return;
    opened.current = null; setSwapNote("");
    dispatch({ type: "market", market: m });
    setCustomize(""); setReading(null); setOpenError(null);
    writeLabQuery({ customize: "", strategy: "", published: "" });
  };

  const run = async (spec: Spec = state.spec) => {
    if (!ticker) return; // no selectable instrument on this market
    const n = ++runSeq.current;
    setResult(null); setRunError(null); setRunning(true);
    try {
      const r = await lab.run(market, spec, {
        ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage),
      });
      if (n !== runSeq.current) return;
      setResult(r);
      dispatch({ type: "ran", key: runKey(spec, savedMarketValue, ticker), combos: 0 });
    } catch (e) {
      if (n === runSeq.current) setRunError(e);
    } finally {
      if (n === runSeq.current) setRunning(false);
    }
  };

  const loadSeries = useCallback((q: { id: string; tf: string; p?: Record<string, number> }) => lab.series(market, { ...q, ticker }), [market, ticker]);
  const problems = useMemo(() => (cat ? specProblems(state.spec, cat) : []), [state.spec, cat]);
  const methodNames = useMemo(() => Object.fromEntries(templates?.templates.map((t) => [t.method.id, t.method.name]) ?? []), [templates]);
  const glNote = loaded && "gl" in loaded ? loaded.gl?.provisional_note : undefined;
  const hasVolume = !!option?.hasVolume;

  let work: ReactNode;
  if (loaded && "error" in loaded) work = <ErrorNotice error={loaded.error} market={market} />;
  else if (!cat || !options) work = <Loading text="Loading the Indicators lab…" />;
  else work = (
    <div className="lab-grid">
      <Part>
        <LibraryRail catalogue={cat} market={market} hasVolume={hasVolume} tf={state.tf}
          onAdd={(ind) => { if (availability(ind, { market, hasVolume, tf: state.tf }).ok) dispatch({ type: "chip-add", indicator: ind }); }}
          onRead={(ind) => setReading((cur) => (cur?.id === ind.id ? null : ind))} />
      </Part>
      <div className="lab-main">
        {reading && <Part><IndicatorDetail indicator={reading} catalogue={cat} onClose={() => setReading(null)} /></Part>}
        <Part>
          {ticker ? (
            <Workbench market={savedMarketValue} ticker={ticker} tf={state.tf} range={state.range} catalogue={cat} chips={state.chips}
              trades={result?.trades} loadSeries={loadSeries} instrumentLabel={option?.instrumentLabel}
              onChipChange={(key, params) => dispatch({ type: "chip-set", key, params })}
              onChipRemove={(key) => dispatch({ type: "chip-remove", key })} />
          ) : <div className="box empty">No instrument is available on this market.</div>}
        </Part>
        <div className="two">
          <Part>
            {openError ? <ErrorNotice error={openError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} /> : null}
            <RuleBuilder spec={state.spec} catalogue={cat} onChange={(spec) => dispatch({ type: "spec", spec })}
              settings={settings} onSettings={(patch) => setSettings((s) => ({ ...s, ...patch }))}
              problems={problems} running={running} onRun={() => void run()} />
          </Part>
          <Part>
            {runError ? <ErrorNotice error={runError} market={savedMarketValue} instrumentLabel={option?.instrumentLabel} />
              : result ? <RunResults result={result} market={savedMarketValue} />
              : running ? <Loading text="Running the backtest…" /> : null}
          </Part>
        </div>
        <Part key={`${market}|${ticker}`}>{/* a ticker change remounts it: an optimization belongs to one instrument */}
          <OptimizePanel spec={state.spec} catalogue={cat} market={savedMarketValue} role={role} dates={result?.dates ?? []}
            onOptimize={(spec, split) => lab.optimize(market, spec, {
              ticker, from: settings.from || undefined, to: settings.to || undefined, ...runQuery(spec, settings.fees, settings.slippage), split,
            })}
            onApply={(spec) => { dispatch({ type: "spec", spec }); void run(spec); }}
            onTried={(n) => dispatch({ type: "tried", n })} tried={state.tried} instrumentLabel={option?.instrumentLabel} />
        </Part>
        <Part>
          <CustomizePanel market={savedMarketValue} ticker={ticker} selected={customize}
            templates={templates} templatesError={templatesError} template={template} templateError={templateError}
            onSelect={(id) => { setCustomize(id); writeLabQuery({ customize: id }); }}
            onClear={() => { setCustomize(""); writeLabQuery({ customize: "" }); }}
            loadFidelity={(id, tk) => lab.fidelity(market, id, tk, settings.from || undefined, settings.to || undefined)} />
        </Part>
        <Part>
          <SavedStrategies role={role} api={lab.strategies} editor={editorState(state, savedMarketValue)} methodNames={methodNames}
            onOpen={openRow} onSaved={(row) => dispatch({ type: "row", row })} />
        </Part>
        {role === "admin" && <Part><ReviewQueue api={lab.strategies} onOpen={openRow} /></Part>}
      </div>
    </div>
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
      {swapNote && <p className="rnote" data-lab-swap>{swapNote}</p>}
      {work}
    </div>
  );
}
