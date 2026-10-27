import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ChartPopup } from "../src/features/lab/ChartPopup";
import { mount, unmount } from "./fixtures/lab";

let observe: ResizeObserverCallback;
beforeEach(() => {
  vi.stubGlobal('innerWidth', 1000); vi.stubGlobal('innerHeight', 800);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(() => ({ width:Math.min(460, window.innerWidth - 24), height:300, left:0, top:0, right:460, bottom:300, x:0, y:0, toJSON(){} }));
  vi.stubGlobal('ResizeObserver', class { constructor(callback: ResizeObserverCallback) { observe = callback; } observe() {} disconnect() {} });
});
afterEach(async () => { await unmount(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
const popup = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="dialog"]')!;
const point = (target: HTMLElement, type: string, x: number, y: number) => act(async () => {
  const event = new MouseEvent(type, { clientX:x, clientY:y, button:0, bubbles:true });
  Object.defineProperty(event, 'pointerId', { value:3 }); target.dispatchEvent(event);
});
const contents = <><div className="lab-params-h"><b>Indicator</b><button>Close</button></div><input aria-label="Length" /></>;

it('drags from the existing header, respects viewport bounds, and stops on release', async () => {
  const host = await mount(<ChartPopup title="Indicator settings" onClose={vi.fn()}>{contents}</ChartPopup>);
  const box = popup(host), header = host.querySelector<HTMLElement>('.lab-params-h')!;
  expect(box.getAttribute('aria-modal')).toBe('false');
  expect(box.style.left).toBe('270px');
  await point(header,'pointerdown',650,120); await point(box,'pointermove',950,700);
  expect(box.style.left).toBe('528px'); expect(box.style.top).toBe('488px');
  await point(box,'pointermove',0,0);
  expect(box.style.left).toBe('12px'); expect(box.style.top).toBe('12px');
  await point(box,'pointerup',0,0); await point(box,'pointermove',500,500);
  expect(box.style.left).toBe('12px');
});

it('leaves header buttons interactive and supports Alt+arrows and viewport refitting', async () => {
  const host = await mount(<ChartPopup title="Settings" onClose={vi.fn()}>{contents}</ChartPopup>);
  const box = popup(host);
  await point(host.querySelector('button')!,'pointerdown',650,120); await point(box,'pointermove',100,100);
  expect(box.style.left).toBe('270px');
  await act(async () => box.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowLeft',altKey:true,bubbles:true})));
  expect(box.style.left).toBe('246px');
  vi.stubGlobal('innerWidth',500); vi.stubGlobal('innerHeight',400);
  await act(async () => window.dispatchEvent(new Event('resize')));
  expect(box.style.left).toBe('28px'); expect(box.style.top).toBe('88px');
  vi.stubGlobal('innerWidth',450);
  await act(async () => observe([],{} as ResizeObserver));
  expect(box.style.left).toBe('12px');
});

it('Escape closes the window and returns focus to its opener', async () => {
  function Harness(){const [open,setOpen]=useState(false);return <><button onClick={()=>setOpen(true)}>Indicators</button>{open&&<ChartPopup title="Indicators" onClose={()=>setOpen(false)}>{contents}</ChartPopup>}</>;}
  const host = await mount(<Harness/>), opener=host.querySelector<HTMLButtonElement>('button')!;
  await act(async()=>{opener.focus();opener.click();});
  expect(host.querySelector('[role="dialog"]')).not.toBeNull();
  expect(document.activeElement).not.toBe(opener);
  await act(async()=>popup(host).dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})));
  expect(host.querySelector('[role="dialog"]')).toBeNull(); expect(document.activeElement).toBe(opener);
});
