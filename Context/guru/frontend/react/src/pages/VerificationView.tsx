import { useCallback, useEffect, useMemo, useState } from "react";
import "./VerificationView.css";
import { research, fmtDate, fmtStamp, ago } from "../services/research";
import {
  loadTracker, statusSlices, filterRows, booksFor, strategyHref, pct, reportFolder, STATUS_LABEL, resultGroup, errorStatuses, filterErrors, errorTone,
  type Tracker, type TrackerRow, type TrackerResult, type AuditReport, type Filters, type StatusKey, type ResultGroup, type TrackerError,
} from "../services/verification";

/* Verification tab of the Research page: Yash Darji's tracker workbook, synced every 5 minutes. Read-only. */

const num = (n: number | undefined) => (n ?? 0).toLocaleString("en-IN");
const REFRESH_MS = 5 * 60 * 1000, LIST_PER = 50, AUTHORS_SHOWN = 12, RESULTS_SHOWN = 8, ERRORS_PER = 25;
const dateOf = (s?: string | null) => (s && !Number.isNaN(Date.parse(s)) ? fmtDate(s) : s || "");

export function VerificationView() {
  const [state, setState] = useState<{ data?: Tracker; error?: string }>({});
  const [sites, setSites] = useState<number>();
  const load = useCallback(() => loadTracker().then(data => setState({ data }), (e: Error) => setState(s => ({ ...s, error: e.message }))), []);
  useEffect(() => {
    void load();
    research.summary().then(s => setSites(s.totals?.strategies), () => undefined);
    const t = setInterval(load, REFRESH_MS);
    return () => clearInterval(t);
  }, [load]);
  if (!state.data) return (
    <section className="vt">
      <h1>Verification tracker</h1>
      {state.error ? <div className="rp-problem" role="alert">{state.error} <button type="button" onClick={load}>Try again</button></div> : <p className="rp-loading">Loading the tracker…</p>}
    </section>
  );
  return <TrackerBody t={state.data} sites={sites} stale={state.error} />;
}

function TrackerBody({ t, sites, stale }: { t: Tracker; sites?: number; stale?: string }) {
  const k = t.overview.kpi;
  const [filters, setFilters] = useState<Filters>({ q: "", author: "", book: "", status: "" });
  const [bookAuthor, setBookAuthor] = useState(t.overview.focus || t.authors[0]?.author || "");
  const showList = (patch: Partial<Filters>) => {
    setFilters({ q: "", author: "", book: "", status: "", ...patch });
    requestAnimationFrame(() => document.getElementById("vt-list")?.scrollIntoView({ behavior: "smooth", block: "start" }));
  };
  const progress = t.overview.progress ?? (k.testable ? k.done / k.testable : 0);
  return (
    <section className="vt">
      <div className="vt-head">
        <div>
          <h1>Verification tracker</h1>
          <p className="rp-muted">Yash Darji's independent check of every strategy version on the site, read from his tracker workbook. Nothing here is edited on the site.</p>
        </div>
        <p className="vt-fresh" title={`Workbook: ${t.snapshot || "Verification Tracker.xlsx"}`}>
          <i aria-hidden="true" />Tracker updated <b>{fmtStamp(t.source_mtime)}</b> <span className="rp-muted">({ago(t.source_mtime)})</span> · synced {fmtStamp(t.synced_at)}
        </p>
      </div>
      {stale && <p className="rp-problem" role="status">Showing the last copy: {stale}</p>}

      <div className="vt-kpis">
        <div className="vt-ringcard rp-panel">
          <Ring value={progress} />
          <div>
            <b>{num(k.done)}</b> of <b>{num(k.testable)}</b> testable versions done
            <p className="rp-small rp-muted">{num(k.versions)} versions in the tracker{sites ? <> · the site lists <b>{num(sites)}</b> strategies (one strategy can have several versions)</> : null}</p>
          </div>
        </div>
        <Kpi n={k.testable} label="Can be tested" cls="vt-testable" />
        <Kpi n={k.done} label="Done" cls="st-done" />
        <Kpi n={k.todo} label="To do" cls="st-todo" />
        <Kpi n={k.cant_test} label="Can't test on site" cls="st-cant_test" />
      </div>
      {t.overview.notes.length > 0 && <div className="vt-notes">{t.overview.notes.map((n, i) => <p key={i} className="rp-small rp-muted">{n}</p>)}</div>}

      <div className="vt-cols">
        <AuthorBars t={t} active={bookAuthor} pick={a => { setBookAuthor(a); setFilters(f => ({ ...f, author: a, book: "" })); }} />
        <div className="vt-stack">
          <StatusDonut t={t} pick={s => showList({ status: s })} />
          <BookBars t={t} author={bookAuthor} pick={book => showList({ author: bookAuthor, book })} />
        </div>
      </div>

      <Results t={t} />
      <Errors t={t} />
      <Bugs t={t} />
      <StrategyList t={t} filters={filters} setFilters={setFilters} />

      {(t.legend.length > 0 || (t.howto?.length ?? 0) > 0) && (
        <details className="rp-panel vt-how">
          <summary>How to read the tracker</summary>
          <dl>
            {t.legend.map(l => <div key={l.label}><dt><Status s={l.key} label={l.label} /></dt><dd>{l.text}</dd></div>)}
            {t.howto?.map(h => <div key={h.term}><dt>{h.term}</dt><dd>{h.text}</dd></div>)}
          </dl>
          <p className="rp-small rp-muted">The tracker counts versions: one testable variant of a strategy, each with its own page on the site. Source: Yash Darji's Verification Tracker.xlsx (read-only copy {t.snapshot}).</p>
        </details>
      )}
    </section>
  );
}

const Kpi = ({ n, label, cls }: { n: number; label: string; cls: string }) => (
  <div className={`rp-stat vt-kpi ${cls}`}><b>{num(n)}</b><span>{label}<small>versions</small></span></div>
);

function Ring({ value }: { value: number }) {
  const r = 46, c = 2 * Math.PI * r, v = Math.max(0, Math.min(1, value));
  return (
    <svg className="vt-ring" viewBox="0 0 120 120" role="img" aria-label={`${pct(v, 1)} of testable versions done`}>
      <circle cx="60" cy="60" r={r} className="track" />
      <circle cx="60" cy="60" r={r} className="fill" strokeDasharray={`${c * v} ${c}`} transform="rotate(-90 60 60)" />
      <text x="60" y="58" textAnchor="middle" className="big">{pct(v, v < 0.1 ? 1 : 0)}</text>
      <text x="60" y="78" textAnchor="middle" className="small">done</text>
    </svg>
  );
}

const Status = ({ s, label }: { s: StatusKey; label?: string }) => (
  <span className={`rp-pill vt-st st-${s}`}><i aria-hidden="true" />{(label || STATUS_LABEL[s] || STATUS_LABEL.other).replace(/^[^\p{L}\d]+/u, "")}</span>
);

function AuthorBars({ t, active, pick }: { t: Tracker; active: string; pick: (a: string) => void }) {
  const [all, setAll] = useState(false);
  const rows = all ? t.authors : t.authors.slice(0, AUTHORS_SHOWN);
  const w = (n: number, of: number) => `${(100 * n) / Math.max(1, of)}%`;
  return (
    <div className="rp-panel vt-authors">
      <h2>Progress by author</h2>
      <p className="rp-small rp-muted">Each bar is one author's versions. Click an author to see its books and filter the list.</p>
      <ul className="vt-key" aria-label="Bar colours">
        <li><i className="st-done" />Done</li><li><i className="st-done_earlier" />Done earlier (trades only)</li><li><i className="st-todo" />To do</li><li><i className="st-cant_test" />Can't test</li>
      </ul>
      <ol className="vt-abars">
        {rows.map(a => (
          <li key={a.author}>
            <button type="button" aria-pressed={a.author === active} onClick={() => pick(a.author)}
              title={`${a.author}: ${a.done} done, ${a.done_earlier} done earlier, ${a.todo} to do, ${a.cant_test} can't test (${a.versions} versions)`}>
              <span className="vt-aname">{a.author}</span>
              <span className="vt-track" aria-hidden="true">
                <i className="st-done" style={{ width: w(a.done, a.versions) }} />
                <i className="st-done_earlier" style={{ width: w(a.done_earlier, a.versions) }} />
                <i className="st-todo" style={{ width: w(a.todo, a.versions) }} />
                <i className="st-cant_test" style={{ width: w(a.cant_test, a.versions) }} />
              </span>
              <span className="vt-anum"><b>{pct(a.progress)}</b><small>{num(a.done + a.done_earlier)} / {num(a.testable)}</small></span>
            </button>
          </li>
        ))}
      </ol>
      {t.authors.length > AUTHORS_SHOWN && <button type="button" className="rp-link" onClick={() => setAll(!all)}>{all ? `Show the first ${AUTHORS_SHOWN}` : `Show all ${num(t.authors.length)} authors`}</button>}
    </div>
  );
}

function StatusDonut({ t, pick }: { t: Tracker; pick: (s: StatusKey) => void }) {
  const slices = statusSlices(t), total = slices.reduce((a, s) => a + s.count, 0);
  const [hover, setHover] = useState<StatusKey>();
  const r = 52, c = 2 * Math.PI * r, gap = slices.length > 1 ? 2 : 0;
  let at = 0;
  const shown = slices.find(s => s.key === hover);
  return (
    <div className="rp-panel vt-donut">
      <h2>Versions by status</h2>
      <div className="vt-donutrow">
        <svg viewBox="0 0 140 140" role="img" aria-label={slices.map(s => `${s.label}: ${s.count}`).join(", ")}>
          <circle cx="70" cy="70" r={r} className="track" />
          {slices.map(s => {
            const len = (c * s.count) / Math.max(1, total), start = at;
            at += len;
            return (
              <circle key={s.key} cx="70" cy="70" r={r} className={`seg st-${s.key}${hover && hover !== s.key ? " dim" : ""}`}
                strokeDasharray={`${Math.max(0.5, len - gap)} ${c}`} strokeDashoffset={-start} transform="rotate(-90 70 70)"
                onMouseEnter={() => setHover(s.key)} onMouseLeave={() => setHover(undefined)} onClick={() => pick(s.key)}>
                <title>{`${s.label}: ${num(s.count)} (${pct(s.count / Math.max(1, total), 1)})`}</title>
              </circle>
            );
          })}
          <text x="70" y="68" textAnchor="middle" className="big">{num(shown ? shown.count : total)}</text>
          <text x="70" y="86" textAnchor="middle" className="small">{shown ? pct(shown.count / Math.max(1, total), 1) : "versions"}</text>
        </svg>
        <ul className="vt-slices">
          {slices.map(s => (
            <li key={s.key} onMouseEnter={() => setHover(s.key)} onMouseLeave={() => setHover(undefined)}>
              <button type="button" onClick={() => pick(s.key)} title={`Show the ${s.label} versions in the list`}>
                <i className={`st-${s.key}`} aria-hidden="true" /><span>{s.label}</span><b>{num(s.count)}</b><small>{pct(s.count / Math.max(1, total), 1)}</small>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function BookBars({ t, author, pick }: { t: Tracker; author: string; pick: (book: string) => void }) {
  const { rows, fromOverview } = useMemo(() => booksFor(t, author), [t, author]);
  return (
    <div className="rp-panel vt-books">
      <h2>By book <span className="rp-muted">· {author}</span></h2>
      <p className="rp-small rp-muted">{fromOverview ? "From the tracker's Overview (current focus author)." : "Counted from the Strategy list."} Done of testable versions; click a book to list its versions.</p>
      <ol className="vt-bbars">
        {rows.map(b => (
          <li key={b.book}>
            <button type="button" onClick={() => pick(b.book)} title={`${b.book}: ${b.done} of ${b.testable} testable done, ${b.todo} to do, ${b.versions} versions`}>
              <span className="vt-bname">{b.book}</span>
              <span className="vt-track" aria-hidden="true"><i className="st-done" style={{ width: pct(b.testable ? b.done / b.testable : 0, 2) }} /></span>
              <span className="vt-anum"><b>{num(b.done)}/{num(b.testable)}</b>{b.testable === 0 && <small>nothing testable</small>}</span>
            </button>
          </li>
        ))}
      </ol>
      {!rows.length && <p className="rp-empty">No versions for {author}.</p>}
    </div>
  );
}

/** Audit report as text, never a link. `short`: the file name only, the share folder on hover (dense tables). */
const Report = ({ r, short }: { r?: AuditReport | null; short?: boolean }) => r ? (
  <p className="vt-report rp-small" title={short ? r.path : undefined}><span className="rp-muted">Audit report</span> <b>{r.file}</b>{!short && <> <span className="rp-muted vt-path">in {reportFolder(r)}</span></>}</p>
) : null;

function TradeBar({ r }: { r: TrackerResult }) {
  if (!r.trades) return <span className="rp-small rp-muted">no trades checked</span>;
  const w = (n: number) => `${(100 * n) / r.trades}%`;
  return (
    <div className="vt-trades" title={`${r.right} right, ${r.wrong} wrong, ${r.to_check} to check of ${r.trades} trades`}>
      <span className="vt-track" aria-hidden="true"><i className="st-done" style={{ width: w(r.right) }} /><i className="bad" style={{ width: w(r.wrong) }} /><i className="st-needs_data" style={{ width: w(r.to_check) }} /></span>
      <span className="rp-small"><b>{num(r.right)}</b> right{r.wrong ? <> · <b className="vt-bad">{num(r.wrong)}</b> wrong</> : null}{r.to_check ? <> · {num(r.to_check)} to check</> : null} <span className="rp-muted">of {num(r.trades)}</span></span>
    </div>
  );
}

const GROUPS: { key: ResultGroup; label: string; note: string }[] = [
  { key: "right", label: "Trades right", note: "trades checked, none wrong or left to check" },
  { key: "wrong", label: "Wrong trades", note: "at least one trade wrong or still to check" },
  { key: "none", label: "No trades", note: "no trades checked (e.g. the site produced no exports)" },
];

function Results({ t }: { t: Tracker }) {
  const [all, setAll] = useState(false);
  const [group, setGroup] = useState<ResultGroup>("right");
  const by = useMemo(() => {
    const m: Record<ResultGroup, TrackerResult[]> = { right: [], wrong: [], none: [] };
    for (const r of t.results) m[resultGroup(r)].push(r); // the sync keeps them newest first
    return m;
  }, [t]);
  const rows = by[group], R = t.runs, g = GROUPS.find(x => x.key === group)!;
  return (
    <>
      <h2 className="rp-section">Latest results</h2>
      <p className="rp-muted rp-small">{num(t.results.length)} verified versions, newest first. Across all {num(R.runs)} checked backtests: {num(R.trades)} trades, <b>{num(R.right)}</b> right, <b>{num(R.wrong)}</b> wrong{R.last_verified ? `; last run ${dateOf(R.last_verified)}` : ""}.</p>
      <div className="vt-seg" role="tablist" aria-label="Results">
        {GROUPS.map(x => (
          <button key={x.key} type="button" role="tab" id={`vt-tab-${x.key}`} aria-selected={x.key === group} aria-controls="vt-results" className={`vt-g-${x.key}`}
            onClick={() => { setGroup(x.key); setAll(false); }} title={x.note}>
            <i aria-hidden="true" />{x.label}<b>{num(by[x.key].length)}</b>
          </button>
        ))}
      </div>
      <p className="rp-small rp-muted vt-segnote">{num(rows.length)} {rows.length === 1 ? "result" : "results"}: {g.note}.</p>
      <ul className="vt-results" id="vt-results" role="tabpanel" aria-labelledby={`vt-tab-${group}`}>
        {(all ? rows : rows.slice(0, RESULTS_SHOWN)).map((r, i) => (
          <li key={i} className="rp-panel">
            <time>{dateOf(r.checked)}</time>
            <div className="vt-rmain">
              <p className="vt-rtitle">{r.id ? <a href={strategyHref(r.id)}>{r.strategy}</a> : <b>{r.strategy}</b>} {r.check && <span className={`rp-pill vt-check${/full/i.test(r.check) ? " full" : ""}`}>{r.check}</span>}</p>
              <p className="rp-small rp-muted">{r.author} · {r.book}</p>
              <p>{r.result}</p>
              {r.notes && r.notes !== r.result && <p className="rp-small rp-muted">{r.notes}</p>}
              <Report r={r.report} />
            </div>
            <TradeBar r={r} />
          </li>
        ))}
      </ul>
      {!rows.length && <p className="rp-empty">No result in this group.</p>}
      {rows.length > RESULTS_SHOWN && <button type="button" className="rp-link" onClick={() => setAll(!all)}>{all ? "Show fewer" : `Show all ${num(rows.length)}`}</button>}
    </>
  );
}

function Errors({ t }: { t: Tracker }) {
  const [status, setStatus] = useState("");
  const [q, setQ] = useState("");
  const [page, setPage] = useState(1);
  const statuses = useMemo(() => errorStatuses(t), [t]);
  const known = useMemo(() => new Set(t.strategies.map(s => s.id)), [t]);
  const rows = useMemo(() => filterErrors(t, status, q), [t, status, q]);
  useEffect(() => setPage(1), [status, q]);
  if (!t.errors?.length) return null;
  const pages = Math.max(1, Math.ceil(rows.length / ERRORS_PER)), at = Math.min(page, pages);
  return (
    <>
      <h2 className="rp-section" id="vt-errors">Errors &amp; mismatches</h2>
      <p className="rp-muted rp-small">Every problem Yash found, one row per strategy, run or kind of problem, with why it happened. Open ones first.</p>
      <div className="rp-chips vt-echips" role="group" aria-label="Error status">
        <button type="button" aria-pressed={!status} onClick={() => setStatus("")}>All <b>{num(t.errors.length)}</b></button>
        {statuses.map(s => (
          <button key={s.status} type="button" aria-pressed={status === s.status} disabled={!s.rows} onClick={() => setStatus(status === s.status ? "" : s.status)}
            title={`${s.status}: ${num(s.strategies)} ${s.strategies === 1 ? "strategy" : "strategies"} · ${num(s.rows)} rows`}>
            <i className={`vt-dot et-${s.tone}`} aria-hidden="true" />{s.status} <b>{num(s.rows)}</b>
          </button>
        ))}
      </div>
      <div className="vt-filters vt-efilter">
        <input type="search" placeholder="Find a strategy, ID, index, problem or reason…" value={q} onChange={e => setQ(e.target.value)} aria-label="Find an error" />
      </div>
      <p className="rp-small rp-muted">{num(rows.length)} of {num(t.errors.length)} rows{(q || status) && <> · <button type="button" className="rp-link" onClick={() => { setQ(""); setStatus(""); }}>Clear filters</button></>}</p>
      <div className="rp-panel vt-listpanel">
        <table className="rp-table vt-list vt-errors">
          <thead><tr><th>Status</th><th>Strategy</th><th>Problem</th><th className="n">Count</th><th>Reason</th><th>Why</th></tr></thead>
          <tbody>{rows.slice((at - 1) * ERRORS_PER, at * ERRORS_PER).map((e, i) => <ErrorRow key={`${e.n ?? ""}-${i}`} e={e} linked={!!e.id && known.has(e.id)} />)}</tbody>
        </table>
        {!rows.length && <p className="rp-empty">No row matches these filters.</p>}
      </div>
      {pages > 1 && (
        <nav className="rp-pager" aria-label="Error pages">
          <button type="button" disabled={at <= 1} onClick={() => setPage(at - 1)}>Previous</button>
          <span>Page {at} of {num(pages)}</span>
          <button type="button" disabled={at >= pages} onClick={() => setPage(at + 1)}>Next</button>
        </nav>
      )}
    </>
  );
}

function ErrorRow({ e, linked }: { e: TrackerError; linked: boolean }) {
  const where = [e.index, e.from && `from ${dateOf(e.from)}`, e.settings].filter(Boolean).join(" · ");
  return (
    <tr>
      <td data-label="Status"><span className={`rp-pill vt-st et-${errorTone(e.status)}`}><i aria-hidden="true" />{e.status}</span></td>
      <td data-label="Strategy">{linked ? <a className="vt-elink" href={strategyHref(e.id!)}>{e.strategy}</a> : <b>{e.strategy}</b>}
        {e.id && <><br /><code className="rp-small rp-muted">{e.id}</code></>}
        {e.book && <><br /><span className="rp-small rp-muted">{e.book}</span></>}
        {where && <><br /><span className="rp-small">{where}</span></>}</td>
      <td data-label="Problem" className="rp-small">{e.problem}</td>
      <td data-label="Count" className="n">{e.count != null ? num(e.count) : ""}</td>
      <td data-label="Reason" className="rp-small vt-result">{e.reason && <span title={e.reason}>{e.reason}</span>}{e.example && <p className="rp-muted vt-eex" title={e.example}>e.g. {e.example}</p>}</td>
      <td data-label="Why" className="rp-small vt-ewhy">{e.why}<Report r={e.report} short /></td>
    </tr>
  );
}

function Bugs({ t }: { t: Tracker }) {
  if (!t.bugs.length) return null;
  return (
    <>
      <h2 className="rp-section">Site bugs</h2>
      <p className="rp-muted rp-small">Problems Yash found in the site itself while verifying. They are not strategy errors.</p>
      <ul className="vt-bugs">
        {t.bugs.map((b, i) => (
          <li key={i} className="rp-panel">
            <p className="rp-meta"><span className={`rp-pill vt-bug ${/open/i.test(b.status) ? "open" : "closed"}`}>{b.status || "No status"}</span> {b.n != null && <>#{b.n} · </>}found {dateOf(b.found)}{b.where && <> · {b.where}</>}</p>
            <p>{b.problem}</p>
            {b.evidence && <p className="rp-small rp-muted">How we know: {b.evidence}</p>}
          </li>
        ))}
      </ul>
    </>
  );
}

function StrategyList({ t, filters: f, setFilters }: { t: Tracker; filters: Filters; setFilters: (f: Filters) => void }) {
  const [page, setPage] = useState(1);
  const set = (patch: Partial<Filters>) => setFilters({ ...f, ...patch });
  useEffect(() => setPage(1), [f.q, f.author, f.book, f.status]);
  const authors = useMemo(() => { const has = new Set(t.strategies.map(s => s.author)); return t.authors.map(a => a.author).filter(a => has.has(a)); }, [t]);
  const books = useMemo(() => [...new Set(t.strategies.filter(s => !f.author || s.author === f.author).map(s => s.book))].sort(), [t, f.author]);
  const rows = useMemo(() => filterRows(t.strategies, f), [t, f]);
  const slices = statusSlices(t);
  const pages = Math.max(1, Math.ceil(rows.length / LIST_PER)), at = Math.min(page, pages);
  return (
    <>
      <h2 className="rp-section" id="vt-list">Strategy list</h2>
      <div className="vt-filters">
        <input type="search" placeholder="Find a strategy, version, book or ID…" value={f.q} onChange={e => set({ q: e.target.value })} aria-label="Find a strategy" />
        <select value={f.author} onChange={e => set({ author: e.target.value, book: "" })} aria-label="Author">
          <option value="">All authors ({num(authors.length)})</option>
          {authors.map(a => <option key={a} value={a}>{a}</option>)}
        </select>
        <select value={f.book} onChange={e => set({ book: e.target.value })} aria-label="Book">
          <option value="">All books ({num(books.length)})</option>
          {books.map(b => <option key={b} value={b}>{b}</option>)}
        </select>
      </div>
      <div className="rp-chips" role="group" aria-label="Status">
        <button type="button" aria-pressed={!f.status} onClick={() => set({ status: "" })}>All</button>
        <button type="button" aria-pressed={f.status === "done_any"} onClick={() => set({ status: "done_any" })}>Any done</button>
        {slices.map(s => <button key={s.key} type="button" aria-pressed={f.status === s.key} onClick={() => set({ status: s.key })}><i className={`vt-dot st-${s.key}`} aria-hidden="true" />{s.label}</button>)}
      </div>
      <p className="rp-small rp-muted">{num(rows.length)} of {num(t.strategies.length)} versions{(f.q || f.author || f.book || f.status) && <> · <button type="button" className="rp-link" onClick={() => set({ q: "", author: "", book: "", status: "" })}>Clear filters</button></>}</p>
      <div className="rp-panel vt-listpanel">
        <table className="rp-table vt-list">
          <thead><tr><th className="n">#</th><th>Strategy · version</th><th>Author · book</th><th>Status</th><th>Result</th><th>Checked</th><th><span className="rp-visually-hidden">Open on site</span></th></tr></thead>
          <tbody>
            {rows.slice((at - 1) * LIST_PER, at * LIST_PER).map(r => <ListRow key={r.id} r={r} />)}
          </tbody>
        </table>
        {!rows.length && <p className="rp-empty">No version matches these filters.</p>}
      </div>
      {pages > 1 && (
        <nav className="rp-pager" aria-label="Pages">
          <button type="button" disabled={at <= 1} onClick={() => setPage(at - 1)}>Previous</button>
          <span>Page {at} of {num(pages)}</span>
          <button type="button" disabled={at >= pages} onClick={() => setPage(at + 1)}>Next</button>
        </nav>
      )}
    </>
  );
}

function ListRow({ r }: { r: TrackerRow }) {
  return (
    <tr>
      <td className="n rp-muted" data-label="#">{r.n}</td>
      <td data-label="Strategy"><b>{r.strategy}</b>{r.good && <span className="vt-star" title="Good runs in the tracker"> {r.good}</span>}<br /><span className="rp-small">{r.version}</span>{r.type && <span className="rp-small rp-muted"> · {r.type}</span>}<br /><code className="rp-small rp-muted">{r.id}</code></td>
      <td data-label="Author" className="rp-small">{r.author}<br /><span className="rp-muted">{r.book}</span></td>
      <td data-label="Status"><Status s={r.status} /></td>
      <td data-label="Result" className="rp-small vt-result">{r.result && <span title={r.result}>{r.result}</span>}<Report r={r.report} /></td>
      <td data-label="Checked" className="rp-small">{dateOf(r.checked)}</td>
      <td className="vt-opencell"><a className="vt-open" href={strategyHref(r.id)}>Open ▸</a></td>
    </tr>
  );
}
