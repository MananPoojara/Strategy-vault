#!/usr/bin/env python3
"""Build research-work.json for the jay Research page ("Claude's work" timeline).

Reads the git history of a branch (default `jay`) with read-only git commands (log, diff, grep,
status) and writes one JSON file. Nothing in the repository, the index or any database is changed.

    python3 Context/guru/webapp/scripts/build_research_work.py [OUT] [--branch jay] [--repo PATH]

OUT defaults to $HOME/.cache/strategy-vault-dev-jay/frontend-data/research-work.json, which the site
serves as `data/research-work.json`. The output is generated data and stays outside Git.

YT strategy ids
  An id is a string constant declared at the start of a line in Context/guru/backend/src/books/yt*.rs:
      pub const NAME: &str = "yt<code>_<variant>";
  Collection (book) ids such as BOOK = "yt_adr" start with "yt_" and are not counted.
  * count  = the number of distinct such ids in the tree at that commit (git grep on the commit).
  * added  = ids on added diff lines of the commit (against its first parent) that were not in the
             parent's tree. A constant moved between files is therefore not counted as new.
  * removed = ids in the parent's tree that are gone at this commit.
  Limits: the engine catalogue is built from these constants, but a constant that is declared and never
  registered would still be counted, and an id written only as a string literal inside an array (no
  `pub const`) would be missed. Ids are counted by their string value, so the same id in two files counts once.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone

YT_PATHSPEC = "Context/guru/backend/src/books/yt*.rs"
# Same pattern for git grep (POSIX ERE) and for Python.
ID_ERE = r'^pub const [A-Z0-9_]+: &str = "yt[a-z0-9]+_[a-z0-9_]*"'
ID_RE = re.compile(r'^pub const [A-Z0-9_]+: &str = "(yt[a-z0-9]+_[a-z0-9_]*)"')
EMPTY_TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
IST = timezone(timedelta(hours=5, minutes=30))
DEFAULT_OUT = os.path.join(os.path.expanduser("~"), ".cache", "strategy-vault-dev-jay", "frontend-data", "research-work.json")
SEP_REC, SEP_FLD = "\x1e", "\x1f"


def ids_in_lines(lines) -> set[str]:
    """YT strategy ids declared on the given source lines."""
    out = set()
    for line in lines:
        m = ID_RE.match(line)
        if m:
            out.add(m.group(1))
    return out


def added_ids_in_diff(diff: str) -> set[str]:
    """Ids declared on added lines ('+...') of a unified diff, ignoring the '+++ b/file' headers."""
    return ids_in_lines(l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++"))


TRAILER_RE = re.compile(r"^\s*Co-Authored-By:\s*([^<]+?)\s*<[^>]*>\s*$", re.I | re.M)


def split_trailers(body: str) -> tuple[str, list[str]]:
    """Commit body without its Co-Authored-By lines, and the co-author names (e.g. "Claude Opus 5.5")."""
    names = TRAILER_RE.findall(body)
    return TRAILER_RE.sub("", body).strip(), names


def ist_day(iso: str) -> str:
    return datetime.fromisoformat(iso).astimezone(IST).date().isoformat()


class Git:
    def __init__(self, repo: str):
        self.repo = repo

    def __call__(self, *args: str, ok=(0,)) -> str:
        # --no-optional-locks: `status` must not refresh (write) the index.
        r = subprocess.run(["git", "--no-optional-locks", "-C", self.repo, *args], capture_output=True, text=True)
        if r.returncode not in ok:
            raise RuntimeError(f"git {' '.join(args)} failed: {r.stderr.strip()}")
        return r.stdout

    def tree_ids(self, rev: str) -> set[str]:
        if rev == EMPTY_TREE:
            return set()
        out = self("grep", "-h", "-o", "-E", ID_ERE, rev, "--", YT_PATHSPEC, ok=(0, 1))
        return ids_in_lines(out.splitlines())


def commits(git: Git, branch: str) -> list[dict]:
    fmt = SEP_FLD.join(["%H", "%P", "%aI", "%an", "%s", "%b"]) + SEP_REC
    raw = git("log", branch, "--topo-order", f"--format={fmt}")
    out = []
    for rec in raw.split(SEP_REC):
        rec = rec.strip("\n")
        if not rec:
            continue
        sha, parents, at, who, subject, body = (rec.split(SEP_FLD) + [""] * 6)[:6]
        out.append({"sha": sha, "parents": parents.split(), "at": at, "who": who, "label": subject.strip(), "work": body.strip()})
    return out


def numstat(git: Git, parent: str, sha: str) -> tuple[int, int, int]:
    files = ins = dels = 0
    for line in git("diff", "--numstat", "--no-renames", parent, sha).splitlines():
        a, d, *_ = line.split("\t")
        files += 1
        ins += int(a) if a.isdigit() else 0
        dels += int(d) if d.isdigit() else 0
    return files, ins, dels


def build(repo: str, branch: str) -> dict:
    git = Git(repo)
    head = git("rev-parse", branch).strip()
    log = commits(git, branch)
    tree_cache: dict[str, set[str]] = {}

    def tree(rev: str) -> set[str]:
        if rev not in tree_cache:
            tree_cache[rev] = git.tree_ids(rev)
        return tree_cache[rev]

    events = []
    for c in log:
        parent = c["parents"][0] if c["parents"] else EMPTY_TREE
        now, before = tree(c["sha"]), tree(parent)
        diff = git("diff", "-U0", "--no-renames", parent, c["sha"], "--", YT_PATHSPEC)
        added = sorted((added_ids_in_diff(diff) & now) - before)
        removed = sorted(before - now)
        files, ins, dels = numstat(git, parent, c["sha"])
        work, coauthors = split_trailers(c["work"])
        events.append({
            "key": c["sha"], "short": c["sha"][:8], "at": c["at"], "label": c["label"], "who": c["who"], "work": work, "with": coauthors,
            "files": files, "insertions": ins, "deletions": dels, "merge": len(c["parents"]) > 1,
            "added": added, "removed": removed, "count": len(now),
        })
    events.sort(key=lambda e: datetime.fromisoformat(e["at"]), reverse=True)

    days: dict[str, dict] = {}
    for e in events:  # newest first, so the first event seen on a day carries the day's closing count
        d = days.setdefault(ist_day(e["at"]), {"date": ist_day(e["at"]), "commits": 0, "added": 0, "removed": 0, "count": e["count"], "authors": []})
        d["commits"] += 1
        d["added"] += len(e["added"])
        d["removed"] += len(e["removed"])
        if e["who"] not in d["authors"]:
            d["authors"].append(e["who"])

    status = git("status", "--porcelain")
    dirty = [l[3:] for l in status.splitlines() if l.strip()]
    return {
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "branch": branch, "head": head,
        "uncommitted": {"files": len(dirty), "note": "The working tree had uncommitted changes when this file was built; they are not in the timeline." if dirty else ""},
        "method": "count = distinct `pub const X: &str = \"yt<code>_...\"` ids in books/yt*.rs at the commit (git grep); added = such ids on added diff lines that were not in the first parent.",
        "totals": {"commits": len(events), "days": len(days), "strategies": len(tree(head)), "added": sum(len(e["added"]) for e in events),
                   "authors": sorted({e["who"] for e in events})},
        "days": sorted(days.values(), key=lambda d: d["date"], reverse=True),
        "events": events,
    }


def write(data: dict, out: str) -> None:
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=os.path.dirname(os.path.abspath(out)), prefix=".research-work.")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
        f.write("\n")
    os.chmod(tmp, 0o644)  # the site reads the mounted folder as another user
    os.replace(tmp, out)


def main(argv=None) -> int:
    here = os.path.dirname(os.path.abspath(__file__))
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("out", nargs="?", default=DEFAULT_OUT, help=f"output JSON (default {DEFAULT_OUT})")
    ap.add_argument("--branch", default="jay")
    ap.add_argument("--repo", default=os.path.abspath(os.path.join(here, "..", "..", "..", "..")))
    a = ap.parse_args(argv)
    data = build(a.repo, a.branch)
    write(data, a.out)
    t = data["totals"]
    print(f"{a.out}: {t['commits']} commits over {t['days']} days, {t['strategies']} YT strategy ids at {data['head'][:8]}, "
          f"{t['added']} added in the history; uncommitted files: {data['uncommitted']['files']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
