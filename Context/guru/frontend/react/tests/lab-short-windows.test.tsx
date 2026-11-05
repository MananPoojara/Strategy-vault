import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ShortWindowsPanel } from "../src/features/lab/ShortWindowsPanel";
import * as m from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const w = (from: string, to: string): [string, string][] => [[from, to]];
const sets = [{ name: "Master 5DP", windows: w("2021-03-24", "2021-04-02") }, { name: "QTR Midcap", windows: w("2021-06-30", "2021-07-30") }];
const short = (over: Partial<Spec> = {}) => ({ ...m.defaultSpec(), direction: "short", ...over }) as Spec;
const byText = (scope: string, text: string) => [...document.querySelectorAll<HTMLElement>(`[aria-label='${scope}'] button`)].find((b) => b.textContent === text)!;
const click = (el: HTMLElement) => act(async () => el.click());
const active = (scope: string) => document.querySelector(`[aria-label='${scope}'] .active`)?.textContent;

it("shows for a short strategy only", async () => {
  await mount(<ShortWindowsPanel spec={{ ...m.defaultSpec(), direction: "long" } as Spec} onChange={vi.fn()} sets={sets} />);
  expect(document.querySelector("[data-lab-short-windows]")).toBeNull();
  await unmount();
  await mount(<ShortWindowsPanel spec={short()} onChange={vi.fn()} sets={sets} />);
  expect(active("Master 5DP")).toBe("Off");
  expect(active("QTR")).toBe("Off");
});

it("Master 5DP and QTR (Midcap) are each Off, Fresh or Running and stack, with no index choice", async () => {
  const onChange = vi.fn();
  await mount(<ShortWindowsPanel spec={short()} onChange={onChange} sets={sets} />);
  await click(byText("Master 5DP", "Running"));
  const m5 = onChange.mock.lastCall![0] as Spec;
  expect(m5.blackouts).toEqual([{ name: "Master 5DP", windows: sets[0].windows, reentry: "running" }]);
  await unmount();
  await mount(<ShortWindowsPanel spec={m5} onChange={onChange} sets={sets} />);
  await click(byText("QTR", "Fresh"));
  const both = onChange.mock.lastCall![0] as Spec;
  expect(both.blackouts).toEqual([m5.blackouts![0], { name: "QTR Midcap", windows: sets[1].windows, reentry: "fresh" }]);
  await unmount();
  await mount(<ShortWindowsPanel spec={both} onChange={onChange} sets={sets} />);
  expect(document.querySelector("[aria-label='QTR index']")).toBeNull();
  await click(byText("Master 5DP", "Off"));
  expect((onChange.mock.lastCall![0] as Spec).blackouts).toEqual([both.blackouts![1]]);
});
