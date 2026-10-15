import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { RuleList } from "../src/features/lab/NodeEditor";
import { nodeKind } from "../src/features/lab/rule-tree";
import type { Cond, RuleNode } from "../src/features/lab/types";
import { catalogue7Fixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const C: Cond = { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } };
const setup = async (list: RuleNode[], extra: Partial<Parameters<typeof RuleList>[0]> = {}) => {
  await unmount();
  const onChange = vi.fn();
  const host = await mount(<RuleList list={list} onChange={onChange} listPath="entry.all" title="Entry · all of these" tag="Entry" catalogue={catalogue7Fixture}
    caps={{ leaves: false, nodes: false, top: false }} {...extra} />);
  return { host, onChange, last: () => onChange.mock.calls.at(-1)?.[0] as RuleNode[] };
};
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const type = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const click = async (el: HTMLElement) => act(async () => el.click());

it("renders every node kind with its kind attribute", async () => {
  const list: RuleNode[] = [C, { lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } }, { lhs: { s: "close" }, op: "rising", bars: 3 },
    { all: [C] }, { any: [C] }, { not: C }, { at_least: 2, of: 3, when: C }, { then: [C, C], within: 5 }];
  const { host } = await setup(list);
  expect([...host.querySelectorAll("[data-lab-list='entry.all'] > [data-lab-node]")].map((n) => n.getAttribute("data-lab-kind")))
    .toEqual(["cmp", "range", "trend", "all", "any", "not", "at_least", "then"]);
});

it("the add menu appends each kind and resets", async () => {
  const { host, onChange } = await setup([C]);
  const add = host.querySelector<HTMLSelectElement>("[data-lab-list='entry.all'] > select[data-lab-add]")!;
  for (const k of ["cond", "all", "any", "not", "at_least", "then"]) await choose(add, k);
  // the list prop is static (onChange is a spy), so every call appends to [C]
  expect(onChange.mock.calls.map(([l]) => (l as RuleNode[]).map(nodeKind)[1])).toEqual(["cmp", "all", "any", "not", "at_least", "then"]);
  expect(add.value).toBe("");
});

it("the operator select switches comparison, range and trend", async () => {
  const { host, last } = await setup([C]);
  await choose(host.querySelector('select[aria-label="Operator"]')!, "between");
  expect(nodeKind(last()[0])).toBe("range");
  const r = await setup([{ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } }]);
  await choose(r.host.querySelector('select[aria-label="Operator"]')!, "rising");
  expect(r.last()[0]).toEqual({ lhs: { s: "rsi" }, op: "rising", bars: 3 });
});

it("node menu wraps, negates, unwraps and moves; remove deletes", async () => {
  const { host, last } = await setup([C, { lhs: { s: "close" }, op: "<", rhs: { value: 5 } }]);
  const menu = () => host.querySelectorAll<HTMLSelectElement>("select[data-lab-node-menu]");
  await choose(menu()[0], "wrap_any");
  expect(last()[0]).toEqual({ any: [C] });
  await choose(menu()[0], "wrap_not");
  expect(last()[0]).toEqual({ not: C });
  await choose(menu()[1], "up");
  expect(last()[0]).toMatchObject({ rhs: { value: 5 } });
  expect([...menu()[0].options].find((o) => o.value === "up")!.disabled).toBe(true);
  await click(host.querySelector('button[aria-label="Remove Entry 1"]')!);
  expect(last()).toHaveLength(1);
  const g = await setup([{ all: [C] }]);
  await choose(g.host.querySelector("select[data-lab-node-menu]")!, "unwrap");
  expect(g.last()[0]).toEqual(C);
});

it("groups edit children, collapse to a summary, and at_least / then edit their numbers", async () => {
  const { host, last } = await setup([{ any: [C] }, { at_least: 2, of: 3, when: C }, { then: [C, C], within: 5 }]);
  const group = host.querySelector("[data-lab-kind='any']")!;
  await choose(group.querySelector<HTMLSelectElement>(".lab-group-body > select[data-lab-add]")!, "cond");
  expect((last()[0] as { any: RuleNode[] }).any).toHaveLength(2);
  await click(group.querySelector<HTMLButtonElement>("button[aria-expanded]")!);
  expect(group.querySelector(".lab-group-body")).toBeNull();
  expect(group.textContent).toContain("Any of 1: close is above Exponential Moving Average(50)");
  await type(host.querySelector('input[aria-label="at least"]')!, "3");
  expect(last()[1]).toMatchObject({ at_least: 3, of: 3 });
  await type(host.querySelector('input[aria-label="within (bars)"]')!, "8");
  expect(last()[2]).toMatchObject({ within: 8 });
  const steps = host.querySelector("[data-lab-kind='then'] .lab-group-body > select[data-lab-add]") as HTMLSelectElement;
  expect(steps).not.toBeNull();
});

it("caps disable adding and wrapping", async () => {
  const { host } = await setup([C], { caps: { leaves: true, nodes: false, top: false } });
  expect(host.querySelector<HTMLOptionElement>("select[data-lab-add] option[value='cond']")!.disabled).toBe(true);
  const n = await setup([C], { caps: { leaves: false, nodes: true, top: false } });
  expect(n.host.querySelector<HTMLOptionElement>("select[data-lab-node-menu] option[value='wrap_all']")!.disabled).toBe(true);
  const top = await setup([C], { caps: { leaves: false, nodes: false, top: true } });
  expect(top.host.querySelector<HTMLOptionElement>("[data-lab-list] > select[data-lab-add] option[value='any']")!.disabled).toBe(true);
});

it("unknown nodes are kept read-only and can be removed", async () => {
  const { host, last } = await setup([{ fancy: 1 } as never, C]);
  expect(host.querySelector("[data-lab-kind='unknown'] .lab-kept")!.textContent).toContain('{"fancy":1}');
  await click(host.querySelector('button[aria-label="Remove Entry 1"]')!);
  expect(last()).toEqual([C]);
});

it("bars held is offered only in top-level exit comparisons", async () => {
  const { host } = await setup([C, { not: C }], { listPath: "exit.any", tag: "Exit" });
  const kinds = [...host.querySelectorAll<HTMLSelectElement>('select[aria-label="Operand type"]')].map((s) => !!s.querySelector('option[value="bars_held"]'));
  expect(kinds).toEqual([true, true, false, false]);
});

it("nested group changes keep metadata and siblings without changing the source tree", async () => {
  const sibling = { ...C, tag: "sibling" }, first = { ...C, tag: "first" };
  const list: RuleNode[] = [{ all: [first, sibling], future_group_setting: { kept: true } }];
  const before = structuredClone(list), row = await setup(list);
  await choose(row.host.querySelector('select[aria-label="Group type"]')!, "any");
  expect(row.last()).toEqual([{ any: [first, sibling], future_group_setting: { kept: true } }]);
  const children = row.host.querySelectorAll<HTMLElement>('.lab-group-body > [data-lab-node]');
  await choose(children[0].querySelector('select[aria-label="Operator"]')!, "<=");
  expect(row.last()).toEqual([{ ...list[0], all: [{ ...first, op: "<=" }, sibling] }]);
  await choose(children[1].querySelector('select[data-lab-node-menu]')!, "up");
  expect(row.last()).toEqual([{ ...list[0], all: [sibling, first] }]);
  await click(children[0].querySelector('button[aria-label="Remove Entry 1 › all 1"]')!);
  expect(row.last()).toEqual([{ ...list[0], all: [sibling] }]);
  expect(list).toEqual(before);
});

it("sequence step cap and nested caps block additions without blocking a group's edits", async () => {
  const list: RuleNode[] = [{ then: [C, C, C, C], within: 5 }], row = await setup(list);
  const add = row.host.querySelector<HTMLSelectElement>('.lab-group-body > select[data-lab-add]')!;
  expect([...add.options].filter((o) => o.value).map((o) => o.value)).toEqual(["cond", "all", "any", "not"]);
  expect([...add.options].filter((o) => o.value).every((o) => o.disabled)).toBe(true);
  await choose(add, "cond");
  expect(row.onChange).not.toHaveBeenCalled();
  const nested = await setup([{ any: [C] }], { caps: { leaves: false, nodes: false, top: true } });
  const inside = nested.host.querySelector<HTMLSelectElement>('.lab-group-body > select[data-lab-add]')!;
  expect(inside.querySelector<HTMLOptionElement>('option[value="cond"]')!.disabled).toBe(false);
  await choose(inside, "cond");
  expect(nested.last()).toEqual([{ any: [C, expect.any(Object)] }]);
});

it("state-operand conditions recurse through this editor and cannot use Bars held", async () => {
  const node: RuleNode = { lhs: { bars_since: { any: [C] }, future_operand_setting: "keep" }, op: ">", rhs: { value: 1 }, tag: "parent" };
  const before = structuredClone(node), row = await setup([node], { listPath: "exit.any", tag: "Exit" });
  const condition = row.host.querySelector<HTMLElement>('.lab-expr [data-lab-kind="cmp"]')!;
  expect(condition.querySelector('option[value="bars_held"]')).toBeNull();
  await choose(condition.querySelector('select[aria-label="Operator"]')!, "crosses_below");
  expect(row.last()).toEqual([{ ...node, lhs: { bars_since: { any: [{ ...C, op: "crosses_below" }] }, future_operand_setting: "keep" } }]);
  expect(node).toEqual(before);
});

it("single-child groups cannot remove their required child, and numeric controls reject invalid whole values", async () => {
  const row = await setup([{ not: C }, { at_least: 2, of: 3, when: C }, { lhs: { s: "close" }, op: "rising", bars: 2 }]);
  const single = row.host.querySelector('[data-lab-kind="not"] .lab-group-body')!;
  expect(single.querySelector('button[aria-label^="Remove"]')).toBeNull();
  expect(single.querySelector('select[data-lab-add]')).toBeNull();
  const bars = row.host.querySelector<HTMLInputElement>('[data-lab-kind="trend"] input[aria-label="bars"]')!;
  for (const value of ["0", "1001", "1.5"]) await type(bars, value);
  expect(row.onChange).not.toHaveBeenCalled();
  await type(bars, "1000");
  expect(row.last()[2]).toMatchObject({ bars: 1000 });
});

it("single-child group labels follow the engine's not/when paths without invented indexes", async () => {
  const row = await setup([{ not: C }, { at_least: 1, of: 2, when: C }]);
  expect(row.host.querySelector('select[aria-label="Actions for Entry 1 › not"]')).not.toBeNull();
  expect(row.host.querySelector('select[aria-label="Actions for Entry 2 › when"]')).not.toBeNull();
});
