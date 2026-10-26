import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import type { Spec } from "../src/features/lab/types";

const saved: { name: string; bytes: Uint8Array }[] = [];
vi.mock("../src/features/lab/xlsx", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/xlsx")>()), saveBytes: (bytes: Uint8Array, name: string) => saved.push({ name, bytes }) }));
const { comboName, comboSpec, TradesheetButton, TradesheetDownloads } = await import("../src/features/lab/TradesheetDownloads");
const { defaultSpec } = await import("../src/features/lab/spec-model");
const { mount, runFixture, unmount } = await import("./fixtures/lab");

afterEach(async () => { await unmount(); saved.length = 0; });
// one entry, two exits switched on and off by the optimizer
const ran: Spec = { ...defaultSpec(), exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }, { lhs: { s: "rsi" }, op: ">", rhs: { value: 70 } }] },
  optimize: [{ path: "exit.any[0].off", from: 0, to: 1, step: 1 }, { path: "exit.any[1].off", from: 0, to: 1, step: 1 }] } as Spec;
const dims = [{ path: "exit.any[0].off" }, { path: "exit.any[1].off" }];
const label = (p: string) => (p.startsWith("exit.any[0]") ? "Exit 1: switched off (0 on, 1 off)" : "Exit 2: switched off (0 on, 1 off)");
const table = [[0, 1, 0.12], [1, 0, 0.05], [0, 0, 0.2]];
const entries = (zip: Uint8Array) => new DataView(zip.buffer).getUint16(zip.length - 22 + 10, true);
const text = (b: Uint8Array) => new TextDecoder().decode(b);

it("names a combination and builds its spec from the optimized one", () => {
  expect(comboName(dims, [0, 1], label)).toBe("Exit 1 on · Exit 2 off");
  expect(comboName([{ path: "entry.all[0].rhs.value" }], [30], () => "Entry 1 right")).toBe("Entry 1 right 30");
  const s = comboSpec(ran, dims, [1, 0]);
  expect(s.optimize).toBeUndefined();
  expect((s.exit.any[0] as { off?: boolean }).off).toBe(true);
});

it("Download all: each combination is run again; separate files come as one ZIP with a summary, or as one workbook", async () => {
  const onRun = vi.fn(async (_spec: Spec) => runFixture);
  const host = await mount(<TradesheetDownloads table={table} dims={dims} ranSpec={ran} label={label} onRun={onRun} ticker="NIFTY 50" />);
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-tradesheets] button")!.click());
  expect(onRun).toHaveBeenCalledTimes(3);
  expect(onRun.mock.calls[1][0].exit.any.map((r) => (r as { off?: boolean }).off === true)).toEqual([true, false]);
  expect(saved[0].name).toBe("tradesheets NIFTY 50.zip");
  expect(entries(saved[0].bytes)).toBe(4);
  expect(text(saved[0].bytes)).toContain("01 Exit 1 on · Exit 2 off.xlsx");
  const sel = host.querySelector<HTMLSelectElement>('select[aria-label="Tradesheet format"]')!;
  await act(async () => { sel.value = "book"; sel.dispatchEvent(new Event("change", { bubbles: true })); });
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-tradesheets] button")!.click());
  expect(saved[1].name).toBe("tradesheets NIFTY 50.xlsx");
  expect(text(saved[1].bytes)).toContain('<sheet name="Summary"');
  expect(text(saved[1].bytes)).toContain('<sheet name="03 Exit 1 on · Exit 2 on"');
});

it("one combination's Excel button saves its own file; a portfolio result is refused with a message", async () => {
  const host = await mount(<TradesheetButton dims={dims} values={[1, 0]} ranSpec={ran} label={label} onRun={async () => runFixture} ticker="NIFTY 50" />);
  await act(async () => host.querySelector("button")!.click());
  expect(saved[0].name).toBe("tradesheet NIFTY 50 Exit 1 off · Exit 2 on.xlsx");
  await unmount();
  const bad = await mount(<TradesheetButton dims={dims} values={[0, 0]} ranSpec={ran} label={label} onRun={async () => ({ ...runFixture, kind: "lab_portfolio_simulation" }) as never} ticker="X" />);
  await act(async () => bad.querySelector("button")!.click());
  expect(bad.querySelector('[role="alert"]')!.textContent).toContain("one instrument");
});
