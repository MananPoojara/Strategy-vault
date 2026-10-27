import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { OperandEditor } from "../src/features/lab/OperandEditor";
import { catalogue7Fixture as cat, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const type = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const key = async (el: Element, k: string) => act(async () => { el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })); });

it("a rule's indicator is searched by typing any part of its name; arrows and Enter pick it, Escape closes", async () => {
  const onChange = vi.fn();
  const host = await mount(<OperandEditor operand={{ s: "rsi", p: { length: 14 } }} side="entry" catalogue={cat} onChange={onChange} label="Entry 1 left" />);
  const button = host.querySelector<HTMLButtonElement>('button[aria-label="Indicator"]')!;
  expect(button.getAttribute("data-value")).toBe("rsi");
  await act(async () => button.click());
  const search = host.querySelector<HTMLInputElement>('input[aria-label="Search indicators"]')!;
  expect(host.querySelector(".lab-ind-group")!.textContent).toBe("Popular"); // nothing typed: Popular first, then the categories
  await type(search, "moving");
  expect([...host.querySelectorAll('[role="option"]')].map((o) => o.getAttribute("data-id"))).toEqual(["ema"]); // "Exponential Moving Average"
  await type(search, "osc");
  expect([...host.querySelectorAll('[role="option"]')].map((o) => o.getAttribute("data-id"))).toEqual(expect.arrayContaining(["rsi", "macd", "stoch"])); // by category
  await type(search, "macd");
  await key(search, "Enter");
  expect(onChange.mock.calls.at(-1)![0]).toEqual({ s: "macd" }); // a new indicator drops the old parameters
  expect(host.querySelector('input[aria-label="Search indicators"]')).toBeNull();
  await act(async () => button.click());
  await type(host.querySelector<HTMLInputElement>('input[aria-label="Search indicators"]')!, "zzz");
  expect(host.querySelector(".lab-ind-none")!.textContent).toContain("No indicator matches");
  await key(host.querySelector('input[aria-label="Search indicators"]')!, "Escape");
  expect(host.querySelector('input[aria-label="Search indicators"]')).toBeNull();
});
