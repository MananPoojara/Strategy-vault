import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ParamRule } from "../src/features/lab/ParamRule";
import type { RuleNode } from "../src/features/lab/types";
import { catalogueFixture as cat, mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const keys = (host: HTMLElement) => [...host.querySelectorAll(".lab-param-k")].map((k) => k.textContent);
const rule = (node: RuleNode) => mount(<ParamRule node={node} catalogue={cat} label="Entry 1" onChange={vi.fn()} onLhs={vi.fn()} />);

it("an AmiBroker calculation stays on screen without opening More; Bars ago and the symbol wait behind More (AFL Ref(x, -n))", async () => {
  const host = await rule({ lhs: { s: "stoch", p: { length: 14, variant: 1 } }, op: ">", rhs: { value: 80 } } as RuleNode);
  expect(keys(host)).toContain("Calculation");
  expect(keys(host)).not.toContain("Bars ago");
  const more = host.querySelector<HTMLButtonElement>("[data-lab-param-more]")!;
  expect(more.textContent).toBe("More ▾");
  await act(async () => more.click());
  expect(keys(host)).toContain("Bars ago");
});

it("a rule that already reads n bars ago opens with Bars ago shown", async () => {
  const host = await rule({ lhs: { s: "stoch", p: { length: 14 }, offset: 2 }, op: ">", rhs: { value: 80 } } as RuleNode);
  expect(keys(host)).toContain("Bars ago");
  expect(host.querySelector("[data-lab-param-more]")).toBeNull();
});
