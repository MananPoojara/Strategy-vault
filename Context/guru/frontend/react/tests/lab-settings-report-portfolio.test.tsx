import { afterEach, expect, it, vi } from "vitest";
vi.mock("../src/features/lab/client", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/client")>()),
  tickers: vi.fn(async () => (await import("./fixtures/lab")).tickersFixture) }));
import { act } from "react";
import { ReportTab } from "../src/features/lab/settings/ReportTab";
import { CASH_BLOCKED, NOT_IN_LAB, ONE_INSTRUMENT } from "../src/features/lab/settings/common";
import { PortfolioTab } from "../src/features/lab/settings/PortfolioTab";
import { defaultSpec } from "../src/features/lab/spec-model";
import { withMode, type InstrumentContext } from "../src/features/lab/execution-model";
import type { Spec } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";
import { portfolioCatalogueFixture } from "./fixtures/portfolio";

afterEach(async () => { await unmount(); });
const stock: InstrumentContext = { kind: "individual_stock", market: "in" }, index: InstrumentContext = { kind: "index", market: "in" };
// found by attribute value: the test DOM's selector parser trips on "&" (Portfolio B&H symbol)
const input = (host: HTMLElement, name: string) => [...host.querySelectorAll<HTMLInputElement>("[aria-label]")].find((e) => e.getAttribute("aria-label") === name)!;
const row = (host: HTMLElement, name: string) => input(host, name).closest(".lab-row")!.textContent;
const type = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const portfolio: Spec = { ...withMode(defaultSpec(), "money"), portfolio: { universe: { basket: "nifty50" }, mode: "signals", max_open: 2 } };

it("Report tab: every AmiBroker option in order; rates edit execution.report, the rest show what the lab does with the reason", async () => {
  const onChange = vi.fn();
  const host = await mount(<ReportTab spec={defaultSpec()} instrument={index} onChange={onChange} />);
  const labels = [...host.querySelectorAll("[aria-label]")].map((e) => e.getAttribute("aria-label"));
  // only what the lab does, in AmiBroker's order; options it cannot do are left out
  expect(labels).toEqual(["Risk-free rate for Sharpe ratio (%)", "Risk-free rate for UPI (%)", "Always in the lab report"]);
  // the report's fixed contents read as facts, every AmiBroker option still named
  const fixed = [...host.querySelectorAll("[data-lab-report-fixed] li")].map((li) => li.textContent!.trim());
  expect(fixed).toEqual(["✓ Distribution charts spacing: 5 %", "✓ MAE/MFE distribution charts", "✓ Trade list in the report", "✓ Formula in the report", "✓ Buy-and-hold statistics of the tested instrument"]);
  // only the rates are editable
  expect([...host.querySelectorAll<HTMLInputElement>("input, select")].filter((e) => !e.disabled).map((e) => e.getAttribute("aria-label")))
    .toEqual(["Risk-free rate for Sharpe ratio (%)", "Risk-free rate for UPI (%)"]);
  await type(input(host, "Risk-free rate for UPI (%)"), "3");
  expect((onChange.mock.lastCall![0] as Spec).execution?.report).toEqual({ rf_upi: 3 });
});

it("Report tab on a portfolio: the report pages say single-instrument only", async () => {
  const host = await mount(<ReportTab spec={portfolio} instrument={stock} onChange={vi.fn()} />);
  const fixed = host.querySelector("[data-lab-report-fixed]")!;
  expect(fixed.textContent).toContain("single-instrument runs only");
  expect([...fixed.querySelectorAll("li")].every((li) => li.textContent!.trim().startsWith("–"))).toBe(true);
});

it("Portfolio tab: % returns keeps the money-only limits visible but disabled with the reason; crypto cannot use cash", async () => {
  const host = await mount(<PortfolioTab spec={defaultSpec()} catalogue={portfolioCatalogueFixture} instrument={{ kind: "crypto", market: "gl" }} onChange={vi.fn()} portfolio={<div data-mode-picker />} />);
  expect(host.firstElementChild!.nextElementSibling!.hasAttribute("data-mode-picker")).toBe(true); // strategy mode stays at the top
  const labels = [...host.querySelectorAll("[data-lab-portfolio-tab] [aria-label]")].map((e) => e.getAttribute("aria-label"));
  expect(labels).toEqual(["Max. Open Positions", "Limit trade size as % of entry bar volume (zero means no-limit)", "Disable trade size limit when bar volume is zero", "Use previous bar equity for position sizing"]);
  expect(input(host, "Disable trade size limit when bar volume is zero").checked).toBe(true); // AmiBroker's default, the engine's only behaviour
  expect([...host.querySelectorAll<HTMLInputElement>("[data-lab-portfolio-tab] input")].every((e) => e.disabled)).toBe(true);
  expect(row(host, "Max. Open Positions")).toContain("Needs a portfolio");
  expect(row(host, "Use previous bar equity for position sizing")).toContain(CASH_BLOCKED);
});

it("Portfolio tab in Cash amounts on one stock: volume limit (0 = none) and previous-bar equity write execution.limits", async () => {
  const onChange = vi.fn(), money = setLimits({ round_lot: 10 });
  const host = await mount(<PortfolioTab spec={money} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} />);
  await type(input(host, "Limit trade size as % of entry bar volume (zero means no-limit)"), "2.5");
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toEqual({ round_lot: 10, max_volume_pct: 2.5 });
  await act(async () => input(host, "Use previous bar equity for position sizing").click());
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toEqual({ round_lot: 10, prev_bar_equity: true });
  await unmount();
  const host2 = await mount(<PortfolioTab spec={setLimits({ max_volume_pct: 2.5 })} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} />);
  await type(input(host2, "Limit trade size as % of entry bar volume (zero means no-limit)"), "0");
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toBeUndefined();
});

it("Portfolio tab on a portfolio: Max. Open Positions edits portfolio.max_open within the lab's cap; limits are single-instrument", async () => {
  const onChange = vi.fn();
  const host = await mount(<PortfolioTab spec={portfolio} catalogue={portfolioCatalogueFixture} instrument={{ kind: "portfolio", market: "in" }} onChange={onChange} />);
  expect(input(host, "Max. Open Positions").max).toBe("50");
  expect(row(host, "Max. Open Positions")).toContain("The lab allows up to 50 (AmiBroker sheet: 500)");
  await type(input(host, "Max. Open Positions"), "7");
  expect((onChange.mock.lastCall![0] as Spec).portfolio?.max_open).toBe(7);
  expect(input(host, "Use previous bar equity for position sizing").disabled).toBe(true);
  expect(row(host, "Use previous bar equity for position sizing")).toContain(ONE_INSTRUMENT);
});

function setLimits(limits: NonNullable<NonNullable<Spec["execution"]>["limits"]>): Spec {
  const money = withMode(defaultSpec(), "money");
  return { ...money, execution: { ...money.execution, limits } };
}
