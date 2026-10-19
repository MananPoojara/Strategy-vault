import { afterEach, expect, it } from "vitest";
import { act } from "react";
import { MonteCarloPanel } from "../src/features/lab/MonteCarlo";
import { ENVELOPE_POINTS, equityPath, fixedReturns, MAX_WORK, mulberry32, percentile, PERCENTILES, runMonteCarlo, spanYears, tradeReturns } from "../src/features/lab/monte-carlo";
import type { LedgerRow, Trade } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const close = (a: number, b: number) => expect(a).toBeCloseTo(b, 10);

it("interpolates percentiles of a known small case", () => {
  const s = [1, 2, 3, 4, 5];
  expect([0, 10, 25, 50, 99, 100].map((p) => percentile(s, p))).toEqual([1, 1.4, 2, 3, 4.96, 5]);
  expect(percentile([], 50)).toBeNaN();
});

it("builds drawdown, lowest equity and annual return from a path starting at 1.0", () => {
  const p = equityPath([0.1, -0.5, 0.2], { kind: "none" }, 1);
  close(p.final, 0.66); close(p.maxDd, -0.5); close(p.maxDdAbs, -0.55); close(p.lowest, 0.55); close(p.cagr!, -0.34); // drawdown ≤ 0, as in AmiBroker
  expect(equityPath([0.1, 0.1], { kind: "none" }).lowest).toBe(1); // the start counts; no cagr without years
  expect(equityPath([0.1], { kind: "none" }).cagr).toBeNull();
  const ruin = equityPath([-2, 0.5], { kind: "none" }, 2); // a short can lose more than 100 %: equity floors at 0
  expect([ruin.final, ruin.lowest, ruin.maxDd, ruin.cagr]).toEqual([0, 0, -1, -1]);
});

it("fixed fraction applies the trade % to that share of equity", () => {
  const p = equityPath([0.1, -0.5, 0.2], { kind: "fraction", pct: 10 });
  close(p.final, 1.01 * 0.95 * 1.02); close(p.maxDd, -0.05); close(p.lowest, 1.01 * 0.95);
});

it("is deterministic for a seed and resamples with replacement", () => {
  const rets = [0.05, -0.03, 0.12, -0.08, 0.02, 0.01];
  const input = { rets, runs: 200, sizing: { kind: "none" } as const, seed: 1, years: 2 };
  const a = runMonteCarlo(input), b = runMonteCarlo(input), c = runMonteCarlo({ ...input, seed: 2 });
  expect(a).toEqual(b);
  expect(c.finals).not.toEqual(a.finals);
  expect(a.rows.map((r) => r.p)).toEqual([...PERCENTILES]);
  expect(a.finals).toEqual([...a.finals].sort((x, y) => x - y));
  expect(a.envelope.median).toHaveLength(rets.length + 1);
  expect(a.envelope.min[0]).toBe(1);
  a.envelope.min.forEach((m, k) => expect(m).toBeLessThanOrEqual(a.envelope.max[k]));
  expect(new Set(a.finals).size).toBeGreaterThan(1); // sampling with replacement, not a permutation (which would fix the final)
  const r = mulberry32(7); const xs = Array.from({ length: 1000 }, r);
  expect(xs.every((x) => x >= 0 && x < 1)).toBe(true);
  expect(mulberry32(7)()).toBe(xs[0]);
});

it("one trade gives the same path every run; years come from the backtest dates", () => {
  const res = runMonteCarlo({ rets: [0.21], runs: 10, sizing: { kind: "none" }, seed: 1, years: 2 });
  res.rows.forEach((row) => { close(row.final, 1.21); close(row.cagr!, 0.1); expect(row.maxDd).toBe(0); expect(row.lowest).toBe(1); });
  close(spanYears("2020-01-01", "2022-01-01")!, 731 / 365.25);
  expect(spanYears("2020-01-01", "2020-01-01")).toBeNull();
});

const t = (ret: number, open = false) => ({ ret, open, bars: 3, dir: 1 }) as Trade;
const choose = async (el: Element, value: string) => act(async () => { (el as HTMLSelectElement).value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("runs on click, excludes open trades and shows the AmiBroker table and five charts", async () => {
  const host = await mount(<MonteCarloPanel trades={[t(0.1), t(-0.05), t(0.02), t(0.3, true)]} from="2020-01-01" to="2022-01-01" />);
  expect(host.querySelector("h3")!.textContent).toBe("Monte Carlo resampled simulated trades · simulated · not a book method · not a forecast");
  expect(host.querySelector("[data-lab-mc-note]")!.textContent).toBe("Resamples 3 closed trades with replacement; 1 open trade is excluded.");
  expect(host.querySelector("[data-lab-mc-table]")).toBeNull();
  expect(host.querySelector('[aria-label="Percent of equity"]')).toBeNull();
  await choose(host.querySelector('[aria-label="Sizing"]')!, "fraction");
  expect((host.querySelector('[aria-label="Percent of equity"]') as HTMLInputElement).value).toBe("5");
  const button = [...host.querySelectorAll("button")].find((b) => b.textContent === "Run Monte Carlo")!;
  await act(async () => button.click());
  expect(host.querySelectorAll("[data-lab-mc-table] tbody tr")).toHaveLength(9);
  expect(button.classList.contains("primary")).toBe(true);
  expect([...host.querySelectorAll("[data-lab-mc-table] thead th")].map((th) => th.textContent)).toEqual(["Percentile", "Final Equity", "Annual Return", "Max. Drawdown ×", "Max. Drawdown %", "Lowest Eq."]);
  expect([...host.querySelectorAll("[data-lab-mc-chart]")].map((f) => f.getAttribute("data-lab-mc-chart"))).toEqual(["envelope", "final", "drawdown", "drawdown-abs", "lowest"]);
  expect([...host.querySelectorAll("[data-lab-mc-chart] figcaption")].map((f) => f.firstChild!.textContent)).toEqual(["MC Min/Max Equity", "MC Final Equity", "MC Drawdown %", "MC Drawdown ×", "MC Lowest Equity"]);
  expect(host.querySelector("[data-lab-mc-chart=final] .lab-report-axis")!.textContent).toBe("0Percentile100");
  expect(host.querySelectorAll("[data-lab-mc-chart=envelope] polyline")).toHaveLength(3);
});

it("money mode shows equity and the drawdown in money from the capital", async () => {
  const trades = [{ ...t(0.1), entry: "2020-01-02" }, { ...t(-0.05), entry: "2020-02-03" }];
  const ledger = [{ entry: "2020-01-02", pnl: 1000 }, { entry: "2020-02-03", pnl: -505 }] as LedgerRow[];
  const host = await mount(<MonteCarloPanel trades={trades} from="2020-01-01" to="2022-01-01" ledger={ledger} capital={100_000} />);
  await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "Run Monte Carlo")!.click());
  expect(host.querySelectorAll("[data-lab-mc-table] thead th")[3].textContent).toBe("Max. Drawdown $");
  const cells = [...host.querySelectorAll("[data-lab-mc-table] tbody tr:last-child td")].map((td) => td.textContent);
  expect(Number(cells[1]!.replace(/,/g, ""))).toBeGreaterThan(100_000); // the 99% final equity in money
  expect(Number(cells[5]!.replace(/,/g, ""))).toBeLessThanOrEqual(100_000);
});

it("refuses to run without closed trades", async () => {
  const host = await mount(<MonteCarloPanel trades={[t(0.3, true)]} from="2020-01-01" to="2022-01-01" />);
  expect(host.querySelector("[data-lab-mc-note]")!.textContent).toBe("No closed trades to resample.");
  expect(([...host.querySelectorAll("button")].find((b) => b.textContent === "Run Monte Carlo") as HTMLButtonElement).disabled).toBe(true);
});

it("the 1% row holds the worst (most negative) drawdown", () => {
  const res = runMonteCarlo({ rets: [0.1, -0.2, 0.05, -0.1, 0.3], runs: 500, sizing: { kind: "none" }, seed: 3, years: null });
  const dd = res.rows.map((r) => r.maxDd);
  expect(dd.every((d) => d <= 0)).toBe(true);
  expect(dd[0]).toBeLessThanOrEqual(dd[dd.length - 1]);
  expect(res.drawdowns[0]).toBe(Math.min(...res.drawdowns));
  expect(res.rows[0].maxDdAbs).toBeLessThanOrEqual(res.rows.at(-1)!.maxDdAbs);
  expect(res.drawdownsAbs).toEqual([...res.drawdownsAbs].sort((x, y) => x - y));
  expect(res.lowests).toHaveLength(res.runs);
});

it("keeps at most ENVELOPE_POINTS envelope columns and caps runs × trades", () => {
  const rets = Array.from({ length: 4000 }, (_, i) => (i % 3 ? 0.01 : -0.015));
  const res = runMonteCarlo({ rets, runs: 5000, sizing: { kind: "none" }, seed: 1, years: null });
  expect(res.runs).toBe(Math.floor(MAX_WORK / 4000));
  expect(res.capped).toBe(true);
  expect(res.envelope.at).toHaveLength(ENVELOPE_POINTS);
  expect([res.envelope.at[0], res.envelope.at.at(-1)]).toEqual([0, 4000]);
  expect(res.envelope.median).toHaveLength(ENVELOPE_POINTS);
  expect(runMonteCarlo({ rets: [0.1, -0.05], runs: 100, sizing: { kind: "none" }, seed: 1, years: null }).capped).toBe(false);
});

it("money mode resamples ledger P&L over the equity before each trade, not the full-position return", () => {
  const trades = [{ ...t(0.1), entry: "2020-01-02" }, { ...t(-0.05), entry: "2020-02-03" }, { ...t(0.2, true), entry: "2020-03-02" }];
  const ledger = [{ entry: "2020-01-02", pnl: 1000 }, { entry: "2020-02-03", pnl: -505 }, { entry: "2020-03-02", pnl: 50 }] as LedgerRow[];
  const r = tradeReturns(trades, ledger, 100_000);
  expect(r).toHaveLength(2); // the open trade is left out
  close(r[0], 0.01); close(r[1], -505 / 101_000);
  expect(tradeReturns(trades)).toEqual([0.1, -0.05]); // unit mode: the trade return is already its effect on equity
  expect(tradeReturns(trades, ledger.slice(1), 100_000)).toEqual([0.1, -0.05]); // an unaligned ledger is not used
});

it("Fixed size and Fixed value: per-trade P&L over the capital, added to equity, not compounded; ruin stops trading", () => {
  const trades = [{ ...t(0.1), entry_px: 200 }, { ...t(-0.05), entry_px: 100 }, { ...t(0.3, true), entry_px: 50 }, { ...t(0.2), entry_px: null }] as Trade[];
  const shares = fixedReturns(trades, "shares", 10, 1000);
  expect(shares).toHaveLength(2); // the open trade and the one without an entry price are left out
  close(shares[0], 0.2); close(shares[1], -0.05); // 10 × 200 × 10 % = 200 of 1000; 10 × 100 × −5 % = −50
  const value = fixedReturns(trades, "value", 500, 1000);
  expect(value).toHaveLength(3);
  close(value[0], 0.05); close(value[1], -0.025); close(value[2], 0.1);
  const p = equityPath([0.2, -0.05, 0.2], { kind: "fixed" });
  close(p.final, 1.35); close(p.maxDd, -0.05 / 1.2); close(p.maxDdAbs, -0.05); expect(p.lowest).toBe(1);
  expect(equityPath([-1.5, 0.5], { kind: "fixed" }).final).toBe(0);
});

it("the panel offers Fixed size and Fixed value in money mode only, with the amount and a disclosure", async () => {
  const unit = await mount(<MonteCarloPanel trades={[t(0.1)]} from="2020-01-01" to="2022-01-01" />);
  expect([...unit.querySelectorAll<HTMLOptionElement>('[aria-label="Sizing"] option')].map((o) => [o.textContent, o.disabled]))
    .toEqual([["Don't change", false], ["Fixed size", true], ["Fixed value", true], ["Percent of equity", false]]);
  await unmount();
  const trades = [{ ...t(0.1), entry: "2020-01-02", entry_px: 100 }, { ...t(-0.05), entry: "2020-02-03", entry_px: 100 }];
  const ledger = [{ entry: "2020-01-02", pnl: 1000 }, { entry: "2020-02-03", pnl: -505 }] as LedgerRow[];
  const host = await mount(<MonteCarloPanel trades={trades} from="2020-01-01" to="2022-01-01" ledger={ledger} capital={100_000} />);
  await choose(host.querySelector('[aria-label="Sizing"]')!, "shares");
  expect((host.querySelector('[aria-label="Shares per trade"]') as HTMLInputElement).value).toBe("100");
  expect(host.querySelector("[data-lab-mc-note]")!.textContent).toContain("Fixed size: shares × entry price × trade % is added to equity per trade (not compounded)");
  await choose(host.querySelector('[aria-label="Sizing"]')!, "value");
  const input = host.querySelector('[aria-label="Value per trade"]') as HTMLInputElement;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "0"); input.dispatchEvent(new Event("input", { bubbles: true })); });
  expect(host.querySelector("[data-lab-mc-note]")!.textContent).toBe("Value per trade must be above 0.");
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "50000"); input.dispatchEvent(new Event("input", { bubbles: true })); });
  await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "Run Monte Carlo")!.click());
  // every run draws from +5,000 and −2,500: finals lie between 100,000 − 2 × 2,500 and 100,000 + 2 × 5,000
  const finals = [...host.querySelectorAll("[data-lab-mc-table] tbody tr")].map((tr) => Number(tr.children[1].textContent!.replace(/,/g, "")));
  expect(Math.min(...finals)).toBeGreaterThanOrEqual(95_000); expect(Math.max(...finals)).toBeLessThanOrEqual(110_000);
});
