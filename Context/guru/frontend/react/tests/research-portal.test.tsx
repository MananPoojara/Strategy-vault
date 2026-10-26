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

const asha = { employee: { id: 1, name: "Asha" }, admin: false, ready: true };
const tick = () => act(async () => { await new Promise(r => setTimeout(r, 0)); });
function type(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

it("asks for a name and the PIN before showing anything", async () => {
  const calls = api({ "GET /api/app/research/me": { employee: null, admin: false, ready: true }, "POST /api/app/research/enter": { employee: { id: 1, name: "Manan" } } });
  const el = await mount("#/research");
  expect(el.textContent).toContain("Who is reviewing?");
  const [name, pin] = [...el.querySelectorAll<HTMLInputElement>(".rp-gate input")];
  await act(async () => { type(name, "Manan"); type(pin, "123"); });
  await act(async () => el.querySelector<HTMLFormElement>(".rp-gate")!.requestSubmit());
  const post = calls.find(c => c.init?.method === "POST")!;
  expect(JSON.parse(String(post.init!.body))).toEqual({ name: "Manan", pin: "123" });
});

it("author cards lead to book cards and strategy cards", async () => {
  api({
    "GET /api/app/research/me": asha,
    "GET /api/app/research/summary": { totals: { verified: 587, held: 810, unreviewed: 299 }, team: { verdicts: { matches: 3, mismatch: 1, unclear: 0 } } },
    "GET /api/app/research/browse": { items: [{ id: "connors", name: "Larry Connors", author: "Larry Connors", books: 25, strategies: 178, verified: 110, held: 40, unreviewed: 28, checked: 4 }] },
    "GET /api/app/research/strategies": { items: [strategy], total: 1, page: 1, size: 24, pages: 1, facets: { authors: [], books: [], reviewers: [] } },
  });
  const el = await mount("#/research");
  expect(el.textContent).toContain("Hi, Asha");
  const card = el.querySelector<HTMLAnchorElement>(".rp-grid .rp-card")!;
  expect(card.textContent).toContain("Larry Connors");
  expect(card.getAttribute("href")).toBe("#/research?author=connors");
  history.replaceState(null, "", "/#/research?author=connors&book=stc9"); dispatchEvent(new HashChangeEvent("hashchange")); await tick();
  expect(el.querySelector(".rp-s")!.textContent).toContain("The 4 Keys");
  expect(el.querySelector(".rp-s")!.textContent).toContain("Mismatch");
});

it("records a verdict and a note under the employee's name, sent by the server", async () => {
  const calls = api({
    "GET /api/app/research/me": asha,
    "GET /api/app/research/strategy": { strategy, verdict: { verdict: "mismatch", author_name: "Asha", updated_at: "2026-10-26T05:00:00Z" }, history: [], notes: [] },
    "PUT /api/app/research/verdict": { previous: "mismatch", changed: true },
    "POST /api/app/research/notes": { id: 1 },
  });
  const el = await mount(`#/research?s=${encodeURIComponent(strategy.key)}`);
  expect(el.querySelector(".rp-rules")!.textContent).toContain("Buy on the close (p.13)");
  await act(async () => [...el.querySelectorAll<HTMLButtonElement>(".rp-verdicts button")].find(b => b.textContent?.includes("Matches"))!.click());
  expect(JSON.parse(String(calls.find(c => c.init?.method === "PUT")!.init!.body))).toEqual({ key: strategy.key, verdict: "matches" });
  await act(async () => [...el.querySelectorAll<HTMLButtonElement>(".rp-noteform .rp-chips button")].find(b => b.textContent === "Rule error")!.click());
  await act(async () => type(el.querySelector<HTMLTextAreaElement>(".rp-noteform textarea")!, "Book exits at the open (p.14)"));
  await act(async () => el.querySelector<HTMLFormElement>(".rp-noteform")!.requestSubmit());
  const sent = JSON.parse(String(calls.find(c => c.init?.method === "POST")!.init!.body));
  expect(sent).toEqual({ target_type: "strategy", target_key: strategy.key, note_type: "rule_error", body: "Book exits at the open (p.14)" });
});

it("admin sees employee cards and removes one after confirming", async () => {
  const calls = api({
    "GET /api/app/research/me": { employee: null, admin: true, ready: true },
    "GET /api/app/research/employees": [{ id: 7, name: "Manan", created_at: "2026-10-26T05:00:00Z", last_seen_at: "2026-10-26T06:00:00Z", removed_at: null, checks: 12, mismatches: 2, notes: 5, rule_errors: 1 }],
    "DELETE /api/app/research/employees/7": { removed: 7 },
  });
  const el = await mount("#/research?v=team");
  expect(el.querySelector(".rp-emp")!.textContent).toContain("12 checks");
  const remove = () => [...el.querySelectorAll<HTMLButtonElement>(".rp-emp button")];
  await act(async () => remove()[0].click());
  expect(calls.some(c => c.init?.method === "DELETE")).toBe(false);
  await act(async () => remove().find(b => b.textContent?.startsWith("Yes"))!.click());
  expect(calls.some(c => c.init?.method === "DELETE" && c.url.endsWith("/employees/7"))).toBe(true);
});

it("one person's work is grouped by day, newest first", async () => {
  api({
    "GET /api/app/research/me": asha,
    "GET /api/app/research/work": { person: "Asha", totals: { strategies: 1, matches: 1, mismatch: 0, unclear: 0, notes: 1 }, items: [
      { author: "Larry Connors", author_id: "connors", books: [{ book: "Swing Trading College IX", book_id: "stc9", strategies: [{ key: "k", name: "The 4 Keys", verdict: "matches", notes: 1, errors: 1, at: "2026-10-26T06:00:00Z" }] }] }] },
    "GET /api/app/research/timeline": { total: 2, page: 1, size: 30, pages: 1, items: [
      { kind: "note", at: "2026-10-26T06:00:00Z", who: "Asha", actor_id: "u1", action: "created", detail: "rule_error", text: "Exit differs", ref: 1, target_type: "strategy", target_key: "k", target_label: "The 4 Keys (STC IX)", backtested: null, proof: "" },
      { kind: "verdict", at: "2026-10-21T13:37:00Z", who: "Asha", actor_id: "u1", action: "set", detail: "matches", text: null, ref: 2, target_type: "strategy", target_key: "k", target_label: "The 4 Keys (STC IX)", backtested: null, proof: "" },
    ] },
  });
  const el = await mount("#/research?v=mine");
  expect(el.querySelector(".rp-work")!.textContent).toContain("Swing Trading College IX");
  expect(el.querySelector(".rp-work")!.textContent).toContain("Matches the book");
  expect([...el.querySelectorAll(".rp-day h3")].map(d => d.textContent)).toEqual(["26 Oct 2026", "21 Oct 2026"]);
  expect(el.textContent).toContain("Exit differs");
});

it("Claude's work lists platform deploys with the backtested count", async () => {
  const calls = api({
    "GET /api/app/research/me": asha,
    "GET /api/app/research/timeline": { total: 1, page: 1, size: 30, pages: 1, items: [
      { kind: "platform", at: "2026-10-26T10:58:00Z", who: "Platform", actor_id: null, action: "work_timeline", detail: "26 Oct 16:28", text: "Deploy: MAE/MFE shared formula", ref: null, target_type: null, target_key: "x", target_label: null, backtested: 587, proof: "aeb9905" },
    ] },
  });
  const el = await mount("#/research?v=claude");
  expect(calls.some(c => c.url.includes("kind=platform"))).toBe(true);
  expect(el.querySelector(".rp-claude")!.textContent).toContain("587");
  expect(el.querySelector(".rp-claude")!.textContent).toContain("Deploy: MAE/MFE shared formula");
});
