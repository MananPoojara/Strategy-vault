import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ErrorNotice, LabErrorBoundary, MetricTiles, ProvisionalStrip, Segmented, ServerLabel, isProvisional } from "../src/features/lab/ui";
import { int, num, pct } from "../src/features/lab/format";
import { PROVISIONAL_NOTE } from "../src/features/lab/labels";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.useRealTimers(); });
const err = (fields: Record<string, unknown>) => Object.assign(new Error(String(fields.message)), fields);

it("formats numbers with a dash for missing values", () => {
  expect([pct(0.1234), pct(null), pct(Number.NaN), num(1234.5), int(12.6)]).toEqual(["12.3%", "—", "—", "1,234.50", "13"]);
});

it("shows engine errors as given with a status heading and no provisional note on India", async () => {
  const host = await mount(<ErrorNotice market="in" error={err({ message: "NIFTY 50: window starts before 2001", status: 400, kind: "engine", engineStatus: "needs_data" })} />);
  expect(host.querySelector("[data-lab-error]")?.getAttribute("role")).toBe("alert");
  expect(host.textContent).toContain("Data needed");
  expect(host.textContent).toContain("NIFTY 50: window starts before 2001");
  expect(host.querySelector("[data-lab-provisional]")).toBeNull();
});

it("labels gl errors as provisional even when a gateway error has no flag", async () => {
  const host = await mount(<ErrorNotice market="crypto" instrumentLabel="Provisional · TradingView export CRYPTO:BTCUSD, daily"
    error={err({ message: "The server is busy. Please retry shortly.", status: 429, kind: "gateway" })} />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.textContent).toContain("CRYPTO:BTCUSD, daily");
  expect(isProvisional("gl") && isProvisional("crypto") && !isProvisional("us")).toBe(true);
});

it("429 shows a countdown, then a working retry", async () => {
  vi.useFakeTimers();
  const onRetry = vi.fn();
  const host = await mount(<ErrorNotice market="in" onRetry={onRetry} error={err({ message: "An optimization is already running.", status: 429, kind: "gateway", retryAfter: 2 })} />);
  const button = () => host.querySelector("button")!;
  expect(button().disabled).toBe(true);
  expect(button().textContent).toBe("Try again in 2 s");
  await act(async () => { vi.advanceTimersByTime(2000); });
  expect(button().disabled).toBe(false);
  await act(async () => button().click());
  expect(onRetry).toHaveBeenCalledOnce();
});

it("the boundary keeps a crashing part from blanking the page", async () => {
  const Boom = () => { throw new Error("bad series"); };
  vi.spyOn(console, "error").mockImplementation(() => {});
  const host = await mount(<div><h1>Header</h1><LabErrorBoundary><Boom /></LabErrorBoundary></div>);
  expect(host.textContent).toContain("Header");
  expect(host.querySelector("[role=alert]")?.textContent).toContain("bad series");
});

it("segmented control, labels, strip and tiles", async () => {
  const onChange = vi.fn();
  const host = await mount(<>
    <Segmented label="Market" value="in" onChange={onChange}
      options={[{ value: "in", label: "India" }, { value: "gl", label: "Global", disabled: true, title: "No data" }]} />
    <ServerLabel text="User-defined indicator strategy · simulated · not a book method" />
    <ProvisionalStrip instrumentLabel="Provisional · TradingView export IG:DAX, daily" />
    <MetricTiles items={[{ label: "CAGR", value: "12.0%", note: "net" }]} />
  </>);
  const [india, global] = host.querySelectorAll<HTMLButtonElement>(".join-item");
  expect(india.className).toContain("active");
  expect(india.getAttribute("aria-pressed")).toBe("true");
  expect(global.disabled).toBe(true);
  await act(async () => india.click());
  expect(onChange).toHaveBeenCalledWith("in");
  expect(host.querySelector("[data-server-label]")?.textContent).toBe("User-defined indicator strategy · simulated · not a book method");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.querySelector(".metric b")?.textContent).toBe("12.0%");
});
