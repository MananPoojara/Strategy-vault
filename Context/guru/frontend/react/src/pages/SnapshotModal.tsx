import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { claudeWorkDaily, type DailyWork, type DailyWorkRow, type SnapEvent, type SnapItem, type SnapStats } from "../services/research";

/* Popup for a "Live snapshot" row on the Research timeline: the numbers, then names in collapsible lists. Real data only:
   anything the generator did not record shows "not recorded". */

const num = (n: number | undefined | null) => (n ?? 0).toLocaleString("en-IN");
const SEARCH_ABOVE = 30;

export const DAILY_HEADERS = ["Date", "New strategies added", "Strategies backtested (day)", "Time taken - backtesting", "Tokens used - backtesting", "Strategy errors solved", "Time taken - error fixing", "Tokens used - error fixing",
  "Newly held (with reason)", "Held balance (end of day)", "Backtested total (end of day)", "Books added", "Deploys", "Worked by", "Avg tokens per strategy", "Proof"];
const NUMERIC = new Set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14]);
export const fmtDate = (iso: string) => iso.split("-").reverse().join("/");
export const fmtTokens = (n: number | null) => n == null ? "—" : (n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${Math.round(n / 1e3)}k` : String(n));
export const fmtTime = (min: number | null) => min == null ? "—" : `${Math.floor(min / 60)}:${String(min % 60).padStart(2, "0")}`;

/** The daily table: one row per IST day, newest first; "Newly held" opens the list of strategies with their reasons. */
export function DailyWorkTable({ data }: { data: DailyWork }) {
  const [openDay, setOpenDay] = useState<string | null>(null);
  const cells = (r: DailyWorkRow): ReactNode[] => [fmtDate(r.date), num(r.new), num(r.backtested_day), fmtTime(r.bt_minutes), fmtTokens(r.bt_tokens), num(r.errors_solved), fmtTime(r.fix_minutes), fmtTokens(r.fix_tokens),
    r.newly_held == null ? <span className="rp-muted" title="The run file for this day is not kept">n/a</span>
      : r.newly_held_list.length ? <button type="button" className="rp-linkbtn" aria-expanded={openDay === r.date} onClick={() => setOpenDay(openDay === r.date ? null : r.date)}>{num(r.newly_held)}</button> : num(r.newly_held),
    num(r.held), num(r.backtested_total), num(r.books_added), num(r.deploys), r.worked_by, r.avg_tokens_per_strategy == null ? "" : fmtTokens(r.avg_tokens_per_strategy), r.proof];
  return (
    <div className="rp-daily">
      <div className="rp-daily-scroll">
        <table>
          <thead><tr>{DAILY_HEADERS.map((h, i) => <th key={h} className={NUMERIC.has(i) ? "n" : undefined}>{h}</th>)}</tr></thead>
          <tbody>{data.rows.map(r => (
            <Fragment key={r.date}><tr>{cells(r).map((c, i) => <td key={i} className={NUMERIC.has(i) ? "n" : undefined}>{c}</td>)}</tr>
              {openDay === r.date && <tr className="rp-daily-held"><td colSpan={DAILY_HEADERS.length}>
                <ul className="rp-snap-list">{r.newly_held_list.map((x, i) => <li key={i}><b>{x.strategy}</b> <span className="rp-muted">{x.book}</span> · {x.reason}</li>)}</ul></td></tr>}</Fragment>
          ))}</tbody>
        </table>
      </div>
      <p className="rp-small rp-muted">Time is active minutes in Claude Code sessions (gaps of 10 minutes or less between messages, IST days); tokens are input + output + cache-creation tokens of those sessions, sorted into backtesting and error-fixing lanes by project folder; a dash means no session of that kind that day (Codex-led work on 14-16 Nov is not in Claude's logs). Strategy errors solved = verifier tracker rows answered FIXED/Live. Other work (lead, frontend, covers) is in the data file only.</p>
    </div>
  );
}
function DailyWorkSection() {
  const [d, setD] = useState<DailyWork | null | undefined>(undefined);
  useEffect(() => { let on = true; claudeWorkDaily().then(x => on && setD(x)); return () => { on = false; }; }, []);
  return <section className="rp-snap-sec"><h3>Daily work</h3>{d === undefined ? <p className="rp-small rp-muted">Loading...</p> : d ? <DailyWorkTable data={d} /> : <p className="rp-small rp-muted">Daily work is not available (claude-work-daily.json missing).</p>}</section>;
}
export const STEPS = [
  "Read the rule as printed, at its page in the book",
  "Code the strategy card",
  "Independent pandas reference: 0 mismatches",
  "Full test suite",
  "Private engine run on the book's market and window",
  "Review",
  "Release refresh compared with the live site",
  "Deploy",
];

function Delta({ now, prev }: { now: number; prev?: number }) {
  if (prev == null || now === prev) return null;
  const d = now - prev;
  return <small className={d > 0 ? "up" : "down"}> {d > 0 ? "+" : "−"}{num(Math.abs(d))} vs previous</small>;
}

function Section({ title, count, children, note }: { title: string; count?: number | null; children: ReactNode; note?: string }) {
  const [open, setOpen] = useState(false);
  return (
    <details className="rp-snap-sec" open={open} onToggle={e => setOpen((e.currentTarget as HTMLDetailsElement).open)}>
      <summary>{title}{count != null && <b> {num(count)}</b>}{count == null && <span className="rp-muted"> not recorded</span>}</summary>
      {open && (count == null ? <p className="rp-small rp-muted">{note || "Names not recorded: the release run for this snapshot is not kept."}</p> : children)}
    </details>
  );
}

function Names({ items, extra }: { items: SnapItem[]; extra?: boolean }) {
  const [q, setQ] = useState("");
  const shown = useMemo(() => { const t = q.trim().toLowerCase(); return t ? items.filter(i => i.join(" ").toLowerCase().includes(t)) : items; }, [items, q]);
  if (!items.length) return <p className="rp-small rp-muted">None in this period.</p>;
  return (
    <>
      {items.length > SEARCH_ABOVE && <input type="search" aria-label="Search names" placeholder={`Search ${num(items.length)} names`} value={q} onChange={e => setQ(e.target.value)} />}
      <ul className="rp-snap-list">
        {shown.map((i, n) => (
          <li key={n}><b>{i[0]}</b> <span className="rp-muted">{[i[1], i[2]].filter(Boolean).join(" · ")}</span>
            {extra && i[4] && <span className="rp-small rp-muted"> · {i[4]}{i[5] ? ` · ${i[5]}` : ""}</span>}</li>
        ))}
      </ul>
      {!shown.length && <p className="rp-small rp-muted">No match.</p>}
    </>
  );
}

function Spark({ v }: { v: number[] }) {
  const max = Math.max(1, ...v), w = 220, h = 36;
  const pts = v.map((y, i) => `${v.length > 1 ? (i * w) / (v.length - 1) : w / 2},${h - (y / max) * (h - 4) - 2}`).join(" ");
  return <svg className="rp-spark" viewBox={`0 0 ${w} ${h}`} role="img" aria-label="Memory pressure over the period"><polyline points={pts} fill="none" stroke="currentColor" strokeWidth="1.6" /></svg>;
}

const FOCUSABLE = 'a[href],button:not([disabled]),input,summary,[tabindex]:not([tabindex="-1"])';

export function SnapshotModal({ ev, stats, at, healthStarts, onClose }: { ev: SnapEvent | null; stats: SnapStats | null; at: string; healthStarts?: string; onClose: () => void }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null, prev = document.body.style.overflow;
    document.body.style.overflow = "hidden"; // the page's Lenis scrolling pauses while the body is locked
    box.current?.querySelector<HTMLElement>("button")?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopPropagation(); onClose(); return; }
      if (e.key !== "Tab" || !box.current) return;
      const f = [...box.current.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(x => x.offsetParent !== null || x === document.activeElement);
      if (!f.length) return;
      const first = f[0], last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", key, true);
    return () => { document.removeEventListener("keydown", key, true); document.body.style.overflow = prev; opener?.focus?.(); };
  }, [onClose]);
  const prev = ev?.prev, log = ev?.log ?? [], h = ev?.health;
  const incidents = log.filter(l => l[1] === "incident");
  return (
    <div className="rp-modal-back" onMouseDown={e => e.target === e.currentTarget && onClose()}>
      <div className="rp-modal" role="dialog" aria-modal="true" aria-labelledby="rp-snap-t" ref={box}>
        <header><h2 id="rp-snap-t">Live snapshot · {ev?.at_ist ?? at} IST</h2><button type="button" onClick={onClose} aria-label="Close">Close</button></header>
        <div className="rp-snap-body" data-lenis-prevent>
          <DailyWorkSection />
          {stats ? (
            <p className="rp-nums">
              <span><b>{num(stats.backtested)}</b> backtested<Delta now={stats.backtested} prev={prev?.backtested} /></span>
              <span><b>{num(stats.held)}</b> held<Delta now={stats.held} prev={prev?.held} /></span>
              <span><b>{num(stats.total)}</b> strategies<Delta now={stats.total} prev={prev?.total} /></span>
              <span><b>{num(stats.books)}</b> books<Delta now={stats.books} prev={prev?.books} /></span>
            </p>
          ) : <p className="rp-muted">Numbers not recorded.</p>}
          {!ev && <p className="rp-small rp-muted">Detail for this snapshot is not available (research-snapshot-detail.json missing).</p>}
          {ev?.note && <p className="rp-small rp-muted">{ev.note}</p>}
          {ev && <>
            <Section title="Gained: first backtested in this period" count={ev.gained?.length ?? null}><Names items={ev.gained ?? []} /></Section>
            <Section title="Lost" count={ev.lost?.length ?? null}><Names items={ev.lost ?? []} /></Section>
            <Section title="Claude coded in this period" count={ev.claude?.length ?? null} note="Period start not recorded (no earlier snapshot)."><Names items={ev.claude ?? []} /></Section>
            <Section title="Verification errors fixed" count={ev.fixed.length}><Names items={ev.fixed} extra /></Section>
            <Section title="Claude work log" count={log.length}>
              {!log.length ? <p className="rp-small rp-muted">No HANDOFF lines in this period.</p> : (
                <ul className="rp-snap-list log">
                  {log.map((l, i) => (
                    <li key={i}><span className="rp-small rp-muted">{l[0].slice(5)} IST</span> <span className="rp-pill">{l[1]}</span> {l[2]}
                      {l[3].length > 0 && <span className="rp-small rp-muted"> · {l[3].join(", ")}</span>}</li>
                  ))}
                </ul>
              )}
            </Section>
            <Section title="Machine health" count={h ? h.n : 0}>
              {h ? (
                <>
                  <Spark v={h.psi10} />
                  <p className="rp-small">Peak memory pressure {h.peak.psi10} (10 s) / {h.peak.psi60} (60 s) · least memory free {num(h.peak.mem_available_mb_min)} MB · swap up to {num(h.peak.swap_mb_max)} MB · disk free down to {h.peak.disk_free_gb_min} GB · {h.peak.engines_max} engines · {h.peak.claude_procs_max} Claude processes</p>
                </>
              ) : (
                <>
                  <p className="rp-small rp-muted">Health samples start {healthStarts ?? "18 Nov 19:11"} IST; none cover this period.</p>
                  {incidents.length ? <ul className="rp-snap-list log">{incidents.map((l, i) => <li key={i}><span className="rp-small rp-muted">{l[0].slice(5)} IST</span> {l[2]}</li>)}</ul>
                    : <p className="rp-small rp-muted">No incident is named in HANDOFF for this period.</p>}
                </>
              )}
            </Section>
          </>}
          <section className="rp-snap-steps">
            <h3>How Claude backtests a strategy</h3>
            <ol>{STEPS.map(s => <li key={s}>{s}</li>)}</ol>
          </section>
        </div>
      </div>
    </div>
  );
}
