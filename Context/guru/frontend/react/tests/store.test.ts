import { describe, it, expect } from "vitest";
import { createStore } from "../src/state/store";
describe("view transitions", () => {
  it("discards a late response after a newer route has started", () => {
    const store = createStore();
    const first = store.begin("/s/first"),
      second = store.begin("/s/second");
    expect(store.commit(first, { html: "outdated" })).toBe(false);
    expect(store.commit(second, { html: "current" })).toBe(true);
    expect(store.getSnapshot().html).toBe("current");
  });
  it("fills a page in place without a new revision, and ignores a fill for a page already left", () => {
    const store = createStore();
    const ticket = store.begin("/");
    store.commit(ticket, { html: "shell" });
    const revision = store.getSnapshot().revision;
    expect(store.fill(ticket, { html: "filled" })).toBe(true);
    expect(store.getSnapshot()).toMatchObject({ html: "filled", revision, loading: false });
    store.begin("/strategies");
    expect(store.fill(ticket, { html: "late" })).toBe(false);
    expect(store.getSnapshot().html).toBe("filled");
  });
  it("publishes stable snapshots and cleans subscriptions", () => {
    const store = createStore();
    let calls = 0;
    const unsubscribe = store.subscribe(() => calls++);
    expect(store.getSnapshot()).toBe(store.getSnapshot());
    const ticket = store.begin("/");
    store.commit(ticket, { html: "ready" });
    expect(calls).toBe(2);
    unsubscribe();
    store.begin("/saved");
    expect(calls).toBe(2);
  });
  it("clears previous result on error and ignores stale errors", () => {
    const store = createStore();
    const old = store.begin("/s/old"),
      current = store.begin("/s/new");
    expect(store.fail(old, "old error")).toBe(false);
    expect(store.fail(current, "Unavailable")).toBe(true);
    expect(store.getSnapshot().error).toBe("Unavailable");
    expect(store.getSnapshot().html).toBe("");
  });
});
