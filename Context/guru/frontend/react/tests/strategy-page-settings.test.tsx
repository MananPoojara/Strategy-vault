import {afterEach, expect, it, vi} from "vitest";
import {act} from "react";
import {createRoot, type Root} from "react-dom/client";
import {StrategyPage} from "../src/pages/ResearchPages";
import {sourceAccount,sourceAccountView,type SourceAccountView} from "../src/services/source-account";
import {bandyAccountFixture} from "./helpers/bandy-source-account-fixture";
import {simpleAccountFixture,simpleAdmissionFixture} from "./helpers/bandy-simple-account-fixture";
import {accountFixture} from "./helpers/source-account-fixture";
import type {ViewSnapshot} from "../src/state/store";

let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
});
const snapshot = (state: Record<string, string>): ViewSnapshot => ({
  revision: 1, loading: false, html: '<header><span data-react-slot="strategy-settings"></span></header><div data-react-slot="source-account-notice"></div>',
  error: null, route: null, status: "Strategy", state, home: null, reading: null,
});
async function mount(state: Record<string, string>, source: boolean | SourceAccountView = false) {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const patch = vi.fn();
  window.VaultRuntime = {patch} as unknown as Window["VaultRuntime"];
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<StrategyPage snapshot={{...snapshot(state), strategyAccount:typeof source === "object" ? source : source?sourceAccountView(sourceAccount(accountFixture(),"elcim_impulse","elcim")!):null}} />));
  return patch;
}
const dialog = () => document.querySelector<HTMLElement>('[data-strategy-settings]')!;
const radio = (value: string) => dialog().querySelector<HTMLInputElement>(`input[value="${value}"]`)!;
const button = (text: string) => [...dialog().querySelectorAll<HTMLButtonElement>("button")].find(b => b.textContent === text)!;

it("uses the committed report mode and applies only the chosen display parameter", async () => {
  const patch = await mount({report_mode: "compound", tk: "basket:nifty50:RELIANCE", slippage: "12"});
  expect(radio("compound").checked).toBe(true);
  await act(async () => radio("fixed").click());
  await act(async () => button("Apply").click());
  expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode: "fixed"});
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});
it("defaults an unknown display mode to fixed and Cancel leaves the route and result unchanged", async () => {
  const patch = await mount({report_mode: "invalid"});
  expect(radio("fixed").checked).toBe(true);
  await act(async () => radio("compound").click());
  await act(async () => button("Undo changes").click());
  expect(patch).not.toHaveBeenCalled();
});
it("keeps the basket's portfolio accounting and never offers closing-order trade compounding", async () => {
  const patch = await mount({run_scope: "basket", report_mode: "fixed"});
  expect(dialog().textContent).toContain("equal-slot portfolio calculation");
  expect(radio("compound").matches(":disabled")).toBe(true);
  expect(radio("fixed").matches(":disabled")).toBe(true);
  expect(button("Apply").disabled).toBe(true);
  await act(async () => button("Apply").click());
  expect(patch).not.toHaveBeenCalled();
});

it("explains protected source sizing and applies only the closed weighted-return display", async () => {
 const patch=await mount({report_mode:"fixed",tk:"basket:us_sp500:AAPL"},true);
 expect(document.querySelector('[data-source-account-notice]')?.textContent).toContain('discretionary turning-point resets');
 expect(dialog().textContent).toContain('Closed weighted-return display');expect(dialog().textContent).toContain('Hypothetical closed compounding');
 expect(dialog().textContent).toContain('mandatory 2%/6% risk controller');expect(dialog().textContent).not.toContain('Fixed quantity');
 await act(async()=>radio('compound').click());await act(async()=>button('Apply').click());expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode:'compound'});
});

for (const macd of [false,true]) it(`keeps Bandy ${macd?'source25%':'declared100%'} sizing separate from the display-only setting`,async()=>{
 const account=sourceAccount(bandyAccountFixture(macd),macd?'hbiab_macd_example':'hbiab_ma5_20','hbiab')!;
 const patch=await mount({report_mode:'fixed',tk:'basket:us_sp500:AAPL'},sourceAccountView(account));
 const notice=document.querySelector('[data-source-account-notice]')!;
 expect(notice.textContent).toContain(macd?'25%':'100%');expect(notice.textContent).toContain(macd?'Source allocation':'Declared research convention');
 expect(notice.textContent).not.toMatch(/2%|6%|monthly|SafeZone|₹/);expect(dialog().textContent).not.toMatch(/2%|6%|Fixed quantity/);
 expect(dialog().textContent).toContain('reviewed fixed allocation');
 await act(async()=>radio('compound').click());await act(async()=>button('Apply').click());
 expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode:'compound'});
});

it('keeps Simple teaching/source conventions and the accepted cash account separate from its display-only setting',async()=>{
 const account=sourceAccount(simpleAccountFixture(),'hbiab_simple_ma25','hbiab','basket:us_sp500:AAPL',simpleAdmissionFixture())!;
 const patch=await mount({report_mode:'fixed',tk:'basket:us_sp500:AAPL'},sourceAccountView(account));const notice=document.querySelector('[data-source-account-notice]')!;
 expect(notice.textContent).toContain('SimpleSystem teaching example');expect(notice.textContent).toContain('100%');expect(notice.textContent).toContain('every true raw Buy resets');expect(notice.textContent).toContain('not five bars from entry');
 expect(notice.textContent).not.toMatch(/2%|6%|monthly|SafeZone|₹/);expect(dialog().textContent).toContain('reviewed fixed allocation');
 await act(async()=>radio('compound').click());await act(async()=>button('Apply').click());expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode:'compound'});
});

it("shows the settings open on the page, with no icon button and no pop-up", async () => {
  await mount({report_mode: "fixed"});
  expect(dialog()).not.toBeNull();
  expect(host!.querySelector('[aria-label="Strategy settings"]')).toBeNull();
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(button("Apply").disabled).toBe(true);
  expect(button("Undo changes").disabled).toBe(true);
  await act(async () => radio("compound").click());
  expect(button("Apply").disabled).toBe(false);
  await act(async () => button("Undo changes").click());
  expect(radio("fixed").checked).toBe(true);
});
