import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ChartPaneDivider } from "../src/features/lab/ChartPaneDivider";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.restoreAllMocks(); });
function Harness() {
  const [height, setHeight] = useState(240);
  return <ChartPaneDivider label="Resize RSI pane" height={height} defaultHeight={240} onChange={setHeight} />;
}
const pointer = (element: HTMLElement, type: string, y: number, id = 7, button = 0) => act(async () => {
  const event = new MouseEvent(type, { clientY:y, button, bubbles:true });
  Object.defineProperty(event, 'pointerId', {value:id}); element.dispatchEvent(event);
});
const key = (element: HTMLElement, value: string) => act(async () => { element.dispatchEvent(new KeyboardEvent('keydown', {key:value,bubbles:true})); });

it('resizes the preceding pane within bounds and stops after pointer release', async () => {
  const host = await mount(<Harness />), divider = host.querySelector<HTMLElement>('[role="separator"]')!;
  const capture = vi.fn(), release = vi.fn();
  Object.assign(divider, {setPointerCapture:capture,hasPointerCapture:()=>true,releasePointerCapture:release});
  expect(divider.getAttribute('aria-orientation')).toBe('horizontal');
  await pointer(divider,'pointerdown',200);
  expect(capture).toHaveBeenCalledWith(7);
  await pointer(divider,'pointermove',280,8);
  expect(divider.getAttribute('aria-valuenow')).toBe('240');
  await pointer(divider,'pointermove',280);
  expect(divider.getAttribute('aria-valuenow')).toBe('320');
  await pointer(divider,'pointermove',1000);
  expect(divider.getAttribute('aria-valuenow')).toBe('700');
  await pointer(divider,'pointermove',-200);
  expect(divider.getAttribute('aria-valuenow')).toBe('80');
  await pointer(divider,'pointerup',-200);
  expect(release).toHaveBeenCalledWith(7);
  await pointer(divider,'pointermove',400);
  expect(divider.getAttribute('aria-valuenow')).toBe('80');
});

it('supports keyboard limits, 24px increments and a double-click reset', async () => {
  const host = await mount(<Harness />), divider = host.querySelector<HTMLElement>('[role="separator"]')!;
  await key(divider,'ArrowDown'); expect(divider.getAttribute('aria-valuenow')).toBe('264');
  await key(divider,'ArrowUp'); expect(divider.getAttribute('aria-valuenow')).toBe('240');
  await key(divider,'Home'); await key(divider,'ArrowUp'); expect(divider.getAttribute('aria-valuenow')).toBe('80');
  await key(divider,'End'); await key(divider,'ArrowDown'); expect(divider.getAttribute('aria-valuenow')).toBe('700');
  await act(async () => divider.dispatchEvent(new MouseEvent('dblclick', {bubbles:true})));
  expect(divider.getAttribute('aria-valuenow')).toBe('240');
});

it('ignores nonprimary presses and releases capture on cancellation or unmount', async () => {
  const host = await mount(<Harness />), divider = host.querySelector<HTMLElement>('[role="separator"]')!;
  const release = vi.fn();
  Object.assign(divider, {setPointerCapture:vi.fn(),hasPointerCapture:()=>true,releasePointerCapture:release});
  await pointer(divider,'pointerdown',200,7,2); await pointer(divider,'pointermove',300);
  expect(divider.getAttribute('aria-valuenow')).toBe('240');
  await pointer(divider,'pointerdown',200); await pointer(divider,'pointercancel',200);
  expect(release).toHaveBeenCalledTimes(1);
  await pointer(divider,'pointermove',300); expect(divider.getAttribute('aria-valuenow')).toBe('240');
  await pointer(divider,'pointerdown',200); await unmount();
  expect(release).toHaveBeenCalledTimes(2);
});
