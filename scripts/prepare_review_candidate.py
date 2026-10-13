#!/usr/bin/env python3
"""Host-only Git handoff for sandboxed conversion and independent review jobs.

Prepare reconstructs scoped edits on the current clean Manan base, preserving
the complete old tree privately. Finalize validates independent evidence before
creating the candidate's single commit. Neither operation publishes or deploys.
"""
import argparse
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tempfile

import publish_reviewed_batch as publisher


def require(condition, message):
    publisher.require(condition, message)


def run_git(repo, *args, env=None, check=True):
    process = subprocess.run(["git", "-C", str(repo), *args], env=env, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
    require(not check or process.returncode == 0, "Host Git operation failed; candidate work and private backups are preserved")
    return process


def text_git(repo, *args):
    return run_git(repo, *args).stdout.decode().strip()


def private_tree(root):
    """Keep executability while removing access by other local users."""
    for directory, dirs, files in os.walk(root, followlinks=False):
        os.chmod(directory, 0o700)
        for name in files:
            path = Path(directory) / name
            if not path.is_symlink():
                os.chmod(path, 0o600 | (path.stat().st_mode & 0o100))


def snapshot_size(root):
    total = 0
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in files:
            path = Path(directory) / name
            if not path.is_symlink():
                total += path.stat().st_size
                require(total <= 512 * 1024 * 1024, "Candidate backup exceeds the 512 MiB source-tree budget")
    return total


def tree_fingerprint(root):
    """Detect concurrent source, ignored-work or Git metadata edits during preparation."""
    records = []
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in sorted(dirs + files):
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                records.append((relative, "link", os.readlink(path)))
            elif path.is_file():
                records.append((relative, path.stat().st_mode & 0o100, publisher.sha256(path)))
    return hashlib.sha256(json.dumps(sorted(records), separators=(",", ":")).encode()).hexdigest()


def contained_copy_path(root, name):
    relative = PurePosixPath(name)
    require(not relative.is_absolute() and ".." not in relative.parts, "Private copy path escapes its clone")
    root = Path(root).resolve()
    path = root
    for part in relative.parts[:-1]:
        path /= part
        require(not path.is_symlink(), "Ignored work conflicts with a symlinked parent; original work is preserved")
    path = root / relative
    require(path.parent.resolve().is_relative_to(root), "Ignored work copy escapes its private clone")
    return path


def working_changes(candidate, base):
    records = run_git(candidate, "diff", "--name-status", "--no-renames", "-z", base, "--").stdout.split(b"\0")
    changes = {}
    for index in range(0, len(records) - 1, 2):
        status, name = records[index].decode(), records[index + 1].decode()
        require(status in {"A", "M"}, "Deleted or conflicted source requires an explicit manual handoff")
        changes[name] = status
    untracked = run_git(candidate, "ls-files", "--others", "--exclude-standard", "-z").stdout.split(b"\0")
    for name in untracked:
        if name:
            changes[name.decode()] = "A"
    for name in changes:
        require(publisher.allowed_file(name), "Candidate has an unapproved file or path")
        path = publisher.candidate_source(candidate, name)
        require(path.stat().st_size <= 10 * 1024 * 1024, "Candidate source file exceeds the public source budget")
    require(len(changes) <= 200, "Candidate has too many changed source files for one reviewed handoff")
    return changes


def root_base(repo):
    require(text_git(repo, "branch", "--show-current") == "Manan", "Authoritative checkout must be on Manan")
    require(not text_git(repo, "status", "--porcelain"), "Authoritative Manan checkout must be clean")
    return text_git(repo, "rev-parse", "HEAD")


def candidate_preconditions(repo, candidate, result):
    repo, candidate, result = Path(repo).resolve(), Path(candidate).resolve(), Path(result).resolve()
    require(repo != candidate and candidate.is_dir() and (candidate / ".git").is_dir() and not (candidate / ".git").is_symlink(), "Candidate must be a separate local Git clone with its own Git metadata")
    candidate_common = Path(text_git(candidate, "rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()
    repo_common = Path(text_git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()
    require(candidate_common != repo_common and candidate_common.is_relative_to(candidate), "Candidate Git metadata must not share the authoritative Git directory")
    require(all("/gvfs/" not in str(path) for path in (repo, candidate, result)), "Git handoff must never write original shares")
    require(not result.is_relative_to(repo) and not result.is_relative_to(candidate), "Handoff artifacts must remain outside Git checkouts")
    require(text_git(candidate, "branch", "--show-current") == "Manan", "Isolated candidate must use its own Manan branch")
    result.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(result.parent, 0o700)
    return repo, candidate, result


def install_hooks(candidate):
    """Only execute the unchanged hook installer from the current trusted base."""
    installer = publisher.candidate_source(candidate, "scripts/install-hooks.sh")
    trusted = run_git(candidate, "show", "HEAD:scripts/install-hooks.sh").stdout
    require(installer.read_bytes() == trusted, "Candidate hook installer differs from the trusted base")
    process = subprocess.run(["sh", str(installer)], cwd=candidate, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    require(process.returncode == 0, "Host candidate hook installation failed")


def atomic_exchange(first, second):
    """One Linux rename exchange keeps every queued candidate cwd present."""
    library = ctypes.CDLL(None, use_errno=True)
    rename = getattr(library, "renameat2", None)
    require(rename is not None, "Atomic directory exchange is unavailable; original candidate is unchanged")
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    result = rename(-100, os.fsencode(first), -100, os.fsencode(second), 2)
    require(result == 0, "Atomic directory exchange failed; original candidate and backup are preserved")


def initial_report():
    return {"schema_version": 1, "status": "held", "source_reviewed": False,
            "implementation_ready": False, "ledger_checked": False, "checked_trades": 0,
            "admitted": False, "evidence": []}


def prepare(repo, candidate, result):
    report = initial_report()
    result = Path(result).resolve()
    try:
        repo, candidate, result = candidate_preconditions(repo, candidate, result)
        base = root_base(repo)
        old_head = text_git(candidate, "rev-parse", "HEAD")
        backup = Path(tempfile.mkdtemp(prefix=result.stem + "-backup-", dir=result.parent))
        report.update(backup_dir=str(backup), original_backup=str(backup / "original"),
                      original_head=old_head, base_commit=base)
        snapshot_size(candidate)
        original_fingerprint = tree_fingerprint(candidate)
        shutil.copytree(candidate, backup / "original", symlinks=True)
        private_tree(backup / "original")
        snapshot = backup / "original"
        require(tree_fingerprint(snapshot) == original_fingerprint, "Candidate changed while its private backup was copied; preserve and retry")
        stage = backup / "rebased-candidate"
        run_git(result.parent, "clone", "--quiet", "--no-hardlinks", "--branch", "Manan", str(repo), str(stage))
        run_git(stage, "fetch", "--quiet", "--no-tags", str(snapshot), old_head)
        ancestor = text_git(stage, "merge-base", base, old_head)
        changes = working_changes(snapshot, ancestor)
        report["changed_files"] = sorted(changes)
        index = backup / "private-index"
        env = {**os.environ, "GIT_INDEX_FILE": str(index)}
        run_git(snapshot, "read-tree", ancestor, env=env)
        if changes:
            run_git(snapshot, "add", "-A", "--", *sorted(changes), env=env)
        patch = backup / "candidate.patch"
        patch.write_bytes(run_git(snapshot, "diff", "--cached", "--binary", "--full-index", "--no-renames", ancestor, "--", env=env).stdout)
        os.chmod(patch, 0o600)
        inventory = backup / "snapshot.json"
        publisher.write_result(inventory, {"original_head": old_head, "base_commit": base, "common_ancestor": ancestor,
            "changed_files": sorted(changes), "file_hashes": {name: publisher.sha256(snapshot / name) for name in changes},
            "patch_sha256": publisher.sha256(patch)})
        report["evidence"] = [str(inventory), str(patch)]
        if changes:
            applied = run_git(stage, "apply", "--3way", "--index", str(patch), check=False)
            log = backup / "patch-application.log"
            log.write_bytes(applied.stdout + applied.stderr)
            os.chmod(log, 0o600)
            report["evidence"].append(str(log))
            if applied.returncode:
                report["conflict_candidate"] = str(stage)
                raise publisher.Held("Rebase conflicts are held in the private staging clone; original candidate is unchanged")
        # Retain ignored operational work too. Do not overwrite a newly tracked base file.
        ignored = run_git(snapshot, "ls-files", "--others", "--ignored", "--exclude-standard", "-z").stdout.split(b"\0")
        for encoded in ignored:
            if not encoded:
                continue
            name = encoded.decode()
            source = contained_copy_path(snapshot, name)
            destination = contained_copy_path(stage, name)
            require(not destination.exists() and not destination.is_symlink(), "Ignored candidate work conflicts with a current base file; preserve both copies for review")
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination, follow_symlinks=False)
        private_tree(stage)
        install_hooks(stage)
        require(root_base(repo) == base, "Authoritative base advanced during preparation; original candidate remains unchanged")
        require(text_git(stage, "rev-parse", "HEAD") == base, "Prepared clone has an unexpected base")
        require(set(working_changes(stage, base)) == set(changes), "Prepared clone changed the exact conversion file scope")
        require(tree_fingerprint(candidate) == original_fingerprint, "Candidate changed during preparation; original work remains active and snapshots are preserved")
        atomic_exchange(candidate, stage)
        # stage now contains the complete untouched live original, not a disposable tree.
        report.update(status="prepared", preserved_original_tree=str(stage), candidate=str(candidate),
                      summary="Host refreshed the local candidate base and retained all scoped conversion edits and private original work; independent review remains pending.")
    except (publisher.Held, OSError, ValueError, TypeError, subprocess.TimeoutExpired) as error:
        report.update(status="held", error=str(error), summary="Host preparation held; existing candidate work and any private snapshots are preserved.")
    publisher.write_result(result, report)
    return report


def finalize(repo, candidate, review_path, result):
    report = initial_report()
    result = Path(result).resolve()
    try:
        repo, candidate, result = candidate_preconditions(repo, candidate, result)
        base = root_base(repo)
        review_path = Path(review_path).resolve()
        require(not review_path.is_relative_to(repo) and not review_path.is_relative_to(candidate), "Independent review must remain private")
        review = json.loads(review_path.read_text())
        require(review.get("base_commit") == base and text_git(candidate, "rev-parse", "HEAD") == base,
                "Candidate review is not on the current clean base; prepare and independently review again")
        require(not review.get("candidate_commit"), "Provisional review must leave candidate_commit empty")
        changes = working_changes(candidate, base)
        require(changes and set(changes) == set(review.get("approved_files", [])) == set(review.get("candidate_hashes", {})),
                "Independent review does not cover the exact tracked and untracked change set")
        publisher.validate_review(review, candidate)
        # The sandboxed reviewer never installs hooks or modifies .git; the host does.
        install_hooks(candidate)
        for key in ("user.name", "user.email"):
            value = text_git(repo, "config", key)
            require(value, "Authoritative Git identity is missing")
            run_git(candidate, "config", key, value)
        run_git(candidate, "add", "--", *sorted(changes))
        require(set(working_changes(candidate, base)) == set(changes), "Candidate file scope changed during host staging")
        require(root_base(repo) == base, "Authoritative source advanced before candidate commit")
        publisher.validate_review(review, candidate)
        commit = run_git(candidate, "commit", "-m", "Independently reviewed strategies: " + str(review["conversion_session_id"]))
        log = result.parent / (result.stem + "-commit.log")
        log.write_bytes(commit.stdout + commit.stderr)
        os.chmod(log, 0o600)
        final_sha = text_git(candidate, "rev-parse", "HEAD")
        review["candidate_commit"], review["base_commit"] = final_sha, base
        publisher.validate_candidate(candidate, repo, review)
        publisher.write_result(review_path, review)
        report.update(status="finalized", source_reviewed=True, implementation_ready=True,
                      ledger_checked=True, checked_trades=review["checked_trades"], admitted=False,
                      candidate_commit=final_sha, base_commit=base, review_path=str(review_path),
                      evidence=[*review["evidence"], str(review_path), str(log)],
                      summary="Host validated exact independent source/ledger evidence and created one candidate commit; publication and live admission remain separate gates.")
    except (publisher.Held, OSError, ValueError, TypeError, KeyError, subprocess.TimeoutExpired) as error:
        report.update(status="held", error=str(error), summary="Host finalization held; candidate work is preserved and no publication or deployment was performed.")
    publisher.write_result(result, report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    for name in ("prepare", "finalize"):
        command = subcommands.add_parser(name)
        command.add_argument("--repo", type=Path, required=True)
        command.add_argument("--candidate", type=Path, required=True)
        command.add_argument("--result", type=Path, required=True)
        if name == "finalize":
            command.add_argument("--review", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "prepare":
        report = prepare(args.repo, args.candidate, args.result)
    else:
        report = finalize(args.repo, args.candidate, args.review, args.result)
    return 0 if report["status"] in {"prepared", "finalized"} else 1


if __name__ == "__main__":
    raise SystemExit(main())
