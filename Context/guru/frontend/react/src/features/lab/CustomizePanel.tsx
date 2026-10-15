import { useEffect, useRef, useState } from "react";
import { bookLabel, bookTitle, notAdmittedReason, requirementText } from "./books";
import { COPY, unavailableText, variantLabel } from "./labels";
import { NumField } from "./OperandEditor";
import { canonicalJson, describeSpec, setAt, specSummary, type NumSlot, type Phrase } from "./spec-model";
import type { Catalogue, FidelityResult, FTrade, SavedMarket, Spec, TemplateListItem, TemplateResponse, TemplatesResponse } from "./types";
import { ErrorNotice, Loading, ServerLabel } from "./ui";
import "./customize.css";

type Fidelity = { loading: true } | { result: FidelityResult } | { error: unknown };

/** Scrolls another lab section into view and moves keyboard focus there. */
function jumpTo(selector: string) {
  const el = document.querySelector<HTMLElement>(selector);
  if (!el) return;
  el.scrollIntoView?.({ behavior: "smooth", block: "start" });
  if (!el.hasAttribute("tabindex")) el.setAttribute("tabindex", "-1");
  el.focus({ preventScroll: true });
}

const summaryOf = (t: TemplateResponse, cat: Catalogue) =>
  t.status === "available" ? specSummary(t.template.spec, cat) : unavailableText(t.reason);

export function CustomizePanel(props: {
  market: SavedMarket; ticker: string; selected: string; instrumentType?: string;
  catalogue: Catalogue; spec: Spec; onSpec(spec: Spec): void;
  templates: TemplatesResponse | null; templatesError?: unknown;
  template: TemplateResponse | null; templateError?: unknown;
  onSelect(methodId: string): void; onClear(): void;
  loadTemplate(methodId: string): Promise<TemplateResponse>;
  loadFidelity(methodId: string, ticker: string): Promise<FidelityResult>;
  running: boolean; canRun: boolean; resultReady: boolean; onRun(): void;
  onSaveVariant(name: string): void; onShowRules?(): void; onShowResult?(): void;
}) {
  const { market, ticker, selected, templates, template, catalogue } = props;
  const [query, setQuery] = useState("");
  const [fidelity, setFidelity] = useState<Fidelity | null>(null);
  const [summaries, setSummaries] = useState<Record<string, string>>({});
  const requested = useRef(new Set<string>());
  const seq = useRef(0);
  // A new method or ticker invalidates both the shown result and any request still in flight.
  useEffect(() => { seq.current++; setFidelity(null); }, [selected, ticker]);

  const head = <h2 className="box-h">Customize a book method <small>pick a method · change its numbers · run it</small></h2>;
  if (market === "gl" || market === "crypto") {
    return <section className="box lab-customize" data-lab-customize>{head}<p className="rnote">{COPY.customizeGlOnly}</p></section>;
  }

  // Card summaries load as cards come into view (or on hover/focus): one cached request per method, through the client's 4 slots.
  const want = (id: string) => {
    const key = `${market}:${id}`;
    if (requested.current.has(key)) return;
    requested.current.add(key);
    props.loadTemplate(id).then(
      (t) => setSummaries((s) => ({ ...s, [key]: summaryOf(t, catalogue) })),
      () => requested.current.delete(key), // try again on the next hover
    );
  };

  const q = query.trim().toLowerCase();
  const shown = (templates?.templates ?? []).filter(({ method: m }) =>
    !q || [m.name, m.id, m.book, bookLabel(m.book), m.group].some((t) => t.toLowerCase().includes(q)));
  const groups = new Map<string, TemplateListItem[]>();
  for (const t of shown) groups.set(t.method.book, [...(groups.get(t.method.book) ?? []), t]);

  const compare = (methodId: string) => {
    const n = ++seq.current;
    setFidelity({ loading: true });
    props.loadFidelity(methodId, ticker).then(
      (result) => { if (n === seq.current) setFidelity({ result }); },
      (error) => { if (n === seq.current) setFidelity({ error }); },
    );
  };
  const clear = <button type="button" className="dbtn sm ghost" onClick={props.onClear}>Close</button>;

  return (
    <section className="box lab-customize" data-lab-customize>
      {head}
      {props.templatesError ? <ErrorNotice error={props.templatesError} market={market} />
        : !templates ? <Loading text="Loading book templates…" />
        : (
          <div>
            <ServerLabel text={templates.label} />
            <div className="lab-row lab-cz-tools">
              <label className="dfield">
                <input type="search" aria-label="Search book methods" placeholder="Search methods or books" value={query} onChange={(e) => setQuery(e.target.value)} />
              </label>
              <span className="cap">{templates.templates.length} templates · {templates.admitted} admitted methods · {templates.methods} catalogued</span>
            </div>
            <div className="lab-books">
              {[...groups].map(([book, items]) => (
                <details key={book} className="lab-book" open>
                  <summary><b>{bookLabel(book)}</b> <span>{items.length} method{items.length === 1 ? "" : "s"}</span></summary>
                  <ul className="lab-methods">
                    {items.map((t) => (
                      <MethodCard key={`${market}:${t.method.id}`} item={t} selected={t.method.id === selected} onSelect={props.onSelect}
                        reason={notAdmittedReason(t.requirement, t.allowed_types, props.instrumentType)}
                        summary={summaries[`${market}:${t.method.id}`]} onWant={() => want(t.method.id)} />
                    ))}
                  </ul>
                </details>
              ))}
              {groups.size === 0 && <p className="cap">No method matches “{query}”.</p>}
            </div>
          </div>
        )}
      {selected && (props.templateError ? <ErrorNotice error={props.templateError} market={market} />
        : !template ? <Loading text="Loading template…" />
        : template.status === "unavailable" ? <div className="lab-row"><p className="rnote">{unavailableText(template.reason)}</p>{clear}</div>
        : (() => {
          const m = template.method;
          const original = template.template.spec;
          const inSync = props.spec.origin?.method === m.id;
          const changed = inSync && canonicalJson(props.spec) !== canonicalJson(original);
          const fill = original.entry.limit ? "a limit order" : original.execution?.fill === "close" ? "the signal close" : "the next open";
          return (
            <div className="lab-template" data-lab-template>
              <ServerLabel text={template.label} />
              <div className="lab-cz-head">
                <h3>Customize this method: {m.name}</h3>
                <button type="button" className="dbtn sm ghost" data-lab-action="jump-rules" onClick={() => props.onShowRules ? props.onShowRules() : jumpTo(".lab-rules")}>Edit indicators & rules</button>
                {clear}
              </div>
              <p className="cap">{bookLabel(m.book)} · {m.group}</p>
              <ol className="lab-steps">
                <li>
                  <h4>What it does</h4>
                  <p data-lab-summary>{specSummary(original, catalogue)}</p>
                  <p className="cap">{original.direction === "short" ? "Short" : "Long"} · {requirementText(template.instrument.requirement)} · entries fill at {fill}</p>
                  {changed && <p data-lab-variant-summary><b>Your version:</b> {specSummary(props.spec, catalogue)}</p>}
                </li>
                <li>
                  <h4>Quick number edits</h4>
                  <p className="cap">To replace indicators, add conditions or change orders, choose <b>Edit indicators &amp; rules</b>. Your changes stay in this variant.</p>
                  {inSync ? <RulePhrases spec={props.spec} catalogue={catalogue} onSpec={props.onSpec} />
                    : <p className="rnote">Strategy rules now hold other rules. Pick this method again in the list to load it.</p>}
                </li>
                <li>
                  <h4>Run it</h4>
                  <div className="lab-row">
                    <button type="button" className="dbtn sm primary" data-lab-action="customize-run" disabled={props.running || !props.canRun || !inSync} onClick={props.onRun}>
                      {props.running ? "Running…" : "Run backtest"}
                    </button>
                    <button type="button" className="dbtn sm" data-lab-action="fidelity" disabled={fidelity !== null && "loading" in fidelity}
                      onClick={() => compare(m.id)}>Compare with the book (display only)</button>
                    {props.resultReady && <button type="button" className="dbtn sm ghost" data-lab-action="jump-result" onClick={() => props.onShowResult ? props.onShowResult() : jumpTo("[data-lab-result]")}>See the backtest</button>}
                  </div>
                  {changed && <p className="cap">The comparison runs the book template's own numbers, not your changes.</p>}
                  {fidelity && ("loading" in fidelity ? <Loading text="Comparing with the book engine…" />
                    : "error" in fidelity ? <ErrorNotice error={fidelity.error} market={market} />
                    : <FidelityView r={fidelity.result} />)}
                </li>
                <li>
                  <h4>Save as my variant</h4>
                  <div className="lab-row">
                    <button type="button" className="dbtn sm" data-lab-action="customize-save" onClick={() => props.onSaveVariant(`My ${m.name}`)}>Save as my variant…</button>
                    <span className="cap">Saved as: {variantLabel(bookTitle(m.book), m.name)}</span>
                  </div>
                </li>
              </ol>
              <details className="lab-about">
                <summary>About this template</summary>
                <p>Instrument rule: {requirementText(template.instrument.requirement)} (admits {template.instrument.allowed_types.join(", ")})</p>
                {template.template.notes.length > 0 && <ul>{template.template.notes.map((n) => <li key={n}>{n}</li>)}</ul>}
                <p className="cap">Template version {template.template.version} · method id {m.id}</p>
              </details>
            </div>
          );
        })())}
    </section>
  );
}

function MethodCard({ item, selected, reason, summary, onSelect, onWant }: {
  item: TemplateListItem; selected: boolean; reason?: string; summary?: string; onSelect(id: string): void; onWant(): void;
}) {
  const ref = useRef<HTMLLIElement>(null);
  const loaded = summary !== undefined;
  useEffect(() => {
    const el = ref.current;
    if (loaded || !el || typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver((entries) => { if (entries.some((e) => e.isIntersecting)) { io.disconnect(); onWant(); } }, { rootMargin: "120px" });
    io.observe(el);
    return () => io.disconnect();
  }, [loaded]); // eslint-disable-line react-hooks/exhaustive-deps
  const m = item.method;
  return (
    <li ref={ref} onMouseEnter={onWant} onFocus={onWant}>
      <button type="button" data-lab-method={m.id} aria-pressed={selected} disabled={!!reason} onClick={() => onSelect(m.id)}>
        <span className="lab-m-name">{m.name}</span>
        <span className="lab-m-sum" title={summary}>{summary ?? " "}</span>
        <small>{m.dir.toLowerCase() === "short" ? "Short" : "Long"} · {requirementText(item.requirement)}</small>
        {reason && <small className="lab-m-why">{reason}</small>}
      </button>
    </li>
  );
}

/** The live rules in plain words; every number is an inline input that edits the shared spec (origin untouched). */
function RulePhrases({ spec, catalogue, onSpec }: { spec: Spec; catalogue: Catalogue; onSpec(spec: Spec): void }) {
  const d = describeSpec(spec, catalogue);
  const put = (slot: NumSlot) => (n: number | undefined) => { if (n !== undefined) onSpec(setAt(spec, slot.path, n)); };
  const line = (p: Phrase, key: string) => (
    <li key={key}>
      {p.map((x, i) => (typeof x === "string" ? x
        : <NumField key={i} hideLabel label={x.label} value={x.value} min={x.min} max={x.max} step={x.step} onChange={put(x)} />))}
    </li>
  );
  const exits = [...d.exitWhen, ...d.exitOther];
  return (
    <div className="lab-phrases" data-lab-phrases>
      <p><b>{d.buy} when all of these are true</b></p>
      <ul>{d.entry.map((p, i) => line(p, `e${i}`))}{d.entryOrder && line(d.entryOrder, "limit")}</ul>
      <p><b>{d.sell} {exits.length > 1 ? "on the first of these" : "when this happens"}</b></p>
      <ul>{exits.map((p, i) => line(p, `x${i}`))}</ul>
      <p className="cap">These edits also update Strategy & backtest. Use Edit indicators & rules to replace an indicator or add and remove conditions.</p>
    </div>
  );
}

const cell = (t: FTrade | null, k: "entry" | "exit") => t?.[k] ?? "—";

function FidelityView({ r }: { r: FidelityResult }) {
  return (
    <div className="lab-fidelity">
      <ServerLabel text={r.label} />
      {r.reproduces ? <p className="lab-badge ok" data-lab-fidelity-badge>Reproduces the book engine's trades {r.matched}/{r.book_trades}</p>
        : r.book_trades === 0 && r.template_trades === 0 ? <p>{COPY.fidelityNothing}</p>
        : (
          <>
            <p className="lab-badge warn" data-lab-fidelity-badge>Matched {r.matched} of {r.book_trades} book trades and {r.template_trades} template trades</p>
            {r.differences.length > 0 && (
              <details className="lab-about">
                <summary>Show the differences</summary>
                <div className="tlog">
                  <table className="tbl tl">
                    <thead><tr><th>Book entry</th><th>Book exit</th><th>Template entry</th><th>Template exit</th></tr></thead>
                    <tbody>
                      {r.differences.map((d, i) => (
                        <tr key={i}><td>{cell(d.book, "entry")}</td><td>{cell(d.book, "exit")}</td><td>{cell(d.template, "entry")}</td><td>{cell(d.template, "exit")}</td></tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </details>
            )}
            {r.differences_total > r.differences.length && <p className="cap">{r.differences_total} differences in all</p>}
          </>
        )}
      <p className="cap">{r.note}</p>
    </div>
  );
}
