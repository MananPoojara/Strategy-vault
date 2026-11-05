import { expect, it } from "vitest";
import { fiveDpRanges, master5dp, regimeSets, tradeLossMonths, type Bar } from "../src/features/lab/master5dp";
import { MASTER_5DP_NIFTY, QTR_MIDCAP } from "../src/features/lab/hardpunch";

/** Flat weekday bars from..to (o = h = l = c = px(date)). */
const bars = (from: string, to: string, px: (d: string) => number = () => 100): Bar[] => {
  const out: Bar[] = [];
  for (let t = Date.parse(`${from}T00:00:00Z`); t <= Date.parse(`${to}T00:00:00Z`); t += 864e5) {
    const d = new Date(t).toISOString().slice(0, 10), wd = new Date(t).getUTCDay();
    if (wd !== 0 && wd !== 6) { const p = px(d); out.push({ d, o: p, h: p, l: p, c: p }); }
  }
  return out;
};
/** Falls through the given months (each month's close below its open), flat elsewhere. */
const falling = (months: string[]) => {
  let p = 1000;
  return (d: string) => (months.includes(d.slice(0, 7)) ? (p -= 1) : p);
};

it("a 5DP range starts on the 6th-from-last full day of a 31-day month, the 5th otherwise, and ends on the 2nd day of the next", () => {
  const r = fiveDpRanges(bars("2021-01-01", "2021-03-31"));
  expect(r.find((x) => x.mo === "2021-01")).toMatchObject({ start: "2021-01-22", end: "2021-02-02" });
  expect(r.find((x) => x.mo === "2021-02")).toMatchObject({ start: "2021-02-22", end: "2021-03-02" });
  expect(r.find((x) => x.mo === "2021-03")?.end).toBeNull(); // April has not traded yet
  // a weekday Muhurat session is a real bar but not a full trading day, so it does not count
  expect(fiveDpRanges(bars("2022-10-01", "2022-11-30")).find((x) => x.mo === "2022-10")?.start).toBe("2022-10-21");
});

it("2 Trade Loss emits the range after two losing ranges and keeps counting until a gain or flat range", () => {
  const r = (mo: string, ec: number) => ({ mo, start: mo, end: mo, sc: 1, ec });
  expect([...tradeLossMonths([r("a", 0), r("b", 0), r("c", 0), r("d", 2), r("e", 1), r("f", 0)])]).toEqual(["c", "d"]);
});

const input = (nifty: Bar[], midcap: Bar[], ad: [string, number][] = []) => ({ nifty, bank: bars("2020-12-01", "2022-01-31"), midcap, ad: new Map(ad) });

it("QDP+Nov months always emit; 0.11 ADR emits on its first day inside [start, end); 5/45 is above the threshold", () => {
  const flat = bars("2020-12-01", "2022-01-31");
  const w = master5dp(input(flat, flat, [["2021-01-27", 5 / 46], ["2021-04-27", 5 / 45], ["2021-06-02", 0.05], ["2021-10-26", 0.1], ["2021-10-27", 0]]));
  expect(w.map((x) => [x.from, x.to])).toEqual([
    ["2021-01-27", "2021-02-02"], ["2021-03-24", "2021-04-02"], ["2021-06-24", "2021-07-02"], ["2021-09-24", "2021-10-04"],
    ["2021-10-26", "2021-11-02"], ["2021-11-24", "2021-12-02"], ["2021-12-24", "2022-01-04"]]);
  expect(w[0].systems).toEqual(["0.11 ADR 2021-01-27"]);
  expect(w[2].systems).toEqual(["QDP+Nov"]); // 2021-06-02 is May's range end, not inside it
});

it("a QTR date inside the range moves the entry, unless it is also a Base-1 start (kept at the range start)", () => {
  const midcap = bars("2020-12-01", "2022-01-31", falling(["2021-05", "2021-06"]));
  const moved = master5dp(input(bars("2020-12-01", "2022-01-31"), midcap));
  expect(moved.find((x) => x.to === "2021-07-02")?.from).toBe("2021-06-30"); // June's 2MR Midcap month end
  expect(moved.find((x) => x.to === "2021-08-03")?.systems).toEqual(["2 Trade Loss (NIFTYMIDCAP100)"]);
  const nifty = bars("2020-12-01", "2022-01-31", falling(["2021-04", "2021-05", "2021-06"])); // 3MR NIFTY + 2MR Midcap: a Base-1 start
  expect(master5dp(input(nifty, midcap)).find((x) => x.to === "2021-07-02")?.from).toBe("2021-06-24");
});

it("the page's sets are the AFLs' hard-punched dates: Master 5DP as typed (Mahek's updated 125 windows), QTR Midcap held to the next month's last session", () => {
  const sets = regimeSets({ midcap: bars("2008-01-01", "2026-01-31") });
  expect(sets.map((s) => s.name)).toEqual(["Master 5DP", "QTR Midcap"]);
  const m5 = sets[0].windows, qtr = sets[1].windows;
  expect(m5).toHaveLength(125);
  expect([m5[0], m5.at(-1)]).toEqual([["2008-03-31", "2008-04-02"], ["2026-09-24", "2026-10-05"]]);
  expect(m5).toContainEqual(["2012-10-23", "2012-11-02"]); // a 0.11 ADR window the April list did not have
  expect(m5).not.toContainEqual(["2024-02-28", "2024-03-02"]); // dropped in the 05-11-2026 update
  expect(m5).toEqual(MASTER_5DP_NIFTY.map(([a, b]) => [a, b]));
  expect(QTR_MIDCAP).toHaveLength(24);
  expect(qtr[0]).toEqual(["2008-03-31", "2008-04-30"]); // 1-month hold: to April's last session
  expect(qtr.find((w) => w[0] === "2023-02-28")).toEqual(["2023-02-28", "2023-04-28"]); // 28-02 and 31-03-2023 holds overlap: joined
  expect(qtr.at(-1)).toEqual(["2025-02-28", "2025-03-31"]); // the list ends here: nothing generated after it
  expect(qtr.every(([from]) => QTR_MIDCAP.includes(from))).toBe(true);
});
