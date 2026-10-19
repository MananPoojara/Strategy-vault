import { afterEach, expect, it, vi } from "vitest";
import { Toolbar } from "../src/features/lab/Toolbar";
import { ErrorNotice } from "../src/features/lab/ui";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const props = { market: "us" as const, onMarket: vi.fn(), ticker: "", onTicker: vi.fn(), tf: "D" as const, timeframes: ["D" as const], onTf: vi.fn(), range: "3Y" as const, onRange: vi.fn() };

it("a market without instruments says so; the chart window control is named Chart range", async () => {
  const host = await mount(<Toolbar {...props} options={[]} />);
  const select = host.querySelector<HTMLSelectElement>(".lab-toolbar select")!;
  expect(select.disabled).toBe(true);
  expect(select.textContent).toBe("No instruments available");
  expect(host.querySelector('[role="group"][aria-label="Chart range"]')).not.toBeNull();
  expect(host.textContent).toContain("Chart range");
});

it("a 503 from the gateway reads as an unavailable market, not a refusal", async () => {
  const error = Object.assign(new Error("No local us engine is configured (LAB_ENGINE_US)."), { status: 503, kind: "gateway" });
  const host = await mount(<ErrorNotice error={error} market="us" />);
  expect(host.querySelector("[data-lab-error] b")!.textContent).toBe("Market not available here");
});
