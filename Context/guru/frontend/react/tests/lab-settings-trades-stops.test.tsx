import { act } from "react";
import { expect, it, vi } from "vitest";
import { toTradePrices } from "../src/features/lab/AmiSettings";
import { withDirection, withEntryOrder } from "../src/features/lab/execution-model";
import { StopsTab } from "../src/features/lab/settings/StopsTab";
import { TradesTab } from "../src/features/lab/settings/TradesTab";
import { ONE_INSTRUMENT } from "../src/features/lab/settings/common";
import { defaultSpec, setExecution } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture, mount } from "./fixtures/lab";

const pick = (host: HTMLElement, selector: string, v: string) => act(async () => {
  const el = host.querySelector<HTMLSelectElement>(selector)!; el.value = v; el.dispatchEvent(new Event("change", { bubbles: true }));
});
const press = (host: HTMLElement, group: string, text: string) => act(async () => {
  [...host.querySelectorAll<HTMLButtonElement>(`[aria-label="${group}"] button`)].find((b) => b.textContent === text)!.click();
});
const portfolio = (s: Spec) => ({ ...s, portfolio: { universe: { basket: "nifty50" } } }) as Spec;

it("Trades keeps both groups visible: short fields greyed with a reason on a long-only strategy", async () => {
  const onChange = vi.fn();
  const host = await mount(<TradesTab spec={toTradePrices(defaultSpec())} catalogue={catalogueFixture} onChange={onChange} />);
  const groups = [...host.querySelectorAll<HTMLFieldSetElement>("[data-lab-trade-group]")];
  expect(groups.map((g) => [g.querySelector("legend")!.textContent, g.disabled])).toEqual([["Long trades", false], ["Short trades", true]]);
  expect(groups[1].textContent).toContain("This strategy takes long trades only.");
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Cover price"]')).not.toBeNull();
  await pick(host, 'select[aria-label="Sell price"]', "close");
  expect(onChange.mock.calls.at(-1)![0].execution.prices).toEqual({ buy: "open", sell: "close" });
});

it("Trades greys the grid under a scenario, the buy row of a limit entry, and the whole grid in a portfolio", async () => {
  let host = await mount(<TradesTab spec={setExecution(defaultSpec(), { fill: "close" })} catalogue={catalogueFixture} onChange={vi.fn()} />);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Buy price"]')!.value).toBe("close"); // the scenario as prices
  expect(host.textContent).toContain("Set by the scenario; choose Custom to edit.");
  host = await mount(<TradesTab spec={withEntryOrder(toTradePrices(withDirection(defaultSpec(), "both")), "", "limit")} catalogue={catalogueFixture} onChange={vi.fn()} />);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Buy price"]')!.disabled).toBe(true);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Sell price"]')!.disabled).toBe(false);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Scenario"]')!.disabled).toBe(true);
  expect(host.textContent).toContain("Enters by a limit or stop order");
  host = await mount(<TradesTab spec={portfolio(defaultSpec())} catalogue={catalogueFixture} onChange={vi.fn()} />);
  expect([...host.querySelectorAll<HTMLFieldSetElement>("[data-lab-trade-group]")].every((g) => g.disabled)).toBe(true);
  expect(host.textContent).toContain(ONE_INSTRUMENT);
});

it("Stops keeps the current defaults: intraday exits and Activate stops immediately checked", async () => {
  const host = await mount(<StopsTab spec={defaultSpec()} onChange={vi.fn()} />);
  expect([...host.querySelectorAll("legend")].map((l) => l.textContent)).toEqual(["Maximum loss stop", "Profit target", "Trailing stop", "N-bar stop"]);
  expect(host.querySelector<HTMLInputElement>(".lab-check input")!.checked).toBe(true);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Stop-loss exit mode"]')!.value).toBe("1");
  // Percent of profit only on the trailing stop, as in AmiBroker
  expect(host.querySelector('[aria-label="Stop-loss type"]')!.textContent).not.toContain("Percent of profit");
  expect(host.querySelector('[aria-label="Trailing stop type"]')!.textContent).toContain("Percent of profit");
});

it("Stops edits the Exit card's fields: stop kind and amount, exit mode, re-entry delay, N-bar and Has priority", async () => {
  const onChange = vi.fn();
  let host = await mount(<StopsTab spec={defaultSpec()} onChange={onChange} />);
  await press(host, "Stop-loss type", "Percent");
  expect(onChange.mock.calls.at(-1)![0].exit.stop_loss).toEqual({ pct: 5 });
  await press(host, "Trailing stop type", "Percent of profit");
  expect(onChange.mock.calls.at(-1)![0].exit.trailing).toEqual({ profit_pct: 40 });
  await pick(host, 'select[aria-label="Target exit mode"]', "0");
  expect(onChange.mock.calls.at(-1)![0].execution.exit_at_stop).toEqual({ target: 0 });
  await press(host, "N-bar stop type", "Disabled");
  expect(onChange.mock.calls.at(-1)![0].exit.any).toEqual([]);
  const spec: Spec = { ...defaultSpec(), exit: { ...defaultSpec().exit, stop_loss: { pct: 3 } } };
  host = await mount(<StopsTab spec={spec} onChange={onChange} />);
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Stop-loss loss (%)"]')!.value).toBe("3");
  await act(async () => { host.querySelector<HTMLInputElement>('[data-lab-stop="n_bar"] .lab-check input')!.click(); });
  expect(onChange.mock.calls.at(-1)![0].execution.nbar_has_priority).toBe(true);
});

it("Stops shows both sides' amounts on Long and short, keeps an entry-candle stop as is, and greys single-instrument settings in a portfolio", async () => {
  let host = await mount(<StopsTab spec={withDirection(defaultSpec(), "both")} onChange={vi.fn()} />);
  expect(host.querySelector('[aria-label="Long stop-loss type"]')).not.toBeNull();
  expect(host.querySelector('[aria-label="Short stop-loss type"]')).not.toBeNull();
  host = await mount(<StopsTab spec={{ ...defaultSpec(), exit: { ...defaultSpec().exit, stop_loss: { candle: "low" } } } as unknown as Spec} onChange={vi.fn()} />);
  expect(host.querySelector('[data-lab-stop="stop_loss"]')!.textContent).toContain('"candle":"low"');
  host = await mount(<StopsTab spec={portfolio(defaultSpec())} onChange={vi.fn()} />);
  expect(host.querySelector<HTMLInputElement>(".lab-check input")!.disabled).toBe(true);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Stop-loss exit mode"]')!.closest("fieldset:disabled")).not.toBeNull();
  expect(host.querySelector<HTMLButtonElement>('[aria-label="Stop-loss type"] button')!.disabled).toBe(false); // the stop itself still runs
  expect(host.textContent).toContain(ONE_INSTRUMENT);
});
