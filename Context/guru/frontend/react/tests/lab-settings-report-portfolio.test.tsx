import { afterEach, expect, it, vi } from "vitest";
vi.mock("../src/features/lab/client", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/client")>()),
  tickers: vi.fn(async () => (await import("./fixtures/lab")).tickersFixture) }));
import { act } from "react";
import { ReportTab } from "../src/features/lab/settings/ReportTab";
import { CASH_BLOCKED, NOT_IN_LAB, ONE_INSTRUMENT } from "../src/features/lab/settings/common";
import { PortfolioTab } from "../src/features/lab/settings/PortfolioTab";
import { BacktestParams } from "../src/features/lab/settings/BacktestParams";
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

it("Portfolio tab on one instrument: only the single-instrument settings; % returns keeps them visible but disabled with the reason; crypto cannot use cash", async () => {
  const host = await mount(<PortfolioTab spec={defaultSpec()} catalogue={portfolioCatalogueFixture} instrument={{ kind: "crypto", market: "gl" }} onChange={vi.fn()} portfolio={<div data-mode-picker />} />);
  expect(host.querySelector(".lab-portfolio-card-body")!.firstElementChild!.hasAttribute("data-mode-picker")).toBe(true); // strategy mode stays at the top
  expect(host.querySelector("[data-lab-portfolio-tab] legend")!.textContent).toBe("Single-instrument settings");
  const labels = [...host.querySelectorAll("[data-lab-portfolio-tab] [aria-label]")].map((e) => e.getAttribute("aria-label"));
  expect(labels).toEqual(["Limit trade size as % of entry bar volume (zero means no-limit)", "Disable trade size limit when bar volume is zero"]); // Maximum position and previous-bar equity are in Backtest parameters
  expect(input(host, "Disable trade size limit when bar volume is zero").checked).toBe(true); // AmiBroker's default, the engine's only behaviour
  expect([...host.querySelectorAll<HTMLInputElement>("[data-lab-portfolio-tab] input")].every((e) => e.disabled)).toBe(true);
  expect(row(host, "Limit trade size as % of entry bar volume (zero means no-limit)")).toContain(CASH_BLOCKED);
});

it("Portfolio tab in Cash amounts on one stock: the volume limit (0 = none) writes execution.limits", async () => {
  const onChange = vi.fn(), money = setLimits({ round_lot: 10 });
  const host = await mount(<PortfolioTab spec={money} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} />);
  await type(input(host, "Limit trade size as % of entry bar volume (zero means no-limit)"), "2.5");
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toEqual({ round_lot: 10, max_volume_pct: 2.5 });
  await unmount();
  const host2 = await mount(<PortfolioTab spec={setLimits({ max_volume_pct: 2.5 })} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} />);
  await type(input(host2, "Limit trade size as % of entry bar volume (zero means no-limit)"), "0");
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toBeUndefined();
});

it("Portfolio tab on a portfolio: no settings of its own (Maximum position and previous-bar equity are in Backtest parameters)", async () => {
  const host = await mount(<PortfolioTab spec={portfolio} catalogue={portfolioCatalogueFixture} instrument={{ kind: "portfolio", market: "in" }} onChange={vi.fn()} portfolio={<div data-mode-picker />} />);
  expect(host.querySelector("[data-lab-portfolio-tab]")).toBeNull();
  expect(host.querySelector("[data-mode-picker]")).not.toBeNull();
});

it("Backtest parameters: previous-bar equity writes execution.limits, for a portfolio and in Cash amounts on one stock; % returns greys it", async () => {
  const onChange = vi.fn();
  const host = await mount(<BacktestParams spec={portfolio} catalogue={portfolioCatalogueFixture} instrument={{ kind: "portfolio", market: "in" }} onChange={onChange} unitCosts={null} orders={null} />);
  await type(input(host, "Maximum position"), "7");
  expect((onChange.mock.lastCall![0] as Spec).portfolio?.max_open).toBe(7);
  await act(async () => input(host, "Use previous bar equity for position sizing").click());
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toEqual({ prev_bar_equity: true });
  await unmount();
  const host2 = await mount(<BacktestParams spec={setLimits({ round_lot: 10 })} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} unitCosts={null} orders={null} />);
  await act(async () => input(host2, "Use previous bar equity for position sizing").click());
  expect((onChange.mock.lastCall![0] as Spec).execution?.limits).toEqual({ round_lot: 10, prev_bar_equity: true });
  await unmount();
  const host3 = await mount(<BacktestParams spec={defaultSpec()} catalogue={portfolioCatalogueFixture} instrument={{ kind: "crypto", market: "gl" }} onChange={onChange} unitCosts={null} orders={null} />);
  expect(input(host3, "Use previous bar equity for position sizing").disabled).toBe(true);
});

function setLimits(limits: NonNullable<NonNullable<Spec["execution"]>["limits"]>): Spec {
  const money = withMode(defaultSpec(), "money");
  return { ...money, execution: { ...money.execution, limits } };
}

it("Backtest parameters: Turnover filter Yes adds a Years button opening the year-wise minimums (AFL's by default); edits, an added year and a reset write execution.turnover_floor", async () => {
  const onChange = vi.fn();
  const on: Spec = { ...defaultSpec(), execution: { ...defaultSpec().execution, turnover_filter: true } };
  const host = await mount(<BacktestParams spec={on} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} unitCosts={null} orders={null} />);
  expect(host.querySelector("[data-lab-turnover-years]")).toBeNull(); // hidden until the user asks for it
  const years = () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent!.startsWith("Years"))!;
  await act(async () => years().click());
  const box = host.querySelector("[role=dialog] [data-lab-turnover-years]")!;
  expect(box.textContent).toContain("AFL default");
  const cells = [...box.querySelectorAll("li > span")].map((x) => x.textContent);
  expect(cells).toEqual(["Up to 1999", "2000–2003", "2004", "2005", "2006", "2007–2009", "2010", "2011", "2012", "2013", "2014", "2015–2016", "2017", "2018", "2019", "2020", "2021", "2022 on"]);
  expect(input(host, "Minimum for 2004").value).toBe("200000");
  await type(input(host, "Minimum for 2004"), "250000");
  const edited = (onChange.mock.lastCall![0] as Spec).execution?.turnover_floor!;
  expect(edited[2]).toEqual([2004, 250000]);
  expect(edited).toHaveLength(18);
  await unmount();
  // an added year starts at that year's minimum and sorts into place
  const host2 = await mount(<BacktestParams spec={on} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} unitCosts={null} orders={null} />);
  await act(async () => [...host2.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent!.startsWith("Years"))!.click());
  await type(input(host2, "Add a year"), "2008");
  await act(async () => [...host2.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Add year")!.click());
  const added = (onChange.mock.lastCall![0] as Spec).execution?.turnover_floor!;
  expect(added.slice(5, 8)).toEqual([[2007, 800000], [2008, 800000], [2010, 1000000]]);
  await unmount();
  // a saved table shows as edited; Reset removes it
  const mine: Spec = { ...on, execution: { ...on.execution, turnover_floor: [[0, 1000], [2010, 5000]] } };
  const host3 = await mount(<BacktestParams spec={mine} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={onChange} unitCosts={null} orders={null} />);
  await act(async () => [...host3.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent!.startsWith("Years"))!.click());
  expect([...host3.querySelectorAll("[data-lab-turnover-years] li > span")].map((x) => x.textContent)).toEqual(["Up to 2009", "2010 on"]);
  await act(async () => [...host3.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Reset to AFL values")!.click());
  expect((onChange.mock.lastCall![0] as Spec).execution?.turnover_floor).toBeUndefined();
  await unmount();
  const off = await mount(<BacktestParams spec={defaultSpec()} catalogue={portfolioCatalogueFixture} instrument={stock} onChange={vi.fn()} unitCosts={null} orders={null} />);
  expect(off.querySelector("[data-lab-turnover-years]")).toBeNull();
  expect([...off.querySelectorAll("button")].some((b) => b.textContent!.startsWith("Years"))).toBe(false); // no Years button while the filter is off
});
