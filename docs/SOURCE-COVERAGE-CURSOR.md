# Private source coverage cursor

The conversion coordinator tracks exact source bytes and reviewed page scopes separately from task completion. A terminal task is a bounded research result. It does not establish that a whole PDF, authored book or author has been exhausted, or that any strategy is executable or publicly admitted.

This feature writes only private coverage records and residual **proposals**. It never appends tasks to a backlog, resets terminal markers, overrides usage controls, changes financial calculations or grants source/instrument admission. Root publication receipts remain separate from historical worker results.

## Compatibility and activation

Existing task objects, result schemas, job commands, terminal statuses and the original `continuous_conversion_prompt.txt` remain unchanged. New tasks may opt in with `source_cursor_required: true` and optional `source_cursor_contract_version: 1`.

A flagged task receives job context v2 only if neither phase already has a prompt. Its exact task digest and both prompt templates are then pinned under `source-job-contexts/`. Conversion, independent review and retries reuse those pinned templates, so an unrelated template update cannot silently change the job signature. Any task with an existing legacy phase prompt keeps both phases on the legacy contract. Its coverage is explicitly unproven, even if the task was later flagged. Migrating that scope requires a new task ID; never rewrite a running job's prompt.

`source-cursors.json` is seeded from expected SHA metadata, without opening every source. Legacy tasks retain `legacy_coverage_unproven` or `legacy_prompt_coverage_unproven`. Expected source metadata is not an accepted coverage record. Opt-in v1 currently requires exactly one existing PDF with its expected SHA and an actual `pdfinfo` page count. Other formats require a separate future format contract and remain unproven.

## Sidecars and physical source scopes

Both phases write `jobs/<task-id>.<phase>/source-coverage.json` using the schema in `scripts/source_cursor_prompt_v1.txt`. Each sidecar binds the exact source SHA, actual physical PDF page count, actor job ID, bibliographic completeness declaration and a cumulative inventory of source sections and discovered methods/components.

Physical pages start at 1. A cell owns either the whole page or its left/right half; whole-page ownership overlaps either half. Sections cannot overlap or invent absent pages. Context may be cited by evidence without assigning the same page to two sections. Complete inventory requires ownership of every actual page, including front matter and end matter. Printed-page gaps remain explicit; `source_missing` sections have no physical page cells. Missing, unreadable and unread sections are separate states.

Read sections require existing, hashed, page-bound evidence in that phase's own artifact directory. Paths cannot be absolute, traverse parents, escape the job directory or use symlinks. The validator checks existence, SHA, bounds and agreement. A different reviewer must still read the actual source and assess whether the evidence and classifications are supported; file hashes alone cannot prove reading or correctness.

Each discovered item has a stable method ID, family ID, kind (`strategy`, `component`, `reading`) and state. Source-only checking is distinct from ledger correctness, integration and release. A reading item cannot imply a pending strategy. A held item requires a precise reason. At most three `batch_method_ids` belong to a batch; other discovered methods stay visible and pending. Three parameter variants do not establish three algorithm families.

Limits are deliberate: 10,000 physical pages per PDF, 2,048 sections, 256 methods per section, 256 evidence files and 32 MiB total evidence per phase. `pdfinfo` has a 20-second wall limit, 10-second CPU limit and 512 MiB address-space bound. The coordinator does not render or backtest a corpus during seeding.

## Frozen converter and independent acceptance

After a successful conversion, the coordinator validates its sidecar and creates a frozen copy plus a coordinator-owned binding outside the phase artifact directory. The binding records the original and frozen sidecar hashes, source SHA and normalized coverage digest. Repeating the freeze is idempotent; a changed original, frozen copy or binding cannot replace the earlier record under the same task ID.

The reviewer writes separate evidence and a separate sidecar with its own exact review job ID, `accepted: true` and the bound original converter-sidecar SHA. Acceptance requires the same normalized logical coverage from both phases and unchanged converter/frozen files. A disagreement or missing sidecar fails the task's coverage validation; it is not interpreted as an empty book or a finished source.

Accepted coverage accumulates by exact source SHA. It cannot erase earlier sections, change accepted physical bounds, regress read sections or erase/change method identities. Genuine held states cannot be automatically reopened. A changed-input repair needs a separate explicit reviewed revision contract, not a blind retry. This v1 deliberately does not implement that repair override or an inventory-repartition override.

The registry keeps each accepted pair's hashes and actor identity. The current cursor exposes unread, unreadable, missing, pending and held scopes. `all_available_sections_reviewed` means the complete inventory's available physical pages were read/classified; it says nothing about all methods being runnable. `complete_bibliographic_work_reviewed` is always false in v1: physical page coverage cannot certify a complete authored work, even when a sidecar declares its scan complete.

## Residual proposals and their limits

Only a different-reviewer-accepted sidecar pair can generate residual proposals. IDs are stable functions of the source SHA and exact logical scope. A batch produces at most three proposals, each for one unread section of at most 32 physical pages, or at most three pending method IDs from a previously read section.

Unread-section proposals have disjoint page ownership. Method follow-ups may share the already reviewed source pages while their explicit method-ID groups remain disjoint; rereading that context is intentional. All remaining unread sections and pending methods stay in the cursor after the proposal cap is reached. Oversized unread sections stay in `oversized_sections_needing_reviewed_subdivision`; v1 does not infer chapter boundaries or silently split them. A future explicit independently reviewed repartition contract is needed for an already accepted oversized inventory.

Held, reading-only, unreadable and missing scopes do not create automatic retries. Proposals are saved under the review job with `not_appended_to_backlog: true`, `proposal_pending_root_review` and `admitted: false`. Before root creates new immutable task objects, reconcile proposed IDs/scopes against the existing backlog, other handoffs and release receipts. An existing manual chapter task must not be duplicated just because it lacks a v1 cursor.

## Verification and rollout

Run bounded Python checks from the checkout:

```sh
PYTHONPATH=scripts python3 -m unittest \
  scripts/test_source_cursor.py scripts/test_continuous_conversion.py \
  scripts/test_overnight_worker.py scripts/test_publish_reviewed_batch.py
```

Tests use a generated PDF fixture. They cover actual page count/hash checks, held batches retaining unseen chapters, extra methods after the cap, partial/missing pages, independent disagreement, path/hash/overlap failures, immutable freezes, monotonic holds, bounded proposals, original prompt/retry identity and a complete fake-worker sidecar cycle. They do not assert a source book was fully read or a strategy was numerically verified.

Review and integrate this change before rollout. Upgrade the coordinator at an actual job boundary while preserving all prompts, task specifications, source evidence and worker markers. Observation timeouts are not permission to interrupt or restart a running conversion. Existing running tasks continue under their original contract.
