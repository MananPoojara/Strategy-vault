import { useEffect, useRef, useState } from "react";
import { COPY, unavailableText } from "./labels";
import type { FidelityResult, FTrade, SavedMarket, TemplateResponse, TemplatesResponse } from "./types";
import { ErrorNotice, Loading, ServerLabel } from "./ui";
import "./customize.css";

type Fidelity = { loading: true } | { result: FidelityResult } | { error: unknown };

export function CustomizePanel(props: {
  market: SavedMarket; ticker: string; selected: string;
  templates: TemplatesResponse | null; templatesError?: unknown;
  template: TemplateResponse | null; templateError?: unknown;
  onSelect(methodId: string): void; onClear(): void;
  loadFidelity(methodId: string, ticker: string): Promise<FidelityResult>;
}) {
  const { market, ticker, selected, templates, template } = props;
  const [query, setQuery] = useState("");
  const [fidelity, setFidelity] = useState<Fidelity | null>(null);
  const seq = useRef(0);
  // A new method or ticker invalidates both the shown result and any request still in flight.
  useEffect(() => { seq.current++; setFidelity(null); }, [selected, ticker]);

  const head = <h2 className="box-h">Customize a book method</h2>;
  if (market === "gl" || market === "crypto") {
    return <section className="box lab-customize" data-lab-customize>{head}<p className="rnote">{COPY.customizeGlOnly}</p></section>;
  }

  const q = query.trim().toLowerCase();
  const shown = (templates?.templates ?? []).filter(({ method: m }) => !q || [m.name, m.id, m.book].some((t) => t.toLowerCase().includes(q)));

  const compare = (methodId: string) => {
    const n = ++seq.current;
    setFidelity({ loading: true });
    props.loadFidelity(methodId, ticker).then(
      (result) => { if (n === seq.current) setFidelity({ result }); },
      (error) => { if (n === seq.current) setFidelity({ error }); },
    );
  };
  const clear = <button type="button" className="dbtn sm ghost" onClick={props.onClear}>Clear</button>;

  return (
    <section className="box lab-customize" data-lab-customize>
      {head}
      {props.templatesError ? <ErrorNotice error={props.templatesError} market={market} />
        : !templates ? <Loading text="Loading book templates…" />
        : (
          <div>
            <ServerLabel text={templates.label} />
            <p className="cap">{templates.templates.length} templates · {templates.admitted} admitted methods · {templates.methods} catalogued</p>
            <label className="dfield">
              <input type="search" aria-label="Search book methods" placeholder="Search book methods" value={query} onChange={(e) => setQuery(e.target.value)} />
            </label>
            <ul className="lab-methods">
              {shown.map(({ method: m, requirement }) => (
                <li key={m.id}>
                  <button type="button" data-lab-method={m.id} aria-pressed={m.id === selected} onClick={() => props.onSelect(m.id)}>
                    <span>{m.name} · {m.book}</span><small>{requirement}</small>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      {selected && (props.templateError ? <ErrorNotice error={props.templateError} market={market} />
        : !template ? <Loading text="Loading template…" />
        : template.status === "unavailable" ? <div className="lab-row"><p className="rnote">{unavailableText(template.reason)}</p>{clear}</div>
        : (
          <div className="lab-template">
            <ServerLabel text={template.label} />
            <h3>{template.method.name} · {template.method.book}</h3>
            <p>Instrument: {template.instrument.requirement} ({template.instrument.allowed_types.join(", ")})</p>
            {template.template.notes.length > 0 && <ul>{template.template.notes.map((n) => <li key={n}>{n}</li>)}</ul>}
            <div className="lab-row"><span className="cap">Template version {template.template.version}</span>{clear}</div>
            <div className="lab-fidelity">
              <div>
                <button type="button" className="dbtn sm" data-lab-action="fidelity" disabled={fidelity !== null && "loading" in fidelity}
                  onClick={() => compare(template.method.id)}>Compare with the book engine (display only)</button>
              </div>
              {fidelity && ("loading" in fidelity ? <Loading text="Comparing with the book engine…" />
                : "error" in fidelity ? <ErrorNotice error={fidelity.error} market={market} />
                : <FidelityView r={fidelity.result} />)}
            </div>
          </div>
        ))}
    </section>
  );
}

const cell = (t: FTrade | null, k: "entry" | "exit") => t?.[k] ?? "—";

function FidelityView({ r }: { r: FidelityResult }) {
  return (
    <div>
      <ServerLabel text={r.label} />
      <p className="rnote">{r.note}</p>
      {r.reproduces ? <p><b>Template reproduces the book engine's trades: {r.matched}/{r.book_trades}</b></p>
        : r.book_trades === 0 && r.template_trades === 0 ? <p>{COPY.fidelityNothing}</p>
        : (
          <>
            <p><b>Matched {r.matched} of {r.book_trades} book trades and {r.template_trades} template trades</b></p>
            {r.differences.length > 0 && (
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
            )}
            {r.differences_total > r.differences.length && <p className="cap">{r.differences_total} differences in all</p>}
          </>
        )}
    </div>
  );
}
