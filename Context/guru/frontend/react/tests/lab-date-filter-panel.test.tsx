import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { DateFilterPanel, defaultDates, type DateSetsState } from "../src/features/lab/DateFilterPanel";
import { BUILT_IN } from "../src/features/lab/builtin-periods";
import * as m from "../src/features/lab/spec-model";
import type { DateSet, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const set = (id: number, name: string, shared = false, editable = true): DateSet => ({ id, name, shared, editable, mine: !shared, version: 1, createdAt: "", updatedAt: "",
  windows: [{ from: "2008-01-01", to: "2008-01-31" }, { from: "2008-11-21", to: "2010-12-31", label: "Recovery" }] });
const q = <T extends Element>(sel: string) => document.querySelector<T>(sel)!;
const byText = (text: string) => [...document.querySelectorAll<HTMLElement>("button, label")].find((b) => b.textContent?.trim() === text)!;
const type = (el: HTMLInputElement | HTMLTextAreaElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const select = (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const click = (el: HTMLElement) => act(async () => el.click());

function setup(spec: Spec, list: DateSet[], over: Partial<DateSetsState> = {}) {
  const onChange = vi.fn();
  const api = { list: vi.fn(), create: vi.fn(async (d: { name: string; windows: DateSet["windows"]; shared?: boolean }) => ({ ...set(9, d.name, !!d.shared), windows: d.windows })),
    update: vi.fn(), remove: vi.fn(async () => null) };
  const sets: DateSetsState = { list, reload: vi.fn(async () => list), canSave: true, canShare: false, api: api as unknown as DateSetsState["api"], ...over };
  return { onChange, api, sets, render: () => mount(<DateFilterPanel spec={spec} onChange={onChange} sets={sets} />) };
}

const base2Bull = BUILT_IN[1].windows.filter((w) => w.label === "Bull").map((w) => [w.from, w.to]);
const base2Bear = BUILT_IN[1].windows.filter((w) => w.label === "Bear").map((w) => [w.from, w.to]);
const dated = (df = defaultDates()) => ({ ...m.defaultSpec(), date_filter: df }) as Spec;

it("Dates offers Base 1 and Base 2 built in; Bull / Bear sends those periods and the details list them", async () => {
  const t = setup(dated(), [set(1, "Base 1"), set(4, "Mine")]); // a saved set named like a built-in one is not listed twice
  await t.render();
  expect([...document.querySelectorAll("[aria-label='Dates'] button")].map((b) => b.textContent)).toEqual(["Base 1", "Base 2", "Mine"]);
  expect(q(".lab-datefilter-dates > summary").textContent).toBe("See the 14 bull periods");
  expect(document.querySelectorAll(".lab-datefilter-table tbody tr")).toHaveLength(14);
  await click(byText("Base 2"));
  expect(t.onChange.mock.lastCall![0].date_filter).toEqual({ set: { name: "Base 2" }, windows: base2Bull });
  await unmount();
  const u = setup(dated({ set: { name: "Base 2" }, windows: base2Bull as [string, string][] }), []);
  await u.render();
  await click(byText("Bear"));
  expect(u.onChange.mock.lastCall![0].date_filter).toEqual({ set: { name: "Base 2" }, windows: base2Bear });
});

it("an open trade closes when its period ends, with no box to tick; a strategy that turned it off can turn it back on", async () => {
  const t = setup(dated(), []);
  await t.render();
  expect(document.body.textContent).not.toContain("when the period ends");
  await unmount();
  const u = setup(dated({ ...defaultDates(), exit_at_change: false } as ReturnType<typeof defaultDates>), []);
  await u.render();
  const exit = [...document.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent?.includes("when the period ends"))!;
  await click(exit.querySelector("input")!);
  expect(u.onChange.mock.lastCall![0].date_filter.exit_at_change).toBeUndefined();
});

it("own dates: pasted in any format, saved for reuse and used for this strategy", async () => {
  const t = setup(dated(), []);
  await t.render();
  q<HTMLDetailsElement>("[data-lab-own-dates]").open = true;
  expect(q<HTMLButtonElement>("[data-lab-date-save]").disabled).toBe(true);
  await type(q("textarea[aria-label='Date windows']"), "Start, End\n01-01-2008, 31-01-2008\n21 Nov 2008 – 2010/12/31\nbear 2011-01-01 26.09.2011");
  expect(q("[data-lab-date-preview]").textContent).toContain("3 periods, 01/01/2008 to 26/09/2011");
  await type(q("input[aria-label='Date filter name']"), "Crashes");
  await click(q("[data-lab-date-save]"));
  expect(t.api.create).toHaveBeenCalledWith({ name: "Crashes", windows: [{ from: "2008-01-01", to: "2008-01-31", label: "Bull" }, { from: "2008-11-21", to: "2010-12-31", label: "Bull" }, { from: "2011-01-01", to: "2011-09-26", label: "Bear" }] });
  expect(t.onChange.mock.lastCall![0].date_filter).toEqual({ set: { id: 9, name: "Crashes" }, windows: [["2008-01-01", "2008-01-31"], ["2008-11-21", "2010-12-31"]] });
});

it("own dates that cannot be saved are still used for this strategy; bad lines say which line", async () => {
  const t = setup(dated(), [], { canSave: false });
  await t.render();
  q<HTMLDetailsElement>("[data-lab-own-dates]").open = true;
  await type(q("textarea[aria-label='Date windows']"), "31-01-2008, 01-01-2008\n30-02-2009, 01-03-2009");
  expect(document.querySelector("[role=alert]")!.textContent).toMatch(/Period 1: the start .* must be before the exit date/);
  expect(document.querySelector("[role=alert]")!.textContent).toMatch(/Line 2/);
  await type(q("textarea[aria-label='Date windows']"), "21/11/2008 31/12/2010");
  await click(q("[data-lab-date-save]"));
  expect(t.api.create).not.toHaveBeenCalled();
  expect(t.onChange.mock.lastCall![0].date_filter).toEqual({ set: { name: "My dates" }, windows: [["2008-11-21", "2010-12-31"]] });
});

it("an active date filter with its window-change exit counts as an exit; switched off or without that exit it does not", () => {
  const noExit = m.removeCond(m.defaultSpec(), "exit", 0);
  const dated = { ...noExit, date_filter: { windows: [["2008-01-01", "2008-01-31"]] as [string, string][] } };
  expect(m.specProblems(dated, cat)).toEqual([]);
  expect(m.specProblems({ ...dated, date_filter: { ...dated.date_filter, exit_at_change: false } }, cat).join()).toMatch(/Pick an exit/);
  expect(m.specProblems({ ...dated, date_filter: { ...dated.date_filter, off: true } }, cat).join()).toMatch(/Pick an exit/);
});
