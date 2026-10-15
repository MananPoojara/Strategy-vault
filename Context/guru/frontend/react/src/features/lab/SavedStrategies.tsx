import { useEffect, useRef, useState } from "react";
import { strategies, type StrategiesApi } from "./client";
import { COPY, variantLabel } from "./labels";
import { ErrorNotice, Loading, isProvisional } from "./ui";
import type { LabStrategy, Role, SavedMarket, Spec, StrategyStatus } from "./types";
import "./saved.css";

export interface EditorState { spec: Spec; market: SavedMarket; ticker: string; row: LabStrategy | null; unchanged: boolean; ranFresh: boolean }
type Api = Pick<StrategiesApi, "list" | "get" | "create" | "update" | "remove" | "duplicate" | "action">;

export const STATUS_LABEL: Record<StrategyStatus, string> = { draft: "Draft", submitted: "Submitted", published: "Published", rejected: "Rejected" };
const editable = (s: StrategyStatus) => s === "draft" || s === "rejected";
const SAVE = -1; // error/notice key for the save row

export function SavedStrategies({ role, api = strategies, editor, methodNames = {}, prefill, onOpen, onSaved }: {
  role: Role | null; api?: Api; editor: EditorState; methodNames?: Record<string, string>;
  /** A new seq puts this name in the box and brings the save row into view (Customize step 4). */
  prefill?: { name: string; seq: number };
  onOpen(row: LabStrategy): void; onSaved(row: LabStrategy): void;
}) {
  const [rows, setRows] = useState<LabStrategy[] | null>(null);
  const [listError, setListError] = useState<unknown>(null);
  const [name, setName] = useState(editor.row?.name ?? "");
  const [errors, setErrors] = useState<Record<number, unknown>>({});
  const [notice, setNotice] = useState<{ id: number; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const canEdit = role === "admin" || role === "researcher";

  useEffect(() => {
    let live = true;
    api.list().then((r) => live && setRows(r), (e: unknown) => live && setListError(e));
    return () => { live = false; };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps -- load once on mount
  useEffect(() => setName(editor.row?.name ?? ""), [editor.row?.id]); // eslint-disable-line react-hooks/exhaustive-deps
  const box = useRef<HTMLElement>(null), nameInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (!prefill) return;
    setName(prefill.name);
    box.current?.scrollIntoView?.({ behavior: "smooth", block: "start" });
    nameInput.current?.focus({ preventScroll: true });
  }, [prefill?.seq]); // eslint-disable-line react-hooks/exhaustive-deps

  // The local row carries the latest version (after a reload or action) even before the page re-renders.
  const own = editor.row ? rows?.find((r) => r.id === editor.row!.id) : undefined;
  const put = (row: LabStrategy, top = false) => setRows((rs) => {
    const list = rs ?? [];
    return list.some((r) => r.id === row.id) ? list.map((r) => (r.id === row.id ? row : r)) : top ? [row, ...list] : [...list, row];
  });
  const touchesEditor = (row: LabStrategy) => { if (row.id === editor.row?.id) onSaved(row); };

  async function mutate(key: number, task: () => Promise<void>) {
    setBusy(true);
    setNotice(null);
    setErrors((e) => ({ ...e, [key]: undefined }));
    try { await task(); } catch (e) { setErrors((all) => ({ ...all, [key]: e })); } finally { setBusy(false); }
  }

  const trimmed = name.trim();
  const draft = () => ({ name: trimmed, spec: editor.spec, market: editor.market, instrument: editor.ticker });
  const saveNew = () => mutate(SAVE, async () => {
    const row = await api.create(draft());
    put(row, true); onSaved(row); setNotice({ id: SAVE, text: `Saved "${row.name}"` });
  });
  const saveChanges = (cur: LabStrategy) => mutate(SAVE, async () => {
    // The page may hold a newer version (e.g. it re-fetched the row itself); never send the older one.
    const row = await api.update(cur.id, Math.max(cur.version, editor.row?.version ?? 0), draft());
    put(row); onSaved(row); setNotice({ id: SAVE, text: `Saved "${row.name}"` });
  });
  const reload = (key: number, id: number) => mutate(key, async () => {
    const fresh = await api.get(id);
    put(fresh);
    // Keeps the spec on screen: the page only takes the new row (version), never its rules.
    touchesEditor(fresh);
    setNotice({ id: key, text: fresh.id === editor.row?.id ? COPY.keptAfterReload(fresh.version) : `Reloaded "${fresh.name}"` });
  });
  const transition = (row: LabStrategy, action: "submit" | "withdraw") => mutate(row.id, async () => {
    const next = await api.action(row.id, action, row.version);
    put(next); touchesEditor(next);
    if (action === "submit") setNotice({ id: row.id, text: COPY.submitted });
  });
  const duplicate = (row: LabStrategy) => mutate(row.id, async () => put(await api.duplicate(row.id), true));
  const remove = (row: LabStrategy) => {
    if (!window.confirm(COPY.deleteConfirm(row.name))) return;
    return mutate(row.id, async () => {
      await api.remove(row.id, row.version);
      setRows((rs) => (rs ?? []).filter((r) => r.id !== row.id));
    });
  };

  const feedback = (key: number, market: SavedMarket, instrument: string, reloadId?: number) => {
    const error = errors[key];
    return (
      <>
        {error !== undefined && (
          <div className="lab-saved-feedback">
            <ErrorNotice error={error} market={market} instrumentLabel={instrument} />
            {(error as { status?: number }).status === 409 && reloadId !== undefined && (
              <button type="button" className="dbtn sm" disabled={busy} onClick={() => reload(key, reloadId)}>Reload this strategy</button>
            )}
          </div>
        )}
        {notice?.id === key && <p className="lab-saved-note" role="status">{notice.text}</p>}
      </>
    );
  };

  const groups = new Map<string, LabStrategy[]>();
  for (const row of rows ?? []) {
    const head = row.origin ? `My variants of ${methodNames[row.origin.method] ?? row.origin.method}` : "Lab strategies";
    groups.set(head, [...(groups.get(head) ?? []), row]);
  }

  const item = (row: LabStrategy) => {
    const method = row.origin ? methodNames[row.origin.method] ?? row.origin.method : "";
    const open = editor.row?.id === row.id;
    const publishable = row.publishable && !isProvisional(row.market);
    const canSubmit = publishable && open && editor.unchanged && editor.ranFresh;
    // Visible (not only a tooltip) so keyboard and screen-reader users get the reason too.
    const reason = !publishable ? COPY.provisionalSubmit : canSubmit ? undefined : COPY.runFirst;
    const reasonId = `lab-submit-why-${row.id}`;
    const showReason = reason !== undefined && canEdit && editable(row.status) && (open || !publishable);
    return (
      <li key={row.id} className="lab-saved-item">
        <div className="lab-saved-row">
          <b>{row.name}</b>
          {row.origin && <span className="tag">{variantLabel(row.origin.book, method).split(" · user variant")[0]}</span>}
          <span className={`tag lab-status lab-status-${row.status}`}>{STATUS_LABEL[row.status]}</span>
          <span className="lab-saved-meta">
            {row.instrument} · {row.market} · {row.spec.entry?.all?.length ?? 0} entry / {row.spec.exit?.any?.length ?? 0} exit · updated {row.updatedAt.slice(0, 10)}
          </span>
          <span className="lab-saved-actions">
            <button type="button" className="dbtn sm" onClick={() => onOpen(row)}>Open</button>
            {canEdit && <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => duplicate(row)}>Duplicate</button>}
            {canEdit && editable(row.status) && (
              <>
                <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => remove(row)}>Delete</button>
                <button type="button" className="dbtn sm" disabled={busy || !canSubmit} onClick={() => transition(row, "submit")}
                  title={reason} aria-describedby={showReason ? reasonId : undefined}>Submit</button>
              </>
            )}
            {canEdit && row.status === "submitted" && (
              <button type="button" className="dbtn sm ghost" disabled={busy} onClick={() => transition(row, "withdraw")}>Withdraw</button>
            )}
          </span>
        </div>
        {showReason && <p className="lab-saved-note" id={reasonId}>{reason}</p>}
        {(row.status === "rejected" || row.status === "published") && row.reviewNote && <p className="lab-saved-note">Review note: {row.reviewNote}</p>}
        {feedback(row.id, row.market, row.instrument, row.id)}
      </li>
    );
  };

  return (
    <section className="box" data-lab-saved ref={box}>
      <h2 className="box-h">My indicator strategies</h2>
      {canEdit && (
        <>
          <div className="lab-saved-row lab-saved-save">
            <input ref={nameInput} className="lab-saved-name" type="text" aria-label="Strategy name" placeholder="Strategy name" maxLength={120}
              value={name} onChange={(e) => setName(e.target.value)} />
            <button type="button" className="dbtn sm primary" data-lab-action="save-new" disabled={busy || !trimmed} onClick={saveNew}>Save as new</button>
            {own && editable(own.status) && (
              <button type="button" className="dbtn sm" data-lab-action="save" disabled={busy || !trimmed} onClick={() => saveChanges(own)}>Save changes</button>
            )}
          </div>
          {feedback(SAVE, editor.market, editor.ticker, editor.row?.id)}
        </>
      )}
      {listError !== null ? <ErrorNotice error={listError} market="in" />
        : rows === null ? <Loading text="Loading saved strategies…" />
        : rows.length === 0 ? <div className="empty">{COPY.savedEmpty}</div>
        : [...groups].map(([head, list]) => (
          <div key={head} className="lab-saved-group">
            <h3>{head}</h3>
            <ul className="lab-saved-list">{list.map(item)}</ul>
          </div>
        ))}
    </section>
  );
}
