import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { SavedStrategies, type EditorState } from "../src/features/lab/SavedStrategies";
import { ReviewQueue } from "../src/features/lab/ReviewQueue";
import { flush, mount, strategyFixture, unmount } from "./fixtures/lab";
import type { LabStrategy } from "../src/features/lab/types";

afterEach(async () => { await unmount(); vi.restoreAllMocks(); });
const conflict = () => Object.assign(new Error("This strategy changed since you opened it (now version 4). Reload before saving."), { status: 409, kind: "auth" });
const api = (rows: LabStrategy[], patch: Record<string, unknown> = {}) => ({
  list: vi.fn().mockResolvedValue(rows), get: vi.fn(), create: vi.fn(), update: vi.fn(), remove: vi.fn().mockResolvedValue(null),
  duplicate: vi.fn(), action: vi.fn(), ...patch,
}) as never;
const editor = (row: LabStrategy | null, extra: Partial<EditorState> = {}): EditorState =>
  ({ spec: strategyFixture().spec, market: "in", ticker: "NIFTY 50", row, unchanged: true, ranFresh: true, ...extra });
const button = (host: HTMLElement, text: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!;

it("lists rows with status and shows review notes only when rejected or published", async () => {
  const rows = [strategyFixture({ id: 1, status: "rejected", reviewNote: "Add a stop" }), strategyFixture({ id: 2, status: "submitted", reviewNote: "hidden" })];
  const host = await mount(<SavedStrategies role="researcher" api={api(rows)} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(host.textContent).toContain("Rejected");
  expect(host.textContent).toContain("Review note: Add a stop");
  expect(host.textContent).not.toContain("hidden");
  expect(button(host, "Withdraw")).toBeDefined();
});

it("save as new maps the market and save changes sends the version", async () => {
  const a = api([strategyFixture()], { create: vi.fn().mockResolvedValue(strategyFixture({ id: 9 })), update: vi.fn().mockResolvedValue(strategyFixture({ version: 4 })) });
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={{ ...editor(strategyFixture()), market: "crypto", ticker: "CRYPTO:BTCUSD" }} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save-new"]')!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  expect((a as { create: ReturnType<typeof vi.fn> }).create.mock.calls[0][0]).toMatchObject({ market: "crypto", instrument: "CRYPTO:BTCUSD", name: "EMA pullback" });
  expect((a as { update: ReturnType<typeof vi.fn> }).update.mock.calls[0].slice(0, 2)).toEqual([7, 3]);
});

it("409 offers a reload that keeps the rules and saves with the new version", async () => {
  const onSaved = vi.fn();
  const a = api([strategyFixture()], { update: vi.fn().mockRejectedValueOnce(conflict()).mockResolvedValueOnce(strategyFixture({ version: 5 })),
    get: vi.fn().mockResolvedValue(strategyFixture({ version: 4 })) });
  const ed = editor(strategyFixture());
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={ed} onOpen={vi.fn()} onSaved={onSaved} />);
  await flush();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  await flush();
  expect(host.textContent).toContain("now version 4");
  await act(async () => button(host, "Reload this strategy").click());
  await flush();
  expect(host.textContent).toContain("Your rules on screen are kept. Save again to replace version 4.");
  expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ version: 4 }));
  // The next save sends version 4 and the rules on screen, even before the page re-renders with the new row.
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  const update = (a as { update: ReturnType<typeof vi.fn> }).update;
  expect(update.mock.calls[1].slice(0, 2)).toEqual([7, 4]);
  expect(update.mock.calls[1][2].spec).toBe(ed.spec);
  expect(host.querySelector("[data-lab-error]")).toBeNull();
});

it("delete asks first and sends the version", async () => {
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const a = api([strategyFixture()]);
  const host = await mount(<SavedStrategies role="researcher" api={a} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => button(host, "Delete").click());
  await flush();
  expect(window.confirm).toHaveBeenCalledWith('Delete "EMA pullback"? This cannot be undone.');
  expect((a as { remove: ReturnType<typeof vi.fn> }).remove).toHaveBeenCalledWith(7, 3);
  expect(host.textContent).toContain("Nothing saved yet. Name the current rules above and save them.");
});

it("submit needs a fresh run of the unchanged row and a publishable market", async () => {
  const row = strategyFixture();
  const stale = await mount(<SavedStrategies role="researcher" api={api([row])} editor={editor(row, { ranFresh: false })} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(button(stale, "Submit").disabled).toBe(true);
  expect(button(stale, "Submit").title).toBe("Run the current rules first; the engine checks the rules only when they run.");
  await unmount();
  // Each guard alone must block Submit: the server flag, and the provisional market even if the flag were wrong.
  for (const row of [strategyFixture({ market: "in", publishable: false }), strategyFixture({ market: "gl", publishable: true }), strategyFixture({ market: "crypto", publishable: true })]) {
    const host = await mount(<SavedStrategies role="researcher" api={api([row])} editor={editor(row)} onOpen={vi.fn()} onSaved={vi.fn()} />);
    await flush();
    const submit = button(host, "Submit");
    expect(submit.disabled).toBe(true);
    expect(submit.title).toBe("Provisional data cannot be published. Rerun on licensed data first.");
    // The reason is visible text tied to the button, not only a tooltip.
    expect(host.querySelector(`#${submit.getAttribute("aria-describedby")}`)?.textContent).toBe(submit.title);
    await unmount();
  }
});

it("an error on a provisional row carries the instrument in its provisional strip", async () => {
  const row = strategyFixture({ market: "gl", instrument: "GL:XAUUSD", publishable: false });
  const fail = Object.assign(new Error("Server busy."), { status: 500, kind: "gateway" });
  const host = await mount(<SavedStrategies role="researcher" api={api([row], { duplicate: vi.fn().mockRejectedValue(fail) })} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => button(host, "Duplicate").click());
  await flush();
  expect(host.querySelector("[data-lab-error] [data-lab-provisional]")?.textContent).toContain("GL:XAUUSD");
});

it("save changes sends the newer of the list and editor versions", async () => {
  const update = vi.fn().mockResolvedValue(strategyFixture({ version: 7 }));
  const host = await mount(<SavedStrategies role="researcher" api={api([strategyFixture({ version: 3 })], { update })} editor={editor(strategyFixture({ version: 6 }))} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="save"]')!.click());
  expect(update.mock.calls[0].slice(0, 2)).toEqual([7, 6]);
});

it("submit on a fresh run sends the version and shows the submitted note", async () => {
  const row = strategyFixture();
  const onSaved = vi.fn();
  const action = vi.fn().mockResolvedValue(strategyFixture({ status: "submitted", version: 4 }));
  const host = await mount(<SavedStrategies role="researcher" api={api([row], { action })} editor={editor(row)} onOpen={vi.fn()} onSaved={onSaved} />);
  await flush();
  expect(button(host, "Submit").disabled).toBe(false);
  await act(async () => button(host, "Submit").click());
  await flush();
  expect(action).toHaveBeenCalledWith(7, "submit", 3);
  expect(host.textContent).toContain("Submitted. An admin reviews it before it appears in YT Strategies.");
  expect(button(host, "Withdraw")).toBeDefined();
  expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ version: 4 }));
});

it("a failed list still shows the save controls and the message as given", async () => {
  const fail = Object.assign(new Error("The strategy store is not configured on this server."), { status: 503, kind: "gateway" });
  const host = await mount(<SavedStrategies role="researcher" api={api([], { list: vi.fn().mockRejectedValue(fail) })} editor={editor(null)} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("The strategy store is not configured on this server.");
  expect(host.querySelector('[data-lab-action="save-new"]')).not.toBeNull();
});

it("viewers get no save controls; variants are grouped by method", async () => {
  const v = strategyFixture({ id: 3, origin: { book: "ststw", method: "rsi2_4", template_version: 1 } });
  const host = await mount(<SavedStrategies role="viewer" api={api([v])} editor={editor(null)} methodNames={{ rsi2_4: "RSI 2 · 4" }} onOpen={vi.fn()} onSaved={vi.fn()} />);
  await flush();
  expect(host.querySelector('[data-lab-action="save-new"]')).toBeNull();
  expect(host.textContent).toContain("My variants of RSI 2 · 4");
  expect(host.textContent).toContain("Customized from ststw · RSI 2 · 4");
  expect(button(host, "Delete")).toBeUndefined();
  expect(host.textContent).not.toMatch(/verified/i);
});

it("review queue: reject needs a note; publish sends the version", async () => {
  const row = strategyFixture({ status: "submitted" });
  const action = vi.fn().mockResolvedValue({ ...row, status: "published" });
  const host = await mount(<ReviewQueue api={{ review: vi.fn().mockResolvedValue([row]), action } as never} onOpen={vi.fn()} />);
  await flush();
  expect(button(host, "Reject").disabled).toBe(true);
  await act(async () => button(host, "Publish").click());
  expect(action).toHaveBeenCalledWith(7, "publish", 3, undefined);
});

it("review queue: reject sends the note and removes the row; errors show as given", async () => {
  const rows = [strategyFixture({ status: "submitted" }), strategyFixture({ id: 8, name: "Second", status: "submitted" })];
  const fail = Object.assign(new Error("Only admins can review."), { status: 403, kind: "auth" });
  const action = vi.fn().mockResolvedValueOnce({ ...rows[0], status: "rejected" }).mockRejectedValueOnce(fail);
  const host = await mount(<ReviewQueue api={{ review: vi.fn().mockResolvedValue(rows), action } as never} onOpen={vi.fn()} />);
  await flush();
  const note = host.querySelector<HTMLTextAreaElement>('textarea[aria-label="Review note"]')!;
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
  await act(async () => { setter.call(note, "  Add a stop  "); note.dispatchEvent(new Event("input", { bubbles: true })); });
  await act(async () => button(host, "Reject").click());
  await flush();
  expect(action).toHaveBeenCalledWith(7, "reject", 3, "Add a stop");
  expect(host.textContent).not.toContain("EMA pullback");
  await act(async () => button(host, "Publish").click());
  await flush();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("Only admins can review.");
  expect(host.textContent).toContain("Second");
});

it("review queue: publish is never enabled on provisional data", async () => {
  const rows = [strategyFixture({ status: "submitted", market: "gl", publishable: true }), strategyFixture({ id: 8, status: "submitted", publishable: false })];
  const host = await mount(<ReviewQueue api={{ review: vi.fn().mockResolvedValue(rows), action: vi.fn() } as never} onOpen={vi.fn()} />);
  await flush();
  const publish = [...host.querySelectorAll<HTMLButtonElement>("button")].filter((b) => b.textContent === "Publish");
  expect(publish).toHaveLength(2);
  for (const b of publish) {
    expect(b.disabled).toBe(true);
    expect(b.title).toBe("Provisional data cannot be published. Rerun on licensed data first.");
  }
});

it("review queue: a 409 offers a queue reload with the instrument on provisional errors", async () => {
  const row = strategyFixture({ status: "submitted", market: "crypto", instrument: "CRYPTO:BTCUSD" });
  const review = vi.fn().mockResolvedValueOnce([row]).mockResolvedValueOnce([]);
  const note = "  Needs a stop  ";
  const host = await mount(<ReviewQueue api={{ review, action: vi.fn().mockRejectedValue(conflict()) } as never} onOpen={vi.fn()} />);
  await flush();
  const area = host.querySelector<HTMLTextAreaElement>('textarea[aria-label="Review note"]')!;
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
  await act(async () => { setter.call(area, note); area.dispatchEvent(new Event("input", { bubbles: true })); });
  await act(async () => button(host, "Reject").click());
  await flush();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("now version 4");
  expect(host.querySelector("[data-lab-error] [data-lab-provisional]")?.textContent).toContain("CRYPTO:BTCUSD");
  await act(async () => button(host, "Reload queue").click());
  await flush();
  expect(review).toHaveBeenCalledTimes(2);
  expect(host.textContent).toContain("No strategies are waiting for review.");
});
