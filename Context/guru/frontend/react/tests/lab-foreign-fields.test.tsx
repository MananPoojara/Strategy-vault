import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ForeignFields, ForeignProvider } from "../src/features/lab/ForeignFields";
import type { ForeignScope } from "../src/features/lab/foreign-model";
import type { Operand } from "../src/features/lab/types";
import { catalogue7Fixture } from "./fixtures/lab";
const scope: ForeignScope = { catalogue: { ...catalogue7Fixture, max_foreign: 4 }, ticker: "SPY", market: "us", volatility: "CBOE VIX", options: [
 { ticker: "SPX", label: "S&P 500", disabled: false, hasVolume: false }, { ticker: "SPY", label: "SPY", disabled: false, hasVolume: true },
 { ticker: "HSI:HSI", label: "HSI", market: "gl", disabled: false, hasVolume: false },
] };
const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(operand: Operand, value: ForeignScope | null = scope) {
 Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
 const host = document.createElement("div"); document.body.append(host); const root = createRoot(host); active.push({ root, host });
 const onChange = vi.fn();
 const rerender = async (o: Operand) => { const body=<ForeignFields operand={o} onChange={onChange} label="Entry left" />; await act(async () => root.render(value ? <ForeignProvider value={value}>{body}</ForeignProvider> : body)); };
 await rerender(operand); return { host, rerender, last: () => onChange.mock.calls.at(-1)?.[0] as Operand };
}
const choose = async (el: Element, value: string) => act(async () => { (el as HTMLSelectElement).value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
it("is absent without context and for formulas/numbers/bars-held, and folded for a local series", async () => {
 expect((await setup({ s: "close" }, null)).host.innerHTML).toBe("");
 for (const o of [{ value: 1 }, { ref: { s: "close" }, bars: 1 }, { s: "bars_held" }]) expect((await setup(o)).host.innerHTML).toBe("");
 const {host}=await setup({ s: "close" }); expect(host.querySelector("details")?.open).toBe(false);
});
it("same-market known choices include VIX; edits preserve metadata, timing and explicit conflicts", async () => {
 const source={ s: "rsi", p: { length: 2 }, tf: "W", offset: 1, future: {keep: true} };
 const {host,last,rerender}=await setup(source);
 await act(async () => host.querySelector("summary")!.click());
 const sourceSelect=host.querySelector<HTMLSelectElement>('[aria-label="Entry left · Read from"]')!;
 expect([...sourceSelect.options].map((x)=>x.value)).toEqual(["", "CBOE VIX", "SPX", "SPY"]);
 await choose(sourceSelect,"SPX"); expect(last()).toEqual({...source,symbol:"SPX"});
 await rerender(last()); await choose(host.querySelector('[aria-label="Entry left · Missing dates"]')!,"previous");
 await rerender(last()); await choose(host.querySelector('[aria-label="Entry left · Read from"]')!,"");
 expect(last()).toEqual({...source,fixup:"previous"});
 await rerender(last()); expect(host.textContent).toContain("fixup requires");
});
it("pair comparison is visible and required, with D-only problems instead of destructive cleanup", async () => {
 const source={s:"ratio",tf:"W",future:"keep"}; const {host,last,rerender}=await setup(source);
 expect(host.querySelector<HTMLDetailsElement>("details")!.open).toBe(true);
 expect(host.textContent).toContain("daily (D)");
 expect(host.textContent).toContain("choose a second instrument");
 await choose(host.querySelector('[aria-label="Entry left · Compared with"]')!,"SPX");
 expect(last()).toEqual({...source,vs:"SPX"}); await rerender(last());
 expect(host.textContent).toContain("daily (D)");
});
it("unknown saved names and malformed bars-held fields stay visible and unchanged", async () => {
 const {host,last}=await setup({s:"close",symbol:"UNKNOWN",future:2});
 expect(host.textContent).toContain("not listed");
 expect(host.querySelector('[aria-label="Entry left · Read from"] option[value="UNKNOWN"]')?.textContent).toContain("Kept");
 expect(last()).toBeUndefined();
 const held=await setup({s:"bars_held",symbol:"SPX"}); expect(held.host.textContent).toContain("Bars held takes no");
 expect(held.host.querySelector("select")).toBeNull();
});
it("legacy catalogue never offers unsupported edits, and malformed saved values stay read-only", async () => {
 const old=await setup({s:"close",symbol:"SPX"},{...scope,catalogue:catalogue7Fixture});
 expect(old.host.textContent).toContain("not advertised"); expect(old.host.querySelector("select")).toBeNull();
 const invalid=await setup({s:"close",symbol:7,fixup:false});
 expect(invalid.host.textContent).toContain("kept"); expect(invalid.host.querySelector("select")).toBeNull();
});
it("portfolio score scope permits pair comparisons but no new foreign symbol choices",async()=>{
 const local=await setup({s:"close"},{...scope,allowSymbol:false});expect(local.host.querySelector('[aria-label="Entry left · Read from"]')).toBeNull();
 const pair=await setup({s:"ratio"},{...scope,allowSymbol:false});expect(pair.host.querySelector('[aria-label="Entry left · Compared with"]')).not.toBeNull();
 const kept=await setup({s:"close",symbol:"SPX"},{...scope,allowSymbol:false});const select=kept.host.querySelector<HTMLSelectElement>('[aria-label="Entry left · Read from"]')!;expect([...select.options].map(o=>o.value)).toEqual(["","SPX"]);
});
it("portfolio rules refer to each actual member rather than the chart ticker",async()=>{
 const {host}=await setup({s:"close"},{...scope,portfolio:true});const option=host.querySelector('[aria-label="Entry left · Read from"] option[value=""]')!;
 expect(option.textContent).toBe("Current member");expect(host.querySelector("summary")?.textContent).toBe("Source");expect(host.querySelector("summary")?.getAttribute("aria-label")).toContain("current member");
});

it("a collapsed default source is compact while active names and accessible context stay explicit",async()=>{
 const local=await setup({s:"close"});const summary=local.host.querySelector("summary")!;
 expect(summary.textContent).toBe("Source");expect(summary.getAttribute("title")).toContain("current instrument");expect(summary.getAttribute("aria-label")).toContain("Entry left");
 const active=await setup({s:"close",symbol:"SPX"});expect(active.host.querySelector("summary")?.textContent).toContain("S&P 500");
});
