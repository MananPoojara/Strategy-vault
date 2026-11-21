import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import "./ResearchWork.css";
import { TradesheetsView } from "./ResearchTradesheets";
import { ResearchYts } from "./ResearchYts";
import {
  loadResearchWork, ytNames, groupByDay, strategiesAdded, helperKind, readView, viewHref, dayKey, fmtDay, fmtTime, fmtStamp, num,
  type ResearchWork as Work, type WorkDay, type WorkEvent, type WorkLoad, type View,
} from "../services/research-work";

/* Research: what was built on the jay branch, commit by commit and day by day, newest first.
   The data is a static file generated from git history (build_research_work.py); see services/research-work.ts. */

const DAYS_PER_STEP = 5;

function useView() {
  const [v, setV] = useState(() => readView());
  useEffect(() => {
    const sync = () => setV(readView());
    addEventListener("hashchange", sync);
    return () => removeEventListener("hashchange", sync);
  }, []);
  return v;
}

export function ResearchWork() {
  const view = useView();
  const [load, setLoad] = useState<WorkLoad | null>(null);
  const [names, setNames] = useState<Record<string, string>>({});
  useEffect(() => { document.title = "Research · Strategy Vault"; }, []);
  useEffect(() => {
    let live = true;
    loadResearchWork().then(r => live && setLoad(r));
    ytNames().then(n => live && setNames(n));
    return () => { live = false; };
  }, []);
  const name = useCallback((id: string) => names[id] || id, [names]);
  return (
    <div className="rw">
      <nav className="crumbs"><a href="#/">Guru's</a><span>/</span><span>Research</span></nav>
      <ResearchBar view={view}>
        {load?.ok && view !== "tradesheets" && <span className="rw-small rw-muted">Branch <b>{load.data.branch}</b> at <code>{load.data.head.slice(0, 8)}</code></span>}
      </ResearchBar>
      {view === "tradesheets" ? <TradesheetsView />
        : !load ? <p className="rw-loading">Loading…</p>
        : !load.ok ? <div className="rw-problem" role="alert"><b>Research timeline not built.</b> {load.problem}</div>
        : view === "strategies" ? <StrategiesView data={load.data} name={name} />
        : <ClaudeView data={load.data} name={name} />}
    </div>
  );
}

/** The Research tab bar; Research YTS keeps its own address (#/research-yts, used by the "Report issue" links). */
export function ResearchBar({ view, children }: { view: View | "yts"; children?: ReactNode }) {
  return (
    <header className="rw-bar">
      <a className="rw-brand" href={viewHref("claude")}>Research</a>
      <nav aria-label="Research views">
        <a href={viewHref("claude")} aria-current={view === "claude" ? "page" : undefined}>Claude's work</a>
        <a href={viewHref("strategies")} aria-current={view === "strategies" ? "page" : undefined}>Strategies added</a>
        <a href={viewHref("tradesheets")} aria-current={view === "tradesheets" ? "page" : undefined}>Tradesheet verified</a>
        <a href="#/research-yts" aria-current={view === "yts" ? "page" : undefined}>Research YTS</a>
      </nav>
      {children}
    </header>
  );
}

/** Research YTS shown as a Research tab: the Research bar above the unchanged Research YTS page. */
export function ResearchYtsTab() {
  return (
    <>
      <div className="rw rw-tabonly"><ResearchBar view="yts" /></div>
      <ResearchYts />
    </>
  );
}

function Totals({ data }: { data: Work }) {
  const t = data.totals;
  return (
    <div className="rw-stats">
      <div className="rw-stat ok"><b>{num(t.strategies)}</b><span>YT strategies coded</span></div>
      <div className="rw-stat team"><b>{num(t.commits)}</b><span>Commits</span></div>
      <div className="rw-stat todo"><b>{num(t.days)}</b><span>Days with commits</span></div>
      <div className="rw-stat held"><b>{num(t.authors.length)}</b><span>Authors ({t.authors.join(", ")})</span></div>
    </div>
  );
}

/* ---------------------------------------------------------------- Claude's work: the daily timeline */

function ClaudeView({ data, name }: { data: Work; name: (id: string) => string }) {
  const [shown, setShown] = useState(DAYS_PER_STEP);
  const [open, setOpen] = useState<WorkEvent | null>(null);
  const days = useMemo(() => groupByDay(data.events), [data.events]);
  const byDate = useMemo(() => Object.fromEntries(data.days.map(d => [d.date, d])), [data.days]);
  return (
    <section>
      <h1>Claude's work</h1>
      <p className="rw-muted">What was built on the <b>{data.branch}</b> branch, commit by commit, newest first. The number beside each commit is how many YT strategies were coded in the engine after it. Open a commit for its full notes and the strategies it added.</p>
      <Totals data={data} />
      {data.uncommitted.files > 0 && <p className="rw-note">{num(data.uncommitted.files)} files had uncommitted changes when this timeline was built; that work is not shown until it is committed.</p>}
      <DailyWork days={data.days} />
      {days.slice(0, shown).map(({ day, items }) => (
        <div key={day} className="rw-day">
          <h3>{fmtDay(day)}<small> · {num(items.length)} {items.length === 1 ? "commit" : "commits"}{byDate[day]?.added ? ` · +${num(byDate[day].added)} strategies` : ""}</small></h3>
          <ul className="rw-feed">
            {items.map(e => (
              <li key={e.key} className={`w-${helperKind(e) || "person"}`}>
                <button type="button" className="rw-event" onClick={() => setOpen(e)} title="Open the detail of this commit">
                  <span className="rw-count"><b>{num(e.count)}</b><small>YT strategies</small></span>
                  <span className="rw-text">
                    <span className="rw-meta">
                      <time dateTime={e.at}>{fmtTime(e.at)}</time>
                      <span className="rw-pill who">{e.who}</span>
                      {e.with.map(w => <span key={w} className="rw-pill helper">with {w}</span>)}
                      {e.added.length > 0 && <span className="rw-pill added">+{num(e.added.length)} {e.added.length === 1 ? "strategy" : "strategies"}</span>}
                      {e.removed.length > 0 && <span className="rw-pill removed">−{num(e.removed.length)} removed</span>}
                      {e.merge && <span className="rw-pill">merge</span>}
                    </span>
                    <span className="rw-label">{e.label}</span>
                    {e.added.length > 0 && <span className="rw-small rw-muted rw-ids">{e.added.slice(0, 4).map(name).join(" · ")}{e.added.length > 4 ? ` · and ${num(e.added.length - 4)} more` : ""}</span>}
                    <span className="rw-small rw-muted">{num(e.files)} {e.files === 1 ? "file" : "files"} changed · <code>{e.short}</code></span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
      {shown < days.length && (
        <div className="rw-more">
          <button type="button" onClick={() => setShown(shown + DAYS_PER_STEP)}>Show {num(Math.min(DAYS_PER_STEP, days.length - shown))} more days</button>
          <span className="rw-small rw-muted">Showing {num(shown)} of {num(days.length)} days</span>
        </div>
      )}
      {open && <CommitModal e={open} name={name} onClose={() => setOpen(null)} />}
      <p className="rw-small rw-muted rw-method">How the numbers are counted: {data.method} Built {fmtStamp(data.generated_at)}.</p>
    </section>
  );
}

function DailyWork({ days }: { days: WorkDay[] }) {
  const [all, setAll] = useState(false);
  const list = all ? days : days.slice(0, 7);
  return (
    <section className="rw-daily" aria-label="Daily work">
      <div className="rw-daily-head"><h3>Daily work</h3>
        {days.length > 7 && <button type="button" className="rw-link" onClick={() => setAll(!all)}>{all ? "Show the last 7 days" : `Show all ${num(days.length)} days`}</button>}</div>
      <div className="rw-daily-scroll">
        <table>
          <thead><tr><th>Day</th><th className="n">Commits</th><th className="n">Strategies added</th><th className="n">YT strategies at day end</th><th>Authors</th></tr></thead>
          <tbody>
            {list.map(d => (
              <tr key={d.date}><td>{fmtDay(d.date)}</td><td className="n">{num(d.commits)}</td>
                <td className="n">{d.added ? `+${num(d.added)}` : "–"}{d.removed ? ` / −${num(d.removed)}` : ""}</td>
                <td className="n">{num(d.count)}</td><td>{d.authors.join(", ")}</td></tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function CommitModal({ e, name, onClose }: { e: WorkEvent; name: (id: string) => string; onClose: () => void }) {
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    close.current?.focus();
    const key = (k: KeyboardEvent) => { if (k.key === "Escape") onClose(); };
    addEventListener("keydown", key);
    return () => { removeEventListener("keydown", key); before?.focus?.(); };
  }, [onClose]);
  return (
    <div className="rw-modal-back" onClick={ev => { if (ev.target === ev.currentTarget) onClose(); }}>
      <div className="rw-modal" role="dialog" aria-modal="true" aria-labelledby="rw-modal-title">
        <header><h2 id="rw-modal-title">{e.label}</h2><button ref={close} type="button" onClick={onClose} aria-label="Close">Close</button></header>
        <div className="rw-modal-body">
          <p className="rw-meta"><span className="rw-pill who">{e.who}</span>{e.with.map(w => <span key={w} className="rw-pill helper">with {w}</span>)}
            <span>{fmtStamp(e.at)}</span><code title={e.key}>{e.short}</code></p>
          <p className="rw-nums"><span><b>{num(e.count)}</b> YT strategies after it</span><span><b>{num(e.files)}</b> files</span>
            <span><b className="up">+{num(e.insertions)}</b> / <b className="down">−{num(e.deletions)}</b> lines</span></p>
          {e.work ? <pre className="rw-work">{e.work}</pre> : <p className="rw-muted">No notes beyond the title.</p>}
          {e.added.length > 0 && <>
            <h3>Strategies added ({num(e.added.length)})</h3>
            <ul className="rw-list">{e.added.map(id => <li key={id}><b>{name(id)}</b>{name(id) !== id && <span className="rw-muted"> · {id}</span>}</li>)}</ul>
          </>}
          {e.removed.length > 0 && <>
            <h3>Strategy ids removed ({num(e.removed.length)})</h3>
            <ul className="rw-list">{e.removed.map(id => <li key={id}>{name(id)}{name(id) !== id && <span className="rw-muted"> · {id}</span>}</li>)}</ul>
          </>}
        </div>
      </div>
    </div>
  );
}

/* ---------------------------------------------------------------- every strategy with the commit that added it */

function StrategiesView({ data, name }: { data: Work; name: (id: string) => string }) {
  const [q, setQ] = useState("");
  const all = useMemo(() => strategiesAdded(data.events), [data.events]);
  const t = q.trim().toLowerCase();
  const list = t ? all.filter(s => `${s.id} ${name(s.id)} ${s.event.label}`.toLowerCase().includes(t)) : all;
  return (
    <section>
      <h1>Strategies added</h1>
      <p className="rw-muted">Each YT strategy id with the commit that first added it, newest first. Names come from the engine's YT catalogue; an id is shown when the catalogue does not list it or cannot be reached.</p>
      <Totals data={data} />
      <input className="rw-search" type="search" aria-label="Search strategies" placeholder={`Search ${num(all.length)} strategies`} value={q} onChange={ev => setQ(ev.target.value)} />
      {!list.length ? <p className="rw-empty">{all.length ? "No match." : "No YT strategies were added in this history."}</p> : (
        <ul className="rw-list rw-strats">
          {list.map(({ id, event }) => (
            <li key={`${id}-${event.key}`}><span><b>{name(id)}</b>{name(id) !== id && <span className="rw-muted rw-small"> · {id}</span>}</span>
              <span className="rw-small rw-muted">{fmtDay(dayKey(event.at))} · {event.label}</span></li>
          ))}
        </ul>
      )}
    </section>
  );
}
