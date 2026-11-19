import { afterEach, expect, it } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { DailyWorkTable, DAILY_HEADERS, fmtTokens, fmtTime, fmtDate } from "../src/pages/SnapshotModal";
import type { DailyWork } from "../src/services/research";

let root: Root | undefined, host: HTMLDivElement | undefined;
afterEach(async () => { if (root) await act(async () => root!.unmount()); host?.remove(); root = host = undefined; });

const data: DailyWork = { schema: 1, method: "", rows: [{ date: "2026-11-19", new: 14, backtested_day: 13, bt_minutes: 936, bt_tokens: 4103696, errors_solved: 1, fix_minutes: 138, fix_tokens: 340000,
  newly_held: 1, newly_held_list: [{ strategy: "Busted Options Program", book: "STC IX", reason: "Needs option prices" }], held: 743, backtested_total: 1744, books_added: 8, deploys: 8,
  worked_by: "Platform", avg_tokens_per_strategy: 315668, proof: "library-progress.json (int-1119a..g)" }] };

it("renders the 16 headers in order, formatted cells and the held list", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<DailyWorkTable data={data} />));
  expect([...host.querySelectorAll("th")].map(t => t.textContent)).toEqual(DAILY_HEADERS);
  expect(DAILY_HEADERS).toHaveLength(16);
  const td = [...host.querySelectorAll("tbody tr td")].map(t => t.textContent);
  expect(td.slice(0, 5)).toEqual(["19/11/2026", "14", "13", "15:36", "4.1M"]);
  expect(td[7]).toBe("340k");
  await act(async () => host!.querySelector<HTMLButtonElement>("tbody button")!.click());
  expect(host.textContent).toContain("Needs option prices");
});
it("formats", () => { expect(fmtTokens(1234567)).toBe("1.2M"); expect(fmtTokens(340400)).toBe("340k"); expect(fmtTime(65)).toBe("1:05"); expect(fmtDate("2026-11-01")).toBe("01/11/2026"); });
