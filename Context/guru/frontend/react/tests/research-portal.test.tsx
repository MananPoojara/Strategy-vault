import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ResearchPortal } from "../src/pages/ResearchPortal";
import { hashWith, query, readHashParams } from "../src/services/research";

let root: Root | undefined, host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined; vi.unstubAllGlobals(); history.replaceState(null, "", "/");
});
const strategy = {
  key: "stc9::stc9:f1a", author: "Larry Connors", author_id: "connors", book: "Swing Trading College IX", book_id: "stc9", name: "The 4 Keys",
  site: "verified", reason: "", rules: [{ kind: "Entry", text: "Buy on the close (p.13)" }], variant: "f1a", variants: 3,
  url: "http://192.168.4.30:8180/#/s/f1a", verdict: "mismatch", verdict_by: "Asha", verdict_at: "2026-10-26T05:00:00Z", notes: 1, rule_errors: 1,
};
function api(routes: Record<string, unknown>) {
  const calls: { url: string; init?: RequestInit }[] = [];
  vi.stubGlobal("fetch", vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    const path = url.split("?")[0];
    const body = routes[`${init?.method || "GET"} ${path}`];
    if (body === undefined) return new Response(JSON.stringify({ error: `no route ${path}` }), { status: 404 });
    return new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
  }));
  return calls;
}
async function mount(hash: string) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.VAULT_USER = { id: "u1", name: "Asha", email: "a@x", role: "researcher" };
  history.replaceState(null, "", `/${hash}`);
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<ResearchPortal />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  return host;
}

it("keeps filters in the hash and builds clean queries", () => {
  expect(query({ a: "1", b: "", c: undefined, page: 2 })).toBe("?a=1&page=2");
  expect(hashWith({ tab: "review", q: "rsi" }, "#/research?tab=progress")).toBe("#/research?tab=review&q=rsi");
  expect(hashWith({ q: undefined }, "#/research?tab=review&q=rsi")).toBe("#/research?tab=review");
  expect(readHashParams("#/research?tab=notes").get("tab")).toBe("notes");
});

it("review lists strategies, opens one and records a verdict and a note as the signed-in employee", async () => {
  const calls = api({
    "GET /api/app/research/strategies": { items: [strategy], total: 26, page: 1, size: 25, pages: 2, facets: { authors: [{ id: "connors", name: "Larry Connors" }], books: [], reviewers: ["Asha"] } },
    "GET /api/app/research/strategy": { strategy, verdict: { verdict: "mismatch", author_name: "Asha", updated_at: "2026-10-26T05:00:00Z" }, history: [], notes: [] },
    "PUT /api/app/research/verdict": { previous: "mismatch", changed: true },
    "POST /api/app/research/notes": { id: 1 },
  });
  const el = await mount("#/research?tab=review");
  expect(el.textContent).toContain("26 strategies");
  expect(el.textContent).toContain("Page 1 of 2");
  await act(async () => el.querySelector<HTMLButtonElement>(`[data-key="${strategy.key}"]`)!.click());
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(location.hash).toContain("key=stc9%3A%3Astc9%3Af1a");
  expect(el.querySelector(".rp-rules")!.textContent).toContain("Buy on the close (p.13)");
  expect(el.querySelector<HTMLAnchorElement>(".rp-site a")!.href).toBe("http://192.168.4.30:8180/#/s/f1a");
  await act(async () => [...el.querySelectorAll<HTMLButtonElement>(".rp-verdicts button")].find(b => b.textContent?.includes("Matches"))!.click());
  const put = calls.find(c => c.init?.method === "PUT")!;
  expect(JSON.parse(String(put.init!.body))).toEqual({ key: strategy.key, verdict: "matches" });
  const area = el.querySelector<HTMLTextAreaElement>(".rp-detail textarea")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(area, "Book exits at the open (p.14)");
    area.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => el.querySelector<HTMLFormElement>(".rp-detail .rp-composer")!.requestSubmit());
  const post = calls.find(c => c.init?.method === "POST")!;
  const sent = JSON.parse(String(post.init!.body));
  expect(sent).toEqual({ target_type: "strategy", target_key: strategy.key, note_type: "rule_error", body: "Book exits at the open (p.14)" });
  expect(sent.author_name).toBeUndefined();
});

it("viewers can read but not record verdicts", async () => {
  api({
    "GET /api/app/research/strategies": { items: [strategy], total: 1, page: 1, size: 25, pages: 1, facets: { authors: [], books: [], reviewers: [] } },
    "GET /api/app/research/strategy": { strategy, verdict: { verdict: "not_checked" }, history: [], notes: [] },
  });
  const el = await mount(`#/research?tab=review&key=${encodeURIComponent(strategy.key)}`);
  window.VAULT_USER = { id: "u2", name: "Vera", email: "v@x", role: "viewer" };
  await act(async () => root!.render(<ResearchPortal />));
  expect([...el.querySelectorAll<HTMLButtonElement>(".rp-verdicts button")].every(b => b.disabled)).toBe(true);
  expect(el.textContent).toContain("Your account can read only");
  expect(el.querySelector(".rp-detail textarea")).toBeNull();
});

it("timeline groups events by day, newest first, and pages", async () => {
  api({
    "GET /api/app/research/timeline": { total: 31, page: 1, size: 30, pages: 2, items: [
      { kind: "note", at: "2026-10-26T06:00:00Z", who: "Asha", actor_id: "u1", action: "created", detail: "rule_error", text: "Exit differs", ref: 1, target_type: "strategy", target_key: "k", target_label: "The 4 Keys (STC IX)", backtested: null, proof: "" },
      { kind: "platform", at: "2026-10-21T13:37:00Z", who: "Platform", actor_id: null, action: "work_timeline", detail: "21 Oct 19:07", text: "Deploy: count goes up", ref: null, target_type: null, target_key: "x", target_label: null, backtested: 147, proof: "snapshot" },
    ] },
    "GET /api/app/research/people": [],
  });
  const el = await mount("#/research?tab=timeline");
  const days = [...el.querySelectorAll(".rp-day")].map(d => d.textContent);
  expect(days).toEqual(["26 Oct 2026", "21 Oct 2026"]);
  expect(el.querySelector(".rp-ev.k-platform .rp-ev-time")!.textContent).toBe("19:07");
  expect(el.textContent).toContain("Page 1 of 2");
});
