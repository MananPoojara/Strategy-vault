import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { YtStrategiesPage } from "../src/pages/YtStrategiesPage";
import { clearLabCache } from "../src/features/lab/client";
import { fetchRouter, flush, jsonResponse, mount, publishedFixture, unmount } from "./fixtures/lab";

beforeEach(clearLabCache);
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); });

it("lists published strategies newest first with open-in-lab links and variant badges", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)]]));
  const host = await mount(<YtStrategiesPage />);
  await flush();
  const cards = [...host.querySelectorAll(".lab-card")];
  expect(cards.map((c) => c.querySelector("h2")!.textContent)).toEqual(["RSI 2 variant", "EMA pullback"]);
  expect(cards[0].textContent).toContain("Customized from ststw · rsi2_4");
  expect(cards[0].textContent).toContain("Former member");
  expect(cards[0].querySelector("a")!.getAttribute("href")).toBe("#/indicators?region=us&tk=SPY&published=12");
  expect(cards[1].querySelector("a")!.getAttribute("href")).toBe("#/indicators?tk=NIFTY+50&published=11");
  expect(host.textContent).toContain("not a book method and not a verification");
});

it("shows rule counts, owner, market and publish date; no badge without origin", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)]]));
  const host = await mount(<YtStrategiesPage />);
  await flush();
  const ema = [...host.querySelectorAll(".lab-card")][1];
  const s = publishedFixture[0].spec;
  expect(ema.textContent).toContain("Nandini · India · NIFTY 50");
  expect(ema.textContent).toContain(`${s.entry.all.length} entry / ${s.exit.any.length} exit · published 2026-10-13`);
  expect(ema.querySelector(".tag")).toBeNull();
  expect(host.querySelector('[data-lab-page="yt-strategies"]')).not.toBeNull();
});

it("empty and error states", async () => {
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse([])]]));
  const host = await mount(<YtStrategiesPage />);
  await flush();
  expect(host.textContent).toContain("Nothing is published yet.");
  await unmount();
  vi.stubGlobal("fetch", fetchRouter([["/api/app/lab/strategies/published", () => jsonResponse({ error: "Please sign in." }, 401)]]));
  const host2 = await mount(<YtStrategiesPage />);
  await flush();
  expect(host2.querySelector("[data-lab-error]")?.textContent).toContain("Please sign in.");
  expect(host2.querySelector(".lab-card")).toBeNull();
});
