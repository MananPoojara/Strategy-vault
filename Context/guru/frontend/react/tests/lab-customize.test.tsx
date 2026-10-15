import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { CustomizePanel } from "../src/features/lab/CustomizePanel";
import { bookLabel, bookTitle, notAdmittedReason, requirementText } from "../src/features/lab/books";
import { specSummary } from "../src/features/lab/spec-model";
import type { Spec, TemplatesResponse } from "../src/features/lab/types";
import { catalogueFixture, fidelityFixture, flush, mount, templateFixture, templatesFixture, unavailableTemplateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const templateSpec = templateFixture.status === "available" ? templateFixture.template.spec : ({} as Spec);
const base = {
  market: "in" as const, ticker: "NIFTY 50", selected: "", templates: templatesFixture, template: null, onSelect: vi.fn(), onClear: vi.fn(), loadFidelity: vi.fn(),
  catalogue: catalogueFixture, spec: templateSpec, onSpec: vi.fn(), loadTemplate: vi.fn().mockResolvedValue(templateFixture),
  running: false, canRun: true, resultReady: false, onRun: vi.fn(), onSaveVariant: vi.fn(),
};
const click = (host: HTMLElement, sel: string) => act(async () => host.querySelector<HTMLButtonElement>(sel)!.click());

it("gl has no Customize", async () => {
  const host = await mount(<CustomizePanel {...base} market="gl" />);
  expect(host.textContent).toContain("Customize runs on India and US book methods only.");
});

it("lists templates with counts and searches by book", async () => {
  const host = await mount(<CustomizePanel {...base} />);
  expect(host.textContent).toContain("2 templates · 164 admitted methods · 4365 catalogued");
  const search = host.querySelector<HTMLInputElement>('input[aria-label="Search book methods"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "etf trading");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const buttons = [...host.querySelectorAll<HTMLButtonElement>("[data-lab-method]")];
  expect(buttons.map((b) => b.dataset.labMethod)).toEqual(["hpetf_rsi"]);
  await act(async () => buttons[0].click());
  expect(base.onSelect).toHaveBeenCalledWith("hpetf_rsi");
});

it("unavailable methods get the prefix once", async () => {
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={unavailableTemplateFixture} />);
  expect(host.textContent).toContain("Customize not available — the book method is not admitted");
  expect(host.textContent).not.toContain("Customize not available — Customize not available");
});

it("available template: label, requirement, notes; fidelity is display only", async () => {
  const loadFidelity = vi.fn().mockResolvedValue(fidelityFixture);
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={loadFidelity} />);
  expect(host.textContent).toContain("Book template · declared adaptation into lab rules · not a verification");
  expect(host.textContent).toContain("index or ETF (admits index, etf)");
  expect(host.textContent).toContain("Entry at the close, as in the book.");
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(loadFidelity).toHaveBeenCalledWith("rsi2_4", "NIFTY 50");
  expect(host.textContent).toContain("Template fidelity · display only · not a verification");
  expect(host.querySelector("[data-lab-fidelity-badge]")?.textContent).toBe("Reproduces the book engine's trades 12/12");
  expect(host.textContent).toContain("It does not verify the book method");
  expect(host.textContent!.replace(/does not verify/g, "")).not.toMatch(/\bverified\b/i);
});

it("0/0 is not a reproduction; differences are listed; refusals are shown as given", async () => {
  const empty = { ...fidelityFixture, book_trades: 0, template_trades: 0, matched: 0, reproduces: false };
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockResolvedValue(empty)} />);
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(host.textContent).toContain("No trades on this window, so there is nothing to compare.");
  expect(host.textContent).not.toContain("reproduces");
  await unmount();
  const diff = { ...fidelityFixture, reproduces: false, matched: 10, template_trades: 11, differences_total: 3,
    differences: [{ book: { entry: "2020-01-02", exit: "2020-01-05", entry_px: 1, exit_px: 2, dir: 1, legs: 1, open: false }, template: null }] };
  const host2 = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockResolvedValue(diff)} />);
  await click(host2, '[data-lab-action="fidelity"]');
  await flush();
  expect(host2.textContent).toContain("Matched 10 of 12 book trades and 11 template trades");
  expect(host2.textContent).toContain("3 differences in all");
  expect(host2.querySelector(".tlog table.tbl tbody tr")?.textContent).toBe("2020-01-022020-01-05——");
  await unmount();
  const refusal = Object.assign(new Error("Customize not available — the method is not admitted on NIFTY 50"), { status: 400, kind: "engine", engineStatus: "unsupported" });
  const host3 = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} loadFidelity={vi.fn().mockRejectedValue(refusal)} />);
  await click(host3, '[data-lab-action="fidelity"]');
  await flush();
  expect(host3.querySelector("[data-lab-error]")?.textContent).toContain("Customize not available — the method is not admitted on NIFTY 50");
});

it("changing the ticker clears the fidelity result and ignores a late answer", async () => {
  let resolve!: (r: typeof fidelityFixture) => void;
  const loadFidelity = vi.fn(() => new Promise<typeof fidelityFixture>((r) => { resolve = r; }));
  let setTicker!: (t: string) => void;
  function Host() {
    const [ticker, set] = useState("NIFTY 50");
    setTicker = set;
    return <CustomizePanel {...base} selected="rsi2_4" template={templateFixture} ticker={ticker} loadFidelity={loadFidelity} />;
  }
  const host = await mount(<Host />);
  await click(host, '[data-lab-action="fidelity"]');
  await act(async () => setTicker("BANKNIFTY"));
  await act(async () => resolve(fidelityFixture));
  await flush();
  expect(host.textContent).not.toContain("Template fidelity");
  expect(host.textContent).not.toContain("12/12");
  loadFidelity.mockResolvedValueOnce(fidelityFixture);
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(loadFidelity).toHaveBeenLastCalledWith("rsi2_4", "BANKNIFTY");
  expect(host.textContent).toContain("12/12");
});

// ---- plain-language Customize (book titles, summaries, admission, inline numbers, save prefill)
const smaCat = { ...catalogueFixture, indicators: [...catalogueFixture.indicators,
  { ...catalogueFixture.indicators[0], id: "sma", name: "Simple Moving Average" }] };
const rsi2: Spec = { v: 1, direction: "long", origin: { book: "spmt", method: "spmt_rsi2_4", template_version: 1 },
  entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "sma", p: { length: 200 } } }, { lhs: { s: "rsi", p: { length: 2 } }, op: "<", rhs: { value: 4 } }] },
  exit: { any: [{ lhs: { s: "rsi", p: { length: 2 } }, op: ">", rhs: { value: 65 } }, { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }], stop_loss: { pct: 3 } },
  execution: { fill: "close" } };

it("book ids map to title and author from the app catalogue, falling back to the id", () => {
  expect(bookLabel("spmt")).toBe("S&P Market Timing Course — Larry Connors");
  expect(bookLabel("hbqts")).toBe("Quantitative Trading Systems — Howard Bandy");
  expect(bookTitle("stc9")).toBe("Swing Trading College IX");
  expect(bookLabel("no_such_book")).toBe("no_such_book");
  expect(requirementText("individual_stock")).toBe("Stocks only");
  expect(requirementText("declared_index_timing")).toBe("Index timing (index or ETF)");
});

it("summarises a template spec in plain words", () => {
  expect(specSummary(rsi2, smaCat)).toBe(
    "Buy when the close is above the 200-day SMA and RSI(2) is below 4. Sell when RSI(2) is above 65 or after 5 days or at a 3% stop-loss.");
  const short: Spec = { ...rsi2, direction: "short", entry: { all: [rsi2.entry.all[1]], limit: { pct: 2 } }, exit: { any: [], target: { pct: 10 } } };
  expect(specSummary(short, smaCat)).toBe("Sell short when RSI(2) is below 4, using a limit order 2% above the signal close. Cover at a 10% profit target.");
});

const stockAndEtf: TemplatesResponse = { ...templatesFixture, templates: [
  ...templatesFixture.templates,
  { method: { id: "hbqts_ma5_25", name: "5/25 MA crossover", book: "hbqts", group: "Ch.5", dir: "Long" }, template_version: 1, requirement: "individual_stock", allowed_types: ["individual_stock"] },
] };

it("groups by book title and greys methods the selected instrument cannot run", async () => {
  expect(notAdmittedReason("individual_stock", ["individual_stock"], undefined)).toBe("Stocks only — pick a stock to run this");
  expect(notAdmittedReason("etf", ["etf"], "index")).toBe("ETF only — pick an ETF to run this");
  expect(notAdmittedReason("index_or_etf", ["index", "etf"], undefined)).toBeUndefined();
  const onSelect = vi.fn();
  const host = await mount(<CustomizePanel {...base} templates={stockAndEtf} onSelect={onSelect} />);
  expect([...host.querySelectorAll(".lab-book > summary b")].map((b) => b.textContent)).toContain("Quantitative Trading Systems — Howard Bandy");
  const stock = host.querySelector<HTMLButtonElement>('[data-lab-method="hbqts_ma5_25"]')!;
  expect(stock.disabled).toBe(true);
  expect(stock.textContent).toContain("Long · Stocks only");
  expect(stock.textContent).toContain("Stocks only — pick a stock to run this");
  await act(async () => stock.click());
  expect(onSelect).not.toHaveBeenCalled();
  expect(host.querySelector<HTMLButtonElement>('[data-lab-method="rsi2_4"]')!.disabled).toBe(false);
  await unmount();
  const onStock = await mount(<CustomizePanel {...base} templates={stockAndEtf} instrumentType="individual_stock" />);
  expect(onStock.querySelector<HTMLButtonElement>('[data-lab-method="hbqts_ma5_25"]')!.disabled).toBe(false);
  expect(onStock.querySelector<HTMLButtonElement>('[data-lab-method="rsi2_4"]')!.disabled).toBe(true);
});

it("card summaries load on hover, once per method, from the cached template", async () => {
  const loadTemplate = vi.fn().mockResolvedValue(templateFixture);
  const host = await mount(<CustomizePanel {...base} loadTemplate={loadTemplate} />);
  const li = host.querySelector('[data-lab-method="rsi2_4"]')!.parentElement!;
  await act(async () => { li.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); });
  await act(async () => { li.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); });
  await flush();
  expect(loadTemplate).toHaveBeenCalledTimes(1);
  expect(host.querySelector('[data-lab-method="rsi2_4"] .lab-m-sum')?.textContent).toBe("Buy when RSI(2) is below 4. Sell when the close is above SMA(5).");
});

it("step 2 edits a number in the shared spec and keeps the origin; steps 3 and 4 call the page", async () => {
  const onSpec = vi.fn(), onRun = vi.fn(), onSaveVariant = vi.fn();
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} onSpec={onSpec} onRun={onRun} onSaveVariant={onSaveVariant} />);
  expect([...host.querySelectorAll(".lab-steps > li > h4")].map((h) => h.textContent)).toEqual(["What it does", "Quick number edits", "Run it", "Save as my variant"]);
  expect(host.querySelector("[data-lab-summary]")?.textContent).toBe("Buy when RSI(2) is below 4. Sell when the close is above SMA(5).");
  expect(host.textContent).toContain("Edit indicators & rules");
  expect(host.querySelector<HTMLDetailsElement>(".lab-about")!.open).toBe(false);
  const input = host.querySelector<HTMLInputElement>('input[aria-label="Entry 1 · Threshold"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "3");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const next = onSpec.mock.calls[0][0] as Spec;
  expect(next.entry.all[0].rhs).toEqual({ value: 3 });
  expect(next.origin).toEqual(templateSpec.origin);
  expect(next.exit).toBe(templateSpec.exit);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="customize-run"]')!.click());
  expect(onRun).toHaveBeenCalled();
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="customize-save"]')!.click());
  expect(onSaveVariant).toHaveBeenCalledWith("My RSI 2 · 4");
  expect(host.textContent).toContain("Saved as: Customized from Short Term Trading Strategies That Work · RSI 2 · 4 · user variant · simulated");
  expect(host.textContent!.replace(/does not verify|not a verification/g, "")).not.toMatch(/\bverified\b/i);
});

it("a changed spec shows both versions; rules from elsewhere are not edited", async () => {
  const edited = { ...templateSpec, entry: { all: [{ ...templateSpec.entry.all[0], rhs: { value: 6 } }] } };
  const host = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} spec={edited} />);
  expect(host.querySelector("[data-lab-variant-summary]")?.textContent).toBe("Your version: Buy when RSI(2) is below 6. Sell when the close is above SMA(5).");
  await unmount();
  const other = await mount(<CustomizePanel {...base} selected="rsi2_4" template={templateFixture} spec={{ ...templateSpec, origin: undefined }} />);
  expect(other.querySelector("[data-lab-phrases]")).toBeNull();
  expect(other.querySelector<HTMLButtonElement>('[data-lab-action="customize-run"]')!.disabled).toBe(true);
});
