#!/usr/bin/env python3
"""Build the start-stamped 5-minute BTST wide CSV from rolling_futures.parquet.

See BTST_VAULT_SPEC.md section 2. Run with:
  ~/.venvs/strategy-vault/bin/python btst_series.py --input X.parquet --output Y.csv [--report R.json]
"""
import argparse
import json

import polars as pl

OPEN_MIN = 9 * 60 + 15   # 09:15
LAST_MIN = 15 * 60 + 30  # 15:30 (lone bar)
STAMPS = list(range(OPEN_MIN, LAST_MIN, 5)) + [LAST_MIN]  # 75 grid bars + 1530
HHMM = [f"{m // 60:02d}{m % 60:02d}" for m in STAMPS]
COLUMNS = ["Date"] + [f"{p}{h}" for h in HHMM for p in "OHLC"]


def build_bars(df: pl.DataFrame):
    """Return (wide frame with Date + 304 cols, info dict)."""
    df = df.sort("ts").with_columns(
        pl.col("open", "high", "low", "close").cast(pl.Float64),
        pl.col("ts").dt.date().alias("date"),
        (pl.col("ts").dt.hour().cast(pl.Int32) * 60 + pl.col("ts").dt.minute().cast(pl.Int32)).alias("mod"),
    )
    inside = (pl.col("mod") >= OPEN_MIN) & (pl.col("mod") <= LAST_MIN)
    outside = df.filter(~inside)
    excluded_dates = sorted(outside["date"].unique().to_list())
    inw = df.filter(inside)
    # stamp: 5-min bin start; minute 15:30 is its own bar
    inw = inw.with_columns(
        pl.when(pl.col("mod") == LAST_MIN).then(LAST_MIN)
        .otherwise(OPEN_MIN + ((pl.col("mod") - OPEN_MIN) // 5) * 5).alias("stamp")
    )
    bars = (
        inw.group_by(["date", "stamp"], maintain_order=True)
        .agg(
            pl.col("open").first().alias("o"),
            pl.col("high").max().alias("h"),
            pl.col("low").min().alias("l"),
            pl.col("close").last().alias("c"),
            pl.len().alias("n"),
        )
    )
    partial = bars.filter(
        ((pl.col("stamp") == LAST_MIN) & (pl.col("n") < 1)) | ((pl.col("stamp") < LAST_MIN) & (pl.col("n") < 5))
    )
    all_dates = sorted(df["date"].unique().to_list())
    kept_dates = sorted(inw["date"].unique().to_list())
    long = bars.with_columns(
        pl.col("stamp").map_elements(lambda m: f"{m // 60:02d}{m % 60:02d}", return_dtype=pl.String).alias("hhmm")
    )
    pieces = []
    for p, src in (("O", "o"), ("H", "h"), ("L", "l"), ("C", "c")):
        pieces.append(long.select("date", (pl.lit(p) + pl.col("hhmm")).alias("col"), pl.col(src).alias("v")))
    stacked = pl.concat(pieces)
    wide = stacked.pivot(on="col", index="date", values="v", aggregate_function="first")
    wide = wide.rename({"date": "Date"})
    wide = (
        pl.DataFrame({"Date": kept_dates})
        .join(wide, on="Date", how="left")
        .with_columns([pl.lit(None, dtype=pl.Float64).alias(c) for c in COLUMNS[1:] if c not in wide.columns])
        .select(COLUMNS)
        .sort("Date")
    )
    info = {
        "partial": partial,
        "excluded_dates": excluded_dates,
        "all_dates": all_dates,
        "kept_dates": kept_dates,
        "inw": inw,
    }
    return wide, info


def make_report(wide, info, df):
    missing = {}
    for h in HHMM:
        missing[h] = int(wide["C" + h].is_null().sum())
    partial = info["partial"]
    exp = (
        df.with_columns(pl.col("ts").dt.date().alias("date"))
        .filter((pl.col("ts").dt.hour().cast(pl.Int32) * 60 + pl.col("ts").dt.minute().cast(pl.Int32)).is_between(OPEN_MIN, LAST_MIN))
        .group_by("date").agg(pl.col("expiry").last().alias("expiry")).sort("date")
    )
    exp = exp.with_columns(pl.col("expiry").shift(1).alias("prev"))
    changes = exp.filter(pl.col("prev").is_not_null() & (pl.col("expiry") != pl.col("prev")))
    return {
        "days_written": wide.height,
        "first_date": str(wide["Date"].min()),
        "last_date": str(wide["Date"].max()),
        "partial_bar_count": partial.height,
        "partial_bars": [
            {"date": str(r["date"]), "hhmm": f"{r['stamp'] // 60:02d}{r['stamp'] % 60:02d}", "minutes": r["n"]}
            for r in partial.iter_rows(named=True)
        ],
        "partial_bar_dates": sorted({str(d) for d in partial["date"].to_list()}),
        "fully_missing_bars_per_hhmm": {k: v for k, v in missing.items() if v},
        "fully_missing_bar_total": sum(missing.values()),
        "excluded_dates": [str(d) for d in info["excluded_dates"]],
        "days_dropped_entirely": [str(d) for d in sorted(set(info["all_dates"]) - set(info["kept_dates"]))],
        "expiry_change_count": changes.height,
        "expiry_change_dates": [str(d) for d in changes["date"].to_list()],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", required=True)
    ap.add_argument("--output", required=True)
    ap.add_argument("--report")
    a = ap.parse_args()
    df = pl.read_parquet(a.input)
    wide, info = build_bars(df)
    wide.with_columns(pl.col("Date").dt.strftime("%Y-%m-%d")).write_csv(a.output, null_value="")
    rep = make_report(wide, info, df)
    if a.report:
        with open(a.report, "w") as f:
            json.dump(rep, f, indent=2)
    print(f"days={rep['days_written']} {rep['first_date']}..{rep['last_date']} partial={rep['partial_bar_count']} "
          f"missing_bars={rep['fully_missing_bar_total']} excluded={len(rep['excluded_dates'])} "
          f"expiry_changes={rep['expiry_change_count']}")


if __name__ == "__main__":
    main()
