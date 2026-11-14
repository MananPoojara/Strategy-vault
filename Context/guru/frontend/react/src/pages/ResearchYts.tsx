import { useCallback, useEffect, useMemo, useRef, useState, type ClipboardEvent, type FormEvent } from "react";
import "./ResearchYts.css";
import {
  yts, ytCollections, CATEGORIES, SEVERITIES, STATUSES, GENERAL, MAX_FILE_BYTES, fileUrl, fmtSize, fmtDate, fmtStamp, readHash, hashWith,
  type Category, type Issue, type IssueEvent, type IssueFile, type Qa, type Severity, type Status, type Summary, type YtCollection, type YtStrategy,
} from "../services/research-yts";

/* Research YTS: the QA team reports problems they find on the YT Strategies pages. Each issue belongs to a YT
   collection and, usually, one strategy in it; developers and QAs move it from Open to Fixed and comment on it. */

const ACTIVE = "open,in_progress";
const STATUS_FILTERS: [string, string][] = [[ACTIVE, "Open & in progress"], ["fixed", "Fixed"], ["not_issue", "Not an issue"], ["all", "All"]];

function useHash() {
  const [p, setP] = useState(() => readHash());
  useEffect(() => {
    const sync = () => setP(readHash());
    addEventListener("hashchange", sync);
    return () => removeEventListener("hashchange", sync);
  }, []);
  return p;
}
const go = (patch: Record<string, string | undefined>) => { location.hash = hashWith(patch); };

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

/** Strategy names as the YT pages show them; two strategies with the same name in one collection get their id added. */
let DUPLICATE_NAMES = new Set<string>();
const strategyLabel = (s: YtStrategy) => DUPLICATE_NAMES.has(`${s.book}|${s.name}`) ? `${s.name} (${s.id})` : s.name;
function markDuplicates(list: YtStrategy[]) {
  const seen = new Map<string, number>();
  for (const s of list) seen.set(`${s.book}|${s.name}`, (seen.get(`${s.book}|${s.name}`) || 0) + 1);
  DUPLICATE_NAMES = new Set([...seen].filter(([, n]) => n > 1).map(([k]) => k));
}

export function ResearchYts() {
  const me = useLoad(yts.me, []);
  useEffect(() => { document.title = "Research YTS · Strategy Vault"; }, []);
  return (
    <div className="yts">
      <nav className="crumbs"><a href="#/">Guru's</a><span>/</span><a href="#/yt">YT Strategies</a><span>/</span><span>Research YTS</span></nav>
      <section className="ahero yts-hero">
        <h1>Research YTS</h1>
        <p className="lede">Report problems you find on the YT Strategies pages. Pick the collection and the strategy, say what is wrong and how to see it. The admin marks each issue fixed; you can reopen it if it is not.</p>
      </section>
      {me.error ? <Problem error={me.error} retry={me.reload} />
        : !me.data ? <p className="yts-muted">Loading…</p>
        : !me.data.qa ? <SignIn done={me.reload} />
        : <Signed qa={me.data.qa} reload={me.reload} />}
    </div>
  );
}

/* ---------------------------------------------------------------- sign-in */

function SignIn({ done }: { done: () => void }) {
  const [username, setUsername] = useState(""), [password, setPassword] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    try { await yts.login(username, password); done(); }
    catch (err) { setError((err as Error).message); setPassword(""); setBusy(false); }
  };
  return (
    <form className="yts-form yts-gate" onSubmit={submit}>
      <h2>Sign in to Research YTS</h2>
      <p className="yts-muted">Use your own Research YTS username and password. They are separate from the app login; ask the Research YTS admin if you do not have one.</p>
      <label className="yts-field"><span>Username</span><input id="yts-username" autoFocus required value={username} onChange={e => setUsername(e.target.value)} autoComplete="username" autoCapitalize="none" spellCheck={false} maxLength={40} /></label>
      <label className="yts-field"><span>Password</span><input id="yts-password" type="password" required value={password} onChange={e => setPassword(e.target.value)} autoComplete="current-password" maxLength={128} /></label>
      {error && <Problem error={error} />}
      <button className="yts-primary" type="submit" disabled={busy}>{busy ? "Signing in…" : "Sign in"}</button>
    </form>
  );
}

function Signed({ qa, reload }: { qa: Qa; reload: () => void }) {
  const p = useHash();
  const catalogue = useLoad(yts.catalogue, []);
  const summary = useLoad(yts.summary, []);
  const [changing, setChanging] = useState(false);
  const collections = useMemo(() => {
    const list = ytCollections();
    const used = new Set((catalogue.data || []).map(s => s.book));
    return list.filter(c => used.has(c.book));
  }, [catalogue.data]);
  const byBook = useMemo(() => Object.fromEntries(collections.map(c => [c.book, c])), [collections]);
  const byId = useMemo(() => { markDuplicates(catalogue.data || []); return Object.fromEntries((catalogue.data || []).map(s => [s.id, s])); }, [catalogue.data]);
  const issue = Number.parseInt(p.get("issue") || "", 10);
  const accounts = qa.admin && p.get("view") === "accounts";
  const leave = async () => { await yts.logout().catch(() => { /* the cookie expires anyway */ }); reload(); };

  return (
    <>
      <div className="yts-who">
        <span>Signed in as <b>{qa.name}</b> <span className="yts-muted">({qa.username})</span>{qa.admin && <span className="yts-badge st-admin">Admin</span>}</span>
        {qa.admin && (accounts
          ? <a className="yts-link" href={hashWith({ view: undefined })}>Issues</a>
          : <a className="yts-link" href={hashWith({ view: "accounts", issue: undefined, new: undefined })}>QA accounts</a>)}
        <button type="button" className="yts-link" onClick={() => setChanging(!changing)}>Change my password</button>
        <button type="button" className="yts-link" onClick={() => void leave()}>Switch person</button>
      </div>
      {changing && <ChangePassword done={() => setChanging(false)} />}
      {!qa.admin && <p className="yts-muted">You see the issues you reported. The admin sees every QA's issues.</p>}
      {accounts ? <Accounts me={qa} /> : (
        <>
          {summary.data && <Totals s={summary.data} />}
          {catalogue.error && <Problem error={`The YT strategy list could not be loaded: ${catalogue.error}`} retry={catalogue.reload} />}
          <div className="yts-cols">
            <aside className="yts-side" aria-label="YT collections">
              <a className={!p.get("c") ? "on" : ""} href={hashWith({ c: undefined, s: undefined, issue: undefined, new: undefined, page: undefined })}>
                <span>All collections</span><b>{summary.data ? open(summary.data.totals) : ""}</b>
              </a>
              <a className={p.get("c") === GENERAL ? "on" : ""} href={hashWith({ c: GENERAL, s: undefined, issue: undefined, new: undefined, page: undefined })}>
                <span>General issues (several strategies)</span><b>{summary.data?.collections[GENERAL] ? open(summary.data.collections[GENERAL]) || "" : ""}</b>
              </a>
              {collections.map(c => (
                <a key={c.book} className={p.get("c") === c.book ? "on" : ""} href={hashWith({ c: c.book, s: undefined, issue: undefined, new: undefined, page: undefined })}>
                  <span>{c.name}</span><b>{summary.data?.collections[c.book] ? open(summary.data.collections[c.book]) || "" : ""}</b>
                </a>
              ))}
            </aside>
            <main className="yts-main">
              {p.get("new") ? (
                <NewIssue p={p} qa={qa} collections={collections} catalogue={catalogue.data || []} done={id => { summary.reload(); go({ new: undefined, issue: String(id), tk: undefined, from: undefined, to: undefined, link: undefined }); }} />
              ) : Number.isSafeInteger(issue) && issue > 0 ? (
                <IssueView id={issue} byBook={byBook} byId={byId} collections={collections} catalogue={catalogue.data || []} changed={summary.reload} />
              ) : (
                <IssueList p={p} qa={qa} byBook={byBook} byId={byId} summary={summary.data} catalogue={catalogue.data || []} />
              )}
            </main>
          </div>
        </>
      )}
    </>
  );
}

function ChangePassword({ done }: { done: () => void }) {
  const [current, setCurrent] = useState(""), [next, setNext] = useState(""), [again, setAgain] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false), [ok, setOk] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (next !== again) { setError("The two new passwords are different."); return; }
    setBusy(true); setError("");
    try { await yts.changePassword(current, next); setOk(true); setTimeout(done, 1200); }
    catch (err) { setError((err as Error).message); setBusy(false); }
  };
  return (
    <form className="yts-form yts-small-form" onSubmit={submit}>
      <div className="yts-grid">
        <label className="yts-field"><span>Current password</span><input type="password" required value={current} onChange={e => setCurrent(e.target.value)} autoComplete="current-password" /></label>
        <label className="yts-field"><span>New password (1 or more characters)</span><input type="password" required minLength={1} maxLength={128} value={next} onChange={e => setNext(e.target.value)} autoComplete="new-password" /></label>
        <label className="yts-field"><span>New password again</span><input type="password" required value={again} onChange={e => setAgain(e.target.value)} autoComplete="new-password" /></label>
      </div>
      {error && <Problem error={error} />}
      {ok ? <p className="yts-ok">Password changed. Other browsers signed in as you are signed out.</p>
        : <p className="yts-actions"><button className="yts-primary" type="submit" disabled={busy}>Change password</button><button type="button" className="yts-link" onClick={done}>Cancel</button></p>}
    </form>
  );
}

/* ---------------------------------------------------------------- QA accounts (admin) */

function Accounts({ me }: { me: Qa }) {
  const list = useLoad(yts.accounts, []);
  const [form, setForm] = useState({ username: "", name: "", password: "", admin: false });
  const [error, setError] = useState(""), [notice, setNotice] = useState(""), [busy, setBusy] = useState(false);
  const [resetting, setResetting] = useState<number | null>(null), [newPassword, setNewPassword] = useState("");
  const act = async (work: () => Promise<unknown>, message: string) => {
    setBusy(true); setError(""); setNotice("");
    try { await work(); setNotice(message); list.reload(); return true; } catch (err) { setError((err as Error).message); return false; } finally { setBusy(false); }
  };
  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (await act(() => yts.addAccount(form), `Added ${form.name}. Give them the username "${form.username.trim().toLowerCase()}" and the password you typed.`)) setForm({ username: "", name: "", password: "", admin: false });
  };
  return (
    <section className="yts-accounts">
      <h2>QA accounts</h2>
      <p className="yts-muted">Each QA signs in to Research YTS with their own username and password. Deactivating someone signs them out and keeps everything they reported.</p>
      <form className="yts-form" onSubmit={add}>
        <h3>Add a QA</h3>
        <div className="yts-grid">
          <label className="yts-field"><span>Name</span><input required minLength={2} maxLength={60} value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} placeholder="e.g. Priya Shah" /></label>
          <label className="yts-field"><span>Username</span><input required minLength={2} maxLength={40} pattern="[A-Za-z0-9][A-Za-z0-9._\-]{1,39}" value={form.username} onChange={e => setForm({ ...form, username: e.target.value })} placeholder="e.g. priya" autoCapitalize="none" spellCheck={false} /></label>
          <label className="yts-field"><span>Password (1 or more characters)</span><input required minLength={1} maxLength={128} value={form.password} onChange={e => setForm({ ...form, password: e.target.value })} autoComplete="new-password" /></label>
        </div>
        <label className="yts-check"><input type="checkbox" checked={form.admin} onChange={e => setForm({ ...form, admin: e.target.checked })} /> Make this person a Research YTS admin (sees every QA's issues and manages accounts)</label>
        <p><button className="yts-primary" type="submit" disabled={busy}>Add the account</button></p>
      </form>
      {error && <Problem error={error} />}
      {notice && <p className="yts-ok" role="status">{notice}</p>}
      {list.error ? <Problem error={list.error} retry={list.reload} /> : !list.data ? <p className="yts-muted">Loading accounts…</p> : (
        <ul className="yts-list">
          {list.data.map(a => (
            <li key={a.id} className={`yts-acct ${a.deactivated_at ? "off" : ""}`}>
              <div>
                <b>{a.name}</b> <span className="yts-muted">({a.username})</span>{a.admin && <span className="yts-badge st-admin">Admin</span>}{a.deactivated_at && <span className="yts-badge st-not_issue">Deactivated</span>}
                <div className="yts-meta">{a.issues} {a.issues === 1 ? "issue" : "issues"} reported · {a.open} open · {a.last_login_at ? `last signed in ${fmtStamp(a.last_login_at)}` : "never signed in"}</div>
                {a.issues > 0 && <a className="yts-link" href={hashWith({ view: undefined, reporter: String(a.id), c: undefined, s: undefined, issue: undefined, status: "all" })}>See their issues</a>}
              </div>
              <div className="yts-acct-actions">
                {resetting === a.id ? (
                  <form className="yts-inline-row" onSubmit={async e => { e.preventDefault(); if (await act(() => yts.resetPassword(a.id, newPassword), `New password set for ${a.name}. They are signed out until they use it.`)) { setResetting(null); setNewPassword(""); } }}>
                    <input required minLength={1} maxLength={128} value={newPassword} onChange={e => setNewPassword(e.target.value)} placeholder="New password" aria-label={`New password for ${a.name}`} autoComplete="new-password" />
                    <button className="yts-primary" type="submit" disabled={busy}>Set</button>
                    <button type="button" className="yts-link" onClick={() => { setResetting(null); setNewPassword(""); }}>Cancel</button>
                  </form>
                ) : (
                  <>
                    <button type="button" className="yts-link" onClick={() => { setResetting(a.id); setNewPassword(""); }}>Reset password</button>
                    {a.id !== me.id && (a.deactivated_at
                      ? <button type="button" className="yts-link" disabled={busy} onClick={() => void act(() => yts.setActive(a.id, true), `${a.name} can sign in again.`)}>Reactivate</button>
                      : <button type="button" className="yts-link danger" disabled={busy} onClick={() => void act(() => yts.setActive(a.id, false), `${a.name} is deactivated and signed out. Their issues are kept.`)}>Deactivate</button>)}
                  </>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

const open = (c: Record<Status, number>) => c.open + c.in_progress;
function Totals({ s }: { s: Summary }) {
  return (
    <dl className="lstrip yts-totals">
      {(Object.keys(STATUSES) as Status[]).map(k => <div key={k}><dt>{STATUSES[k]}</dt><dd>{s.totals[k]}</dd></div>)}
    </dl>
  );
}
function Problem({ error, retry }: { error: string; retry?: () => void }) {
  return <p className="yts-problem" role="alert">{error} {retry && <button type="button" className="yts-link" onClick={retry}>Try again</button>}</p>;
}
const StatusBadge = ({ s }: { s: Status }) => <span className={`yts-badge st-${s}`}>{STATUSES[s]}</span>;

/* ---------------------------------------------------------------- list */

function IssueList({ p, qa, byBook, byId, summary, catalogue }: { p: URLSearchParams; qa: Qa; byBook: Record<string, YtCollection>; byId: Record<string, YtStrategy>; summary?: Summary; catalogue: YtStrategy[] }) {
  const c = p.get("c") || "", s = p.get("s") || "", reporter = qa.admin ? p.get("reporter") || "" : "", status = p.get("status") || ACTIVE, cat = p.get("cat") || "", q = p.get("q") || "";
  const page = Math.max(1, Number.parseInt(p.get("page") || "1", 10) || 1);
  const [text, setText] = useState(q);
  useEffect(() => setText(q), [q]);
  const list = useLoad(() => yts.issues({ reporter, collection: c, strategy: s, status: status === "all" ? undefined : status, category: cat, q, page, size: 20 }), [reporter, c, s, status, cat, q, page]);
  const strategies = useMemo(() => catalogue.filter(x => x.book === c), [catalogue, c]);
  const search = (e: FormEvent) => { e.preventDefault(); go({ q: text.trim() || undefined, page: undefined }); };
  return (
    <>
      <div className="yts-head">
        <h2>{c === GENERAL ? "General issues" : c ? byBook[c]?.name || c : "All collections"}{s && byId[s] ? <small> · {strategyLabel(byId[s])}</small> : null}</h2>
        <a className="yts-primary" href={hashWith({ new: "1", issue: undefined })}>Report an issue</a>
      </div>
      <div className="yts-tools">
        {c && c !== GENERAL && (
          <label className="yts-field"><span>Strategy</span>
            <select value={s} onChange={e => go({ s: e.target.value || undefined, page: undefined })}>
              <option value="">All strategies in this collection</option>
              {strategies.map(x => {
                const n = summary?.strategies[x.id];
                return <option key={x.id} value={x.id}>{strategyLabel(x)}{n && open(n) ? ` (${open(n)} open)` : ""}</option>;
              })}
            </select>
          </label>
        )}
        <label className="yts-field"><span>Type</span>
          <select value={cat} onChange={e => go({ cat: e.target.value || undefined, page: undefined })}>
            <option value="">All types</option>
            {(Object.keys(CATEGORIES) as Category[]).map(k => <option key={k} value={k}>{CATEGORIES[k]}</option>)}
          </select>
        </label>
        <form className="yts-search" onSubmit={search} role="search">
          <input type="search" value={text} onChange={e => setText(e.target.value)} placeholder="Search title, text, name, stock…" aria-label="Search issues" />
        </form>
      </div>
      {c && c !== GENERAL && <p className="yts-muted">General issues that name a strategy in this collection are listed here too.</p>}
      {reporter && <p className="yts-muted">Showing one QA's issues ({list.data?.items[0]?.reporter_name || "no issues with these filters"}). <a className="yts-link" href={hashWith({ reporter: undefined, page: undefined })}>Show every QA</a></p>}
      <div className="yts-chips" role="group" aria-label="Status">
        {STATUS_FILTERS.map(([v, label]) => (
          <button key={v} type="button" className={status === v ? "on" : ""} aria-pressed={status === v} onClick={() => go({ status: v === ACTIVE ? undefined : v, page: undefined })}>{label}</button>
        ))}
      </div>
      {list.error ? <Problem error={list.error} retry={list.reload} /> : !list.data ? <p className="yts-muted">Loading issues…</p> : (
        <>
          <p className="yts-muted">{list.data.total} {list.data.total === 1 ? "issue" : "issues"}</p>
          <ul className="yts-list">
            {list.data.items.map(i => <IssueRow key={i.id} i={i} byBook={byBook} byId={byId} />)}
          </ul>
          {!list.data.items.length && <div className="empty"><b>No issues here</b>{status === ACTIVE ? "Nothing open with these filters." : "Nothing matches these filters."}</div>}
          {list.data.pages > 1 && (
            <nav className="yts-pager" aria-label="Pages">
              <button type="button" disabled={page <= 1} onClick={() => go({ page: String(page - 1) })}>Previous</button>
              <span>Page {page} of {list.data.pages}</span>
              <button type="button" disabled={page >= list.data.pages} onClick={() => go({ page: String(page + 1) })}>Next</button>
            </nav>
          )}
        </>
      )}
    </>
  );
}
function IssueRow({ i, byBook, byId }: { i: Issue; byBook: Record<string, YtCollection>; byId: Record<string, YtStrategy> }) {
  return (
    <li className={`yts-row sev-${i.severity}`}>
      <a href={hashWith({ issue: String(i.id), new: undefined })}>
        <span className="yts-row-top"><StatusBadge s={i.status} /><span className="yts-cat">{CATEGORIES[i.category]}</span>{i.severity === "high" && <span className="yts-badge sev">High</span>}<span className="yts-id">#{i.id}</span></span>
        <b className="yts-title">{i.title}</b>
        <span className="yts-meta">{scopeText(i, byBook, byId)}{i.instrument ? ` · ${i.instrument}` : ""}</span>
        <span className="yts-meta">{i.reporter_name} · {fmtDate(i.created_at)}{i.comments ? ` · ${i.comments} ${i.comments === 1 ? "comment" : "comments"}` : ""}{i.files ? ` · ${i.files} ${i.files === 1 ? "file" : "files"}` : ""}</span>
      </a>
    </li>
  );
}

/** What an issue is about, in one line: "Collection · Strategy", "Collection · Whole collection" or "General issue · 3 strategies". */
function scopeText(i: Issue, byBook: Record<string, YtCollection>, byId: Record<string, YtStrategy>) {
  if (i.collection === GENERAL) {
    if (!i.strategies.length) return "General issue · all YT strategies";
    if (i.strategies.length <= 2) return `General issue · ${i.strategies.map(id => byId[id] ? strategyLabel(byId[id]) : id).join(", ")}`;
    return `General issue · ${i.strategies.length} strategies in ${i.collections.length} ${i.collections.length === 1 ? "collection" : "collections"}`;
  }
  return `${byBook[i.collection]?.name || i.collection} · ${i.strategy_id ? (byId[i.strategy_id] ? strategyLabel(byId[i.strategy_id]) : i.strategy_id) : "Whole collection"}`;
}

/* ---------------------------------------------------------------- strategy picker (general issues) */

type Picked = Map<string, string>; // strategy id -> collection
const pickedList = (m: Picked) => [...m].map(([id, collection]) => ({ id, collection }));

function StrategyPicker({ value, onChange, collections, catalogue }: { value: Picked; onChange: (m: Picked) => void; collections: YtCollection[]; catalogue: YtStrategy[] }) {
  const [book, setBook] = useState(""), [text, setText] = useState("");
  const byBook = useMemo(() => Object.fromEntries(collections.map(c => [c.book, c.name])), [collections]);
  const shown = useMemo(() => {
    const q = text.trim().toLowerCase();
    return catalogue.filter(x => byBook[x.book] && (!book || x.book === book) && (!q || strategyLabel(x).toLowerCase().includes(q) || x.id.toLowerCase().includes(q)));
  }, [catalogue, byBook, book, text]);
  const toggle = (x: YtStrategy, on: boolean) => { const m = new Map(value); if (on) m.set(x.id, x.book); else m.delete(x.id); onChange(m); };
  const all = (on: boolean) => { const m = new Map(value); for (const x of shown) if (on) m.set(x.id, x.book); else m.delete(x.id); onChange(m); };
  const chosen = catalogue.filter(x => value.has(x.id));
  return (
    <fieldset className="yts-picker">
      <legend>Strategies this issue is about</legend>
      <p className="yts-muted">Tick every strategy it affects, from any collection. Leave it empty if it affects all YT strategies.</p>
      <div className="yts-tools">
        <label className="yts-field"><span>Collection</span>
          <select value={book} onChange={e => setBook(e.target.value)}>
            <option value="">All collections</option>
            {collections.map(c => <option key={c.book} value={c.book}>{c.name}</option>)}
          </select>
        </label>
        <label className="yts-field"><span>Find</span><input type="search" value={text} onChange={e => setText(e.target.value)} placeholder="Strategy name" aria-label="Find a strategy" /></label>
        <span className="yts-picker-all">
          <button type="button" className="yts-link" onClick={() => all(true)} disabled={!shown.length}>Tick all {shown.length} shown</button>
          <button type="button" className="yts-link" onClick={() => all(false)} disabled={!shown.some(x => value.has(x.id))}>Untick shown</button>
        </span>
      </div>
      <ul className="yts-picker-list" aria-label="Strategies">
        {shown.map(x => (
          <li key={x.id}><label className="yts-check"><input type="checkbox" checked={value.has(x.id)} onChange={e => toggle(x, e.target.checked)} /> {strategyLabel(x)}{!book && <span className="yts-muted"> · {byBook[x.book]}</span>}</label></li>
        ))}
        {!shown.length && <li className="yts-muted">No strategy matches.</li>}
      </ul>
      <p className="yts-muted" role="status">{chosen.length ? `${chosen.length} ${chosen.length === 1 ? "strategy" : "strategies"} chosen` : "None chosen: the issue is about all YT strategies."}</p>
      {chosen.length > 0 && (
        <ul className="yts-picked">
          {chosen.map(x => <li key={x.id}>{strategyLabel(x)}<button type="button" aria-label={`Remove ${strategyLabel(x)}`} onClick={() => toggle(x, false)}>×</button></li>)}
        </ul>
      )}
    </fieldset>
  );
}

/* ---------------------------------------------------------------- photos and files */

const filesOf = (list: FileList | File[] | null | undefined) => [...(list || [])];
/** Files pasted into the page (a screenshot copied to the clipboard arrives as an image file). */
const pastedFiles = (e: ClipboardEvent) => filesOf(e.clipboardData?.files).map((f, n) =>
  f.name && f.name !== "image.png" ? f : new File([f], `pasted-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-")}${n ? `-${n}` : ""}.${(f.type.split("/")[1] || "png").replace("jpeg", "jpg")}`, { type: f.type }));
const tooBig = (files: File[]) => files.filter(f => f.size > MAX_FILE_BYTES).map(f => f.name);

function FilePicker({ files, onChange }: { files: File[]; onChange: (f: File[]) => void }) {
  const input = useRef<HTMLInputElement>(null);
  const [error, setError] = useState("");
  const add = (more: File[]) => {
    const big = tooBig(more);
    setError(big.length ? `Too large (over ${fmtSize(MAX_FILE_BYTES)}): ${big.join(", ")}` : "");
    onChange([...files, ...more.filter(f => f.size <= MAX_FILE_BYTES && f.size > 0)]);
  };
  return (
    <div className="yts-files">
      <span className="yts-files-head"><b>Photos and files (optional)</b>
        <button type="button" className="yts-link" onClick={() => input.current?.click()}>Choose files</button>
      </span>
      <input ref={input} type="file" multiple hidden onChange={e => { add(filesOf(e.target.files)); e.target.value = ""; }} />
      <p className="yts-muted">Screenshots, the team's trade sheet, AFL files and so on, up to {fmtSize(MAX_FILE_BYTES)} each. You can also paste a screenshot (Ctrl+V) anywhere in this form.</p>
      {files.length > 0 && (
        <ul className="yts-file-list">
          {files.map((f, n) => <li key={`${f.name}-${n}`}><span>{f.name}</span><span className="yts-muted">{fmtSize(f.size)}</span><button type="button" className="yts-link danger" onClick={() => onChange(files.filter((_, k) => k !== n))}>Remove</button></li>)}
        </ul>
      )}
      {error && <Problem error={error} />}
    </div>
  );
}

/** Uploads files one at a time; returns the names that failed with the reason. */
async function uploadAll(id: number, files: File[]) {
  const failed: string[] = [];
  for (const f of files) {
    try { await yts.upload(id, f); } catch (err) { failed.push(`${f.name} (${(err as Error).message})`); }
  }
  return failed;
}

function Attachments({ issue, files, changed }: { issue: Issue; files: IssueFile[]; changed: () => void }) {
  const input = useRef<HTMLInputElement>(null);
  const [error, setError] = useState(""), [busy, setBusy] = useState("");
  const add = async (list: File[]) => {
    if (!list.length) return;
    const big = tooBig(list), ok = list.filter(f => f.size <= MAX_FILE_BYTES && f.size > 0);
    setError(""); setBusy(`Adding ${ok.length} ${ok.length === 1 ? "file" : "files"}…`);
    const failed = [...big.map(n => `${n} (over ${fmtSize(MAX_FILE_BYTES)})`), ...await uploadAll(issue.id, ok)];
    setBusy(""); if (failed.length) setError(`Not added: ${failed.join("; ")}`); changed();
  };
  const remove = async (f: IssueFile) => {
    setError(""); setBusy(`Removing ${f.name}…`);
    try { await yts.removeFile(issue.id, f.id); changed(); } catch (err) { setError((err as Error).message); } finally { setBusy(""); }
  };
  const images = files.filter(f => f.image), others = files.filter(f => !f.image);
  const canAdd = issue.can_edit;
  return (
    <section className="yts-attach" onPaste={canAdd ? e => { const f = pastedFiles(e); if (f.length) { e.preventDefault(); void add(f); } } : undefined}>
      <h3>Photos and files{files.length ? ` (${files.length})` : ""}</h3>
      {images.length > 0 && (
        <ul className="yts-thumbs">
          {images.map(f => (
            <li key={f.id}>
              <a href={fileUrl(issue.id, f.id)} target="_blank" rel="noopener"><img src={fileUrl(issue.id, f.id)} alt={f.name} loading="lazy" /></a>
              <span className="yts-file-name" title={f.name}>{f.name}</span>
              <span className="yts-muted">{fmtSize(f.size)} · {f.uploader_name}</span>
              <span className="yts-actions"><a className="yts-link" href={fileUrl(issue.id, f.id, true)} download={f.name}>Download</a>{f.can_delete && <button type="button" className="yts-link danger" disabled={!!busy} onClick={() => void remove(f)}>Remove</button>}</span>
            </li>
          ))}
        </ul>
      )}
      {others.length > 0 && (
        <ul className="yts-file-list">
          {others.map(f => (
            <li key={f.id}><a className="yts-link" href={fileUrl(issue.id, f.id, true)} download={f.name}>{f.name}</a><span className="yts-muted">{fmtSize(f.size)} · {f.uploader_name}, {fmtStamp(f.created_at)}</span>
              {f.can_delete && <button type="button" className="yts-link danger" disabled={!!busy} onClick={() => void remove(f)}>Remove</button>}</li>
          ))}
        </ul>
      )}
      {!files.length && <p className="yts-muted">No photos or files yet.</p>}
      {canAdd && (
        <p className="yts-actions">
          <button type="button" className="yts-link" disabled={!!busy} onClick={() => input.current?.click()}>Add photos or files</button>
          <span className="yts-muted">or paste a screenshot here (Ctrl+V). Up to {fmtSize(MAX_FILE_BYTES)} each.</span>
          <input ref={input} type="file" multiple hidden onChange={e => { void add(filesOf(e.target.files)); e.target.value = ""; }} />
        </p>
      )}
      {busy && <p className="yts-muted" role="status">{busy}</p>}
      {error && <Problem error={error} />}
    </section>
  );
}

/* ---------------------------------------------------------------- new issue */

function NewIssue({ p, qa, collections, catalogue, done }: { p: URLSearchParams; qa: Qa; collections: YtCollection[]; catalogue: YtStrategy[]; done: (id: number) => void }) {
  const [general, setGeneral] = useState(p.get("c") === GENERAL);
  const [collection, setCollection] = useState(p.get("c") === GENERAL ? "" : p.get("c") || "");
  const [strategy, setStrategy] = useState(p.get("s") || "");
  const [picked, setPicked] = useState<Picked>(new Map());
  const [files, setFiles] = useState<File[]>([]);
  const [saved, setSaved] = useState<{ id: number; failed: string[] } | null>(null);
  const [category, setCategory] = useState<Category | "">("");
  const [severity, setSeverity] = useState<Severity>("normal");
  const [title, setTitle] = useState(""), [body, setBody] = useState("");
  const [instrument, setInstrument] = useState(p.get("tk") || ""), [from, setFrom] = useState(p.get("from") || ""), [to, setTo] = useState(p.get("to") || "");
  const link = p.get("link") || "";
  const [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const strategies = useMemo(() => catalogue.filter(x => x.book === collection), [catalogue, collection]);
  useEffect(() => { if (strategy && catalogue.length && !strategies.some(x => x.id === strategy)) setStrategy(""); }, [strategies, strategy, catalogue.length]);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!category) { setError("Choose what kind of problem it is."); return; }
    setBusy(true); setError("");
    try {
      const made = await yts.create(general
        ? { collection: GENERAL, strategy_id: "", strategies: pickedList(picked), category, severity, title, body, instrument, period_from: from, period_to: to, page_link: link }
        : { collection, strategy_id: strategy, category, severity, title, body, instrument, period_from: from, period_to: to, page_link: link });
      const failed = await uploadAll(made.id, files);
      if (failed.length) { setSaved({ id: made.id, failed }); setBusy(false); return; }
      done(made.id);
    } catch (err) { setError((err as Error).message); setBusy(false); }
  };
  if (saved) return (
    <div className="yts-form">
      <h2>Issue #{saved.id} is saved</h2>
      <Problem error={`These files could not be added: ${saved.failed.join("; ")}. You can add them again on the issue page.`} />
      <button type="button" className="yts-primary" onClick={() => done(saved.id)}>Open the issue</button>
    </div>
  );
  return (
    <form className="yts-form" onSubmit={submit} onPaste={e => { const f = pastedFiles(e); if (f.length) { e.preventDefault(); setFiles(list => [...list, ...f.filter(x => x.size <= MAX_FILE_BYTES)]); } }}>
      <div className="yts-head"><h2>Report an issue</h2><a className="yts-link" href={hashWith({ new: undefined, tk: undefined, from: undefined, to: undefined, link: undefined })}>Cancel</a></div>
      <div className="yts-chips" role="radiogroup" aria-label="What the issue is about">
        <button type="button" role="radio" aria-checked={!general} className={!general ? "on" : ""} onClick={() => setGeneral(false)}>One collection or strategy</button>
        <button type="button" role="radio" aria-checked={general} className={general ? "on" : ""} onClick={() => setGeneral(true)}>General issue (several strategies)</button>
      </div>
      {general && <StrategyPicker value={picked} onChange={setPicked} collections={collections} catalogue={catalogue} />}
      <div className="yts-grid">
        {!general && <><label className="yts-field"><span>Collection</span>
          <select required value={collection} onChange={e => { setCollection(e.target.value); setStrategy(""); }}>
            <option value="">Choose a collection</option>
            {collections.map(c => <option key={c.book} value={c.book}>{c.name}</option>)}
          </select>
        </label>
        <label className="yts-field"><span>Strategy</span>
          <select value={strategy} onChange={e => setStrategy(e.target.value)} disabled={!collection}>
            <option value="">Whole collection</option>
            {strategies.map(x => <option key={x.id} value={x.id}>{strategyLabel(x)}</option>)}
          </select>
        </label></>}
        <label className="yts-field"><span>Type of problem</span>
          <select required value={category} onChange={e => setCategory(e.target.value as Category)}>
            <option value="">Choose a type</option>
            {(Object.keys(CATEGORIES) as Category[]).map(k => <option key={k} value={k}>{CATEGORIES[k]}</option>)}
          </select>
        </label>
        <label className="yts-field"><span>How serious</span>
          <select value={severity} onChange={e => setSeverity(e.target.value as Severity)}>
            {(Object.keys(SEVERITIES) as Severity[]).map(k => <option key={k} value={k}>{SEVERITIES[k]}</option>)}
          </select>
        </label>
      </div>
      <label className="yts-field"><span>Title</span><input required minLength={3} maxLength={160} value={title} onChange={e => setTitle(e.target.value)} placeholder="e.g. Exit on 14-03-2019 should be at the close, not the open" /></label>
      <label className="yts-field"><span>What is wrong, and how to see it</span>
        <textarea required rows={7} maxLength={10000} value={body} onChange={e => setBody(e.target.value)} placeholder={"What you expected, what the page shows, and the steps to get there.\nPaste trade dates, prices or the team's trade-sheet row if you have them."} />
      </label>
      <div className="yts-grid">
        <label className="yts-field"><span>Instrument (optional)</span><input maxLength={80} value={instrument} onChange={e => setInstrument(e.target.value)} placeholder="e.g. NIFTY 50 or RELIANCE" /></label>
        <label className="yts-field"><span>Period from (optional)</span><input type="date" value={from} onChange={e => setFrom(e.target.value)} /></label>
        <label className="yts-field"><span>Period to (optional)</span><input type="date" value={to} onChange={e => setTo(e.target.value)} /></label>
      </div>
      <FilePicker files={files} onChange={setFiles} />
      {link && <p className="yts-muted">The page you came from is attached: <a href={link}>open it</a>.</p>}
      <p className="yts-muted">Reported by <b>{qa.name}</b>.</p>
      {error && <Problem error={error} />}
      <button className="yts-primary" type="submit" disabled={busy}>{busy ? (files.length ? "Saving and adding files…" : "Saving…") : "Report the issue"}</button>
    </form>
  );
}

/* ---------------------------------------------------------------- one issue */

/** The strategy's own page on YT Strategies. */
const strategyPage = (id: string) => `#/s/${encodeURIComponent(id)}`;

const ACTION: Record<IssueEvent["action"], string> = { created: "reported the issue", edited: "edited the issue", status: "changed the status to", comment: "commented", deleted: "deleted the issue", file: "changed the files" };

function IssueView({ id, byBook, byId, collections, catalogue, changed }: { id: number; byBook: Record<string, YtCollection>; byId: Record<string, YtStrategy>; collections: YtCollection[]; catalogue: YtStrategy[]; changed: () => void }) {
  const d = useLoad(() => yts.issue(id), [id]);
  const [comment, setComment] = useState(""), [note, setNote] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const [editing, setEditing] = useState(false), [deleting, setDeleting] = useState(false), [reason, setReason] = useState("");
  const act = async (work: () => Promise<unknown>, after?: () => void) => {
    setBusy(true); setError("");
    try { await work(); after?.(); d.reload(); changed(); } catch (err) { setError((err as Error).message); } finally { setBusy(false); }
  };
  if (d.error) return <Problem error={d.error} retry={d.reload} />;
  if (!d.data) return <p className="yts-muted">Loading the issue…</p>;
  const { issue: i, events, files } = d.data, st = byId[i.strategy_id], general = i.collection === GENERAL;
  return (
    <article className="yts-one">
      <p><a className="yts-link" href={hashWith({ issue: undefined })}>← Back to the list</a></p>
      <div className="yts-row-top"><StatusBadge s={i.status} /><span className="yts-cat">{CATEGORIES[i.category]}</span><span className={`yts-badge sev-${i.severity}`}>{SEVERITIES[i.severity]}</span><span className="yts-id">#{i.id}</span></div>
      {editing ? <EditIssue i={i} collections={collections} catalogue={catalogue} done={() => { setEditing(false); d.reload(); changed(); }} cancel={() => setEditing(false)} /> : (
        <>
          <h2 className="yts-one-title">{i.title}</h2>
          <dl className="yts-facts">
            {general ? <div><dt>Applies to</dt><dd>General issue: {i.strategies.length ? `${i.strategies.length} ${i.strategies.length === 1 ? "strategy" : "strategies"}` : "all YT strategies"}</dd></div> : <>
              <div><dt>Collection</dt><dd>{byBook[i.collection]?.name || i.collection}</dd></div>
              <div><dt>Strategy</dt><dd>{i.strategy_id ? (st ? strategyLabel(st) : i.strategy_id) : "Whole collection"}</dd></div>
            </>}
            {i.instrument && <div><dt>Instrument</dt><dd>{i.instrument}</dd></div>}
            {(i.period_from || i.period_to) && <div><dt>Period</dt><dd>{i.period_from ? fmtDate(i.period_from) : "start"} to {i.period_to ? fmtDate(i.period_to) : "end"}</dd></div>}
            <div><dt>Reported by</dt><dd>{i.reporter_name}, {fmtStamp(i.created_at)}</dd></div>
          </dl>
          {general && i.strategies.length > 0 && (
            <div className="yts-general-list">
              {i.collections.map(book => (
                <div key={book}><b>{byBook[book]?.name || book}</b>
                  <ul>{i.strategies.filter(sid => (byId[sid]?.book ?? book) === book).map(sid => <li key={sid}><a href={strategyPage(sid)}>{byId[sid] ? strategyLabel(byId[sid]) : sid}</a></li>)}</ul>
                </div>
              ))}
            </div>
          )}
          {(i.strategy_id || i.page_link) && (
            <p className="yts-links">
              {/* The saved page opens the strategy with the instrument, dates and settings of the report; without one, the strategy's own page. */}
              {i.page_link ? <a href={i.page_link}>Open the page with issue parameters →</a> : <a href={strategyPage(i.strategy_id)}>Open the strategy →</a>}
            </p>
          )}
          <div className="yts-body">{i.body}</div>
          {i.deleted_at && <p className="yts-problem">Deleted {fmtStamp(i.deleted_at)}: {i.deleted_reason}</p>}
          {(i.can_edit || i.can_delete) && (
            <p className="yts-actions">
              {i.can_edit && <button type="button" className="yts-link" onClick={() => setEditing(true)}>Edit</button>}
              {i.can_delete && <button type="button" className="yts-link danger" onClick={() => setDeleting(!deleting)}>Delete</button>}
            </p>
          )}
          {deleting && (
            <form className="yts-inline" onSubmit={e => { e.preventDefault(); void act(() => yts.remove(i.id, reason), () => { setDeleting(false); setReason(""); }); }}>
              <label className="yts-field"><span>Why delete it? (kept with the issue)</span><input required minLength={3} maxLength={500} value={reason} onChange={e => setReason(e.target.value)} placeholder="e.g. Reported twice, see #12" /></label>
              <button className="yts-primary danger" type="submit" disabled={busy}>Delete the issue</button>
            </form>
          )}
        </>
      )}
      <Attachments issue={i} files={files} changed={() => { d.reload(); changed(); }} />
      {!i.deleted_at && (
        <section className="yts-status">
          <h3>Status</h3>
          <div className="yts-chips" role="group" aria-label="Change status">
            {(Object.keys(STATUSES) as Status[]).map(k => (
              <button key={k} type="button" className={i.status === k ? "on" : ""} aria-pressed={i.status === k} disabled={busy || i.status === k}
                onClick={() => void act(() => yts.setStatus(i.id, k, note), () => setNote(""))}>{STATUSES[k]}</button>
            ))}
          </div>
          <input className="yts-note" value={note} onChange={e => setNote(e.target.value)} maxLength={4000} placeholder="Optional note with the status change, e.g. fixed in the 13 Nov build" aria-label="Note with the status change" />
        </section>
      )}
      {error && <Problem error={error} />}
      <section className="yts-history">
        <h3>History and comments</h3>
        <ol>
          {events.map(e => (
            <li key={e.id} className={`ev-${e.action}`}>
              <span className="yts-meta"><b>{e.actor_name}</b> {ACTION[e.action]}{e.action === "status" && e.status ? <> <StatusBadge s={e.status} /></> : null} · {fmtStamp(e.created_at)}</span>
              {e.body && e.action !== "created" && <div className="yts-body">{e.body}</div>}
            </li>
          ))}
        </ol>
        {!i.deleted_at && (
          <form className="yts-inline" onSubmit={e => { e.preventDefault(); void act(() => yts.comment(i.id, comment), () => setComment("")); }}>
            <label className="yts-field"><span>Add a comment</span><textarea rows={3} required maxLength={4000} value={comment} onChange={e => setComment(e.target.value)} placeholder="More detail, a reply, or what you checked after the fix" /></label>
            <button className="yts-primary" type="submit" disabled={busy}>Comment</button>
          </form>
        )}
      </section>
    </article>
  );
}

function EditIssue({ i, collections, catalogue, done, cancel }: { i: Issue; collections: YtCollection[]; catalogue: YtStrategy[]; done: () => void; cancel: () => void }) {
  const general = i.collection === GENERAL;
  const start = useMemo(() => new Map(i.strategies.map(id => [id, catalogue.find(x => x.id === id)?.book || i.collections[0] || ""])), [i, catalogue]);
  const [picked, setPicked] = useState<Picked>(start);
  const pickedChanged = general && (picked.size !== start.size || [...picked.keys()].some(id => !start.has(id)));
  const [title, setTitle] = useState(i.title), [body, setBody] = useState(i.body), [category, setCategory] = useState<Category>(i.category);
  const [severity, setSeverity] = useState<Severity>(i.severity), [instrument, setInstrument] = useState(i.instrument);
  const [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    const changes = {
      ...(title !== i.title ? { title } : {}), ...(body !== i.body ? { body } : {}), ...(category !== i.category ? { category } : {}),
      ...(severity !== i.severity ? { severity } : {}), ...(instrument !== i.instrument ? { instrument } : {}),
      ...(pickedChanged ? { strategies: pickedList(picked).filter(x => x.collection) } : {}),
    };
    if (!Object.keys(changes).length) { cancel(); return; }
    try { await yts.edit(i.id, changes); done(); } catch (err) { setError((err as Error).message); setBusy(false); }
  };
  return (
    <form className="yts-form" onSubmit={submit}>
      <label className="yts-field"><span>Title</span><input required minLength={3} maxLength={160} value={title} onChange={e => setTitle(e.target.value)} /></label>
      <div className="yts-grid">
        <label className="yts-field"><span>Type of problem</span>
          <select value={category} onChange={e => setCategory(e.target.value as Category)}>{(Object.keys(CATEGORIES) as Category[]).map(k => <option key={k} value={k}>{CATEGORIES[k]}</option>)}</select>
        </label>
        <label className="yts-field"><span>How serious</span>
          <select value={severity} onChange={e => setSeverity(e.target.value as Severity)}>{(Object.keys(SEVERITIES) as Severity[]).map(k => <option key={k} value={k}>{SEVERITIES[k]}</option>)}</select>
        </label>
        <label className="yts-field"><span>Instrument</span><input maxLength={80} value={instrument} onChange={e => setInstrument(e.target.value)} /></label>
      </div>
      <label className="yts-field"><span>What is wrong, and how to see it</span><textarea required rows={7} maxLength={10000} value={body} onChange={e => setBody(e.target.value)} /></label>
      {general && <StrategyPicker value={picked} onChange={setPicked} collections={collections} catalogue={catalogue} />}
      {error && <Problem error={error} />}
      <p className="yts-actions"><button className="yts-primary" type="submit" disabled={busy}>{busy ? "Saving…" : "Save changes"}</button><button type="button" className="yts-link" onClick={cancel}>Cancel</button></p>
    </form>
  );
}
