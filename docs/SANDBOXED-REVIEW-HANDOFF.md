# Sandboxed conversion with host Git handoff

Conversion and independent review jobs retain the `workspace-write` sandbox.
Git metadata is read-only inside that sandbox. They edit allowed source and
write private evidence; they do not fetch, install hooks, stage, commit, push
or deploy. A failed metadata write is an automation boundary to handle on the
host, not a reason to bypass the sandbox or weaken global configuration.

The queue sequence is conversion → host preparation → independent review →
host finalization → deterministic publication. The existing publisher still
performs source, real-ledger, test, API, public-upload and engine release gates.

## Host preparation

```bash
python3 scripts/prepare_review_candidate.py prepare \
  --repo /home/server/.cache/strategy-vault-development \
  --candidate /private/isolated-candidate \
  --result /private/prepare/result.json
```

Preparation requires a clean authoritative `Manan` and an isolated local clone
with its own Git directory. It never changes the authoritative checkout.

It copies the complete old candidate, including Git state and ignored working
material, to a private backup, limited to 512 MiB. Artifacts use private
permissions. All tracked, staged, unstaged and untracked source differences
from the common ancestor must fit the publisher's exact file allowlist.
Deleted files, conflicts, symlinks and source outside the candidate are held.

A private alternate Git index produces one binary patch including untracked
source. A fresh clone of current `Manan` receives that patch with a three-way
application. A conflict leaves the original candidate active and unchanged,
plus the original backup and conflict staging clone for inspection. Ignored
material is also retained without copying through symlinked parents or
overwriting a new base file.

The host installs the unchanged current-base hook installer before handing
the candidate to the reviewer. Full tree fingerprints detect edits during
backup or preparation. If the root advances, or candidate work changes, the
operation holds with the old work preserved.

A single Linux `renameat2` exchange replaces the isolated candidate path only
after those checks pass. Its working directory never disappears between
renames. The complete original live tree is retained privately as well as its
snapshot. No reset, source deletion, force operation or sandbox bypass occurs.
The result is `status: "prepared"`, with `base_commit`, `changed_files`, backup
paths and evidence. This is preparation success, not strategy verification.

## Independent reviewer instructions

Use this instruction in private reviewer prompts:

> Read the host preparation result first. Continue only when its status is
> `prepared`. Read AGENTS.md and independently inspect the actual book/CD,
> source rules, declared adaptations and final candidate code. Recheck the
> compiled candidate against independent actual-price ledgers, including source
> and data hashes. You may fix only allowed source files and rerun evidence.
> Git metadata is intentionally read-only: do not fetch, rebase, install hooks,
> stage, commit, push or deploy. Inspect changes with `git diff HEAD` and list
> untracked source too. Write the complete independent-review.json contract
> with `candidate_commit: ""` and `base_commit` equal to the prepared candidate
> HEAD. Its exact approved_files and candidate_hashes must cover every source
> change, including new untracked files. Missing evidence or unresolved rules
> produces a held review. Leave live admission false; the host and publisher
> complete later gates.

The provisional review otherwise uses all fields in
[REVIEWED-BATCH-PUBLICATION.md](REVIEWED-BATCH-PUBLICATION.md): different review
and conversion job IDs, true review/readiness/ledger flags only for an accepted
subset, exact method IDs, actual checked counts, zero mismatches, source hashes,
hashed ledger reports, focused tests and exact real-instrument smoke cases.

## Host finalization

```bash
python3 scripts/prepare_review_candidate.py finalize \
  --repo /home/server/.cache/strategy-vault-development \
  --candidate /private/isolated-candidate \
  --review /private/review/independent-review.json \
  --result /private/finalize/result.json
```

The candidate HEAD and provisional review base must still equal the current
clean authoritative `Manan` HEAD. The complete tracked/untracked change set
must exactly match approved_files and candidate_hashes. The host invokes the
publisher's independent source/data/code hash and ledger gates before staging
and again before committing. It installs trusted hooks on the host, creates
exactly one candidate commit and validates the resulting clean candidate.

Only after validation does it fill `candidate_commit` and `base_commit` in the
same private review file. `status: "finalized"` reports actual review flags and
checked records while keeping `admitted: false`. A failure holds and preserves
work; it performs no publication, deployment or shared history rewrite.

Run preparation/finalization only between the associated conversion and review
jobs, never against a candidate actively being edited. Serialize host handoffs
for each clone. If the authoritative base advances after a review, prepare and
independently review again; a metadata refresh cannot certify new source.

## Verification

```bash
python3 -m unittest discover -s scripts -p test_prepare_review_candidate.py
```

Tests use actual temporary Git histories to verify updated bases, tracked and
untracked source preservation, ignored work, conflict holds, dirty-root holds,
Git directory isolation, symlink escapes, concurrent edits, exact finalization
scope and no commit on a failed independent gate. Financial gate flow tests
mock approval deliberately; they do not replace actual source/ledger evidence.
