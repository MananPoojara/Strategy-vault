import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { DateField, dayFirst } from "../src/features/lab/DateField";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const box = (label: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
const type = (input: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
});

it("shows an ISO value day-first and sends ISO back", async () => {
  const onChange = vi.fn();
  await mount(<DateField label="From" value="2015-04-01" onChange={onChange} />);
  expect(box("From").value).toBe("01-04-2015");
  expect(box("From").placeholder).toBe("dd-mm-yyyy");
  await type(box("From"), "31/12/2020");
  expect(onChange).toHaveBeenLastCalledWith("2020-12-31");
  await type(box("From"), "");
  expect(onChange).toHaveBeenLastCalledWith("");
});
it("does not send impossible or half-typed dates and says why", async () => {
  const onChange = vi.fn();
  await mount(<DateField label="To" value="" onChange={onChange} />);
  await type(box("To"), "31-02-2020");
  await type(box("To"), "12-0");
  expect(onChange).not.toHaveBeenCalled();
  expect(document.querySelector('[role="alert"]')!.textContent).toMatch(/dd-mm-yyyy/);
});
it("formats only full ISO dates", () => {
  expect(dayFirst("2008-11-21")).toBe("21-11-2008");
  expect(dayFirst("")).toBe("");
});
