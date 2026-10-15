import { readFileSync } from "node:fs";
import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { availability, groupByCategory, searchIndicators } from "../src/features/lab/catalogue";
import { defaultTicker, optionsFromGl, optionsFromTickers, savedMarket } from "../src/features/lab/instruments";
import { LibraryRail } from "../src/features/lab/LibraryRail";
import { IndicatorDetail } from "../src/features/lab/IndicatorDetail";
import { Toolbar } from "../src/features/lab/Toolbar";
import { catalogueFixture as cat, glInstrumentsFixture, mount, tickersFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const byId = (id: string) => cat.indicators.find((i) => i.id === id)!;

it("searches name, id, TradingView and AmiBroker names", () => {
  expect(searchIndicators(cat.indicators, "ta.rsi").map((i) => i.id)).toEqual(["rsi"]);
  expect(searchIndicators(cat.indicators, "ema").map((i) => i.id)).toEqual(["ema"]);
  expect(searchIndicators(cat.indicators, "  ").length).toBe(cat.indicators.length);
  expect(groupByCategory(cat.indicators).map(([c, l]) => [c, l.length])[1]).toEqual(["Oscillators", 3]);
});

it("availability: listed, volume, breadth and other data needs", () => {
  const ctx = { market: "in" as const, hasVolume: false, tf: "D" };
  expect(availability(byId("or_range"), ctx)).toEqual({ ok: false, reason: "Needs intraday (minute) bars; not available yet." });
  expect(availability(byId("obv"), ctx).reason).toBe("Needs volume; this instrument has no volume in its data.");
  expect(availability(byId("obv"), { ...ctx, hasVolume: true }).ok).toBe(true);
  expect(availability(byId("ad_line"), { ...ctx, tf: "W" }).reason).toBe("Breadth data exists only for India and US daily bars.");
  expect(availability(byId("ad_line"), ctx).ok).toBe(true);
  expect(availability(byId("ad_line"), { ...ctx, market: "gl" }).ok).toBe(false);
  const listedOnly = { ...byId("ema"), computed: false };
  expect(availability({ ...listedOnly, note: "X" }, ctx)).toEqual({ ok: false, reason: "X" });
  expect(availability({ ...listedOnly, note: "" }, ctx).reason).toBe("Listed only; the engine does not compute it yet.");
});

it("instrument options per market and allowed types", () => {
  const inOpts = optionsFromTickers(tickersFixture, ["index", "etf"]);
  expect(inOpts.map((o) => [o.label, o.disabled])).toEqual([["NIFTY 50", false], ["NIFTYBANK", false], ["RELIANCE · NIFTY 50", true]]);
  expect(inOpts[2].reason).toBe("Not admitted for this method (needs index, etf)");
  const gl = optionsFromGl(glInstrumentsFixture);
  expect(gl.map((o) => [o.label, o.disabled, o.reason])).toEqual([["Bitcoin / US dollar", false, undefined], ["DAX", true, "IG:DAX is not staged on this server"]]);
  expect(defaultTicker("in", inOpts)).toBe("NIFTY 50");
  expect(defaultTicker("gl", gl)).toBe("CRYPTO:BTCUSD");
  expect(defaultTicker("gl", [gl[1], gl[0]])).toBe("CRYPTO:BTCUSD");
  expect([savedMarket("gl", gl[0]), savedMarket("gl", gl[1]), savedMarket("us")]).toEqual(["crypto", "gl", "us"]);
});

it("rail lists every entry, disables listed ones with their note and adds computed ones", async () => {
  const onAdd = vi.fn(), onRead = vi.fn();
  const host = await mount(<LibraryRail catalogue={cat} market="in" hasVolume={false} tf="D" onAdd={onAdd} onRead={onRead} />);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(cat.indicators.length);
  const listed = host.querySelector<HTMLButtonElement>('[data-lab-item="or_range"]')!;
  expect(listed.disabled).toBe(true);
  expect(listed.closest("li")!.textContent).toContain("Needs intraday (minute) bars; not available yet.");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-item="ema"]')!.click());
  expect(onAdd).toHaveBeenCalledWith(byId("ema"));
  const search = host.querySelector<HTMLInputElement>('input[aria-label="Search indicators"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "ta.rsi");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect([...host.querySelectorAll<HTMLElement>("[data-lab-item]")].map((b) => b.dataset.labItem)).toEqual(["rsi"]);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="About Relative Strength Index"]')!.click());
  expect(onRead).toHaveBeenCalledWith(byId("rsi"));
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "zzz");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(host.querySelector(".empty")?.textContent).toBe('No indicator matches "zzz".');
});

it("disabled rail items get the muted, not-allowed style over the item rule", () => {
  // jsdom ignores specificity, so check the stylesheet: the disabled rule must out-rank `li > button[data-lab-item]`.
  const css = readFileSync("src/features/lab/library.css", "utf8");
  expect(css).toMatch(/\.lab-library li > button\[data-lab-item\]:disabled[^{]*\{[^}]*color: var\(--ink3\);[^}]*cursor: not-allowed;/);
});

it("reference panel shows facts and never claims a TradingView match", async () => {
  const host = await mount(<IndicatorDetail indicator={byId("macd")} catalogue={cat} onClose={() => {}} />);
  expect(host.textContent).toContain("Own pane");
  expect(host.textContent).toContain("25, 33, 33");
  expect(host.textContent).toContain("Not yet checked against TradingView.");
  expect(host.textContent).not.toMatch(/\bverified\b|TradingView-matched/i);
});

it("toolbar shows the provisional strip on gl and reports choices", async () => {
  const onMarket = vi.fn();
  const host = await mount(<Toolbar market="gl" onMarket={onMarket} options={optionsFromGl(glInstrumentsFixture)} ticker="CRYPTO:BTCUSD" onTicker={() => {}}
    tf="D" timeframes={["D", "W", "M", "Q"]} onTf={() => {}} range="3Y" onRange={() => {}} provisionalNote={glInstrumentsFixture.provisional_note} />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("CRYPTO:BTCUSD, daily");
  expect([...host.querySelectorAll<HTMLButtonElement>(".join-item")].map((b) => b.textContent)).toEqual(expect.arrayContaining(["Daily", "Weekly", "Monthly", "Q"]));
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "US")!.click());
  expect(onMarket).toHaveBeenCalledWith("us");
});

it("toolbar has no provisional strip on in/us", async () => {
  for (const market of ["in", "us"] as const) {
    const host = await mount(<Toolbar market={market} onMarket={() => {}} options={optionsFromTickers(tickersFixture)} ticker="NIFTY 50" onTicker={() => {}}
      tf="D" timeframes={["D"]} onTf={() => {}} range="3Y" onRange={() => {}} provisionalNote={glInstrumentsFixture.provisional_note} />);
    expect(host.querySelector("[data-lab-provisional]")).toBeNull();
    await unmount();
  }
});

it("rail groups start collapsed; a search expands only matching groups and clearing restores the manual choices", async () => {
  const host = await mount(<LibraryRail catalogue={cat} market="in" hasVolume={false} tf="D" onAdd={() => {}} onRead={() => {}} />);
  const heads = () => [...host.querySelectorAll<HTMLButtonElement>(".lab-group-h")];
  const open = () => heads().filter((h) => h.getAttribute("aria-expanded") === "true").map((h) => h.textContent);
  const type = async (v: string) => {
    const search = host.querySelector<HTMLInputElement>('input[aria-label="Search indicators"]')!;
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, v); search.dispatchEvent(new Event("input", { bubbles: true })); });
  };
  expect(heads().length).toBeGreaterThan(1);
  expect(open()).toEqual([]);
  expect(host.querySelectorAll("ul[hidden]")).toHaveLength(heads().length);
  await act(async () => heads()[0].click());
  const manual = open();
  expect(manual).toHaveLength(1);
  await type("ta.rsi");
  expect(heads()).toHaveLength(1); // only the group holding a match is shown, and it is open
  expect(open()).toHaveLength(1);
  expect(host.querySelector("ul[hidden]")).toBeNull();
  await type("");
  expect(open()).toEqual(manual);
});
