import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ResearchPortal } from "../src/pages/ResearchPortal";
import { resetSnapshotDetail, type SnapEvent } from "../src/services/research";

let root: Root | undefined, host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined; vi.unstubAllGlobals(); history.replaceState(null, "", "/"); resetSnapshotDetail();
  document.body.style.overflow = "";
});
const AT = "2026-11-17T19:58:02.365769+00:00";
const row = { kind: "platform", at: AT, who: "Platform", actor_id: null, action: "snapshot", detail: "Live snapshot",
  text: "Live site snapshot: 1731 of 2473 strategies backtested, 742 held with a reason, 0 not reviewed yet (838 books)", ref: null, target_type: null, target_key: "k", target_label: null, backtested: 1731, proof: "library-progress.json" };
const many = Array.from({ length: 40 }, (_, i): string[] => [`Card ${i}`, "Author A", "Book B", "Claude"]);
const ev: SnapEvent = {
  at: AT, at_ist: "2026-11-18 01:28", stats: { backtested: 1731, held: 742, total: 2473, books: 838 }, prev: { backtested: 1722, held: 742, total: 2464, books: 837 },
  gained: [["Smitlener streak system", "S&C", "Active Trader", "Claude"]], lost: [], claude: many, fixed: [["btp 8-day", "Best Trading Patterns", "BTP", "Claude", "Yash 17 Nov tracker", "btp2_8day"]],
  log: [["2026-11-18 02:15", "release", "DEPLOYED int-1118a", ["scsmit_streak"]]], health: null,
};
function api(detail: unknown) {
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    const path = url.split("?")[0];
    const routes: Record<string, unknown> = {
      "/api/app/research/me": { employee: { id: 1, name: "Asha" }, admin: false, ready: true },
      "/api/app/research/timeline": { total: 1, page: 1, size: 30, pages: 1, items: [row] },
      "data/research-snapshot-detail.json": detail,
    };
    const body = routes[path];
    return body === undefined ? new Response("{}", { status: 404 }) : new Response(JSON.stringify(body), { status: 200 });
  }));
}
async function open() {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.VAULT_USER = { id: "u1", name: "Asha", email: "a@x", role: "researcher" };
  history.replaceState(null, "", "/#/research?v=claude");
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<ResearchPortal />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  await act(async () => { host!.querySelector<HTMLElement>('li[role="button"]')!.click(); });
}
const dlg = () => document.querySelector<HTMLElement>('[role="dialog"]');
const openSection = async (name: string) => act(async () => {
  const d = [...document.querySelectorAll<HTMLDetailsElement>("details")].find(x => x.textContent!.startsWith(name))!;
  d.open = true; d.dispatchEvent(new Event("toggle"));
});

it("opens with numbers, changes, lists, search over 30 names, and closes with Esc and the button", async () => {
  api({ schema: 1, events: { k1: ev } });
  await open();
  expect(dlg()).toBeTruthy();
  expect(dlg()!.textContent).toContain("2026-11-18 01:28 IST");
  expect(dlg()!.textContent).toContain("+9 vs previous");
  expect(dlg()!.querySelector("[data-lenis-prevent]")).toBeTruthy();
  expect(document.body.style.overflow).toBe("hidden");
  await openSection("Gained");
  expect(dlg()!.textContent).toContain("Smitlener streak system");
  expect(dlg()!.querySelector('input[type="search"]')).toBeNull(); // 1 name: no search box
  await openSection("Claude coded");
  const box = dlg()!.querySelector<HTMLInputElement>('input[type="search"]')!;
  expect(box).toBeTruthy();
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(box, "card 39");
    box.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(dlg()!.querySelectorAll(".rp-snap-list li").length).toBe(2); // Card 39 + the Gained row
  await openSection("Verification errors fixed");
  expect(dlg()!.textContent).toContain("btp2_8day");
  expect(dlg()!.textContent).toContain("How Claude backtests a strategy");
  await act(async () => { document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
  expect(dlg()).toBeNull();
  expect(document.body.style.overflow).toBe("");
  await act(async () => { host!.querySelector<HTMLElement>('li[role="button"]')!.click(); });
  await act(async () => { dlg()!.querySelector<HTMLButtonElement>("button")!.click(); });
  expect(dlg()).toBeNull();
});

it("says 'not recorded' for lists the generator could not fill and shows health fallback", async () => {
  api({ schema: 1, health_starts_ist: "2026-11-18 19:11", events: { k1: { ...ev, gained: null, lost: null, claude: null } } });
  await open();
  expect(dlg()!.textContent).toMatch(/Gained.*not recorded/);
  await openSection("Machine health");
  expect(dlg()!.textContent).toContain("Health samples start 2026-11-18 19:11 IST");
});

it("without the detail file the popup shows the numbers from the row only", async () => {
  api(undefined);
  await open();
  expect(dlg()!.textContent).toContain("1,731");
  expect(dlg()!.textContent).toContain("742");
  expect(dlg()!.textContent).toContain("not available");
  expect(dlg()!.querySelector("details")).toBeNull();
});
