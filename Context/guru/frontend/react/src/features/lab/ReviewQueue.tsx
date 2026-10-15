import { useEffect, useState } from "react";
import { strategies, type StrategiesApi } from "./client";
import { ErrorNotice, Loading } from "./ui";
import type { LabStrategy } from "./types";
import "./saved.css";

export function ReviewQueue({ api = strategies, onOpen }: { api?: Pick<StrategiesApi, "review" | "action">; onOpen(row: LabStrategy): void }) {
  const [rows, setRows] = useState<LabStrategy[] | null>(null);
  const [listError, setListError] = useState<unknown>(null);
  const [notes, setNotes] = useState<Record<number, string>>({});
  const [errors, setErrors] = useState<Record<number, unknown>>({});
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    api.review().then((r) => live && setRows(r), (e: unknown) => live && setListError(e));
    return () => { live = false; };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps -- load once on mount

  async function decide(row: LabStrategy, action: "publish" | "reject", note: string | undefined) {
    setBusy(true);
    setErrors((e) => ({ ...e, [row.id]: undefined }));
    try {
      await api.action(row.id, action, row.version, note);
      setRows((rs) => (rs ?? []).filter((r) => r.id !== row.id));
    } catch (e) {
      setErrors((all) => ({ ...all, [row.id]: e }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="box" data-lab-review>
      <h2 className="box-h">Review queue</h2>
      {listError !== null ? <ErrorNotice error={listError} market="in" />
        : rows === null ? <Loading text="Loading the review queue…" />
        : rows.length === 0 ? <div className="empty">No strategies are waiting for review.</div>
        : (
          <ul className="lab-saved-list">
            {rows.map((row) => {
              const note = (notes[row.id] ?? "").trim();
              return (
                <li key={row.id} className="lab-saved-item">
                  <div className="lab-saved-row">
                    <b>{row.name}</b>
                    <span className="lab-saved-meta">
                      {row.ownerName} · {row.market} · {row.instrument} · submitted {row.submittedAt?.slice(0, 10) ?? "—"}
                    </span>
                    <span className="lab-saved-actions">
                      <button type="button" className="dbtn sm" onClick={() => onOpen(row)}>Open in lab</button>
                    </span>
                  </div>
                  <div className="lab-saved-row">
                    <textarea className="lab-review-note" aria-label="Review note" maxLength={1000} rows={2}
                      value={notes[row.id] ?? ""} onChange={(e) => setNotes((n) => ({ ...n, [row.id]: e.target.value }))} />
                    <button type="button" className="dbtn sm primary" disabled={busy} onClick={() => decide(row, "publish", note || undefined)}>Publish</button>
                    <button type="button" className="dbtn sm ghost" disabled={busy || note.length < 1 || note.length > 1000}
                      onClick={() => decide(row, "reject", note)}>Reject</button>
                  </div>
                  {errors[row.id] !== undefined && <ErrorNotice error={errors[row.id]} market={row.market} />}
                </li>
              );
            })}
          </ul>
        )}
    </section>
  );
}
