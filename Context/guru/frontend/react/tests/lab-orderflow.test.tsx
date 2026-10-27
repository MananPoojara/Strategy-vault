import { afterEach, expect, it } from "vitest";
import { availability, ORDER_FLOW_NOTE } from "../src/features/lab/catalogue";
import { LibraryRail } from "../src/features/lab/LibraryRail";
import { IndicatorDetail } from "../src/features/lab/IndicatorDetail";
import { ErrorNotice } from "../src/features/lab/ui";
import type { Catalogue, Indicator } from "../src/features/lab/types";
import { catalogueFixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const base = catalogueFixture.indicators.find((i) => i.id === "obv")!;
const none = { ...base.needs, volume: false };
const of = (patch: Partial<Indicator>): Indicator => ({ ...base, category: "Order flow", ...patch });
const cvd = of({ id: "cvd", name: "Cumulative Volume Delta", needs: { ...none, order_flow: true } });
const footprint = of({ id: "footprint_imbalance_ts", name: "Delta Min/Max", computed: false, needs: { ...none, order_flow: true, intraday: true },
  note: "Needs intraday (tick-ordered) order-flow data; not available yet." });
const dom = of({ id: "dom_heatmap", name: "Order Book Heatmap", computed: false, needs: { ...none, order_book: true }, note: "" });
const cat: Catalogue = { ...catalogueFixture, indicators: [cvd, footprint, dom] };
const ctx = { market: "in" as const, hasVolume: false, tf: "D" };

it("availability: footprint indicators stay addable on daily bars with a visible note; order book and tick-ordered ones are listed", () => {
  expect(availability(cvd, ctx)).toEqual({ ok: true, reason: ORDER_FLOW_NOTE });
  expect(ORDER_FLOW_NOTE).toMatch(/^Needs order-flow data \(trades at bid\/ask\)/);
  expect(availability(cvd, { ...ctx, tf: "W" })).toEqual({ ok: false, reason: "Order-flow indicators use daily (D) bars only." });
  expect(availability(footprint, ctx)).toEqual({ ok: false, reason: footprint.note });
  expect(availability(dom, ctx)).toEqual({ ok: false, reason: "Listed only; the engine does not compute it yet." });
  expect(availability({ ...dom, computed: true }, ctx)).toEqual({ ok: false, reason: "Needs order-book (depth) data; not available yet." });
});

it("library shows the order-flow note on an addable item and needs-data tags on listed ones", async () => {
  const host = await mount(<LibraryRail catalogue={cat} market="in" hasVolume={false} tf="D" onAdd={() => {}} onRead={() => {}} />);
  const item = host.querySelector<HTMLButtonElement>('[data-lab-item="cvd"]')!;
  expect(item.disabled).toBe(false);
  expect(item.textContent).toContain(ORDER_FLOW_NOTE);
  const listed = host.querySelector<HTMLButtonElement>('[data-lab-item="dom_heatmap"]')!;
  expect(listed.disabled).toBe(true);
  expect(listed.closest("li")!.querySelector(".tag")?.textContent).toBe("needs data");
});

it("detail names order-flow and order-book data in Needs", async () => {
  let host = await mount(<IndicatorDetail indicator={cvd} catalogue={cat} onClose={() => {}} />);
  expect(host.textContent).toContain("Needsorder-flow data (trades at bid/ask)");
  expect(host.textContent).toContain(ORDER_FLOW_NOTE);
  await unmount();
  host = await mount(<IndicatorDetail indicator={dom} catalogue={cat} onClose={() => {}} />);
  expect(host.textContent).toContain("Needsorder-book (depth) data");
});

it("a needs_data answer from series or run shows the engine's note", async () => {
  const note = "needs order-flow data (trades at bid/ask) for NIFTY 50 — supply NIFTY_50__FOOTPRINT.csv in the order-flow folder";
  const host = await mount(<ErrorNotice market="in" error={Object.assign(new Error(note), { status: 400, kind: "engine", engineStatus: "needs_data" })} />);
  expect(host.querySelector("[data-lab-error] b")?.textContent).toBe("Data needed");
  expect(host.textContent).toContain(note);
});


it("floating indicator picker preserves the data note for enabled order-flow indicators", async () => {
  const host = await mount(<LibraryRail docked catalogue={cat} market="in" hasVolume={false} tf="D" onAdd={() => {}} onRead={() => {}} />);
  const item = host.querySelector<HTMLButtonElement>('[data-lab-item="cvd"]')!;
  expect(item.disabled).toBe(false);
  expect(item.textContent).toContain(ORDER_FLOW_NOTE);
  expect(item.title).toBe(ORDER_FLOW_NOTE);
});
