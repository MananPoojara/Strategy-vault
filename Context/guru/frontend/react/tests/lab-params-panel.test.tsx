import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ParamsDialog, type Look } from "../src/features/lab/ParamsDialog";
import type { Indicator } from "../src/features/lab/types";
import { catalogueFixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const indicator: Indicator = { ...catalogueFixture.indicators.find((i) => i.id === "ema")!, params: [
  { key: "length", label: "Length", default: 14, min: 2, max: 2000, step: 1, integer: true },
] };
const input = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const click = async (el: Element) => act(async () => (el as HTMLElement).click());
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("uses nonmodal Inputs / Style tabs and preserves invalid input drafts when switching", async () => {
  const onParams = vi.fn();
  function Harness() {
    const [params, set] = useState({ length: 14 });
    return <ParamsDialog name={indicator.name} indicator={indicator} index={0} params={params} showComparison={false}
      onParams={(p) => { onParams(p); set(p as { length: number }); }} onLook={vi.fn()} onClose={vi.fn()} />;
  }
  const host = await mount(<Harness />);
  expect(host.querySelector('.lab-params')!.getAttribute("role")).toBe("region");
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  const tabs = host.querySelectorAll<HTMLButtonElement>('[role="tab"]');
  expect([...tabs].map((t) => t.textContent)).toEqual(["Inputs", "Style"]);
  const length = host.querySelector<HTMLInputElement>('input[type="number"]')!;
  await input(length, "0");
  expect(onParams).not.toHaveBeenCalled();
  await click(tabs[1]);
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
  expect(length.closest<HTMLElement>('[role="tabpanel"]')!.hidden).toBe(true);
  expect(host.querySelector('select[aria-label$=" Style"]')!.closest<HTMLElement>('[role="tabpanel"]')!.hidden).toBe(false);
  await click(tabs[0]);
  expect(host.querySelector('input[type="number"]')).toBe(length);
  expect(length.value).toBe("0");
  await input(length, "21");
  expect(onParams).toHaveBeenLastCalledWith({ length: 21 });
  tabs[0].focus();
  await act(async () => tabs[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(document.activeElement).toBe(tabs[1]);
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
});

it("does not offer style settings when the chart type owns its appearance", async () => {
  const host = await mount(<ParamsDialog name={indicator.name} indicator={indicator} index={0} params={{ length: 14 }}
    showComparison={false} showLooks={false} onParams={vi.fn()} onLook={vi.fn()} onClose={vi.fn()} />);
  expect([...host.querySelectorAll('[role="tab"]')].map((t) => t.textContent)).toEqual(["Inputs"]);
  expect(host.querySelector('select[aria-label$=" Style"]')).toBeNull();
  expect(host.textContent).not.toContain("Axes & Grid");
});

it("keeps MFI smoothing when switching calculations while only showing it for TradeTiger", async () => {
  const mfi: Indicator = { ...indicator, id: "mfi", name: "Money Flow Index", params: [...indicator.params,
    { key: "variant", label: "Variant (0 TradingView / AmiBroker, 1 TradeTiger)", default: 0, min: 0, max: 1, step: 1, integer: true },
    { key: "smoothing", label: "Smoothing (0 SMA, 1 EMA, 2 WMA, 3 Wilders)", default: 0, min: 0, max: 3, step: 1, integer: true },
  ] };
  const onParams = vi.fn();
  function Harness() {
    const [params, set] = useState<Record<string, number>>({ length: 14, variant: 0, smoothing: 2 });
    return <ParamsDialog name={mfi.name} indicator={mfi} index={0} params={params} showComparison={false}
      onParams={(p) => { onParams(p); set(p); }} onLook={vi.fn()} onClose={vi.fn()} />;
  }
  const host = await mount(<Harness />);
  expect(host.textContent).toContain("Calculation");
  expect(host.querySelector('select[aria-label*="Smoothing"]')).toBeNull();
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label*="Variant"]')!, "1");
  expect(host.querySelector<HTMLSelectElement>('select[aria-label*="Smoothing"]')!.value).toBe("2");
  expect(onParams).toHaveBeenLastCalledWith({ length: 14, variant: 1, smoothing: 2 });
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label*="Variant"]')!, "0");
  expect(host.querySelector('select[aria-label*="Smoothing"]')).toBeNull();
  expect(onParams).toHaveBeenLastCalledWith({ length: 14, variant: 0, smoothing: 2 });
});


it("Cancel restores the initial parameters, style, timeframe, comparison and levels after live previews", async () => {
  const onClose = vi.fn();
  const initial = { params:{length:14}, looks:{ema:{color:"red" as const}}, tf:"W", vs:"SPX", levels:[70,30] };
  function Harness() {
    const [settings, set] = useState<{params:Record<string,number>;looks?:Record<string,Look>;tf?:string;vs?:string;levels:number[]}>(initial);
    return <><output data-settings>{JSON.stringify(settings)}</output><ParamsDialog name={indicator.name} indicator={indicator} index={0} {...settings}
      showComparison comparisonOptions={[{ticker:"SPX",label:"S&P 500",disabled:false,hasVolume:false},{ticker:"NDX",label:"Nasdaq",disabled:false,hasVolume:false}]}
      timeframes={["D","W","M"]} onParams={(params) => set(s => ({...s,params}))} onLook={(looks) => set(s => ({...s,looks}))}
      onTf={(tf) => set(s => ({...s,tf}))} onVs={(vs) => set(s => ({...s,vs}))} onLevels={(levels) => set(s => ({...s,levels}))} onClose={onClose} /></>;
  }
  const host = await mount(<Harness />);
  await input(host.querySelector<HTMLInputElement>('input[type=number]')!, "21");
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label$="Timeframe"]')!, "M");
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label$="Compared with"]')!, "NDX");
  const levels = host.querySelector<HTMLInputElement>('input[aria-label$="Levels"]')!;
  await input(levels,"80, 20");
  await act(async () => levels.dispatchEvent(new FocusEvent("focusout",{bubbles:true})));
  await click([...host.querySelectorAll('[role=tab]')].find(e => e.textContent === "Style")!);
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label$="Color"]')!,"blue");
  await choose(host.querySelector<HTMLSelectElement>('select[aria-label$="Style"]')!,"dashed");
  expect(JSON.parse(host.querySelector('[data-settings]')!.textContent!)).toMatchObject({params:{length:21},tf:"M",vs:"NDX",levels:[80,20],looks:{ema:{color:"blue",style:"dashed"}}});
  await click([...host.querySelectorAll('button')].find(e => e.textContent === "Cancel")!);
  expect(JSON.parse(host.querySelector('[data-settings]')!.textContent!)).toEqual(initial);
  expect(onClose).toHaveBeenCalledOnce();
});

it("OK keeps valid previews, while an invalid draft stays open and never reaches the chart", async () => {
  const onParams = vi.fn(), onClose = vi.fn();
  function Harness() {
    const [params,set] = useState<Record<string,number>>({length:14});
    return <ParamsDialog name={indicator.name} indicator={indicator} index={0} params={params} showComparison={false}
      onParams={p => {onParams(p);set(p);}} onLook={() => {}} onClose={onClose} />;
  }
  const host = await mount(<Harness />);
  const number = host.querySelector<HTMLInputElement>('input[type=number]')!;
  const ok = [...host.querySelectorAll('button')].find(e => e.textContent === "OK")!;
  await input(number, "0");
  await click(ok);
  expect(onClose).not.toHaveBeenCalled();
  expect(onParams).not.toHaveBeenCalled();
  expect(number.getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(number);
  await input(number, "21");
  await click(ok);
  expect(onParams).toHaveBeenLastCalledWith({length:21});
  expect(onClose).toHaveBeenCalledOnce();
});

it("the close icon and Escape cancel parameter edits", async () => {
  for (const exit of ["icon","escape"]) {
    const onParams = vi.fn(), onClose = vi.fn();
    const host = await mount(<ParamsDialog name={indicator.name} indicator={indicator} index={0} params={{length:14}} showComparison={false}
      onParams={onParams} onLook={() => {}} onClose={onClose} />);
    await input(host.querySelector<HTMLInputElement>('input[type=number]')!,"22");
    if (exit === "icon") await click(host.querySelector('[aria-label="Close parameters"]')!);
    else await act(async () => host.querySelector('.lab-params')!.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})));
    expect(onParams).toHaveBeenLastCalledWith({length:14});
    expect(onClose).toHaveBeenCalledOnce();
    await unmount();
  }
});
