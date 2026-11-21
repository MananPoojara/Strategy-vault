import { useEffect, useMemo, useState } from "react";
import {
  loadTradesheets, headline, statusClass, filterStrategies, STATUSES,
  type Tradesheets, type TsFilter, type TsLoad, type TsSnapshot,
} from "../services/research-tradesheets";
import { fmtDay, num } from "../services/research-work";

/* Research → "Tradesheet verified": how many YT strategies match the team's trade sheets, how many are pending and
   how many mismatch, with the history of those counts. Styles live in ResearchWork.css (rw-ts-*). */

const PER = 60;
const SHORT: Record<string, string> = { "Matching": "Matching", "Partly matching": "Partly", "Not matching": "Not matching", "Not compared yet": "Not compared yet", "No trade sheets": "No trade sheets" };

export function TradesheetsView() {
  const [load, setLoad] = useState<TsLoad | null>(null);
  useEffect(() => {
    let live = true;
    loadTradesheets().then(r => live && setLoad(r));
    return () => { live = false; };
  }, []);
  if (!load) return <p className="rw-loading">Loading…</p>;
  if (!load.ok) return <div className="rw-problem" role="alert"><b>Trade-sheet status not built.</b> {load.problem}</div>;
  return <Body data={load.data} />;
}

function Body({ data }: { data: Tradesheets }) {
  const h = headline(data.totals);
  const [filter, setFilter] = useState<TsFilter>("all");
  const pick = (f: TsFilter) => { setFilter(f); document.getElementById("rw-ts-list")?.scrollIntoView?.({ behavior: "smooth", block: "start" }); };
  const s = data.source;
  return (
    <section className="rw-ts">
      <h1>Tradesheet verified</h1>
      <p className="rw-muted">How many of the {num(h.total)} YT strategies give the same trades as the team's trade sheets. A strategy is <b>done</b> when every compared trade sheet matches, a <b>mismatch</b> when some or none match, and <b>pending</b> when it has trade sheets that are not compared yet.</p>
      <div className="rw-stats">
        <Card cls="ok" n={h.done} label={`Done: every trade sheet matches (incl. ${num(h.jay)} on the Status workbook's Complete sheet)`} go={() => pick("Matching")} />
        <Card cls="bad" n={h.mismatch} label={`Mismatch: ${num(h.notMatching)} not matching, ${num(h.partly)} partly`} go={() => pick("Not matching")} />
        <Card cls="pending" n={h.pending} label="Pending verification (not compared yet)" go={() => pick("Not compared yet")} />
        <Card cls="none" n={h.none} label="No trade sheets" go={() => pick("No trade sheets")} />
      </div>
      <Bar counts={data.totals} total={h.total} />
      <History history={data.history} />
      <Categories data={data} />
      <StrategyList data={data} filter={filter} setFilter={setFilter} />
      <p className="rw-small rw-muted rw-method">
        Source: <b>{s.file}</b>, made {s.made || "(no date in the workbook)"}{s.date_from && s.date_from !== "Notes: Made" ? ` (date from the ${s.date_from})` : ""}.
        {s.strategies_note && <> {s.strategies_note}</>}
        {s.caveat && <> Caveat: {s.caveat}</>}
        {s.status_file && <> Complete sheet (part of Done): {s.status_file}{s.status_made ? ` (made ${s.status_made})` : ""}. {data.verified_rule}</>}
      </p>
    </section>
  );
}

function Card({ cls, n, label, go }: { cls: string; n: number; label: string; go: () => void }) {
  return <button type="button" className={`rw-stat rw-ts-card ${cls}`} onClick={go} title="Show these strategies"><b>{num(n)}</b><span>{label}</span></button>;
}

/** One stacked bar of the five statuses. */
function Bar({ counts, total }: { counts: Record<string, number>; total: number }) {
  const label = STATUSES.map(k => `${k} ${counts[k] || 0}`).join(", ");
  return (
    <div className="rw-ts-bar" role="img" aria-label={label} title={label}>
      {STATUSES.map(k => (counts[k] ? <i key={k} className={`s-${statusClass(k)}`} style={{ width: `${(100 * counts[k]) / Math.max(1, total)}%` }} /> : null))}
    </div>
  );
}

function History({ history }: { history: TsSnapshot[] }) {
  if (!history.length) return null;
  const rows = [...history].reverse(); // newest first
  return (
    <section aria-label="History">
      <h3>History <small>· {num(history.length)} {history.length === 1 ? "snapshot" : "snapshots"}, newest first</small></h3>
      <p className="rw-ts-legend">{STATUSES.map(k => <span key={k}><i className={`s-${statusClass(k)}`} />{k}</span>)}</p>
      <div className="rw-daily-scroll">
        <table className="rw-ts-table">
          <thead><tr><th>Snapshot</th><th className="rw-ts-barcol">Status mix</th>{STATUSES.map(k => <th key={k} className="n">{SHORT[k]}</th>)}<th className="n">Total</th></tr></thead>
          <tbody>
            {rows.map((h, i) => (
              <tr key={`${h.date}-${h.kind}-${i}`}>
                <td><b>{fmtDay(h.date)}</b><br /><span className="rw-small rw-muted">{h.label}</span>
                  {h.changes.length > 0 && (
                    <details className="rw-ts-changes"><summary>{num(h.changes.length)} {h.changes.length === 1 ? "change" : "changes"} since the previous snapshot</summary>
                      <ul>{h.changes.map(c => <li key={c.id}><b>{c.name}</b> <span className="rw-muted">{c.from ?? "new"} → {c.to ?? "removed"}</span></li>)}</ul>
                    </details>)}
                </td>
                <td className="rw-ts-barcol"><Bar counts={h.counts} total={h.total} /></td>
                {STATUSES.map(k => <td key={k} className="n">{num(h.counts[k])}</td>)}
                <td className="n">{num(h.total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function Categories({ data }: { data: Tradesheets }) {
  return (
    <section aria-label="By category">
      <h3>By category <small>· {num(data.categories.length)} categories</small></h3>
      <div className="rw-daily-scroll">
        <table className="rw-ts-table">
          <thead><tr><th>Category</th>{STATUSES.map(k => <th key={k} className="n">{SHORT[k]}</th>)}<th className="n">Total</th><th className="n">Of done: Complete sheet</th></tr></thead>
          <tbody>
            {data.categories.map(c => (
              <tr key={c.category}><td>{c.category || "(none)"}</td>
                {STATUSES.map(k => <td key={k} className={`n${c.counts[k] ? ` c-${statusClass(k)}` : " rw-muted"}`}>{c.counts[k] ? num(c.counts[k]) : "–"}</td>)}
                <td className="n">{num(c.total)}</td><td className="n">{c.verified_by_jay ? num(c.verified_by_jay) : "–"}</td></tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function StrategyList({ data, filter, setFilter }: { data: Tradesheets; filter: TsFilter; setFilter: (f: TsFilter) => void }) {
  const [q, setQ] = useState(""), [shown, setShown] = useState(PER);
  const list = useMemo(() => filterStrategies(data.strategies, filter, q), [data.strategies, filter, q]);
  useEffect(() => setShown(PER), [filter, q]);
  const count = (f: TsFilter) => filterStrategies(data.strategies, f, "").length;
  const chips: [TsFilter, string][] = [["all", "All"], ...STATUSES.map(k => [k, k] as [TsFilter, string]), ["jay", "Done: on the Complete sheet"]];
  return (
    <section id="rw-ts-list" aria-label="Strategies">
      <h3>Strategies</h3>
      <div className="rw-ts-chips" role="group" aria-label="Filter by status">
        {chips.map(([f, label]) => (
          <button key={f} type="button" aria-pressed={filter === f} className={filter === f ? "on" : ""} onClick={() => setFilter(f)}>{label} <small>{num(count(f))}</small></button>
        ))}
      </div>
      <input className="rw-search" type="search" aria-label="Search strategies" placeholder="Search name, id, category or detail" value={q} onChange={e => setQ(e.target.value)} />
      {!list.length ? <p className="rw-empty">No strategy matches.</p> : (
        <ul className="rw-list rw-ts-rows">
          {list.slice(0, shown).map(s => (
            <li key={s.id} className={`s-${statusClass(s.status)}`}>
              <span className="rw-ts-head"><b>{s.name}</b><span className={`rw-pill st-${statusClass(s.status)}`}>{s.status}</span>
                {s.verified_by_jay && <span className="rw-pill helper" title={`Status workbook, Complete sheet: ${s.verified_as}`}>Complete sheet</span>}</span>
              <span className="rw-small rw-muted">{s.category} · <code>{s.id}</code>{s.sheets > 0 && <> · {num(s.matched)} of {num(s.sheets)} trade sheets match</>}</span>
              {s.detail && <span className="rw-small">{s.detail}</span>}
            </li>
          ))}
        </ul>
      )}
      {shown < list.length && (
        <div className="rw-more"><button type="button" onClick={() => setShown(shown + PER)}>Show {num(Math.min(PER, list.length - shown))} more</button>
          <span className="rw-small rw-muted">Showing {num(shown)} of {num(list.length)}</span></div>
      )}
    </section>
  );
}
