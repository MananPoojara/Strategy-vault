import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { CustomizePanel } from "../src/features/lab/CustomizePanel";
import { fidelityFixture, flush, mount, templateFixture, templatesFixture, unavailableTemplateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const base = { market: "in" as const, ticker: "NIFTY 50", selected: "", templates: templatesFixture, template: null, onSelect: vi.fn(), onClear: vi.fn(), loadFidelity: vi.fn() };
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
  expect(host.textContent).toContain("index or ETF (index, etf)");
  expect(host.textContent).toContain("Entry at the close, as in the book.");
  await click(host, '[data-lab-action="fidelity"]');
  await flush();
  expect(loadFidelity).toHaveBeenCalledWith("rsi2_4", "NIFTY 50");
  expect(host.textContent).toContain("Template fidelity · display only · not a verification");
  expect(host.textContent).toContain("Template reproduces the book engine's trades: 12/12");
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
