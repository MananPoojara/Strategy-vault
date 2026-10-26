import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import "./ResearchPortal.css";
import {
  research, VERDICTS, NOTE_TYPES, SITE, fmtDate, fmtStamp, fmtTime, ago, dayKey, readHashParams, hashWith,
  type Note, type NoteHistory, type NoteType, type Page, type Person, type Site, type Strategy, type StrategyDetail,
  type Summary, type TargetType, type TimelineItem, type Verdict,
} from "../services/research";

const TABS = [
  ["progress", "Progress"], ["review", "Strategy review"], ["notes", "Notes"], ["timeline", "Timeline"], ["people", "People"],
] as const;
type Tab = (typeof TABS)[number][0];
const num = (n: number | undefined) => (n ?? 0).toLocaleString("en-IN");
const user = () => window.VAULT_USER;
const canWrite = () => ["admin", "researcher"].includes(user()?.role || "");

/** Filters live in the hash; replaceState keeps them shareable without re-running the vault router. */
function useHashParam(name: string): [string, (v: string | undefined, extra?: Record<string, string | undefined>) => void] {
  const [value, setValue] = useState(() => readHashParams().get(name) || "");
  useEffect(() => {
    const sync = () => setValue(readHashParams().get(name) || "");
    addEventListener("hashchange", sync); addEventListener("rp-hash", sync);
    return () => { removeEventListener("hashchange", sync); removeEventListener("rp-hash", sync); };
  }, [name]);
  const set = useCallback((v: string | undefined, extra: Record<string, string | undefined> = {}) => {
    history.replaceState(null, "", location.pathname + location.search + hashWith({ [name]: v, ...extra }));
    dispatchEvent(new Event("rp-hash"));
  }, [name]);
  return [value, set];
}

function useLoad<T>(load: () => Promise<T>, deps: unknown[]) {
  const [state, setState] = useState<{ data?: T; error?: string; loading: boolean }>({ loading: true });
  const [tick, setTick] = useState(0);
  useEffect(() => {
    let live = true;
    setState(s => ({ ...s, loading: true }));
    load().then(data => live && setState({ data, loading: false }), (e: Error) => live && setState({ error: e.message, loading: false }));
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, tick]);
  return { ...state, reload: () => setTick(t => t + 1) };
}

export function ResearchPortal() {
  const [tab, setTab] = useState<Tab>(() => (readHashParams().get("tab") as Tab) || "progress");
  useEffect(() => {
    const sync = () => { const t = readHashParams().get("tab") as Tab; setTab(TABS.some(x => x[0] === t) ? t : "progress"); };
    addEventListener("hashchange", sync);
    return () => removeEventListener("hashchange", sync);
  }, []);
  useEffect(() => { document.title = "Research notebook · Strategy Vault"; }, []);
  useEffect(() => {
    const keys = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      if (el.closest("input,textarea,select,[contenteditable]") || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "/") { const s = document.querySelector<HTMLInputElement>(".rp [data-search]"); if (s) { e.preventDefault(); s.focus(); } }
      if (e.key === "[" || e.key === "]") document.querySelector<HTMLButtonElement>(`.rp [data-page="${e.key === "[" ? "prev" : "next"}"]`)?.click();
      if (/^[1-5]$/.test(e.key)) location.hash = `#/research?tab=${TABS[Number(e.key) - 1][0]}`;
    };
    addEventListener("keydown", keys);
    return () => removeEventListener("keydown", keys);
  }, []);
  return (
    <div className="rp">
      <header className="rp-head">
        <div>
          <h1>Research notebook</h1>
          <p className="rp-sub">What the research team has checked, found and written down, next to the platform's own progress.</p>
        </div>
        <p className="rp-keys" aria-hidden="true"><kbd>1</kbd>–<kbd>5</kbd> tabs <kbd>/</kbd> search <kbd>[</kbd> <kbd>]</kbd> pages</p>
      </header>
      <nav className="rp-tabs" aria-label="Research sections">
        {TABS.map(([id, label]) => (
          <a key={id} href={`#/research?tab=${id}`} aria-current={tab === id ? "page" : undefined}>{label}</a>
        ))}
      </nav>
      {tab === "progress" && <ProgressTab />}
      {tab === "review" && <ReviewTab />}
      {tab === "notes" && <NotesTab />}
      {tab === "timeline" && <TimelineTab />}
      {tab === "people" && <PeopleTab />}
    </div>
  );
}

/* ---------------------------------------------------------------- shared bits */

function Pager({ page, onPage, label }: { page: Pick<Page<unknown>, "page" | "pages" | "total">; onPage: (p: number) => void; label: string }) {
  if (page.total === 0) return null;
  return (
    <nav className="rp-pager" aria-label={`${label} pages`}>
      <button type="button" data-page="prev" disabled={page.page <= 1} onClick={() => onPage(page.page - 1)}>Previous</button>
      <span>Page {page.page} of {page.pages} <span className="rp-muted">({num(page.total)} {label})</span></span>
      <button type="button" data-page="next" disabled={page.page >= page.pages} onClick={() => onPage(page.page + 1)}>Next</button>
    </nav>
  );
}
function Problem({ error, retry }: { error?: string; retry?: () => void }) {
  if (!error) return null;
  return <div className="rp-problem" role="alert">{error} {retry && <button type="button" onClick={retry}>Try again</button>}</div>;
}
function Field({ label, children }: { label: string; children: ReactNode }) {
  return <label className="rp-field"><span>{label}</span>{children}</label>;
}
const SiteBadge = ({ site }: { site: Site }) => <span className={`rp-badge site-${site}`}><i aria-hidden="true" />{SITE[site]}</span>;
const VerdictBadge = ({ v }: { v: Verdict }) => <span className={`rp-badge v-${v}`}><i aria-hidden="true" />{VERDICTS[v]}</span>;
const TypeBadge = ({ t }: { t: NoteType }) => <span className={`rp-type t-${t}`}>{NOTE_TYPES[t]}</span>;
const Stamp = ({ at }: { at: string }) => <time dateTime={at} title={fmtStamp(at)}>{fmtStamp(at)} <span className="rp-muted">({ago(at)})</span></time>;

/* ---------------------------------------------------------------- progress */

function ProgressTab() {
  const { data, error, reload } = useLoad(research.summary, []);
  if (error) return <Problem error={error} retry={reload} />;
  if (!data) return <p className="rp-loading">Loading progress…</p>;
  const t = data.totals;
  if (!t.strategies) return <div className="rp-empty"><b>No progress file yet.</b> Run make_research_progress.py after the deploy, then reload this page.</div>;
  return (
    <section className="rp-progress">
      <p className="rp-lede">
        <strong>{num(t.done)}</strong> of {num(t.strategies)} strategies on the site are done: {num(t.verified)} backtested
        and {num(t.held)} held with a reason. <strong>{num(t.remaining)}</strong> are still waiting to be reviewed.
      </p>
      <p className="rp-muted rp-asof">Live snapshot {fmtStamp(data.as_of)} IST. Same counts as the Excel master tracker. Updated {ago(data.generated_at)}.</p>
      <CorpusStrip data={data} />
      <div className="rp-facts">
        <Fact n={t.site_authors} of={t.share_authors} label="authors on the site" note="of authors with a book in the research share" />
        <Fact n={t.site_books} of={t.share_books} label="books on the site" note="of books in the research share" />
        <Fact n={t.books_with_verified} label="books with a backtested strategy" />
        <Fact n={t.books_complete} label="books complete" note="every strategy backtested or held with a reason" />
        <Fact n={data.team.verdicts.matches + data.team.verdicts.mismatch + data.team.verdicts.unclear} of={t.strategies} label="strategies checked by the team" note={`${num(data.team.verdicts.mismatch)} mismatch, ${num(data.team.verdicts.unclear)} unclear`} />
      </div>
      <div className="rp-charts">
        <figure className="rp-card">
          <figcaption><h2>Backtested strategies, deploy by deploy</h2></figcaption>
          <StepChart points={data.deploys} />
        </figure>
        <figure className="rp-card">
          <figcaption><h2>Team activity, last 60 days</h2><p className="rp-muted">Verdicts and new notes per day</p></figcaption>
          <ActivityBars rows={data.activity} />
        </figure>
      </div>
      <section className="rp-card">
        <h2>Why strategies are not backtested</h2>
        <Reasons rows={data.reasons} />
      </section>
      <AuthorsTable data={data} />
      <TodoAuthors rows={data.todo_authors} />
    </section>
  );
}
function Fact({ n, of, label, note }: { n: number; of?: number; label: string; note?: string }) {
  return (
    <div className="rp-fact">
      <b>{num(n)}{of !== undefined && <span className="rp-of"> / {num(of)}</span>}</b>
      <span>{label}</span>
      {note && <small>{note}</small>}
    </div>
  );
}

/** Every strategy on the site as one tick, grouped by author; the lower row marks the team's verdicts. */
function CorpusStrip({ data }: { data: Summary }) {
  const [hover, setHover] = useState<{ x: number; author: string; c: Record<string, number> } | null>(null);
  const groups = useMemo(() => {
    const by = new Map<string, [string, string, string][]>();
    for (const s of data.strip) by.set(s[0], [...(by.get(s[0]) || []), s]);
    const order = { v: 0, h: 1, u: 2 } as Record<string, number>;
    return [...by.entries()].sort((a, b) => b[1].length - a[1].length)
      .map(([id, rows]) => ({ id, rows: rows.sort((a, b) => order[a[1]] - order[b[1]]) }));
  }, [data]);
  const GAP = 6;
  let x = 0;
  const layout = groups.map(g => { const start = x; x += g.rows.length + GAP; return { ...g, start }; });
  const width = Math.max(1, x - GAP);
  const colour: Record<string, string> = { v: "var(--rp-ok)", h: "var(--rp-held)", u: "var(--rp-todo)" };
  const mark: Record<string, string> = { m: "var(--rp-ok)", u: "var(--rp-held)" };
  const t = data.totals;
  return (
    <figure className="rp-strip">
      <figcaption className="rp-legend">
        <span><i className="sw site-verified" />Backtested {num(t.verified)}</span>
        <span><i className="sw site-held" />Held with a reason {num(t.held)}</span>
        <span><i className="sw site-unreviewed" />Not reviewed yet {num(t.unreviewed)}</span>
        <span className="rp-legend-sep"><i className="sw tick v-mismatch" />Team: mismatch</span>
        <span><i className="sw tick v-matches" />matches the book</span>
        <span><i className="sw tick v-unclear" />unclear</span>
      </figcaption>
      <div className="rp-strip-plot" onMouseLeave={() => setHover(null)}>
        <svg viewBox={`0 0 ${width} 64`} preserveAspectRatio="none" role="img"
          aria-label={`${num(t.strategies)} strategies by author: ${num(t.verified)} backtested, ${num(t.held)} held, ${num(t.unreviewed)} not reviewed`}>
          {layout.map(g => (
            <g key={g.id} onMouseEnter={() => {
              const c = { v: 0, h: 0, u: 0, team: 0 } as Record<string, number>;
              for (const r of g.rows) { c[r[1]]++; if (r[2] !== "n") c.team++; }
              setHover({ x: (g.start + g.rows.length / 2) / width, author: g.id, c });
            }}>
              <rect x={g.start} y={0} width={g.rows.length} height={64} fill="transparent" />
              {g.rows.map((r, i) => <rect key={i} x={g.start + i} y={0} width={1} height={46} fill={colour[r[1]]} />)}
              {g.rows.map((r, i) => r[2] !== "n" && <rect key={`v${i}`} x={g.start + i} y={52} width={1} height={12} fill={mark[r[2]] || "var(--red)"} />)}
            </g>
          ))}
        </svg>
        {hover && (
          <div className="rp-tip" style={{ left: `${Math.min(88, Math.max(12, hover.x * 100))}%` }}>
            <b>{authorLabel(hover.author, data)}</b>
            <span>{num(hover.c.v)} backtested, {num(hover.c.h)} held, {num(hover.c.u)} not reviewed</span>
            <span>{num(hover.c.team)} checked by the team</span>
          </div>
        )}
      </div>
      <p className="rp-muted rp-strip-note">Each thin line is one strategy (variants counted inside it), grouped by author, largest first. Point at a group to see the author.</p>
    </figure>
  );
}
const authorLabel = (id: string, data: Summary) => data.author_index.find(a => a.id === id)?.name || id;

function StepChart({ points }: { points: Summary["deploys"] }) {
  const [hover, setHover] = useState<number | null>(null);
  if (points.length < 2) return <p className="rp-muted">Not enough deploys recorded yet.</p>;
  const W = 600, H = 220, L = 44, R = 12, T = 14, B = 28;
  const t0 = Date.parse(points[0].at), t1 = Date.parse(points[points.length - 1].at);
  const max = Math.ceil(Math.max(...points.map(p => p.count)) / 100) * 100;
  const X = (at: string) => L + ((Date.parse(at) - t0) / Math.max(1, t1 - t0)) * (W - L - R);
  const Y = (v: number) => T + (1 - v / max) * (H - T - B);
  let d = `M${X(points[0].at)},${Y(points[0].count)}`;
  for (let i = 1; i < points.length; i++) d += `H${X(points[i].at)}V${Y(points[i].count)}`;
  const ticks = [0, max / 2, max];
  const days = [...new Set(points.map(p => dayKey(p.at)))];
  const step = Math.max(1, Math.ceil(days.length / 6));
  const p = hover === null ? null : points[hover];
  return (
    <div className="rp-chart" onMouseLeave={() => setHover(null)}>
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`Backtested strategies grew from ${points[0].count} to ${points[points.length - 1].count}`}
        onMouseMove={e => {
          const box = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
          const x = ((e.clientX - box.left) / box.width) * W;
          let best = 0;
          points.forEach((q, i) => { if (Math.abs(X(q.at) - x) < Math.abs(X(points[best].at) - x)) best = i; });
          setHover(best);
        }}>
        {ticks.map(v => <g key={v}><line x1={L} x2={W - R} y1={Y(v)} y2={Y(v)} className="grid" /><text x={L - 8} y={Y(v) + 4} textAnchor="end">{v}</text></g>)}
        {days.filter((_, i) => i % step === 0).map(day => {
          const at = points.find(q => dayKey(q.at) === day)!.at;
          return <text key={day} x={X(at)} y={H - 8} textAnchor="middle">{fmtDate(at).replace(/ \d{4}$/, "")}</text>;
        })}
        <path d={d} className="line" />
        <circle cx={X(points[points.length - 1].at)} cy={Y(points[points.length - 1].count)} r={4} className="dot" />
        <text x={X(points[points.length - 1].at) - 8} y={Y(points[points.length - 1].count) - 10} textAnchor="end" className="label">{points[points.length - 1].count}</text>
        {p && <g><line x1={X(p.at)} x2={X(p.at)} y1={T} y2={H - B} className="cross" /><circle cx={X(p.at)} cy={Y(p.count)} r={4} className="dot" /></g>}
      </svg>
      {p && (
        <div className="rp-tip" style={{ left: `${(X(p.at) / W) * 100}%` }}>
          <b>{p.count} backtested</b><span>{p.label}</span><span className="rp-tip-work">{p.work}</span>
        </div>
      )}
    </div>
  );
}

function ActivityBars({ rows }: { rows: Summary["activity"] }) {
  const [hover, setHover] = useState<number | null>(null);
  const days = useMemo(() => {
    const map = new Map(rows.map(r => [r.day, r]));
    return Array.from({ length: 60 }, (_, i) => {
      const day = dayKey(new Date(Date.now() - (59 - i) * 86400000).toISOString());
      return map.get(day) || { day, verdicts: 0, notes: 0 };
    });
  }, [rows]);
  const max = Math.max(4, ...days.map(d => d.verdicts + d.notes));
  if (!rows.length) return <div className="rp-empty small">No verdicts or notes yet. Open <a href="#/research?tab=review">Strategy review</a> to record the first one.</div>;
  const W = 600, H = 220, B = 28, T = 14, bw = W / 60;
  const h = hover === null ? null : days[hover];
  return (
    <div className="rp-chart" onMouseLeave={() => setHover(null)}>
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Verdicts and notes per day">
        <line x1={0} x2={W} y1={H - B} y2={H - B} className="grid" />
        {days.map((d, i) => {
          const hv = (d.verdicts / max) * (H - T - B), hn = (d.notes / max) * (H - T - B);
          return (
            <g key={d.day} onMouseEnter={() => setHover(i)}>
              <rect x={i * bw} y={T} width={bw} height={H - T - B} fill="transparent" />
              {hv > 0 && <rect x={i * bw + 1} y={H - B - hv} width={bw - 2} height={hv} className="bar-v" rx={1.5} />}
              {hn > 0 && <rect x={i * bw + 1} y={H - B - hv - hn - (hv ? 2 : 0)} width={bw - 2} height={hn} className="bar-n" rx={1.5} />}
            </g>
          );
        })}
        <text x={0} y={H - 8}>{fmtDate(days[0].day).replace(/ \d{4}$/, "")}</text>
        <text x={W} y={H - 8} textAnchor="end">Today</text>
      </svg>
      <p className="rp-legend small"><span><i className="sw bar-v" />Verdicts</span><span><i className="sw bar-n" />New notes</span></p>
      {h && <div className="rp-tip" style={{ left: `${((hover! + 0.5) / 60) * 100}%` }}><b>{fmtDate(h.day)}</b><span>{h.verdicts} verdicts, {h.notes} notes</span></div>}
    </div>
  );
}

function Reasons({ rows }: { rows: Summary["reasons"] }) {
  const max = Math.max(1, ...rows.map(r => r.count));
  return (
    <ul className="rp-reasons">
      {rows.map(r => (
        <li key={r.reason}>
          <span className="name">{r.reason}</span>
          <span className="bar"><i style={{ width: `${(r.count / max) * 100}%` }} /></span>
          <span className="n">{num(r.count)}</span>
        </li>
      ))}
    </ul>
  );
}

function AuthorsTable({ data }: { data: Summary }) {
  const [page, setPage] = useState(1), [q, setQ] = useState(""), [open, setOpen] = useState<string | null>(null);
  const rows = data.authors.filter(a => a.name.toLowerCase().includes(q.toLowerCase()));
  const size = 15, pages = Math.max(1, Math.ceil(rows.length / size)), cur = Math.min(page, pages);
  return (
    <section className="rp-card">
      <div className="rp-card-head">
        <h2>Authors on the site</h2>
        <input type="search" placeholder="Find an author" aria-label="Find an author" value={q} onChange={e => { setQ(e.target.value); setPage(1); }} />
      </div>
      <div className="rp-table-wrap">
        <table className="rp-table">
          <thead><tr><th>Author</th><th className="n">Books in share</th><th className="n">Books on site</th><th className="n">Strategies</th><th>Progress</th><th className="n">Backtested</th><th className="n">Held</th><th className="n">Not reviewed</th><th>Author loop</th></tr></thead>
          <tbody>
            {rows.slice((cur - 1) * size, cur * size).map(a => {
              const books = data.books.filter(b => b.author === a.name);
              return [
                <tr key={a.name} className={open === a.name ? "open" : ""}>
                  <td><button type="button" className="rp-link" aria-expanded={open === a.name} onClick={() => setOpen(open === a.name ? null : a.name)}>{a.name}</button></td>
                  <td className="n">{a.share_books || "–"}</td><td className="n">{a.site_books}</td><td className="n">{a.strategies}</td>
                  <td><Meter v={a.verified} h={a.held} u={a.unreviewed} /></td>
                  <td className="n">{a.verified}</td><td className="n">{a.held}</td><td className="n">{a.unreviewed}</td>
                  <td><span className={`rp-loop l-${a.loop.replace(/\s/g, "").toLowerCase()}`}>{a.loop}</span></td>
                </tr>,
                open === a.name && (
                  <tr key={`${a.name}-books`} className="rp-books"><td colSpan={9}>
                    <ul>{books.map(b => (
                      <li key={b.title}>
                        <a href={`#/research?tab=review&q=${encodeURIComponent(b.title)}`}>{b.title}</a>{b.year ? <span className="rp-muted"> ({b.year})</span> : null}
                        <Meter v={b.verified} h={b.held} u={b.unreviewed} />
                        <span className="rp-muted">{b.verified} backtested, {b.held} held, {b.unreviewed} not reviewed. {b.status}</span>
                      </li>
                    ))}</ul>
                  </td></tr>
                ),
              ];
            })}
          </tbody>
        </table>
      </div>
      <Pager page={{ page: cur, pages, total: rows.length }} onPage={setPage} label="authors" />
    </section>
  );
}
function Meter({ v, h, u }: { v: number; h: number; u: number }) {
  const t = Math.max(1, v + h + u);
  return (
    <span className="rp-meter" role="img" aria-label={`${v} backtested, ${h} held, ${u} not reviewed`}>
      <i className="site-verified" style={{ width: `${(v / t) * 100}%` }} /><i className="site-held" style={{ width: `${(h / t) * 100}%` }} /><i className="site-unreviewed" style={{ width: `${(u / t) * 100}%` }} />
    </span>
  );
}
function TodoAuthors({ rows }: { rows: Summary["todo_authors"] }) {
  const [page, setPage] = useState(1);
  const size = 20, pages = Math.max(1, Math.ceil(rows.length / size));
  if (!rows.length) return null;
  return (
    <section className="rp-card">
      <h2>Authors in the research share, not on the site yet</h2>
      <p className="rp-muted">{num(rows.filter(r => r.status === "Queued").length)} still queued for the author loop. Largest first.</p>
      <div className="rp-table-wrap">
        <table className="rp-table">
          <thead><tr><th>Author</th><th className="n">Books in share</th><th className="n">All works</th><th>Status</th></tr></thead>
          <tbody>{rows.slice((page - 1) * size, page * size).map(r => (
            <tr key={r.name}><td>{r.name}</td><td className="n">{r.share_books}</td><td className="n">{r.works}</td><td>{r.status}</td></tr>
          ))}</tbody>
        </table>
      </div>
      <Pager page={{ page, pages, total: rows.length }} onPage={setPage} label="authors" />
    </section>
  );
}

/* ---------------------------------------------------------------- review */

function ReviewTab() {
  const [q, setQ] = useHashParam("q"), [author, setAuthor] = useHashParam("author"), [book, setBook] = useHashParam("book");
  const [site, setSite] = useHashParam("site"), [verdict, setVerdict] = useHashParam("verdict"), [reviewer, setReviewer] = useHashParam("reviewer");
  const [pageS, setPage] = useHashParam("page"), [selected, setSelected] = useHashParam("key");
  const [text, setText] = useState(q);
  useEffect(() => setText(q), [q]);
  useEffect(() => { const t = setTimeout(() => { if (text !== q) setQ(text, { page: undefined }); }, 250); return () => clearTimeout(t); }, [text, q, setQ]);
  const page = Number(pageS) || 1;
  const list = useLoad(() => research.strategies({ q, author, book, site, verdict, reviewer, page, size: 25 }), [q, author, book, site, verdict, reviewer, page]);
  const items = list.data?.items || [];
  const listRef = useRef<HTMLOListElement>(null);
  const move = useCallback((d: number) => {
    if (!items.length) return;
    const i = items.findIndex(s => s.key === selected);
    const next = items[Math.min(items.length - 1, Math.max(0, i + d))];
    setSelected(next.key);
    listRef.current?.querySelector<HTMLElement>(`[data-key="${CSS.escape(next.key)}"]`)?.focus();
  }, [items, selected, setSelected]);
  useEffect(() => {
    const keys = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input,textarea,select") || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "j") move(1);
      if (e.key === "k") move(-1);
      if (e.key === "n") { const c = document.querySelector<HTMLTextAreaElement>(".rp-detail textarea"); if (c) { e.preventDefault(); c.focus(); } }
      if (e.key === "Escape") setSelected(undefined);
    };
    addEventListener("keydown", keys);
    return () => removeEventListener("keydown", keys);
  }, [move, setSelected]);
  const f = list.data?.facets;
  const reset = { page: undefined };
  return (
    <section className={`rp-review${selected ? " has-detail" : ""}`}>
      <div className="rp-filters" role="search">
        <Field label="Search"><input type="search" data-search placeholder="Strategy, book, author or reason" value={text} onChange={e => setText(e.target.value)} /></Field>
        <Field label="Author"><select value={author} onChange={e => setAuthor(e.target.value, { book: undefined, ...reset })}>
          <option value="">All authors</option>{f?.authors.map(a => <option key={a.id} value={a.id}>{a.name}</option>)}
        </select></Field>
        <Field label="Book"><select value={book} disabled={!author} onChange={e => setBook(e.target.value, reset)}>
          <option value="">{author ? "All books" : "Choose an author first"}</option>{f?.books.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select></Field>
        <Field label="On the site"><select value={site} onChange={e => setSite(e.target.value, reset)}>
          <option value="">Any status</option>{Object.entries(SITE).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </select></Field>
        <Field label="Research verdict"><select value={verdict} onChange={e => setVerdict(e.target.value, reset)}>
          <option value="">Any verdict</option>{Object.entries(VERDICTS).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </select></Field>
        <Field label="Reviewer"><select value={reviewer} onChange={e => setReviewer(e.target.value, reset)}>
          <option value="">Anyone</option>{f?.reviewers.map(r => <option key={r} value={r}>{r}</option>)}
        </select></Field>
      </div>
      <div className="rp-split">
        <div className="rp-list-pane">
          <Problem error={list.error} retry={list.reload} />
          {list.data && (
            <p className="rp-count" aria-live="polite">{num(list.data.total)} {list.data.total === 1 ? "strategy" : "strategies"}{list.loading ? " (updating…)" : ""}. <span className="rp-muted"><kbd>j</kbd> <kbd>k</kbd> move, <kbd>n</kbd> new note, <kbd>Esc</kbd> close</span></p>
          )}
          {list.data && !items.length && <div className="rp-empty">No strategy matches these filters. Clear a filter or the search.</div>}
          <ol className="rp-list" ref={listRef}>
            {items.map(s => (
              <li key={s.key}>
                <button type="button" data-key={s.key} aria-current={selected === s.key ? "true" : undefined} onClick={() => setSelected(s.key)}>
                  <span className="rp-list-name">{s.name}</span>
                  <span className="rp-list-meta">{s.author}, {s.book}</span>
                  <span className="rp-list-tags">
                    <SiteBadge site={s.site} /><VerdictBadge v={s.verdict} />
                    {s.notes > 0 && <span className="rp-muted">{s.notes} note{s.notes > 1 ? "s" : ""}{s.rule_errors ? `, ${s.rule_errors} rule error${s.rule_errors > 1 ? "s" : ""}` : ""}</span>}
                  </span>
                </button>
              </li>
            ))}
          </ol>
          {list.data && <Pager page={list.data} onPage={p => setPage(String(p))} label="strategies" />}
        </div>
        {selected ? <StrategyPanel key={selected} strategyKey={selected} onClose={() => setSelected(undefined)} onChanged={list.reload} />
          : <aside className="rp-detail rp-detail-empty"><p>Choose a strategy to read its rules as coded, compare them with the book and record what you found.</p></aside>}
      </div>
    </section>
  );
}

function StrategyPanel({ strategyKey, onClose, onChanged }: { strategyKey: string; onClose: () => void; onChanged: () => void }) {
  const detail = useLoad(() => research.strategy(strategyKey), [strategyKey]);
  const [saving, setSaving] = useState<Verdict | null>(null), [err, setErr] = useState("");
  const ref = useRef<HTMLElement>(null);
  useEffect(() => { if (window.matchMedia?.("(max-width: 900px)").matches) ref.current?.scrollIntoView({ block: "start" }); }, []);
  const d: StrategyDetail | undefined = detail.data;
  const s = d?.strategy;
  const choose = async (v: Verdict) => {
    setSaving(v); setErr("");
    try { await research.setVerdict(strategyKey, v); detail.reload(); onChanged(); }
    catch (e) { setErr((e as Error).message); }
    finally { setSaving(null); }
  };
  return (
    <aside className="rp-detail" ref={ref} aria-label="Strategy detail">
      <button type="button" className="rp-close" onClick={onClose} aria-label="Close strategy detail">Close</button>
      <Problem error={detail.error} retry={detail.reload} />
      {!d && !detail.error && <p className="rp-loading">Loading…</p>}
      {d && (
        <>
          <h2>{s?.name || strategyKey}</h2>
          {s && <p className="rp-muted">{s.author}, <i>{s.book}</i>. {s.variants} variant{s.variants > 1 ? "s" : ""} inside.</p>}
          {s && (
            <div className="rp-site">
              <SiteBadge site={s.site} />
              <span>{s.site === "verified" ? "Backtested on the site after an independent re-calculation matched the engine trade by trade." : s.reason}</span>
              <a href={s.url} target="_blank" rel="noopener">Open the strategy page</a>
            </div>
          )}
          <h3>Rules as coded</h3>
          {s?.section && <p className="rp-muted small rp-section">Where in the book: {s.section}</p>}
          {s?.rules.length ? (
            <ol className="rp-rules">{s.rules.map((r, i) => <li key={i}><b>{r.kind}</b><span>{r.text}</span></li>)}</ol>
          ) : <p className="rp-muted">No rules are coded for this strategy. It is listed as a held card: {s?.reason || "no reason recorded"}.</p>}
          {s?.coding_notes && <details className="rp-vhist"><summary>How it is coded (implementation notes)</summary><p className="rp-body">{s.coding_notes}</p></details>}
          <h3>Research verdict</h3>
          <p className="rp-muted small">Your judgement after reading the book pages. It does not change the site status or any calculation.</p>
          <div className="rp-verdicts" role="radiogroup" aria-label="Research verdict">
            {(Object.keys(VERDICTS) as Verdict[]).map(v => (
              <button key={v} type="button" role="radio" aria-checked={d.verdict.verdict === v} className={`v-${v}`}
                disabled={!canWrite() || saving !== null || !s} onClick={() => choose(v)}>
                <i aria-hidden="true" />{VERDICTS[v]}{saving === v ? "…" : ""}
              </button>
            ))}
          </div>
          {d.verdict.author_name && d.verdict.updated_at && <p className="rp-muted small">Set by {d.verdict.author_name}, <Stamp at={d.verdict.updated_at} /></p>}
          {!canWrite() && <p className="rp-muted small">Your account can read only. Ask an administrator for researcher access to record verdicts and notes.</p>}
          <Problem error={err} />
          {d.history.length > 1 && (
            <details className="rp-vhist"><summary>Verdict history ({d.history.length})</summary>
              <ul>{d.history.map((h, i) => <li key={i}>{fmtStamp(h.created_at)}: {h.actor_name} set <b>{VERDICTS[h.verdict]}</b>{h.previous ? ` (was ${VERDICTS[h.previous]})` : ""}</li>)}</ul>
            </details>
          )}
          <h3>Notes <span className="rp-muted">({d.notes.filter(n => !n.deleted_at).length})</span></h3>
          {canWrite() && s && <Composer target={{ type: "strategy", key: strategyKey }} defaultType={d.verdict.verdict === "mismatch" ? "rule_error" : "question"} onSaved={() => { detail.reload(); onChanged(); }} />}
          <div className="rp-thread">
            {d.notes.map(n => <NoteCard key={n.id} note={n} onChanged={() => { detail.reload(); onChanged(); }} hideTarget />)}
            {!d.notes.length && <p className="rp-muted">No notes yet.</p>}
          </div>
        </>
      )}
    </aside>
  );
}

function Composer({ target, defaultType = "question", onSaved, compact }: { target: { type: TargetType; key: string }; defaultType?: NoteType; onSaved: (n: Note) => void; compact?: boolean }) {
  const [type, setType] = useState<NoteType>(defaultType), [body, setBody] = useState(""), [busy, setBusy] = useState(false), [err, setErr] = useState("");
  useEffect(() => setType(defaultType), [defaultType]);
  const save = async () => {
    if (!body.trim()) { setErr("Write the note text."); return; }
    setBusy(true); setErr("");
    try { const n = await research.addNote({ target_type: target.type, target_key: target.key, note_type: type, body }); setBody(""); onSaved(n); }
    catch (e) { setErr((e as Error).message); }
    finally { setBusy(false); }
  };
  return (
    <form className={`rp-composer${compact ? " compact" : ""}`} onSubmit={e => { e.preventDefault(); save(); }}>
      <div className="rp-types" role="radiogroup" aria-label="Note type">
        {(Object.keys(NOTE_TYPES) as NoteType[]).map(t => (
          <label key={t} className={`t-${t}`}><input type="radio" name={`type-${target.type}-${target.key}`} checked={type === t} onChange={() => setType(t)} />{NOTE_TYPES[t]}</label>
        ))}
      </div>
      <textarea value={body} onChange={e => setBody(e.target.value)} rows={3} maxLength={10000} aria-label="Note text"
        placeholder={type === "rule_error" ? "What the code does, what the book says, and the page (e.g. p.54)" : "Write what you checked or found. Name the book page when you can."}
        onKeyDown={e => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); save(); } }} />
      <div className="rp-composer-foot">
        <span className="rp-muted small">Saved as {user()?.name || "you"} with the current time. <kbd>Ctrl</kbd>+<kbd>Enter</kbd> saves.</span>
        <button type="submit" className="rp-primary" disabled={busy}>{busy ? "Saving…" : "Save note"}</button>
      </div>
      <Problem error={err} />
    </form>
  );
}

function NoteCard({ note, onChanged, hideTarget }: { note: Note; onChanged: () => void; hideTarget?: boolean }) {
  const [mode, setMode] = useState<"view" | "edit" | "remove">("view"), [body, setBody] = useState(note.body), [type, setType] = useState(note.note_type);
  const [reason, setReason] = useState(""), [err, setErr] = useState(""), [hist, setHist] = useState<NoteHistory[] | null>(null);
  const act = async (fn: () => Promise<unknown>) => { setErr(""); try { await fn(); setMode("view"); onChanged(); } catch (e) { setErr((e as Error).message); } };
  const link = note.target_type === "strategy" ? `#/research?tab=review&key=${encodeURIComponent(note.target_key)}`
    : note.target_type === "general" ? null : `#/research?tab=notes&target_type=${note.target_type}&target_key=${encodeURIComponent(note.target_key)}`;
  return (
    <article className={`rp-note t-${note.note_type}${note.deleted_at ? " removed" : ""}`}>
      <header>
        <TypeBadge t={note.note_type} />
        <span className="who">{note.author_name}{note.imported && <span className="rp-muted"> (from the Excel sheet)</span>}</span>
        <Stamp at={note.created_at} />
      </header>
      {!hideTarget && <p className="rp-target">{link ? <a href={link}>{note.target_label}</a> : note.target_label}</p>}
      {note.deleted_at ? (
        <p className="rp-removed">Removed by {note.deleted_by_name || "a team member"} on {fmtStamp(note.deleted_at)}. Reason: {note.deleted_reason}</p>
      ) : mode === "edit" ? (
        <form onSubmit={e => { e.preventDefault(); act(() => research.editNote(note.id, { body, note_type: type })); }}>
          <select value={type} onChange={e => setType(e.target.value as NoteType)} aria-label="Note type">
            {Object.entries(NOTE_TYPES).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
          </select>
          <textarea value={body} onChange={e => setBody(e.target.value)} rows={4} aria-label="Edit note text" autoFocus />
          <div className="rp-row"><button type="submit" className="rp-primary">Save changes</button><button type="button" onClick={() => { setMode("view"); setBody(note.body); }}>Cancel</button></div>
        </form>
      ) : (
        <p className="rp-body">{note.body}</p>
      )}
      {mode === "remove" && (
        <form className="rp-remove" onSubmit={e => { e.preventDefault(); act(() => research.removeNote(note.id, reason)); }}>
          <label>Why remove this note? The text stays in the audit history.<input value={reason} onChange={e => setReason(e.target.value)} autoFocus maxLength={500} /></label>
          <div className="rp-row"><button type="submit" className="rp-danger">Remove note</button><button type="button" onClick={() => setMode("view")}>Keep it</button></div>
        </form>
      )}
      <footer>
        {note.edits > 0 && !note.deleted_at && <span className="rp-muted">Edited {note.edits} time{note.edits > 1 ? "s" : ""}, last {ago(note.updated_at)}</span>}
        {(note.edits > 0 || note.deleted_at) && (
          <button type="button" className="rp-link" onClick={async () => setHist(hist ? null : await research.noteHistory(note.id))}>{hist ? "Hide history" : "Show history"}</button>
        )}
        {note.can_edit && mode === "view" && <button type="button" className="rp-link" onClick={() => setMode("edit")}>Edit</button>}
        {note.can_delete && mode === "view" && <button type="button" className="rp-link" onClick={() => setMode("remove")}>Remove</button>}
      </footer>
      {hist && <ol className="rp-hist">{hist.map((h, i) => <li key={i}><span className="rp-muted">{fmtStamp(h.created_at)}, {h.actor_name}, {h.action} ({NOTE_TYPES[h.note_type]})</span><p>{h.body}</p></li>)}</ol>}
      <Problem error={err} />
    </article>
  );
}

/* ---------------------------------------------------------------- notes */

function NotesTab() {
  const [type, setType] = useHashParam("type"), [author, setAuthor] = useHashParam("author"), [q, setQ] = useHashParam("q");
  const [targetType, setTargetType] = useHashParam("target_type"), [targetKey] = useHashParam("target_key");
  const [removed, setRemoved] = useHashParam("removed"), [pageS, setPage] = useHashParam("page");
  const [text, setText] = useState(q);
  useEffect(() => { const t = setTimeout(() => { if (text !== q) setQ(text, { page: undefined }); }, 300); return () => clearTimeout(t); }, [text, q, setQ]);
  const page = Number(pageS) || 1;
  const list = useLoad(() => research.notes({ type, author, q, target_type: targetType, target_key: targetKey, deleted: removed ? "include" : undefined, page, size: 20 }),
    [type, author, q, targetType, targetKey, removed, page]);
  const summary = useLoad(research.summary, []);
  const [newTarget, setNewTarget] = useState<{ type: TargetType; key: string }>({ type: "general", key: "" });
  const me = user();
  return (
    <section className="rp-notes">
      <div className="rp-notes-main">
        <div className="rp-filters" role="search">
          <Field label="Search"><input type="search" data-search placeholder="Words in the note" value={text} onChange={e => setText(e.target.value)} /></Field>
          <Field label="Type"><select value={type} onChange={e => setType(e.target.value, { page: undefined })}>
            <option value="">All types</option>{Object.entries(NOTE_TYPES).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
          </select></Field>
          <Field label="Attached to"><select value={targetType} onChange={e => setTargetType(e.target.value, { target_key: undefined, page: undefined })}>
            <option value="">Anything</option><option value="strategy">A strategy</option><option value="book">A book</option><option value="author">An author</option><option value="general">General log</option>
          </select></Field>
          <Field label="Written by"><select value={author} onChange={e => setAuthor(e.target.value, { page: undefined })}>
            <option value="">Anyone</option>{me && <option value={me.id}>Me ({me.name})</option>}
          </select></Field>
          <label className="rp-check"><input type="checkbox" checked={Boolean(removed)} onChange={e => setRemoved(e.target.checked ? "1" : undefined, { page: undefined })} />Show removed notes</label>
        </div>
        {targetKey && <p className="rp-chip-line">Showing notes for <b>{targetKey}</b> <button type="button" className="rp-link" onClick={() => setTargetType(undefined, { target_key: undefined })}>Show all notes</button></p>}
        <Problem error={list.error} retry={list.reload} />
        {list.data && !list.data.items.length && <div className="rp-empty">No notes match. Write the first one on the right, or from a strategy in Strategy review.</div>}
        <div className="rp-thread">{list.data?.items.map(n => <NoteCard key={n.id} note={n} onChanged={list.reload} />)}</div>
        {list.data && <Pager page={list.data} onPage={p => setPage(String(p))} label="notes" />}
      </div>
      {canWrite() && (
        <aside className="rp-card rp-notes-new">
          <h2>New note</h2>
          <p className="rp-muted small">For a single strategy, open it in Strategy review so the note sits next to its rules.</p>
          <Field label="Attach to"><select value={newTarget.type} onChange={e => setNewTarget({ type: e.target.value as TargetType, key: "" })}>
            <option value="general">General log</option><option value="book">A book</option><option value="author">An author</option>
          </select></Field>
          {newTarget.type === "book" && (
            <Field label="Book"><select value={newTarget.key} onChange={e => setNewTarget({ type: "book", key: e.target.value })}>
              <option value="">Choose a book</option>
              {summary.data?.book_index.map(b => <option key={b.id} value={b.id}>{b.author}: {b.title}</option>)}
            </select></Field>
          )}
          {newTarget.type === "author" && (
            <Field label="Author"><select value={newTarget.key} onChange={e => setNewTarget({ type: "author", key: e.target.value })}>
              <option value="">Choose an author</option>
              {summary.data?.author_index.map(a => <option key={a.id} value={a.id}>{a.name}</option>)}
            </select></Field>
          )}
          {(newTarget.type === "general" || newTarget.key) && <Composer target={newTarget} compact onSaved={() => list.reload()} />}
        </aside>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------- timeline */

const KIND_LABEL = { note: "Notes", verdict: "Verdicts", platform: "Platform and deploys" } as const;
function TimelineTab() {
  const [kind, setKind] = useHashParam("kind"), [person, setPerson] = useHashParam("person");
  const [from, setFrom] = useHashParam("from"), [to, setTo] = useHashParam("to"), [pageS, setPage] = useHashParam("page");
  const page = Number(pageS) || 1;
  const list = useLoad(() => research.timeline({ kind, person, from, to, page, size: 30 }), [kind, person, from, to, page]);
  const people = useLoad(research.people, []);
  const groups = useMemo(() => {
    const out: { day: string; items: TimelineItem[] }[] = [];
    for (const it of list.data?.items || []) {
      const d = dayKey(it.at);
      if (out.at(-1)?.day !== d) out.push({ day: d, items: [] });
      out.at(-1)!.items.push(it);
    }
    return out;
  }, [list.data]);
  const reset = { page: undefined };
  return (
    <section className="rp-timeline">
      <div className="rp-filters" role="search">
        <Field label="What"><select value={kind} onChange={e => setKind(e.target.value, reset)}>
          <option value="">Everything</option>{Object.entries(KIND_LABEL).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </select></Field>
        <Field label="Who"><select value={person} onChange={e => setPerson(e.target.value, reset)}>
          <option value="">Everyone</option>
          {people.data?.map(p => <option key={p.id || p.name} value={p.id || p.name}>{p.name}</option>)}
          <option value="Platform">Platform</option><option value="Claude">Claude</option><option value="Codex">Codex</option><option value="Author loop">Author loop</option>
        </select></Field>
        <Field label="From"><input type="date" value={from} onChange={e => setFrom(e.target.value, reset)} /></Field>
        <Field label="To"><input type="date" value={to} onChange={e => setTo(e.target.value, reset)} /></Field>
      </div>
      <Problem error={list.error} retry={list.reload} />
      {list.data && !list.data.items.length && <div className="rp-empty">Nothing in this period. Widen the dates or choose Everyone.</div>}
      <ol className="rp-days">
        {groups.map(g => (
          <li key={g.day}>
            <h2 className="rp-day">{fmtDate(g.items[0].at)}</h2>
            <ol className="rp-events">{g.items.map((it, i) => <TimelineEntry key={`${it.kind}-${it.ref}-${it.at}-${i}`} it={it} />)}</ol>
          </li>
        ))}
      </ol>
      {list.data && <Pager page={list.data} onPage={p => setPage(String(p))} label="events" />}
    </section>
  );
}
function TimelineEntry({ it }: { it: TimelineItem }) {
  const target = it.target_type === "strategy" ? <a href={`#/research?tab=review&key=${encodeURIComponent(it.target_key)}`}>{it.target_label}</a> : it.target_label;
  let what: ReactNode;
  if (it.kind === "platform") what = <><p className="rp-ev-text">{it.text}</p>{(it.backtested != null || it.proof) && <p className="rp-muted small">{it.backtested != null && `${it.backtested} backtested. `}{it.proof && `Proof: ${it.proof}`}</p>}</>;
  else if (it.kind === "verdict") what = <p className="rp-ev-text">Set <VerdictBadge v={it.detail as Verdict} />{it.text ? <span className="rp-muted"> (was {VERDICTS[it.text as Verdict] || it.text})</span> : <span className="rp-muted"> (imported from the Excel sheet)</span>} on {target}</p>;
  else what = (
    <>
      <p className="rp-ev-head">{it.action === "created" ? "Wrote" : it.action === "edited" ? "Edited" : "Removed"} a <TypeBadge t={it.detail as NoteType} /> note{it.target_type !== "general" && <> on {target}</>}</p>
      {it.removed && it.action !== "deleted" ? <p className="rp-muted small">The note was removed later; its text is kept in the audit history.</p> : <p className="rp-ev-text quote">{it.text}</p>}
    </>
  );
  return (
    <li className={`rp-ev k-${it.kind}${it.kind === "platform" ? ` w-${it.who.replace(/\s/g, "").toLowerCase()}` : ""}`}>
      <span className="rp-ev-time">{it.kind === "platform" && /^~?\d+ Oct/.test(it.detail || "") ? (it.detail || "").replace(/^~?\d+ Oct\s*/, "") || "during the day" : fmtTime(it.at)}</span>
      <span className="rp-ev-dot" aria-hidden="true" />
      <div className="rp-ev-body"><p className="rp-ev-who">{it.who}</p>{what}</div>
    </li>
  );
}

/* ---------------------------------------------------------------- people */

function PeopleTab() {
  const { data, error, reload } = useLoad(research.people, []);
  const [page, setPage] = useState(1);
  if (error) return <Problem error={error} retry={reload} />;
  if (!data) return <p className="rp-loading">Loading people…</p>;
  if (!data.length) return <div className="rp-empty">No one has recorded a verdict or note yet.</div>;
  const size = 12, pages = Math.max(1, Math.ceil(data.length / size));
  return (
    <section>
      <ul className="rp-people">
        {data.slice((page - 1) * size, page * size).map((p: Person) => (
          <li key={p.id || p.name} className="rp-card">
            <h2>{p.name}</h2>
            <p className="rp-muted small">{p.imported ? "Reviewer named in the Excel sheet" : p.role === "admin" ? "Administrator" : p.role === "viewer" ? "Viewer (read only)" : "Researcher"}
              {p.last_at ? `. Last active ${ago(p.last_at)}` : ". No activity yet"}</p>
            <dl>
              <div><dt>Verdicts</dt><dd>{p.verdicts}</dd></div>
              <div><dt>Matches the book</dt><dd>{p.matches}</dd></div>
              <div><dt>Mismatch</dt><dd>{p.mismatches}</dd></div>
              <div><dt>Unclear</dt><dd>{p.unclear}</dd></div>
              <div><dt>Notes</dt><dd>{p.notes}</dd></div>
              <div><dt>Rule errors</dt><dd>{p.rule_errors}</dd></div>
            </dl>
            <p className="rp-row">
              <a href={`#/research?tab=timeline&person=${encodeURIComponent(p.id || p.name)}`}>Their timeline</a>
              <a href={`#/research?tab=review&reviewer=${encodeURIComponent(p.name)}`}>Strategies they judged</a>
              {p.id && <a href={`#/research?tab=notes&author=${encodeURIComponent(p.id)}`}>Their notes</a>}
            </p>
          </li>
        ))}
      </ul>
      <Pager page={{ page, pages, total: data.length }} onPage={setPage} label="people" />
    </section>
  );
}
