import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import {
  catalogueFixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, publishedFixture, runFixture,
  seriesFixture, templateFixture, templatesFixture, tickersFixture, unmount,
} from "./fixtures/lab";

const me = (role: string) => jsonResponse({ user: { id: "u1", name: "N", email: "n@x", role }, settings: { theme: "aqua" } });
const series = (url: string) => { const id = new URL(url, "http://x").searchParams.get("id")!; return jsonResponse(seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] })); };
let runAnswer: () => Response;
const routes = (role = "researcher") => fetchRouter([
  ["/api/app/me", () => me(role)],
  ["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)],
  ["/api/app/lab/strategies/review", () => jsonResponse([])],
  ["/api/app/lab/strategies", () => jsonResponse({ error: "Saved strategies need the auth service; this local dev proxy has none." }, 503)],
  [/\/api\/(in|us|gl)\/lab\/catalogue/, () => jsonResponse(catalogueFixture)],
  [/\/api\/(in|us)\/tickers/, () => jsonResponse(tickersFixture)],
  ["/api/gl/lab/instruments", () => jsonResponse(glInstrumentsFixture)],
  [/\/lab\/series/, (url) => series(url)],
  [/\/lab\/templates/, () => jsonResponse(templatesFixture)],
  [/\/lab\/template\?/, () => jsonResponse(templateFixture)],
  [/\/lab\/run/, () => runAnswer()],
]);
beforeEach(() => { clearLabCache(); runAnswer = () => jsonResponse(runFixture); });
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); history.replaceState(null, "", "/"); });
const go = async (hash: string, role = "researcher") => {
  history.replaceState(null, "", `/${hash}`);
  const f = routes(role);
  vi.stubGlobal("fetch", f);
  const host = await mount(<IndicatorsPage />);
  await flush(); await flush();
  return { host, f };
};
const run = async (host: HTMLElement) => { await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.click()); await flush(); };

it("renders the catalogue label, library and default instrument; saved-strategy errors stay local", async () => {
  const { host } = await go("#/indicators");
  expect(host.querySelector('[data-lab-page="indicators"]')).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(catalogueFixture.label);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(catalogueFixture.indicators.length);
  expect(host.querySelector<HTMLSelectElement>(".lab-toolbar select")!.value).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("auth service");
});

it("a failed run clears the previous result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  runAnswer = () => jsonResponse({ status: "needs_data", error: "NIFTY 50: invalid OHLC bar on 2001-03-02", label: catalogueFixture.label }, 400);
  await run(host);
  expect(host.querySelector("[data-lab-result]")).toBeNull();
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

it("a published snapshot opens as an unsaved draft on its market", async () => {
  const { host, f } = await go("#/indicators?region=us&tk=SPY&published=12");
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.pathname).toBe("/api/us/lab/run");
  expect(JSON.parse(url.searchParams.get("spec")!).origin.method).toBe("rsi2_4");
  expect(host.querySelector('[data-lab-action="save"]')).toBeNull();
});
