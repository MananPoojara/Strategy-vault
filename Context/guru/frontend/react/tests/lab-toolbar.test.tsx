import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { Toolbar } from "../src/features/lab/Toolbar";
import { ErrorNotice } from "../src/features/lab/ui";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const props = { market: "us" as const, onMarket: vi.fn(), ticker: "", onTicker: vi.fn(), tf: "D" as const, timeframes: ["D" as const], onTf: vi.fn(), range: "3Y" as const, onRange: vi.fn() };

it("a market without instruments says so; the chart window control is named Chart range", async () => {
  const host = await mount(<Toolbar {...props} options={[]} />);
  const select = host.querySelector<HTMLSelectElement>(".lab-toolbar select")!;
  expect(select.disabled).toBe(true);
  expect(select.textContent).toBe("No instruments available");
  expect(host.querySelector('[role="group"][aria-label="Chart range"]')).not.toBeNull();
  expect(host.textContent).toContain("Chart range");
});

it("a 503 from the gateway reads as an unavailable market, not a refusal", async () => {
  const error = Object.assign(new Error("No local us engine is configured (LAB_ENGINE_US)."), { status: 503, kind: "gateway" });
  const host = await mount(<ErrorNotice error={error} market="us" />);
  expect(host.querySelector("[data-lab-error] b")!.textContent).toBe("Market not available here");
});

it("offers daily, weekly and monthly quick intervals with the selected state and advertised values", async () => {
  const onTf = vi.fn();
  const host = await mount(<Toolbar {...props} options={[]} timeframes={["D", "W", "M", "Q", "Y"]} tf="D" onTf={onTf} />);
  const group = host.querySelector('[role="group"][aria-label="Chart"]')!;
  const buttons = [...group.querySelectorAll<HTMLButtonElement>('button')];
  expect(buttons.map(b => b.textContent)).toEqual(["1D", "1W", "1M"]);
  expect(buttons.map(b => b.getAttribute('aria-label'))).toEqual(["Daily", "Weekly", "Monthly"]);
  expect(buttons.map(b => b.getAttribute('aria-pressed'))).toEqual(["true", "false", "false"]);
  await act(async () => buttons[2].click());
  expect(onTf).toHaveBeenLastCalledWith("M");
  await act(async () => buttons[0].click());
  expect(onTf).toHaveBeenLastCalledWith("D");
  const more = group.querySelector<HTMLSelectElement>('[aria-label="More timeframes"]')!;
  expect([...more.options].map(o => [o.value,o.textContent])).toEqual([["","More intervals"],["Q","3M"],["Y","1Y"]]);
  await act(async () => { more.value = "Q"; more.dispatchEvent(new Event("change", {bubbles:true})); });
  expect(onTf).toHaveBeenLastCalledWith("Q");
});

it("marks an extra interval as selected and never adds unsupported quick intervals", async () => {
  const host = await mount(<Toolbar {...props} options={[]} timeframes={["M", "Q", "5D"]} tf="Q" />);
  const group = host.querySelector('[aria-label="Chart"]')!;
  expect([...group.querySelectorAll('button')].map(b => b.textContent)).toEqual(["1M"]);
  expect(group.querySelector('button')!.getAttribute('aria-pressed')).toBe("false");
  const more = group.querySelector<HTMLSelectElement>('select')!;
  expect(more.value).toBe("Q");
  expect(more.selectedOptions[0].textContent).toBe("3M");
  expect([...more.options].map(o => o.value)).toEqual(["","Q","5D"]);
});

it("chart toolbar opens searchable symbol popup and keeps refused instruments disabled", async () => {
  const onTicker = vi.fn(), onMarket = vi.fn();
  const host = await mount(<Toolbar {...props} chartWorkspace options={[
    {ticker:"NIFTY 50",label:"Nifty 50",disabled:false,hasVolume:true},
    {ticker:"ABC",label:"ABC Limited",disabled:true,reason:"Needs stock data",hasVolume:false},
  ]} ticker="NIFTY 50" onTicker={onTicker} onMarket={onMarket}><button>Indicators</button></Toolbar>);
  expect(host.querySelector('[aria-label="Chart range"]')).toBeNull();
  expect(host.querySelector('[aria-label="Chart controls"]')?.textContent).toContain("Indicators");
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Symbol search"]')!.click());
  const popup = host.querySelector('[role="dialog"][aria-label="Symbol search"]')!;
  expect(popup).not.toBeNull();
  expect(popup.querySelector<HTMLButtonElement>('[title="Needs stock data"]')?.disabled).toBe(true);
  const input = popup.querySelector<HTMLInputElement>('[aria-label="Search instruments"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,"Nifty");
    input.dispatchEvent(new Event("input",{bubbles:true}));
  });
  expect(popup.querySelector('[title="Needs stock data"]')).toBeNull();
  await act(async () => popup.querySelector<HTMLButtonElement>('.lab-symbol-results button')!.click());
  expect(onTicker).toHaveBeenCalledWith("NIFTY 50");
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Symbol search"]')!.click());
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Market"] button')].find(b=>b.textContent==="India")!.click());
  expect(onMarket).toHaveBeenCalledWith("in");
});

it("chart interval menu lists every advertised timeframe and closes after a selection", async () => {
  const onTf = vi.fn();
  const host = await mount(<Toolbar {...props} chartWorkspace options={[]} timeframes={["D","W","M","Q"]} onTf={onTf} />);
  const menu = host.querySelector<HTMLDetailsElement>('details')!;
  expect([...menu.querySelectorAll('[role="menuitemradio"]')].map(e=>e.textContent)).toEqual(["Daily1D","Weekly1W","Monthly1M","Quarterly3M"]);
  expect(menu.querySelector('[aria-checked="true"]')?.textContent).toBe("Daily1D");
  menu.open = true;
  await act(async () => [...menu.querySelectorAll<HTMLButtonElement>('button')].find(b=>b.textContent==="Quarterly3M")!.click());
  expect(onTf).toHaveBeenCalledWith("Q");
  expect(menu.open).toBe(false);
  expect(host.querySelector('[role="menuitemradio"]:last-child')?.textContent).not.toContain("Yearly");
});
