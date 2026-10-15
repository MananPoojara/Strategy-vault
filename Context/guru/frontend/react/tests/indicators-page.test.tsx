import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import {
  catalogueFixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, optimizeFixture, publishedFixture, runFixture,
  seriesFixture, templateFixture, templatesFixture, tickersFixture, unavailableTemplateFixture, unmount,
} from "./fixtures/lab";

const me = (role: string) => jsonResponse({ user: { id: "u1", name: "N", email: "n@x", role }, settings: { theme: "aqua" } });
const series = (url: string) => { const id = new URL(url, "http://x").searchParams.get("id")!; return jsonResponse(seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] })); };
let runAnswer: () => Response | Promise<Response>;
let usTickers = tickersFixture;
let templateAnswer = templateFixture;
const routes = (role = "researcher") => fetchRouter([
  ["/api/app/me", () => me(role)],
  ["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)],
  ["/api/app/lab/strategies/review", () => jsonResponse([])],
  ["/api/app/lab/strategies", () => jsonResponse({ error: "Saved strategies need the auth service; this local dev proxy has none." }, 503)],
  [/\/api\/(in|us|gl)\/lab\/catalogue/, () => jsonResponse(catalogueFixture)],
  ["/api/us/tickers", () => jsonResponse(usTickers)],
  ["/api/in/tickers", () => jsonResponse(tickersFixture)],
  ["/api/gl/lab/instruments", () => jsonResponse(glInstrumentsFixture)],
  [/\/lab\/series/, (url) => series(url)],
  [/\/lab\/templates/, () => jsonResponse(templatesFixture)],
  [/\/lab\/template\?/, () => jsonResponse(templateAnswer)],
  [/\/lab\/run/, () => runAnswer()],
  [/\/lab\/optimize/, () => jsonResponse(optimizeFixture)],
]);
beforeEach(() => { clearLabCache(); runAnswer = () => jsonResponse(runFixture); usTickers = tickersFixture; templateAnswer = templateFixture; });
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); history.replaceState(null, "", "/"); });
const go = async (hash: string, role = "researcher") => {
  history.replaceState(null, "", `/${hash}`);
  const f = routes(role);
  vi.stubGlobal("fetch", f);
  const host = await mount(<IndicatorsPage />);
  await flush(); await flush();
  return { host, f };
};
const pick = async (host: HTMLElement, tk: string) => {
  const sel = host.querySelector<HTMLSelectElement>(".lab-toolbar select")!;
  await act(async () => { sel.value = tk; sel.dispatchEvent(new Event("change", { bubbles: true })); });
  await flush();
};
const run = async (host: HTMLElement) => { await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.click()); await flush(); };

it("renders the catalogue label, library and default instrument; saved-strategy errors stay local", async () => {
  const { host } = await go("#/indicators");
  expect(host.querySelector('[data-lab-page="indicators"]')).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(catalogueFixture.label);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(catalogueFixture.indicators.length);
  expect(host.querySelector<HTMLSelectElement>(".lab-toolbar select")!.value).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("auth service");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("Could not load");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).not.toContain("Not saved");
});

it("an unknown ?tf= falls back to daily once the catalogue loads", async () => {
  await go("#/indicators?tf=Q");
  expect(location.hash).not.toContain("tf=");
});

it("the rules span the full row until a backtest exists or is running, then sit beside it", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  const row = () => host.querySelector<HTMLElement>(".two.lab-two")!;
  expect(row().dataset.labLayout).toBe("full");
  expect(row().querySelector(".lab-rules")).not.toBeNull();
  expect(row().children).toHaveLength(1); // no empty second column
  let release!: (r: Response) => void;
  runAnswer = () => new Promise<Response>((resolve) => { release = resolve; });
  await run(host);
  expect(row().dataset.labLayout).toBe("split");
  expect(row().querySelector("[role=status]")?.textContent).toBe("Running backtest…");
  await act(async () => release(jsonResponse(runFixture)));
  await flush();
  expect(row().dataset.labLayout).toBe("split");
  expect(row().querySelector("[data-lab-result]")).not.toBeNull();
  expect(row().children).toHaveLength(2);
});

it("a failed run clears the previous result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  const tables = host.querySelector("[data-lab-result-tables]")!;
  expect(tables).not.toBeNull();
  expect(tables.closest("[data-lab-result], .two")).toBeNull(); // a full-width row below the rules/backtest pair
  expect(host.querySelector(".two [data-lab-result]")).not.toBeNull();
  runAnswer = () => jsonResponse({ status: "needs_data", error: "NIFTY 50: invalid OHLC bar on 2001-03-02", label: catalogueFixture.label }, 400);
  await run(host);
  expect(host.querySelector("[data-lab-result]")).toBeNull();
  expect(host.querySelector("[data-lab-result-tables]")).toBeNull();
  expect(host.textContent).toContain("NIFTY 50: invalid OHLC bar on 2001-03-02");
});

it("gl shows the provisional strip and hides Customize", async () => {
  const { host } = await go("#/indicators?region=gl");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("Provisional · TradingView export CRYPTO:BTCUSD, daily");
  expect(host.textContent).toContain("Customize runs on India and US book methods only.");
});

it("viewers cannot optimize", async () => {
  const { host } = await go("#/indicators", "viewer");
  expect(host.textContent).toContain("Researcher access is required for optimization.");
  expect(host.querySelector('[data-lab-action="optimize"]')).toBeNull();
});

it("customize loads the template and runs with its origin", async () => {
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  expect(host.textContent).toContain("Book template · declared adaptation into lab rules · not a verification");
  await run(host);
  const runCall = f.calls.find((c) => c.url.includes("/lab/run"))!;
  const spec = JSON.parse(new URL(runCall.url, "http://x").searchParams.get("spec")!);
  expect(spec.origin).toEqual({ book: "ststw", method: "rsi2_4", template_version: 1 });
});

const spy = { ticker: "SPY", first: "2000-01-03", last: "2026-10-14", has_volume: true, series: [] };

it("a published snapshot opens as an unsaved draft on its own market and instrument", async () => {
  usTickers = [...tickersFixture, spy];
  const { host, f } = await go("#/indicators?published=12");
  expect(host.querySelector<HTMLSelectElement>(".lab-toolbar select")!.value).toBe("SPY");
  expect(host.querySelector("[data-lab-swap]")).toBeNull();
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.pathname).toBe("/api/us/lab/run");
  expect(url.searchParams.get("ticker")).toBe("SPY");
  expect(JSON.parse(url.searchParams.get("spec")!).origin.method).toBe("rsi2_4");
  expect(host.querySelector('[data-lab-action="save"]')).toBeNull();
});

it("a snapshot whose instrument is missing says which one replaced it", async () => {
  const { host } = await go("#/indicators?published=12");
  expect(host.querySelector<HTMLSelectElement>(".lab-toolbar select")!.value).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-swap]")?.textContent).toBe("SPY is not available here; showing NIFTY 50.");
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-swap]")).toBeNull();
});

it("changing the instrument clears the run result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-result]")).toBeNull();
});

it("a run answer that arrives after the instrument changed is dropped", async () => {
  let answer!: (r: Response) => void;
  runAnswer = () => new Promise<Response>((r) => { answer = r; });
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  await pick(host, "NIFTYBANK");
  await act(async () => answer(jsonResponse(runFixture)));
  await flush();
  expect(host.querySelector("[data-lab-result]")).toBeNull();
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.textContent).toBe("Run backtest");
});

it("changing the instrument clears the optimization result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLInputElement>("[data-lab-dim] input[type=checkbox]")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-optimize-result]")).not.toBeNull();
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
});

it("an unavailable template leaves the open snapshot named in the URL", async () => {
  templateAnswer = unavailableTemplateFixture;
  usTickers = [...tickersFixture, spy];
  const { host } = await go("#/indicators?published=12");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-method="rsi2_4"]')!.click());
  await flush();
  expect(host.textContent).toContain(unavailableTemplateFixture.status === "unavailable" ? unavailableTemplateFixture.reason : "");
  expect(location.hash).toContain("published=12");
  expect(location.hash).toContain("customize=rsi2_4");
});
