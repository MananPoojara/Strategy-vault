import { describe, expect, it } from "vitest";
import { groupSettings, idleKeys, isSwitch, settingGroup, settingName, settingOptions, settingValueText, type YtSetting } from "../src/services/yt-settings";

const s = (key: string, label: string, def: number, min: number, max: number, step = 1): YtSetting => ({ key, label, def, min, max, step });

// The Monday Reversal settings as the engine lists them.
const MONDAY = [
  s("entry", "Entry condition (1 Fri O>Mon H, 2 Fri L>Mon L, 3 Fri L>Mon C, 4 Fri C>Mon C, 5 Fri H>Mon H)", 1, 1, 5),
  s("entry2", "Entry condition 2, both must hold (0 off, 1 Fri O>Mon H, 2 Fri L>Mon L, 3 Fri L>Mon C, 4 Fri C>Mon C, 5 Fri H>Mon H)", 0, 0, 5),
  s("base2", "BASE 2 filter (0 off, 1 Bull, 2 Bear)", 0, 0, 2),
  s("nc_on", "Candle exit (0 = off, 1 = on)", 1, 0, 1),
  s("candle_type", "Type of exit candle (1 N Candle, 2 Green Candle, 3 UpClose)", 3, 1, 3),
  s("upclose_n", "N candles of that type for exit (0 off)", 1, 0, 10),
  s("atr_on", "Dual ATR exit (0 = off, 1 = on)", 0, 0, 1),
  s("atr_n", "Initial ATR period", 5, 5, 15, 5),
  s("atr_mult", "Initial ATR multiplier", 2, 1, 6),
  s("fav_n", "Favour ATR period", 5, 5, 15, 5),
  s("fav_mult", "Favour ATR multiplier", 2, 1, 8),
  s("hsl_on", "Hard SL (0 = off, 1 = on)", 0, 0, 1),
  s("hsl_pct", "Hard SL %", 5, 0.5, 20, 0.5),
  s("hsl_days", "Hard SL active for N bars", 1, 1, 4),
  s("sl_buffer", "Gap fill buffer % (open through the SL)", 0.25, 0, 10, 0.05),
  s("wd_on", "Weekday exit (0 = off, 1 = on)", 0, 0, 1),
  s("wd_day", "Weekday exit day (1 Wednesday, 2 Friday)", 1, 1, 2),
];

const K = (k: string) => MONDAY.find(x => x.key === k)!;

describe("YT settings panel", () => {
  it("reads choices from a numbered bracket and drops it from the name", () => {
    const t = K("candle_type");
    expect(settingOptions(t)).toEqual([{ value: 1, text: "N Candle" }, { value: 2, text: "Green Candle" }, { value: 3, text: "UpClose" }]);
    expect(settingName(t)).toBe("Type of exit candle");
    expect(settingValueText(t, 3)).toBe("UpClose");
    expect(settingOptions(K("atr_on")).map(o => o.text)).toEqual(["Off", "On"]);
    expect(settingName(K("atr_on"))).toBe("Dual ATR exit");
    expect(isSwitch(K("atr_on"))).toBe(true);
    expect(settingOptions(K("base2")).map(o => o.text)).toEqual(["Off", "Bull", "Bear"]);
    expect(isSwitch(K("base2"))).toBe(false);
    expect(settingOptions(K("entry2"))[0]).toEqual({ value: 0, text: "Off" });
  });

  it("keeps number settings and partial brackets as numbers", () => {
    expect(settingOptions(K("upclose_n"))).toEqual([]); // "(0 off)" names one value only
    expect(settingName(K("upclose_n"))).toBe("N candles of that type for exit (0 off)");
    expect(settingOptions(s("d", "Entry weekday (1 Mon .. 5 Fri)", 1, 1, 5))).toEqual([]);
    expect(settingOptions(s("x", "Mode (0 a, 1 b)", 0, 0, 2))).toEqual([]); // value 2 has no name
    expect(settingValueText(K("hsl_pct"), 5)).toBe("5");
  });

  it("keeps nested brackets and notes inside a choice", () => {
    const st = s("st_point", "Daily ST calculation point (0 mid point, 1 close, 2 pivot (H+L+C)/3, 3 average of mid point, pivot and close)", 0, 0, 3);
    expect(settingOptions(st).map(o => o.text)).toEqual(["Mid point", "Close", "Pivot (H+L+C)/3", "Average of mid point, pivot and close"]);
    const dead = s("dead_bt", "Dead stock backtest (0 YES: buys allowed from the first bar, 1 NO, the AFL default: no buy on the first bar)", 1, 0, 1);
    expect(settingOptions(dead).map(o => o.text)).toEqual(["YES", "NO, the AFL default"]);
  });

  it("shows the Kora Reddy weekday settings as day names", () => {
    const d = s("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 5, 1, 5);
    expect(settingOptions(d).map(o => o.text)).toEqual(["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]);
    expect(settingName(d)).toBe("Entry weekday");
    expect(settingValueText(d, 5)).toBe("Friday");
    expect(settingGroup(s("exit_day", "Exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 1, 1, 5))).toBe("Entry");
  });

  it("shows the other coded YT settings as named choices", () => {
    const src = s("st_src", "Exit ST average value (1 close, 2 open, 3 high, 4 low, 5 (O+C)/2, 6 (H+L)/2, 7 (H+L+C)/3, 8 (O+H+L+C)/4)", 6, 1, 8);
    expect(settingValueText(src, 6)).toBe("(H+L)/2");
    const rsi = s("rsi_dir", "RSI exit when it crosses the threshold (0 below, 1 above)", 0, 0, 1);
    expect(settingOptions(rsi).map(o => o.text)).toEqual(["Below", "Above"]);
    expect(isSwitch(rsi)).toBe(false);
    const ver = s("version", "Version (1 Version 1, 2 Version 2, the same AFL code as version 1, 3 Version 3, adds close below yesterday's low)", 1, 1, 3);
    expect(settingOptions(ver).map(o => o.text)).toEqual(["Version 1", "Version 2, the same AFL code as version 1", "Version 3, adds close below yesterday's low"]);
    expect(settingName(ver)).toBe("Version");
  });

  it("reads the YT Holidays event list", () => {
    const e = s("event", "Event::Budget Day|Holi|Diwali", 2, 1, 3);
    expect(settingName(e)).toBe("Event");
    expect(settingValueText(e, 3)).toBe("Diwali");
  });

  it("groups the Monday settings like the AFL's sections", () => {
    expect(groupSettings(MONDAY).map(g => [g.name, g.settings.map(x => x.key)])).toEqual([
      ["Entry", ["entry", "entry2"]],
      ["Filter", ["base2"]],
      ["Candle exit", ["nc_on", "candle_type", "upclose_n"]],
      ["Dual ATR exit", ["atr_on", "atr_n", "atr_mult", "fav_n", "fav_mult"]],
      ["Exit", ["wd_on", "wd_day"]],
      ["Stop loss", ["hsl_on", "hsl_pct", "hsl_days", "sl_buffer"]],
    ]);
    expect(settingGroup(s("p", "Pin bar range %", 1, 0, 9))).toBe("Strategy settings");
    expect(settingGroup(s("d", "Entry weekday (1 Mon .. 5 Fri)", 1, 1, 5))).toBe("Entry");
    expect(settingOptions(K("wd_day")).map(o => o.text)).toEqual(["Wednesday", "Friday"]);
    expect(settingGroup(s("lookback_ll", "Last N days for entry", 5, 1, 9))).toBe("Entry");
  });

  it("greys a section's settings while its Off/On switch is off", () => {
    const groups = groupSettings(MONDAY), at = (v: Record<string, number>) => (k: string) => v[k] ?? MONDAY.find(x => x.key === k)!.def;
    expect([...idleKeys(groups, at({}))].sort()).toEqual(["atr_mult", "atr_n", "fav_mult", "fav_n", "hsl_days", "hsl_pct", "sl_buffer", "wd_day"]);
    expect(idleKeys(groups, at({ atr_on: 1, hsl_on: 1, wd_on: 1 })).size).toBe(0);
    expect([...idleKeys(groups, at({ nc_on: 0, atr_on: 1, hsl_on: 1, wd_on: 1 }))].sort()).toEqual(["candle_type", "upclose_n"]);
  });
});
