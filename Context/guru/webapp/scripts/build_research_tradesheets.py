#!/usr/bin/env python3
"""Build research-tradesheets.json for the Research page's "Tradesheet verified" view.

Python 3 standard library only (the .xlsx files are read with zipfile + xml); the workbooks are opened read-only
and never written.

    python3 Context/guru/webapp/scripts/build_research_tradesheets.py [--match X.xlsx] [--status Y.xlsx]
        [--seed OLD.xlsx] [--out-dir DIR]

Inputs
  --match   "Tradesheet Match Status" workbook: sheets By strategy, Trade sheets, Notes (Made).
  --status  "Strategy Tradesheet Status" workbook: sheet Complete = strategies Jay verified himself.
  --seed    an older match workbook, added to the history once (optional, repeatable).
Outputs (in --out-dir, default $HOME/.cache/strategy-vault-dev-jay/frontend-data, mode 0644)
  research-tradesheets.json          what the page reads
  research-tradesheets-history.json  every snapshot with its per-strategy statuses (kept between runs)

Snapshot dates come from the workbook's Notes "Made" cell (the first dd-mm-yyyy in it); the file's
modification time is used when there is none. A snapshot whose statuses equal the latest one is not added
again; a new one with the same date and kind as the latest replaces it.

"Verified by Jay": a strategy is marked when a row of the Complete sheet names it. The Code column is
"<module>.rs" optionally followed by "(id, id ...)". Ids written in the brackets are marked as named; a row
with only the module marks the module's ids ("<module>_...") whose match status is Matching.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
import zipfile
from datetime import datetime, timedelta, timezone
from xml.etree import ElementTree as ET

SHARE = "/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Jay/Jay_Files/StrategyVault"
DEFAULT_MATCH = os.path.join(SHARE, "Tradesheet Match Status - 19-11-2026.xlsx")
DEFAULT_STATUS = os.path.join(SHARE, "Strategy Tradesheet Status.xlsx")
DEFAULT_OUT_DIR = os.path.join(os.path.expanduser("~"), ".cache", "strategy-vault-dev-jay", "frontend-data")
OUT_NAME, HISTORY_NAME = "research-tradesheets.json", "research-tradesheets-history.json"
STATUSES = ["Matching", "Partly matching", "Not matching", "Not compared yet", "No trade sheets"]

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
      "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
      "pr": "http://schemas.openxmlformats.org/package/2006/relationships"}
DATE_RE = re.compile(r"\b(\d{1,2})-(\d{1,2})-(\d{4})\b")
ID_RE = re.compile(r"\byt[a-z0-9]+_[a-z0-9_]+\b")
MODULE_RE = re.compile(r"\b(yt[a-z0-9_]*?)\.rs\b")


# ------------------------------------------------------------------ xlsx (stdlib)

def _col(ref: str) -> int:
    n = 0
    for ch in ref:
        if not ch.isalpha():
            break
        n = n * 26 + ord(ch.upper()) - 64
    return n - 1


def read_xlsx(path: str) -> dict[str, list[list]]:
    """All sheets as lists of rows (cell values: str, int/float, bool or None). Formulas give their cached value."""
    with open(path, "rb") as f, zipfile.ZipFile(f) as z:
        names = set(z.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in names:
            for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS):
                shared.append("".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t")))
        rels = {r.get("Id"): r.get("Target") for r in ET.fromstring(z.read("xl/_rels/workbook.xml.rels")).findall("pr:Relationship", NS)}
        out = {}
        for sh in ET.fromstring(z.read("xl/workbook.xml")).find("m:sheets", NS).findall("m:sheet", NS):
            target = rels[sh.get(f"{{{NS['r']}}}id")].lstrip("/")
            target = target if target.startswith("xl/") else "xl/" + target
            rows: list[list] = []
            for row in ET.fromstring(z.read(target)).iter(f"{{{NS['m']}}}row"):
                idx = int(row.get("r", len(rows) + 1)) - 1
                while len(rows) < idx:
                    rows.append([])
                cells: list = []
                for c in row.findall("m:c", NS):
                    i = _col(c.get("r", "")) if c.get("r") else len(cells)
                    t, v = c.get("t"), c.find("m:v", NS)
                    if t == "inlineStr":
                        val = "".join(x.text or "" for x in c.iter(f"{{{NS['m']}}}t"))
                    elif v is None or v.text is None:
                        val = None
                    elif t == "s":
                        val = shared[int(v.text)]
                    elif t == "b":
                        val = v.text == "1"
                    elif t in ("str", "e"):
                        val = v.text
                    else:
                        x = float(v.text)
                        val = int(x) if x.is_integer() else x
                    while len(cells) <= i:
                        cells.append(None)
                    cells[i] = val
                rows.append(cells)
            out[sh.get("name")] = rows
        return out


def cell(row: list, i: int):
    return row[i] if i < len(row) else None


def text(v) -> str:
    return "" if v is None else str(v).strip()


# ------------------------------------------------------------------ workbook content

def notes(sheets: dict) -> dict[str, str]:
    return {text(cell(r, 0)): text(cell(r, 1)) for r in sheets.get("Notes", []) if text(cell(r, 0))}


def made_date(made: str, path: str) -> tuple[str, str]:
    """(YYYY-MM-DD, how it was found) from the Notes "Made" text, else the file's modification time."""
    m = DATE_RE.search(made or "")
    if m:
        d, mo, y = map(int, m.groups())
        return f"{y:04d}-{mo:02d}-{d:02d}", "Notes: Made"
    return datetime.fromtimestamp(os.path.getmtime(path)).date().isoformat(), "file modified time"


def header_index(rows: list[list], *names: str) -> tuple[int, dict[str, int]]:
    for n, r in enumerate(rows[:10]):
        cols = {text(v): i for i, v in enumerate(r) if text(v)}
        if all(x in cols for x in names):
            return n, cols
    raise ValueError(f"header {names} not found")


def by_strategy(sheets: dict) -> list[dict]:
    rows = sheets["By strategy"]
    h, cols = header_index(rows, "Strategy id", "Trade-sheet status")
    out = []
    for r in rows[h + 1:]:
        sid, status = text(cell(r, cols["Strategy id"])), text(cell(r, cols["Trade-sheet status"]))
        if not sid or not status:
            continue
        out.append({"category": text(cell(r, cols.get("Category", -1))) if "Category" in cols else "",
                    "id": sid, "name": text(cell(r, cols["Strategy"])) if "Strategy" in cols else sid,
                    "status": status, "detail": text(cell(r, cols["Detail"])) if "Detail" in cols else ""})
    return out


def sheet_counts(sheets: dict) -> dict[str, dict[str, int]]:
    """Per strategy id: team trade sheets compared (class Matching or Not matching) and matched."""
    rows = sheets.get("Trade sheets") or []
    if not rows:
        return {}
    h, cols = header_index(rows, "Strategy id", "Class")
    out: dict[str, dict[str, int]] = {}
    for r in rows[h + 1:]:
        sid, cls = text(cell(r, cols["Strategy id"])), text(cell(r, cols["Class"]))
        if not sid or cls not in ("Matching", "Not matching"):
            continue
        c = out.setdefault(sid, {"sheets": 0, "matched": 0})
        c["sheets"] += 1
        c["matched"] += cls == "Matching"
    return out


def verified_by_jay(status_sheets: dict, strategies: list[dict]) -> dict[str, str]:
    """id -> the Complete-sheet strategy name that marks it (see the module docstring for the rule)."""
    rows = status_sheets.get("Complete") or []
    if not rows:
        return {}
    h, cols = header_index(rows, "Strategy")
    code_col = next((i for k, i in cols.items() if k.startswith("Code")), None)
    by_id = {s["id"]: s for s in strategies}
    out: dict[str, str] = {}
    for r in rows[h + 1:]:
        label, code = text(cell(r, cols["Strategy"])), text(cell(r, code_col)) if code_col is not None else ""
        if not label or not code:
            continue
        named = [i for i in ID_RE.findall(code) if i in by_id]
        if named:
            for i in named:
                out[i] = label
            continue
        for module in MODULE_RE.findall(code):
            for s in strategies:
                if s["id"].startswith(module + "_") and s["status"] == "Matching":
                    out[s["id"]] = label
    return out


def tally(strategies: list[dict]) -> dict[str, int]:
    counts = {k: 0 for k in STATUSES}
    for s in strategies:
        counts[s["status"]] = counts.get(s["status"], 0) + 1
    return counts


def digest(statuses: dict[str, str]) -> str:
    return hashlib.sha256(json.dumps(statuses, sort_keys=True).encode()).hexdigest()[:16]


def snapshot(path: str, kind: str, label: str) -> dict:
    sheets = read_xlsx(path)
    made = notes(sheets).get("Made", "")
    date, how = made_date(made, path)
    strategies = by_strategy(sheets)
    statuses = {s["id"]: s["status"] for s in strategies}
    return {"date": date, "date_from": how, "made": made, "kind": kind, "label": label, "source": os.path.basename(path),
            "total": len(strategies), "counts": tally(strategies), "digest": digest(statuses), "statuses": statuses}


def merge_history(history: list[dict], snap: dict) -> list[dict]:
    """Seeds are added once (by content); a run equal to the latest snapshot is skipped, and a run with the same
    date as the latest run replaces it."""
    hist = [h for h in history if isinstance(h, dict) and "statuses" in h]
    if snap["kind"] == "seed":
        if not any(h["digest"] == snap["digest"] for h in hist):
            hist.append(snap)
    else:
        if hist and hist[-1]["digest"] == snap["digest"]:
            return sorted(hist, key=_order)
        if hist and hist[-1]["kind"] == "run" and hist[-1]["date"] == snap["date"]:
            hist[-1] = snap
        else:
            hist.append(snap)
    return sorted(hist, key=_order)


def _order(h: dict):
    return (h["date"], 0 if h["kind"] == "seed" else 1)


def changes(prev: dict | None, cur: dict, names: dict[str, str]) -> list[dict]:
    if prev is None:
        return []
    out = []
    for sid, st in cur["statuses"].items():
        before = prev["statuses"].get(sid)
        if before != st:
            out.append({"id": sid, "name": names.get(sid, sid), "from": before, "to": st})
    for sid, st in prev["statuses"].items():
        if sid not in cur["statuses"]:
            out.append({"id": sid, "name": names.get(sid, sid), "from": st, "to": None})
    return out


def build(match: str, status: str | None, seeds: list[str], history: list[dict]) -> tuple[dict, list[dict]]:
    sheets = read_xlsx(match)
    note = notes(sheets)
    date, how = made_date(note.get("Made", ""), match)
    strategies = by_strategy(sheets)
    counts = sheet_counts(sheets)
    status_sheets = read_xlsx(status) if status and os.path.exists(status) else {}
    jay = verified_by_jay(status_sheets, strategies)
    for s in strategies:
        s.update(counts.get(s["id"], {"sheets": 0, "matched": 0}))
        s["verified_by_jay"] = s["id"] in jay
        s["verified_as"] = jay.get(s["id"], "")

    for path in seeds:
        if os.path.exists(path):
            history = merge_history(history, snapshot(path, "seed", "Before the last 8 AFLs were coded (backup workbook)"))
    run = snapshot(match, "run", "Current match workbook")
    # a run is dated by the day it is taken (India time), so each day gets its own row
    run["date"], run["date_from"] = datetime.now(timezone.utc).astimezone(timezone(timedelta(hours=5, minutes=30))).strftime("%Y-%m-%d"), "run date"
    history = merge_history(history, run)

    names = {s["id"]: s["name"] for s in strategies}
    cats: dict[str, dict] = {}
    for s in strategies:
        c = cats.setdefault(s["category"], {"category": s["category"], "counts": {k: 0 for k in STATUSES}, "total": 0, "verified_by_jay": 0})
        c["counts"][s["status"]] = c["counts"].get(s["status"], 0) + 1
        c["total"] += 1
        c["verified_by_jay"] += s["verified_by_jay"]
    status_notes = notes(status_sheets)
    out = {
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source": {"file": os.path.basename(match), "made": note.get("Made", ""), "date": date, "date_from": how,
                   "strategies_note": note.get("Strategies", ""), "caveat": note.get("Caveat", ""),
                   "status_file": os.path.basename(status) if status_sheets else "", "status_made": status_notes.get("Made", "")},
        "statuses": STATUSES,
        "totals": {**tally(strategies), "total": len(strategies), "verified_by_jay": sum(s["verified_by_jay"] for s in strategies)},
        "verified_rule": "Marked from the Complete sheet of Strategy Tradesheet Status.xlsx: ids written in its Code column, "
                         "or, for a row naming only a module (<module>.rs), that module's ids whose status is Matching.",
        "categories": sorted(cats.values(), key=lambda c: c["category"].lower()),
        "strategies": strategies,
        # One row per day (Jay, 20-11-2026): the day's last snapshot, its changes counted against the previous day's row.
        "history": [{k: v for k, v in h.items() if k not in ("statuses", "digest")} | {"changes": changes(daily[i - 1] if i else None, h, names)}
                    for daily in [list({h["date"]: h for h in history}.values())] for i, h in enumerate(daily)],
    }
    return out, history


def write_json(data, path: str) -> None:
    d = os.path.dirname(os.path.abspath(path))
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=d, prefix=".research-tradesheets.")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
        f.write("\n")
    os.chmod(tmp, 0o644)
    os.replace(tmp, path)


def load_history(path: str) -> list[dict]:
    try:
        with open(path, encoding="utf-8") as f:
            d = json.load(f)
        return d.get("snapshots", []) if isinstance(d, dict) else []
    except (OSError, ValueError):
        return []


def main(argv=None) -> int:
    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.abspath(os.path.join(here, "..", "..", "..", ".."))
    default_seed = os.path.join(repo, "jay-local", "backups", "automation-report", "20-11-2026-new-afls", "Tradesheet Match Status - 19-11-2026.xlsx")
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--match", default=DEFAULT_MATCH)
    ap.add_argument("--status", default=DEFAULT_STATUS)
    ap.add_argument("--seed", action="append", default=None, help=f"older match workbook(s) for the history (default {default_seed})")
    ap.add_argument("--out-dir", default=DEFAULT_OUT_DIR)
    a = ap.parse_args(argv)
    hist_path = os.path.join(a.out_dir, HISTORY_NAME)
    data, history = build(a.match, a.status, a.seed if a.seed is not None else [default_seed], load_history(hist_path))
    write_json({"version": 1, "snapshots": history}, hist_path)
    write_json(data, os.path.join(a.out_dir, OUT_NAME))
    t = data["totals"]
    print(f"{os.path.join(a.out_dir, OUT_NAME)}: {t['total']} strategies; " + ", ".join(f"{k} {t[k]}" for k in STATUSES)
          + f"; verified by Jay {t['verified_by_jay']}; {len(history)} history snapshots")
    return 0


if __name__ == "__main__":
    sys.exit(main())
