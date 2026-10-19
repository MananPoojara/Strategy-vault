import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ReportSettings } from "../src/features/lab/ReportSettings";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const type = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const field = (host: HTMLElement, name: string) => host.querySelector<HTMLInputElement>(`input[aria-label="${name}"]`)!;
const SHARPE = "Risk-free rate for Sharpe ratio (%)", UPI = "Risk-free rate for UPI (%)";

it("Settings › Report: AmiBroker's two risk-free rates write execution.report; emptying both removes it", async () => {
  const onChange = vi.fn();
  const host = await mount(<ReportSettings spec={defaultSpec()} onChange={onChange} />);
  expect([field(host, SHARPE).placeholder, field(host, UPI).placeholder]).toEqual(["5", "5.4"]);
  expect(host.textContent).toContain("empty uses AmiBroker's defaults (5 % and 5.4 %)");
  await type(field(host, SHARPE), "2.5");
  expect((onChange.mock.lastCall![0] as Spec).execution?.report).toEqual({ rf_sharpe: 2.5 });
  await unmount();
  const set: Spec = { ...defaultSpec(), execution: { ...defaultSpec().execution, report: { rf_sharpe: 2.5 } } };
  const host2 = await mount(<ReportSettings spec={set} onChange={onChange} />);
  expect(field(host2, SHARPE).value).toBe("2.5");
  await type(field(host2, SHARPE), "");
  expect((onChange.mock.lastCall![0] as Spec).execution?.report).toBeUndefined();
});

it("a portfolio spec disables the rates (single-instrument report only) and offers to remove stale ones", async () => {
  const onChange = vi.fn();
  const spec: Spec = { ...defaultSpec(), execution: { capital: 1000, report: { rf_upi: 3 } }, portfolio: { universe: { basket: "nifty50" }, mode: "signals", max_open: 2 } };
  const host = await mount(<ReportSettings spec={spec} onChange={onChange} />);
  expect([field(host, SHARPE).disabled, field(host, UPI).disabled]).toEqual([true, true]);
  expect(host.textContent).toContain("single-instrument runs only");
  await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
  expect((onChange.mock.lastCall![0] as Spec).execution?.report).toBeUndefined();
});
