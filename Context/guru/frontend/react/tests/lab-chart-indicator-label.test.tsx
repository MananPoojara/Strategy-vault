import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ChartIndicatorLabel } from "../src/features/lab/ChartIndicatorLabel";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const click = async (el: Element) => act(async () => (el as HTMLElement).click());

it("opens settings from the name or settings action and removes the indicator", async () => {
  const settings = vi.fn(), remove = vi.fn(), toggle = vi.fn();
  const host = await mount(<ChartIndicatorLabel name="MFI" settings="14 · TradeTiger · SMA" color="#245a94" onSettings={settings} onRemove={remove} onToggle={toggle} />);
  await click(host.querySelector('[aria-label="Edit MFI indicator"]')!);
  await click(host.querySelector('[aria-label="Indicator settings for MFI"]')!);
  expect(settings).toHaveBeenCalledTimes(2);
  await click(host.querySelector('[aria-label="Remove indicator MFI"]')!);
  expect(remove).toHaveBeenCalledTimes(1);
  await click(host.querySelector('[aria-label="Hide MFI"]')!);
  expect(toggle).toHaveBeenCalledTimes(1);
  expect(host.querySelector('.lab-indicator-settings')!.getAttribute('title')).toBe('14 · TradeTiger · SMA');
  expect(host.querySelector('[aria-label="MFI values"]')!.hasAttribute('aria-live')).toBe(false);
});

it("labels visibility accurately and preserves chart-owned value text through rerenders", async () => {
  let values: HTMLSpanElement | null = null;
  function Harness() {
    const [hidden, setHidden] = useState(false);
    return <ChartIndicatorLabel name="RSI" settings="14" color="#245a94" hidden={hidden}
      valuesRef={(el) => { values = el; }} onSettings={vi.fn()} onRemove={vi.fn()} onToggle={() => setHidden(v => !v)} />;
  }
  const host = await mount(<Harness />);
  const current = host.querySelector<HTMLSpanElement>('[aria-label="RSI values"]')!;
  expect(values).toBe(current);
  current.textContent = '57.23';
  await click(host.querySelector('[aria-label="Hide RSI"]')!);
  const show = host.querySelector('[aria-label="Show RSI"]')!;
  expect(show.getAttribute('aria-pressed')).toBe('true');
  expect(host.querySelector('[aria-label="RSI values"]')).toBe(current);
  expect(current.textContent).toBe('57.23');
  expect(host.querySelector('.lab-indicator-label-hidden')).not.toBeNull();
  await click(show);
  expect(host.querySelector('[aria-label="Hide RSI"]')!.getAttribute('aria-pressed')).toBe('false');
});
