import {afterEach, expect, it, vi} from "vitest";
import {act} from "react";
import {createRoot, type Root} from "react-dom/client";
import {StrategyPage} from "../src/pages/ResearchPages";
import type {ViewSnapshot} from "../src/state/store";

let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
});
const snapshot = (state: Record<string, string>): ViewSnapshot => ({
  revision: 1, loading: false, html: '<header><span data-react-slot="strategy-settings"></span></header>',
  error: null, route: null, status: "Strategy", state, home: null, reading: null,
});
async function mount(state: Record<string, string>) {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const patch = vi.fn();
  window.VaultRuntime = {patch} as unknown as Window["VaultRuntime"];
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<StrategyPage snapshot={snapshot(state)} />));
  await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());
  return patch;
}
const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]')!;
const radio = (value: string) => dialog().querySelector<HTMLInputElement>(`input[value="${value}"]`)!;
const button = (text: string) => [...dialog().querySelectorAll<HTMLButtonElement>("button")].find(b => b.textContent === text)!;

it("uses the committed report mode and applies only the chosen display parameter", async () => {
  const patch = await mount({report_mode: "compound", tk: "basket:nifty50:RELIANCE", slippage: "12"});
  expect(radio("compound").checked).toBe(true);
  await act(async () => radio("fixed").click());
  await act(async () => button("Apply").click());
  expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode: "fixed"});
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});
it("defaults an unknown display mode to fixed and Cancel leaves the route and result unchanged", async () => {
  const patch = await mount({report_mode: "invalid"});
  expect(radio("fixed").checked).toBe(true);
  await act(async () => radio("compound").click());
  await act(async () => button("Cancel").click());
  expect(patch).not.toHaveBeenCalled();
});
it("keeps the basket's portfolio accounting and never offers closing-order trade compounding", async () => {
  const patch = await mount({run_scope: "basket", report_mode: "fixed"});
  expect(dialog().textContent).toContain("equal-slot portfolio calculation");
  expect(radio("compound").matches(":disabled")).toBe(true);
  expect(radio("fixed").matches(":disabled")).toBe(true);
  expect(button("Apply").disabled).toBe(true);
  expect(document.activeElement).toBe(dialog().querySelector('[aria-label="Close strategy settings"]'));
  await act(async () => button("Apply").click());
  expect(patch).not.toHaveBeenCalled();
});
