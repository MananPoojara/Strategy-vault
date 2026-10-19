import { afterEach, beforeEach, expect, it, vi } from "vitest";
import * as lab from "../src/features/lab/client";
import { ApiError } from "../src/services/api";
import { catalogueFixture, jsonResponse, fetchRouter, runFixture, strategyFixture } from "./fixtures/lab";

beforeEach(() => lab.clearLabCache());
afterEach(() => vi.unstubAllGlobals());

it("routes markets to engine bases; crypto runs on gl", () => {
  expect(["in", "us", "gl", "crypto"].map((m) => lab.engineBase(m as never))).toEqual(["/api/in", "/api/us", "/api/gl", "/api/gl"]);
});

it("encodes spec, ticker and p.* settings and always sends the ticker", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse(runFixture)]]);
  vi.stubGlobal("fetch", f);
  await lab.run("in", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "NIFTY 50", cost: 5 });
  await lab.series("gl", { id: "ema", ticker: "CRYPTO:BTCUSD", tf: "W", p: { length: 20 } });
  const [runUrl, seriesUrl] = f.calls.map((c) => new URL(c.url, "http://x"));
  expect(runUrl.pathname).toBe("/api/in/lab/run");
  expect(runUrl.searchParams.get("ticker")).toBe("NIFTY 50");
  expect(JSON.parse(runUrl.searchParams.get("spec")!).v).toBe(1);
  expect(runUrl.searchParams.get("cost")).toBe("5");
  expect(runUrl.searchParams.has("slippage")).toBe(false);
  expect(seriesUrl.pathname).toBe("/api/gl/lab/series");
  expect(seriesUrl.searchParams.get("p.length")).toBe("20");
  expect(f.calls[0].init?.credentials).toBe("same-origin");
});

it("optimize always asks for slim rows and sends split only when given", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse({})]]);
  vi.stubGlobal("fetch", f);
  await lab.optimize("us", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "SPY", split: "2020-01-02" });
  await lab.optimize("us", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "SPY", split: null });
  const [a, b] = f.calls.map((c) => new URL(c.url, "http://x").searchParams);
  expect(a.get("slim")).toBe("1");
  expect(a.get("table")).toBe("1");
  expect(a.get("split")).toBe("2020-01-02");
  expect(b.has("split")).toBe(false);
});

it("keeps engine error fields and never returns numbers", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/gl/lab/run", () => jsonResponse({ provisional: true, provisional_note: "Provisional · x",
    status: "needs_data", error: "window starts before the first admitted bar", label: "User-defined indicator strategy · simulated · not a book method" }, 400)]]));
  const e = await lab.run("gl", { v: 1, entry: { all: [] }, exit: { any: [] } }, { ticker: "CRYPTO:BTCUSD" }).catch((x) => x);
  expect(e).toBeInstanceOf(lab.LabError);
  expect(e).toBeInstanceOf(ApiError);
  expect([e.kind, e.status, e.engineStatus, e.provisionalNote, e.message]).toEqual(
    ["engine", 400, "needs_data", "Provisional · x", "window starts before the first admitted bar"]);
});

it("reads gateway errors, Retry-After and non-JSON bodies", async () => {
  vi.stubGlobal("fetch", fetchRouter([
    ["/api/in/lab/optimize", () => jsonResponse({ error: "An optimization is already running. Please retry when it finishes." }, 429, { "Retry-After": "7" })],
    ["/api/us/lab/run", () => new Response("<html>bad gateway</html>", { status: 502 })],
    ["/api/in/lab/run", () => jsonResponse({ error: "Busy" }, 429)],
  ]));
  const spec = { v: 1 as const, entry: { all: [] }, exit: { any: [] } };
  const busy = await lab.optimize("in", spec, { ticker: "NIFTY 50" }).catch((x) => x);
  expect([busy.kind, busy.status, busy.retryAfter]).toEqual(["gateway", 429, 7]);
  expect((await lab.run("in", spec, { ticker: "NIFTY 50" }).catch((x) => x)).retryAfter).toBe(5);
  const bad = await lab.run("us", spec, { ticker: "SPY" }).catch((x) => x);
  expect([bad.kind, bad.message]).toEqual(["gateway", "Request failed (502)."]);
});

it("caches the catalogue per market and drops a failed load", async () => {
  let n = 0;
  const f = fetchRouter([["/api/in/lab/catalogue", () => (++n === 1 ? jsonResponse({ error: "x" }, 503) : jsonResponse(catalogueFixture))]]);
  vi.stubGlobal("fetch", f);
  await expect(lab.catalogue("in")).rejects.toThrow("x");
  await lab.catalogue("in");
  await lab.catalogue("in");
  expect(f.calls).toHaveLength(2);
});

it("limits engine reads to four in flight", async () => {
  let active = 0, peak = 0;
  vi.stubGlobal("fetch", async () => { active++; peak = Math.max(peak, active); await new Promise((r) => setTimeout(r, 5)); active--; return jsonResponse({}); });
  await Promise.all(Array.from({ length: 7 }, (_, i) => lab.series("in", { id: `x${i}`, ticker: "NIFTY 50" })));
  expect(peak).toBe(4);
});

it("saved strategies: version on every mutation, 204 delete, auth errors", async () => {
  const f = fetchRouter([
    [/\/7\?version=3$/, () => new Response(null, { status: 204 })],
    [/\/7\/submit$/, () => jsonResponse({ error: "This strategy changed since you opened it (now version 4). Reload before saving." }, 409)],
    ["/api/app/lab/strategies", () => jsonResponse(strategyFixture(), 201)],
  ]);
  vi.stubGlobal("fetch", f);
  const draft = { name: "x", spec: strategyFixture().spec, market: "crypto" as const, instrument: "CRYPTO:BTCUSD" };
  await lab.strategies.create(draft);
  await lab.strategies.update(7, 3, draft);
  expect(await lab.strategies.remove(7, 3)).toBeNull();
  const conflict = await lab.strategies.action(7, "submit", 3).catch((x) => x);
  await lab.strategies.action(7, "reject", 3, "Needs a stop").catch(() => null);
  expect(JSON.parse(String(f.calls[0].init?.body))).toMatchObject({ market: "crypto" });
  expect(f.calls[1].init?.method).toBe("PUT");
  expect(JSON.parse(String(f.calls[1].init?.body)).version).toBe(3);
  expect([conflict.kind, conflict.status]).toEqual(["auth", 409]);
  expect(JSON.parse(String(f.calls[4].init?.body))).toEqual({ version: 3, note: "Needs a stop" });
  expect(f.calls[0].init?.headers).toMatchObject({ "Content-Type": "application/json" });
});

it("sends an explicit pair comparison without inventing a default instrument", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse({})]]); vi.stubGlobal("fetch", f);
  await lab.series("us", { id: "ratio", ticker: "SPY", tf: "D", vs: "SPX" } as never);
  await lab.series("us", { id: "ema", ticker: "SPY", tf: "D" });
  expect(new URL(f.calls[0].url,"http://x").searchParams.get("vs")).toBe("SPX");
  expect(new URL(f.calls[1].url,"http://x").searchParams.has("vs")).toBe(false);
});

it("portfolio transport omits single-instrument and split queries even if stale settings were supplied",async()=>{
 const f=fetchRouter([["/api/",()=>jsonResponse({})]]);vi.stubGlobal("fetch",f);
 const spec={v:1 as const,entry:{all:[]},exit:{any:[]},execution:{capital:1000},portfolio:{mode:"rotation",universe:{tickers:["SPY","QQQ"]},score:{s:"close"},max_open:2}};
 await lab.run("us",spec as never,{ticker:"SPX",cost:5,slippage:2});await lab.optimize("us",spec as never,{ticker:"SPX",cost:5,split:"2024-01-01"});
 for(const call of f.calls){const q=new URL(call.url,"http://x").searchParams;for(const key of ["ticker","cost","slippage","split"])expect(q.has(key)).toBe(false);expect(JSON.parse(q.get("spec")!).execution.capital).toBe(1000);}
});
