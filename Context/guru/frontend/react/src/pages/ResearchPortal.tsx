import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";
import "./ResearchPortal.css";
import {
  research, VERDICTS, NOTE_TYPES, SITE, fmtDate, fmtStamp, ago, dayKey, readHashParams, hashWith,
  type Card, type Employee, type Me, type Note, type NoteType, type Page, type Site, type Strategy, type TimelineItem, type Verdict,
} from "../services/research";

/* Research page: employees enter with their name and the team PIN, then browse author → book → strategy cards,
   mark each strategy against the book and leave notes. The admin password shows every employee's card and work. */

const num = (n: number | undefined) => (n ?? 0).toLocaleString("en-IN");

function useHash() {
  const [p, setP] = useState(() => readHashParams());
  useEffect(() => {
    const sync = () => setP(readHashParams());
    addEventListener("hashchange", sync);
    return () => removeEventListener("hashchange", sync);
  }, []);
  return p;
}
const go = (patch: Record<string, string | undefined>, keep = false) => {
  location.hash = keep ? hashWith(patch) : hashWith({ v: undefined, author: undefined, book: undefined, s: undefined, emp: undefined, q: undefined, page: undefined, ...patch });
};

function useLoad<T>(load: () => Promise<T>, deps: unknown[]) {
  const [state, setState] = useState<{ data?: T; error?: string }>({});
  const [tick, setTick] = useState(0);
  useEffect(() => {
    let live = true;
    load().then(data => live && setState({ data }), (e: Error) => live && setState({ error: e.message }));
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, tick]);
  return { ...state, reload: useCallback(() => setTick(t => t + 1), []) };
}

export function ResearchPortal() {
  const me = useLoad(research.me, []);
  useEffect(() => { document.title = "Research · Strategy Vault"; }, []);
  if (me.error) return <div className="rp"><Problem error={me.error} retry={me.reload} /></div>;
  if (!me.data) return <div className="rp"><p className="rp-loading">Loading…</p></div>;
  if (!me.data.employee && !me.data.admin) return <div className="rp"><Gate me={me.data} done={me.reload} /></div>;
  return <div className="rp"><Signed me={me.data} reload={me.reload} /></div>;
}

/* ---------------------------------------------------------------- entry */

function Gate({ me, done }: { me: Me; done: () => void }) {
  const [admin, setAdmin] = useState(false);
  const [name, setName] = useState(""), [pin, setPin] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    try { if (admin) await research.admin(pin); else await research.enter(name, pin); done(); }
    catch (err) { setError((err as Error).message); setBusy(false); }
  };
  return (
    <form className="rp-gate" onSubmit={submit}>
      <h1>{admin ? "Admin" : "Who is reviewing?"}</h1>
      <p className="rp-muted">{admin ? "Enter the admin password to see every employee's work." : "Type your name and the team PIN. Your checks and notes are saved under your name."}</p>
      {!admin && <label className="rp-field"><span>Your name</span><input autoFocus value={name} onChange={e => setName(e.target.value)} placeholder="e.g. Manan" autoComplete="name" required minLength={2} maxLength={60} /></label>}
      <label className="rp-field"><span>{admin ? "Admin password" : "PIN"}</span><input autoFocus={admin} type="password" value={pin} onChange={e => setPin(e.target.value)} required autoComplete="off" /></label>
      {!me.ready && !admin && <p className="rp-problem">The PIN is not set up on the server yet.</p>}
      {error && <p className="rp-problem" role="alert">{error}</p>}
      <button className="rp-primary" type="submit" disabled={busy}>{busy ? "Checking…" : "Enter"}</button>
      <button type="button" className="rp-link" onClick={() => { setAdmin(!admin); setPin(""); setError(""); }}>{admin ? "Back to employee entry" : "I am the admin"}</button>
    </form>
  );
}

/* ---------------------------------------------------------------- signed in */

function Signed({ me, reload }: { me: Me; reload: () => void }) {
  const p = useHash(), v = p.get("v") || "";
  const who = me.employee?.name;
  const leave = async () => { await research.leave(); go({}); reload(); };
  let body: ReactNode;
  if (p.get("s")) body = <StrategyView key={p.get("s")} sKey={p.get("s")!} me={me} />;
  else if (v === "mine" && who) body = <WorkView person={who} title="My work" admin={false} />;
  else if (v === "team" && me.admin) body = <TeamView />;
  else if (v === "claude") body = <ClaudeView />;
  else if (v === "emp" && me.admin && p.get("emp")) body = <WorkView person={p.get("emp")!} title={`${p.get("emp")}'s work`} admin />;
  else if (p.get("book")) body = <BookView author={p.get("author") || ""} book={p.get("book")!} />;
  else if (p.get("author")) body = <AuthorView author={p.get("author")!} />;
  else body = <Home me={me} />;
  return (
    <>
      <header className="rp-bar">
        <a className="rp-brand" href="#/research">Research</a>
        <nav>
          <a href="#/research" aria-current={!v && !p.get("author") && !p.get("s") ? "page" : undefined}>Authors</a>
          {who && <a href={hashWith({ v: "mine" }, "#/research")} aria-current={v === "mine" ? "page" : undefined}>My work</a>}
          <a href={hashWith({ v: "claude" }, "#/research")} aria-current={v === "claude" ? "page" : undefined}>Claude's work</a>
          {me.admin && <a href={hashWith({ v: "team" }, "#/research")} aria-current={v === "team" || v === "emp" ? "page" : undefined}>Employees</a>}
        </nav>
        <span className="rp-who">{who ? <>Hi, <b>{who}</b></> : null}{me.admin && <span className="rp-tag">Admin</span>}
          <button type="button" className="rp-link" onClick={leave}>Switch person</button></span>
      </header>
      {body}
    </>
  );
}

function Problem({ error, retry }: { error?: string; retry?: () => void }) {
  if (!error) return null;
  return <div className="rp-problem" role="alert">{error} {retry && <button type="button" onClick={retry}>Try again</button>}</div>;
}
const Crumbs = ({ items }: { items: [string, string?][] }) => (
  <nav className="rp-crumbs" aria-label="Where you are">
    {items.map(([label, href], i) => <span key={i}>{href ? <a href={href}>{label}</a> : <b>{label}</b>}</span>)}
  </nav>
);
function Bar({ c }: { c: Pick<Card, "strategies" | "verified" | "held" | "unreviewed"> }) {
  const w = (n: number) => `${(100 * n) / Math.max(1, c.strategies)}%`;
  return (
    <div className="rp-bar3" role="img" aria-label={`${c.verified} backtested, ${c.held} held, ${c.unreviewed} waiting`}>
      <i className="ok" style={{ width: w(c.verified) }} /><i className="held" style={{ width: w(c.held) }} /><i className="todo" style={{ width: w(c.unreviewed) }} />
    </div>
  );
}
const Legend = () => (
  <p className="rp-legend"><span><i className="ok" />Backtested</span><span><i className="held" />Held (reason given)</span><span><i className="todo" />Waiting for review</span></p>
);
function Pager({ page, set }: { page: Pick<Page<unknown>, "page" | "pages">; set: (n: number) => void }) {
  if (page.pages <= 1) return null;
  return (
    <nav className="rp-pager" aria-label="Pages">
      <button type="button" disabled={page.page <= 1} onClick={() => set(page.page - 1)}>Previous</button>
      <span>Page {page.page} of {page.pages}</span>
      <button type="button" disabled={page.page >= page.pages} onClick={() => set(page.page + 1)}>Next</button>
    </nav>
  );
}
const PER = 24;
function usePage(): [number, (n: number) => void] {
  const p = useHash(), n = Math.max(1, Number(p.get("page")) || 1);
  return [n, (x: number) => { go({ page: x > 1 ? String(x) : undefined }, true); scrollTo({ top: 0 }); }];
}
const SiteBadge = ({ site }: { site: Site }) => <span className={`rp-pill site-${site}`}>{SITE[site]}</span>;
const VerdictBadge = ({ v }: { v: Verdict }) => <span className={`rp-pill v-${v}`}>{v === "not_checked" ? "Not checked by team" : VERDICTS[v]}</span>;

/* ---------------------------------------------------------------- home: author cards */

function Home({ me }: { me: Me }) {
  const sum = useLoad(research.summary, []);
  const cards = useLoad(() => research.browse(), []);
  const [q, setQ] = useState(""), [page, setPage] = usePage();
  const t = sum.data?.totals;
  const list = (cards.data?.items || []).filter(c => !q || c.name.toLowerCase().includes(q.toLowerCase()));
  const pages = Math.max(1, Math.ceil(list.length / PER));
  return (
    <section>
      {t && (
        <div className="rp-stats">
          <Stat n={t.verified} label="Backtested" cls="ok" />
          <Stat n={t.held} label="Held with a reason" cls="held" />
          <Stat n={t.unreviewed} label="Waiting for review" cls="todo" />
          <Stat n={(sum.data!.team.verdicts.matches || 0) + (sum.data!.team.verdicts.mismatch || 0) + (sum.data!.team.verdicts.unclear || 0)} label="Checked by the team" cls="team" />
        </div>
      )}
      {me.employee && <p className="rp-muted">Pick an author, then a book, then a strategy. Mark whether the backtest matches the book and write a note if something is wrong.</p>}
      <div className="rp-tools">
        <input type="search" placeholder="Find an author…" value={q} onChange={e => { setQ(e.target.value); setPage(1); }} aria-label="Find an author" />
        <Legend />
      </div>
      <Problem error={cards.error || sum.error} retry={cards.reload} />
      {!cards.data ? <p className="rp-loading">Loading authors…</p> : (
        <>
          <div className="rp-grid">
            {list.slice((page - 1) * PER, page * PER).map(c => (
              <a key={c.id} className="rp-card" href={hashWith({ author: c.id }, "#/research")}>
                <h2>{c.name}</h2>
                <p className="rp-muted">{num(c.books)} {c.books === 1 ? "book" : "books"} · {num(c.strategies)} strategies</p>
                <Bar c={c} />
                <p className="rp-small">{num(c.verified)} backtested · {num(c.unreviewed)} waiting · <b>{num(c.checked)}</b> checked by team</p>
              </a>
            ))}
          </div>
          {!list.length && <p className="rp-empty">No author matches “{q}”.</p>}
          <Pager page={{ page, pages }} set={setPage} />
        </>
      )}
    </section>
  );
}
const Stat = ({ n, label, cls }: { n: number; label: string; cls: string }) => (
  <div className={`rp-stat ${cls}`}><b>{num(n)}</b><span>{label}</span></div>
);

/* ---------------------------------------------------------------- author: book cards */

function AuthorView({ author }: { author: string }) {
  const books = useLoad(() => research.browse(author), [author]);
  const name = books.data?.items[0]?.author || author;
  return (
    <section>
      <Crumbs items={[["All authors", "#/research"], [name]]} />
      <h1>{name}</h1>
      <Legend />
      <Problem error={books.error} retry={books.reload} />
      {!books.data ? <p className="rp-loading">Loading books…</p> : (
        <div className="rp-grid">
          {books.data.items.map(b => (
            <a key={b.id} className="rp-card" href={hashWith({ author, book: b.id }, "#/research")}>
              <h2>{b.name}</h2>
              <p className="rp-muted">{num(b.strategies)} strategies</p>
              <Bar c={b} />
              <p className="rp-small">{num(b.verified)} backtested · {num(b.held)} held · {num(b.unreviewed)} waiting · <b>{num(b.checked)}</b> checked</p>
            </a>
          ))}
        </div>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------- book: strategy cards */

function BookView({ author, book }: { author: string; book: string }) {
  const [page, setPage] = usePage();
  const [verdict, setVerdict] = useState("");
  const list = useLoad(() => research.strategies({ author, book, verdict, page, size: PER }), [author, book, verdict, page]);
  const first = list.data?.items[0];
  return (
    <section>
      <Crumbs items={[["All authors", "#/research"], [first?.author || author, hashWith({ author }, "#/research")], [first?.book || book]]} />
      <h1>{first?.book || "Book"}</h1>
      <div className="rp-chips" role="group" aria-label="Show">
        {[["", "All"], ["not_checked", "Not checked yet"], ["matches", "Matches"], ["mismatch", "Mismatch"], ["unclear", "Unclear"]].map(([k, l]) => (
          <button key={k} type="button" aria-pressed={verdict === k} onClick={() => { setVerdict(k); setPage(1); }}>{l}</button>
        ))}
      </div>
      <Problem error={list.error} retry={list.reload} />
      {!list.data ? <p className="rp-loading">Loading strategies…</p> : (
        <>
          <div className="rp-grid">
            {list.data.items.map(s => <StrategyCard key={s.key} s={s} />)}
          </div>
          {!list.data.items.length && <p className="rp-empty">Nothing here with this filter.</p>}
          <Pager page={list.data} set={setPage} />
        </>
      )}
    </section>
  );
}
const StrategyCard = ({ s }: { s: Strategy }) => (
  <a className={`rp-card rp-s v-${s.verdict}`} href={hashWith({ s: s.key }, "#/research")}>
    <h2>{s.name}</h2>
    <p className="rp-pills"><SiteBadge site={s.site} /><VerdictBadge v={s.verdict} /></p>
    {s.site === "held" && s.reason && <p className="rp-small rp-muted">{s.reason}</p>}
    <p className="rp-small">{s.notes ? `${s.notes} ${s.notes === 1 ? "note" : "notes"}` : "No notes"}{s.verdict_by && ` · checked by ${s.verdict_by}`}</p>
  </a>
);

/* ---------------------------------------------------------------- one strategy */

function StrategyView({ sKey, me }: { sKey: string; me: Me }) {
  const d = useLoad(() => research.strategy(sKey), [sKey]);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  if (d.error) return <Problem error={d.error} retry={d.reload} />;
  if (!d.data) return <p className="rp-loading">Loading…</p>;
  const s = d.data.strategy, v = d.data.verdict;
  const mark = async (x: Verdict) => {
    setBusy(true); setError("");
    try { await research.setVerdict(sKey, x); d.reload(); } catch (e) { setError((e as Error).message); } finally { setBusy(false); }
  };
  return (
    <section className="rp-one">
      {s && <Crumbs items={[["All authors", "#/research"], [s.author, hashWith({ author: s.author_id }, "#/research")], [s.book, hashWith({ author: s.author_id, book: s.book_id }, "#/research")], [s.name]]} />}
      <h1>{s?.name || sKey}</h1>
      {s && <p className="rp-pills"><SiteBadge site={s.site} />{s.url && <a href={s.url.replace(/^https?:\/\/[^/#]+/, "")}>Open the backtest →</a>}</p>}
      {s?.site === "held" && <p className="rp-reason"><b>Why it is held:</b> {s.reason}</p>}
      <div className="rp-cols">
        <div className="rp-panel">
          <h2>Rules from the book</h2>
          {s?.rules?.length ? <ul className="rp-rules">{s.rules.map((r, i) => <li key={i}><span>{r.kind}</span>{r.text}</li>)}</ul>
            : <p className="rp-muted">{s?.coding_notes || "No rules listed."}</p>}
        </div>
        <div className="rp-panel">
          <h2>Does the backtest match the book?</h2>
          <div className="rp-verdicts">
            {(["matches", "mismatch", "unclear"] as Verdict[]).map(x => (
              <button key={x} type="button" className={`v-${x}`} aria-pressed={v.verdict === x} disabled={busy || !me.employee} onClick={() => mark(v.verdict === x ? "not_checked" : x)}>
                {{ matches: "✓ Matches", mismatch: "✗ Mismatch", unclear: "? Unclear", not_checked: "" }[x]}
              </button>
            ))}
          </div>
          <p className="rp-small rp-muted">{v.verdict === "not_checked" ? "Nobody has checked this yet." : `${VERDICTS[v.verdict]} — ${v.author_name}, ${v.updated_at ? fmtStamp(v.updated_at) : ""}`}</p>
          {!me.employee && <p className="rp-small rp-muted">Enter with your name to mark or write notes.</p>}
          {error && <p className="rp-problem">{error}</p>}
          <h2>Notes</h2>
          {me.employee && <NoteForm sKey={sKey} done={d.reload} />}
          <Notes notes={d.data.notes} reload={d.reload} />
        </div>
      </div>
    </section>
  );
}

function NoteForm({ sKey, done }: { sKey: string; done: () => void }) {
  const [type, setType] = useState<NoteType>("question"), [body, setBody] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const save = async (e: FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    try { await research.addNote({ target_type: "strategy", target_key: sKey, note_type: type, body }); setBody(""); done(); }
    catch (err) { setError((err as Error).message); } finally { setBusy(false); }
  };
  return (
    <form className="rp-noteform" onSubmit={save}>
      <div className="rp-chips" role="group" aria-label="Kind of note">
        {(Object.keys(NOTE_TYPES) as NoteType[]).map(t => <button key={t} type="button" aria-pressed={type === t} onClick={() => setType(t)}>{NOTE_TYPES[t]}</button>)}
      </div>
      <textarea value={body} onChange={e => setBody(e.target.value)} placeholder="What did you find? (time and your name are added automatically)" rows={3} required maxLength={10000} />
      {error && <p className="rp-problem">{error}</p>}
      <button className="rp-primary" type="submit" disabled={busy || !body.trim()}>Save note</button>
    </form>
  );
}

function Notes({ notes, reload }: { notes: Note[]; reload: () => void }) {
  const live = notes.filter(n => !n.deleted_at);
  if (!live.length) return <p className="rp-muted rp-small">No notes yet.</p>;
  return <ul className="rp-notes">{live.map(n => <NoteItem key={n.id} n={n} reload={reload} />)}</ul>;
}
function NoteItem({ n, reload }: { n: Note; reload: () => void }) {
  const [edit, setEdit] = useState(false), [text, setText] = useState(n.body), [error, setError] = useState("");
  const save = async () => { try { await research.editNote(n.id, { body: text }); setEdit(false); reload(); } catch (e) { setError((e as Error).message); } };
  return (
    <li className={`t-${n.note_type}`}>
      <p className="rp-meta"><span className="rp-type">{NOTE_TYPES[n.note_type]}</span> <b>{n.author_name}</b> · <time dateTime={n.created_at} title={fmtStamp(n.created_at)}>{ago(n.created_at)}</time>{n.edits > 0 && " · edited"}</p>
      {edit ? <textarea value={text} onChange={e => setText(e.target.value)} rows={3} /> : <p className="rp-body">{n.body}</p>}
      {error && <p className="rp-problem">{error}</p>}
      <p className="rp-actions">
        {n.can_edit && (edit ? <><button type="button" onClick={save}>Save</button><button type="button" onClick={() => setEdit(false)}>Cancel</button></> : <button type="button" onClick={() => setEdit(true)}>Edit</button>)}
        {n.can_delete && !edit && <Remove label="Delete" run={() => research.removeNote(n.id, "").then(reload)} />}
      </p>
    </li>
  );
}
/** Two-step delete: the first click asks, the second does it. */
function Remove({ label, run }: { label: string; run: () => Promise<unknown> }) {
  const [ask, setAsk] = useState(false);
  return ask
    ? <span className="rp-confirm">Sure? <button type="button" className="danger" onClick={() => run().catch(() => setAsk(false))}>Yes, {label.toLowerCase()}</button><button type="button" onClick={() => setAsk(false)}>No</button></span>
    : <button type="button" onClick={() => setAsk(true)}>{label}</button>;
}

/* ---------------------------------------------------------------- one person's work */

function WorkView({ person, title, admin }: { person: string; title: string; admin: boolean }) {
  const [page, setPage] = usePage();
  const list = useLoad(() => research.timeline({ person, page, size: 30 }), [person, page]);
  const days = new Map<string, TimelineItem[]>();
  for (const it of list.data?.items || []) { const k = dayKey(it.at); days.set(k, [...(days.get(k) || []), it]); }
  return (
    <section>
      {admin && <Crumbs items={[["Employees", hashWith({ v: "team" }, "#/research")], [person]]} />}
      <h1>{title}</h1>
      <PersonWork person={person} />
      <h2 className="rp-section">Timeline</h2>
      <p className="rp-muted">Every check and note, newest first.</p>
      <Problem error={list.error} retry={list.reload} />
      {!list.data ? <p className="rp-loading">Loading…</p> : !list.data.items.length ? <p className="rp-empty">Nothing yet. Open a strategy and mark it or write a note.</p> : (
        <>
          {[...days].map(([day, items]) => (
            <div key={day} className="rp-day">
              <h3>{fmtDate(items[0].at)}</h3>
              <ul className="rp-feed">
                {items.map((it, i) => (
                  <li key={i} className={`k-${it.kind}`}>
                    <time>{new Date(it.at).toLocaleTimeString("en-IN", { timeZone: "Asia/Kolkata", hour: "2-digit", minute: "2-digit", hour12: false })}</time>
                    <div>
                      {it.kind === "verdict" ? <p>Marked <b>{VERDICTS[it.detail as Verdict] || it.detail}</b></p>
                        : <p><span className="rp-type">{NOTE_TYPES[it.detail as NoteType] || it.detail}</span> {it.action === "created" ? "note" : it.action === "edited" ? "note edited" : "note removed"}{it.removed && it.action !== "deleted" && " (removed later)"}</p>}
                      {it.target_key && <p className="rp-small"><a href={hashWith({ s: it.target_key }, "#/research")}>{it.target_label}</a></p>}
                      {it.kind === "note" && it.text && <p className="rp-body">{it.text}</p>}
                      {admin && it.kind === "note" && it.action === "created" && !it.removed && it.ref && <Remove label="Delete note" run={() => research.removeNote(it.ref!, "").then(list.reload)} />}
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          ))}
          <Pager page={list.data} set={setPage} />
        </>
      )}
    </section>
  );
}

/** What one person checked, as author cards with each book's strategies and their verdict. */
function PersonWork({ person }: { person: string }) {
  const w = useLoad(() => research.work(person), [person]);
  if (w.error) return <Problem error={w.error} retry={w.reload} />;
  if (!w.data) return <p className="rp-loading">Loading…</p>;
  const t = w.data.totals;
  if (!t.strategies) return null;
  return (
    <>
      <div className="rp-stats">
        <Stat n={t.strategies} label="Strategies looked at" cls="team" />
        <Stat n={t.matches} label="Matches the book" cls="ok" />
        <Stat n={t.mismatch} label="Mismatch" cls="bad" />
        <Stat n={t.unclear} label="Unclear" cls="held" />
        <Stat n={t.notes} label="Notes" cls="todo" />
      </div>
      <div className="rp-grid rp-work">
        {w.data.items.map(a => (
          <div key={a.author_id} className="rp-card">
            <h2>{a.author}</h2>
            {a.books.map(b => (
              <div key={b.book_id} className="rp-wbook">
                <p className="rp-small rp-muted">{b.book}</p>
                <ul>
                  {b.strategies.map(s => (
                    <li key={s.key}>
                      <a href={hashWith({ s: s.key }, "#/research")}>{s.name}</a>
                      {s.verdict && s.verdict !== "not_checked" ? <VerdictBadge v={s.verdict} /> : null}
                      {s.notes > 0 && <span className="rp-small rp-muted">{s.notes} {s.notes === 1 ? "note" : "notes"}{s.errors ? ` · ${s.errors} rule error` : ""}</span>}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        ))}
      </div>
    </>
  );
}

/* ---------------------------------------------------------------- Claude's work: the platform timeline */

function ClaudeView() {
  const [page, setPage] = usePage();
  const list = useLoad(() => research.timeline({ kind: "platform", page, size: 30 }), [page]);
  const days = new Map<string, TimelineItem[]>();
  for (const it of list.data?.items || []) { const k = dayKey(it.at); days.set(k, [...(days.get(k) || []), it]); }
  return (
    <section>
      <h1>Claude's work</h1>
      <p className="rp-muted">What Claude, Codex and the author loop built, deploy by deploy, newest first. The number is how many strategies were backtested on the site after that step.</p>
      <Problem error={list.error} retry={list.reload} />
      {!list.data ? <p className="rp-loading">Loading…</p> : (
        <>
          {[...days].map(([day, items]) => (
            <div key={day} className="rp-day">
              <h3>{fmtDate(items[0].at)}</h3>
              <ul className="rp-feed rp-claude">
                {items.map((it, i) => (
                  <li key={i} className={`w-${it.who.toLowerCase().replace(/\W+/g, "-")}`}>
                    <span className="rp-count">{it.backtested != null ? <><b>{num(it.backtested)}</b><small>backtested</small></> : null}</span>
                    <div>
                      <p className="rp-meta"><span className="rp-pill">{it.who}</span>{it.detail && it.detail !== day && <span> {it.detail}</span>}</p>
                      <p>{it.text}</p>
                      {it.proof && <p className="rp-small rp-muted">Proof: {it.proof}</p>}
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          ))}
          <Pager page={list.data} set={setPage} />
        </>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------- admin: employee cards */

function TeamView() {
  const list = useLoad(research.employees, []);
  return (
    <section>
      <h1>Employees</h1>
      <p className="rp-muted">Everyone who has entered with the PIN. Open a card to see their work and remove notes.</p>
      <Problem error={list.error} retry={list.reload} />
      {!list.data ? <p className="rp-loading">Loading…</p> : !list.data.length ? <p className="rp-empty">Nobody has entered yet.</p> : (
        <div className="rp-grid">
          {list.data.map((e: Employee) => (
            <div key={e.id} className={`rp-card rp-emp${e.removed_at ? " removed" : ""}`}>
              <a href={hashWith({ v: "emp", emp: e.name }, "#/research")}><h2>{e.name}</h2></a>
              <p className="rp-nums"><span><b>{num(e.checks)}</b> checks</span><span><b>{num(e.notes)}</b> notes</span><span><b>{num(e.mismatches)}</b> mismatches</span></p>
              <p className="rp-small rp-muted">Since {fmtDate(e.created_at)} · last seen {ago(e.last_seen_at)}</p>
              {e.removed_at ? <p className="rp-small">Removed {fmtDate(e.removed_at)}</p> : <Remove label="Remove employee" run={() => research.removeEmployee(e.id).then(list.reload)} />}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
