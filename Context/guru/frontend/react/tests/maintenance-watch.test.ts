import { expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const code = readFileSync("src/services/maintenance-watch.js", "utf8");
const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));
function boot({ page = false, preview = false, path = "/indicators", active = false, error = false } = {}) {
  const state = { active, error }, status = { textContent: "" }, retry = { disabled: false };
  let interval: () => Promise<void> = async () => {};
  const handlers: Record<string, () => void> = {};
  const location = { pathname: path, search: preview ? "?preview=1" : "", reload: vi.fn(), replace: vi.fn() };
  const fetch = vi.fn(async () => { if (state.error) throw new Error("offline"); return { ok: true, json: async () => ({ maintenance: state.active }) }; });
  runInNewContext(code, {
    document: { hidden: false, documentElement: { hasAttribute: () => page }, getElementById: (id: string) => id === "maintenance-status" ? status : retry, addEventListener: (name: string, fn: () => void) => { handlers[name] = fn; } },
    window: { addEventListener: (name: string, fn: () => void) => { handlers[name] = fn; } },
    location, URLSearchParams, fetch, setInterval: (fn: () => Promise<void>, ms: number) => { expect(ms).toBe(8000); interval = fn; },
  });
  return { state, status, retry, location, fetch, handlers, poll: async () => { await interval(); await settle(); } };
}
it("open application tabs reload once when maintenance is enabled", async () => {
  const b = boot(); await settle(); expect(b.location.reload).not.toHaveBeenCalled();
  b.state.active = true; await b.poll(); await b.poll();
  expect(b.location.reload).toHaveBeenCalledTimes(1);
  expect(b.fetch.mock.calls[0]).toEqual(["/__maintenance/status", { cache: "no-store", credentials: "same-origin" }]);
});
it("maintenance waits while active and reloads the original page when released", async () => {
  const b = boot({ page: true, active: true }); await settle();
  expect(b.status.textContent).toContain("update is in progress");
  expect(b.location.reload).not.toHaveBeenCalled();
  b.state.active = false; await b.poll();
  expect(b.location.reload).toHaveBeenCalledTimes(1);
  expect(b.location.replace).not.toHaveBeenCalled();
});
it("a direct maintenance document returns to the app when released", async () => {
  const b = boot({ page: true, path: "/maintenance.html" }); await settle();
  expect(b.location.replace).toHaveBeenCalledWith("/");
});
it("a maintenance preview stays visible and does not poll or navigate", async () => {
  const b = boot({ page: true, preview: true }); await settle(); await b.poll();
  expect(b.status.textContent).toContain("Maintenance preview");
  expect(b.fetch).not.toHaveBeenCalled(); expect(b.location.reload).not.toHaveBeenCalled();
});
it("a failed check leaves an honest retry state and recovers on the next poll", async () => {
  const b = boot({ page: true, active: true, error: true }); await settle();
  expect(b.status.textContent).toContain("couldn’t reach"); expect(b.retry.disabled).toBe(false);
  b.state.error = false; await b.poll();
  expect(b.status.textContent).toContain("update is in progress"); expect(b.location.reload).not.toHaveBeenCalled();
});
