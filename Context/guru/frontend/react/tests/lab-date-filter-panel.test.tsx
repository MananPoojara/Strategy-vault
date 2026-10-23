import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { DateFilterPanel, type DateSetsState } from "../src/features/lab/DateFilterPanel";
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

it("picking a set puts its windows into the strategy as pairs; 'No date filter' removes it", async () => {
  const t = setup(m.defaultSpec(), [set(3, "Base 2", true, false), set(4, "Mine")]);
  await t.render();
  await select(q("select[aria-label='Date filter set']"), "3");
  expect(t.onChange).toHaveBeenLastCalledWith(expect.objectContaining({ date_filter: { set: { id: 3, name: "Base 2" }, windows: [["2008-01-01", "2008-01-31"], ["2008-11-21", "2010-12-31"]] } }));
  await unmount();
  const s = { ...m.defaultSpec(), date_filter: { set: { id: 3, name: "Base 2" }, windows: [["2008-01-01", "2008-01-31"]] as [string, string][] } };
  const u = setup(s, [set(3, "Base 2", true, false)]);
  await u.render();
  await select(q("select[aria-label='Date filter set']"), "");
  expect(u.onChange.mock.lastCall![0].date_filter).toBeUndefined();
});

it("mode, window-change exit and on/off edit the strategy's date filter", async () => {
  const s = { ...m.defaultSpec(), date_filter: { set: { id: 3, name: "Base 2" }, windows: [["2008-01-01", "2008-01-31"]] as [string, string][] } };
  const t = setup(s, [set(3, "Base 2", true, false)]);
  await t.render();
  await click(byText("Trade only outside the windows"));
  expect(t.onChange.mock.lastCall![0].date_filter.mode).toBe("outside");
  await click(q<HTMLInputElement>("input[aria-label='Date filter on']"));
  expect(t.onChange.mock.lastCall![0].date_filter.off).toBe(true);
  const exit = [...document.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent?.includes("when a window ends"))!;
  await click(exit.querySelector("input")!);
  expect(t.onChange.mock.lastCall![0].date_filter.exit_at_change).toBe(false);
});

it("upload: pasted day-first dates are previewed, saved as a personal set and chosen for the strategy", async () => {
  const t = setup(m.defaultSpec(), []);
  await t.render();
  await click(q("[data-lab-date-manage]"));
  expect(q<HTMLButtonElement>("[data-lab-date-save]").disabled).toBe(true);
  await type(q("textarea[aria-label='Date windows']"), "Start, End, Name\n01-01-2008, 31-01-2008, Crash\n21/11/2008\t31/12/2010");
  expect(q("[data-lab-date-preview]").textContent).toContain("2 windows, 01-01-2008 to 31-12-2010");
  expect(q<HTMLButtonElement>("[data-lab-date-save]").disabled).toBe(true); // no name yet
  await type(q("input[aria-label='Date filter name']"), " Base 2 ");
  await click(q("[data-lab-date-save]"));
  expect(t.api.create).toHaveBeenCalledWith({ name: "Base 2", windows: [{ from: "2008-01-01", to: "2008-01-31", label: "Crash" }, { from: "2008-11-21", to: "2010-12-31" }] });
  expect(t.sets.reload).toHaveBeenCalled();
  expect(t.onChange.mock.lastCall![0].date_filter).toEqual({ set: { id: 9, name: "Base 2" }, windows: [["2008-01-01", "2008-01-31"], ["2008-11-21", "2010-12-31"]] });
  expect(document.body.textContent).toContain("Saved “Base 2”");
});

it("bad lines block saving and say which line; AmiBroker start/end lists are paired in order", async () => {
  const t = setup(m.defaultSpec(), []);
  await t.render();
  await click(q("[data-lab-date-manage]"));
  await type(q("input[aria-label='Date filter name']"), "X");
  await type(q("textarea[aria-label='Date windows']"), "31-01-2008, 01-01-2008\n30-02-2009, 01-03-2009");
  expect(document.querySelector("[role=alert]")!.textContent).toMatch(/Window 1: the start .* must be before the exit date/);
  expect(document.querySelector("[role=alert]")!.textContent).toMatch(/Line 2/);
  expect(q<HTMLButtonElement>("[data-lab-date-save]").disabled).toBe(true);
  await click(byText("AmiBroker start / end lists"));
  await type(q("textarea[aria-label='Start dates']"), 'BASE2_Entry = "01-01-2008, 21-11-2008";');
  await type(q("textarea[aria-label='End dates']"), '"31-01-2008, 31-12-2010"');
  expect(q("[data-lab-date-preview]").textContent).toContain("2 windows");
  expect(q<HTMLButtonElement>("[data-lab-date-save]").disabled).toBe(false);
});

it("only admins see the share option; viewers cannot upload; delete needs a second click", async () => {
  const admin = setup(m.defaultSpec(), [set(4, "Mine")], { canShare: true });
  await admin.render();
  await click(q("[data-lab-date-manage]"));
  expect(document.body.textContent).toContain("Share with everyone");
  await click(q("button[aria-label='Delete Mine']"));
  expect(admin.api.remove).not.toHaveBeenCalled();
  await click(byText("Yes, delete"));
  expect(admin.api.remove).toHaveBeenCalledWith(4, 1);
  await unmount();
  const viewer = setup(m.defaultSpec(), [set(3, "Base 2", true, false)], { canSave: false });
  await viewer.render();
  await click(q("[data-lab-date-manage]"));
  expect(document.querySelector("textarea")).toBeNull();
  expect(document.body.textContent).toContain("cannot upload");
});

it("an active date filter with its window-change exit counts as an exit; switched off or without that exit it does not", () => {
  const noExit = m.removeCond(m.defaultSpec(), "exit", 0);
  const dated = { ...noExit, date_filter: { windows: [["2008-01-01", "2008-01-31"]] as [string, string][] } };
  expect(m.specProblems(dated, cat)).toEqual([]);
  expect(m.specProblems({ ...dated, date_filter: { ...dated.date_filter, exit_at_change: false } }, cat).join()).toMatch(/exit.any needs/);
  expect(m.specProblems({ ...dated, date_filter: { ...dated.date_filter, off: true } }, cat).join()).toMatch(/exit.any needs/);
});
