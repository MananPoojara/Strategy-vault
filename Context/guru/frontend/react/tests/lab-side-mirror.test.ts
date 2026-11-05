import { expect, it } from "vitest";
import { mirrorRule } from "../src/features/lab/side-mirror";
import { withDirection } from "../src/features/lab/execution-model";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { RuleNode, Spec } from "../src/features/lab/types";

const n = (o: unknown) => o as RuleNode;
const ST = { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 } };

it("mirrors each preset shape for the other side", () => {
  // a trend state against 0: Supertrend turns up ↔ down
  expect(mirrorRule(n({ lhs: { ...ST, out: "direction" }, op: "crosses_below", rhs: { value: 0 } }))).toMatchObject({ op: "crosses_above" });
  // a price line against the close (Ritik's entry): Supertrend crosses below ↔ above the close
  expect(mirrorRule(n({ lhs: ST, op: "crosses_below", rhs: { s: "close" } }))).toMatchObject({ op: "crosses_above" });
  // a line against its signal line
  expect(mirrorRule(n({ lhs: { s: "macd" }, op: "crosses_above", rhs: { s: "macd", out: "signal" } }))).toMatchObject({ op: "crosses_below" });
  // an oscillator against its level: RSI crosses above 30 ↔ crosses below 70
  expect(mirrorRule(n({ lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 30 } }))).toMatchObject({ op: "crosses_below", rhs: { value: 70 } });
  // the close against a channel side: breakout above the upper band ↔ below the lower one
  expect(mirrorRule(n({ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "donchian", out: "upper", offset: 1 } })))
    .toMatchObject({ op: "crosses_below", rhs: { out: "lower", offset: 1 } });
  // rising ↔ falling
  expect(mirrorRule(n({ lhs: { s: "obv" }, op: "rising", bars: 3 }))).toMatchObject({ op: "falling", bars: 3 });
  // since-entry exits: RSI 10 below its highest ↔ 10 above its lowest; close 5 % under the entry ↔ 5 % over it
  expect(mirrorRule(n({ lhs: { s: "rsi" }, op: "<", rhs: { since_entry: "highest", of: { s: "rsi" }, add: -10 } })))
    .toMatchObject({ op: ">", rhs: { since_entry: "lowest", add: 10 } });
  expect(mirrorRule(n({ lhs: { s: "close" }, op: "<", rhs: { since_entry: "entry_price", mul: 0.95 } }))).toMatchObject({ op: ">", rhs: { mul: 1.05 } });
  // groups are mirrored inside
  expect(mirrorRule(n({ any: [{ lhs: { s: "obv" }, op: "rising", bars: 3 }] }))).toMatchObject({ any: [{ op: "falling" }] });
});

it("keeps rules that mean the same for both sides", () => {
  for (const r of [{ lhs: { s: "atr", p: { length: 14 } }, op: ">", rhs: { value: 50 } }, { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } },
    { lhs: { s: "close" }, op: ">", rhs: { value: 100 } }, { lhs: { s: "volume" }, op: ">", rhs: { s: "sma", p: { length: 20 }, of: "volume" } }])
    expect(mirrorRule(n(r))).toBe(r as unknown as RuleNode);
});

it("Positions Long → Short → Long gives every rule back; Short mirrors the presets, Long and short keeps the long side long", () => {
  const long = { ...defaultSpec(),
    entry: { all: [{ lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 30 } }, { lhs: { s: "close" }, op: ">", rhs: { s: "sma", p: { length: 200 } }, filter: true }] },
    exit: { any: [{ lhs: { ...ST, out: "direction" }, op: "crosses_above", rhs: { value: 0 } }, { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 10 } }] } } as unknown as Spec;
  const short = withDirection(long, "short");
  expect(short.entry.all).toMatchObject([{ op: "crosses_below", rhs: { value: 70 } }, { op: "<", filter: true }]);
  expect(short.exit.any).toMatchObject([{ op: "crosses_below", rhs: { value: 0 } }, { op: ">=", rhs: { value: 10 } }]);
  expect(withDirection(short, "long")).toEqual(long);
  expect(withDirection(long, "both").entry).toEqual(long.entry); // long stays long
  expect(withDirection(withDirection(long, "both"), "short").entry.all[0]).toMatchObject({ op: "crosses_below", rhs: { value: 70 } });
});

it("a strategy saved with a long Supertrend exit on its short side is corrected when it is opened, and does not count as edited", async () => {
  const { labReducer, initialLabState, editorState } = await import("../src/features/lab/lab-state");
  const st = { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 }, out: "direction" };
  const saved = { ...defaultSpec(), direction: "short", exit: { any: [{ lhs: st, op: "crosses_above", rhs: { value: 0 } }] } } as unknown as Spec; // Ritik's sheet
  const row = { id: 1, name: "NF_En_ST", instrument: "NIFTY 50", spec: saved } as never;
  const s0 = { ...initialLabState({ region: "in", tk: "NIFTY 50", tf: "D", range: "3Y" } as never), ticker: "NIFTY 50" };
  const opened = labReducer(s0, { type: "open", row, spec: saved });
  expect(opened.spec.exit.any[0]).toMatchObject({ op: "crosses_below" }); // covers when the trend turns up
  expect(editorState(opened, "in").unchanged).toBe(true);
  // Long and short: the short side's exit too
  const both = { ...defaultSpec(), direction: "both", short_side: { entry: { all: [] }, exit: { any: [{ lhs: st, op: "crosses_above", rhs: { value: 0 } }] } } } as unknown as Spec;
  expect(labReducer(s0, { type: "spec", spec: both }).spec.short_side!.exit.any[0]).toMatchObject({ op: "crosses_below" });
});

it("picking an indicator sets its condition from the side: long entry / short exit bullish, short entry / long exit bearish", async () => {
  const { pickedComparison } = await import("../src/features/lab/side-mirror");
  const pick = (s: string, rhs: unknown, op: string, bull: boolean) => pickedComparison({ s }, rhs, op, bull);
  // Supertrend against the close (Ritik's rule): a long buys when it crosses below the price, a short sells when it crosses above
  expect(pick("supertrend", { s: "close" }, "crosses_above", true)).toMatchObject({ op: "crosses_below" });
  expect(pick("supertrend", { s: "close" }, "crosses_above", false)).toMatchObject({ op: "crosses_above" });
  expect(pick("ema", { s: "close" }, ">", true)).toMatchObject({ op: "<" }); // a comparison stays a comparison
  // RSI: bullish at 30, bearish at 70, whatever the previous rule's level was
  expect(pick("rsi", { value: 30 }, "crosses_above", true)).toEqual({ op: "crosses_above", rhs: { value: 30 } });
  expect(pick("rsi", { value: 30 }, "crosses_above", false)).toEqual({ op: "crosses_below", rhs: { value: 70 } });
  expect(pick("rsi", { value: 30 }, ">", false)).toEqual({ op: ">", rhs: { value: 70 } });
  // MACD against its signal line
  expect(pick("macd", { s: "macd", out: "signal" }, "crosses_below", true)).toMatchObject({ op: "crosses_above" });
  expect(pick("macd", { s: "macd", out: "signal" }, "crosses_above", false)).toMatchObject({ op: "crosses_below" });
  // no usual comparison: nothing changes
  expect(pick("atr", undefined, ">", true)).toEqual({ rhs: undefined });
});

it("a band setup flips with Positions: below the lower band then a green close back in ↔ above the upper band then a red close back in", async () => {
  const { defaultBand } = await import("../src/features/lab/BandRule");
  const { catalogueFixture } = await import("./fixtures/lab");
  const bb = { ...catalogueFixture.indicators[0], id: "bb", name: "Bollinger Bands", placement: "price", computed: true, outputs: ["basis", "upper", "lower"],
    params: [{ key: "length", label: "Length", default: 20, min: 1, max: 500, step: 1, integer: true }] };
  const cat = { ...catalogueFixture, indicators: [...catalogueFixture.indicators, bb] } as never;
  const o = { s: "bb", p: { length: 20 } };
  const long = { ...defaultSpec(), entry: { all: [defaultBand(o, false)] } } as unknown as Spec;
  const short = withDirection(long, "short", undefined, cat);
  expect(short.entry.all[0]).toEqual(defaultBand(o, true)); // the bearish setup the band button builds
  expect(withDirection(short, "long", undefined, cat).entry.all[0]).toEqual(long.entry.all[0]);
});
