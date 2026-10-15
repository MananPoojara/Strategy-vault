# Reviewed overnight batch publication

`scripts/publish_reviewed_batch.py` is a deterministic release gate after an
independent review. It does not interpret books or substitute a successful
agent response for source or trade verification.

Sandboxed reviewers keep Git metadata read-only. The host
[prepares and finalizes their candidate](SANDBOXED-REVIEW-HANDOFF.md) after
validating the independent review's exact source and evidence hashes; it does
not grant the reviewer deployment access.

The isolated candidate and authoritative checkout must both be clean. The
authoritative branch must be `Manan`. The candidate contains exactly one
commit whose parent is the current authoritative HEAD. A stale base is held
for another review; this script does not merge, reset or discard somebody
else's work. The remote `Manan` must still equal that base before publication.

## Invocation

```bash
python3 scripts/publish_reviewed_batch.py \
  --candidate /private/isolated-candidate \
  --repo /home/server/.cache/strategy-vault-development \
  --review /private/batch/independent-review.json \
  --result /private/batch/publication-result.json \
  --runtime-env /home/server/.cache/strategy-vault-webapp/.env \
  --deploy-user-approved
```

The final flag is required for live deployment. Without it, validated code is
uploaded to `Manan`, and live admission remains false in the result. The user
has authorized correct strategy work and unattended deployment in this
session; the explicit flag records the chosen action in the reviewed queue.
The script never changes global approvals or publishes credentials.

Supporting source changes are bounded separately from strategy modules.
`main.rs` may change only inside its final `#[cfg(test)] mod tests` module;
its production prefix must remain byte-identical. `gate.rs` may update records
only for the exact independently approved method IDs and add tests. Existing
records cannot disappear; its struct and production lookup helpers must stay
identical. Changes to production API handling or shared calculations require
their own implementation and review, rather than entering through a strategy
metadata publication. These checks inspect the actual before/after Git source,
in addition to the review's full file hashes.

## Independent review contract

The private review is a JSON object with these required fields:

| Field | Required value or meaning |
| --- | --- |
| `schema_version` | `1` |
| `reviewer_id` | Independent review job ID |
| `conversion_session_id` | Different conversion job ID |
| `candidate_commit` | Full candidate Git SHA |
| `base_commit` | Full current authoritative Git SHA |
| `approved_files` | Exact changed source-file list |
| `candidate_hashes` | Map of every changed relative file path to its SHA-256 |
| `method_ids` | Exact unique IDs, at most 100; no book prefixes |
| `source_reviewed` | `true` after reviewing the actual book/source |
| `implementation_ready` | `true` after checking the executable interpretation |
| `ledger_checked` | `true` after independent actual-price ledger checks |
| `checked_trades` | Positive integer, matching the evidence's actual record total |
| `row_mismatches` | Integer `0` |
| `source_hashes` | Map of absolute actual source paths to SHA-256 |
| `evidence` | Nonempty list of existing absolute evidence-file paths |
| `ledger_reports` | List of objects with absolute `path` and `sha256` |
| `focused_tests` | Existing Rust test filters, such as `books::kftsm::tests` |
| `smoke_cases` | Reviewed API cases on actual supported instruments |

Include author, book title, source pages, source interpretation, declared
adaptations, data coverage and open questions in additional review fields and
the source evidence. The reviewer must inspect these; hashes alone establish
identity, not correct interpretation.

Each independent ledger report must have true `source_reviewed`,
`implementation_ready` and `ledger_checked` flags. Its `checks` array contains
`method`, `row_mismatches: 0`, nonnegative `actual_records`, equal integer
`expected_records`, and the actual `ohlc_path`/`ohlc_sha256` for each tested
method/instrument case. The union of methods must equal `method_ids`, and the
sum of record counts must equal `checked_trades`. A nonempty
`rust_module_sha256` map is required; every recorded module must match the
candidate source, including at least one reviewed book module changed in the
candidate. Across the reports, every changed book module must have its exact
compiled-source hash, including nested helpers. An old code proof or changed
market file is held.

Each smoke case has:

```json
{
  "method_id": "exact_reviewed_method_id",
  "guru": "connors",
  "market": "india",
  "query": {
    "ticker": "NIFTY 50",
    "from": "2000-01-03",
    "to": "2026-10-07"
  },
  "expected_readiness": "ready",
  "instrument_kind": "index"
}
```

Markets are `india` and `us`; guru routes are `connors`, `ross` and `chande`.
Instrument classes are `index`, `etf` and `individual_stock`. Native stocks
use the actual engine ticker from its admitted basket catalogue, not an index
standing in for a stock. Every method needs at least one ready case. Add
incompatible-instrument cases with `needs_data`, `not_testable` or
`calculation_unavailable`; those run endpoints must reject performance.
Current snapshot membership and declared cash-index adaptations retain their
limitations. No original book result is inferred from a new instrument run.

## Publication checks and limits

Only additions/modifications to these source paths are allowed:

- `Context/guru/backend/src/books/*.rs`
- The explicitly reviewed Bandy helper `Context/guru/backend/src/books/hbqts/reviewed.rs`
- `Context/guru/backend/src/instrument_policy.rs`
- `Context/guru/backend/src/readiness.rs`
- Markdown review documents under `docs/`
- Reviewed code and Markdown under `library/`

No frontend, authentication, gateway, deployment configuration, raw data,
books, binary files, source deletion or unrelated Rust infrastructure changes
are accepted. Those require a separate development review.
Changed files must be regular Git blobs inside the candidate, with no symlink
in the file path. A matching hash reached through an external link is rejected.

The release sequence is:

1. Verify clean Git histories, exact files, source/candidate hashes and hashed
   independent ledger evidence.
2. Scan candidate tracked files against public-upload exclusions and the
   existing private credentials without printing them.
3. Run focused Rust tests and the full Rust suite, with `VAULT_IDX` pointing to
   the existing real index data. A filter matching zero tests is held.
4. Build the candidate engine with the existing two-job Rust Docker build.
5. Start one temporary engine at a time, limited to one CPU and 768 MiB, with
   actual runtime index/ETF and stock datasets mounted read-only. Compare all
   three guru catalogues against the current engine. Newly live methods,
   newly reviewed policies or wider instrument admissions must all belong to
   the exact approved IDs; existing live methods cannot disappear. Unapproved
   methods must keep exactly the same allowed instrument types, including
   against accidental narrowing of stock/index/ETF support.
6. Test reviewed readiness and actual-price runs on the specified instruments.
   Ready runs must return consistent equity dates and a trade ledger; blocked
   instruments must reject run results. Also rerun previously live methods in
   each touched book on deduplicated market/ticker/date windows from the smoke
   cases. Exact approved IDs are excluded so an independently verified
   intentional correction may change their results. Baseline-ready unapproved
   methods must remain ready and preserve their dates, trade ledgers, net/gross
   equity, statistics, benchmark and cost calculation fields. Timing and source
   provenance metadata are excluded from these calculation hashes. Compact
   before/after SHA-256 evidence stays in the private result, rather than
   copying full data payloads into Git. This regression is capped at 200
   method/window cases across both markets; larger batches are held for a
   smaller review. Baseline cases already blocked for required data are counted
   separately and do not produce invented backtests.
   A baseline-ready method can still reach a calculation guard when its actual
   ledger is evaluated. An existing HTTP 400 `calculation_unavailable:` response
   passes regression only if the candidate preserves the exact status and error
   body. These cases have separate hashed `withheld_checks` evidence and a
   `checked_withheld_cases` count; they never count as successful backtests.
   Changed guards, unexpected successful results and all other runtime errors
   hold publication. Date parameters omitted by the reviewed context remain
   omitted, preserving the API's default history window.
7. Recheck source cleanliness, fetch the reviewed local commit, fast-forward
   it on `Manan`, scan the public files and push without force. A concurrent
   remote update causes a held release.
8. When requested, retain the prior engine image under a rollback tag, replace
   only `engine-in` and `engine-us`, and repeat the actual-instrument API cases.
   Authentication, database, gateway and web services are not recreated.

Run this under the bounded overnight service described in
[OVERNIGHT-WORKER.md](OVERNIGHT-WORKER.md). Docker builds use two Cargo jobs;
temporary containers have separate explicit resource limits. No whole basket
portfolio or startup-wide optimization is launched here.

## Failure and progress reporting

`publication-result.json` distinguishes source upload, runtime deployment and
live method admission. Counts are populated only after live API checks pass.
`admitted_variant_count` counts newly live exact catalogue IDs;
`admitted_family_count` and the primary `admitted_method_count` count newly
enabled `(book, family)` pairs. Threshold and direction variants inside an
existing family add zero to the new family count. `reviewed_variant_count`
also records reviewed policy additions that might widen instrument support
without adding another method. These counts do not certify globally unique
economic ideas across books; duplicate source rules need reviewer disclosure.
Private command logs retain validation evidence.

A failed runtime verification restores the previous engine image and checks
that its APIs respond. Reviewed source already pushed remains on `Manan`;
the script does not rewrite shared history. A failed rollback is recorded as
held and needs operator attention. A stale/dirty checkout, missing source or
mismatched ledger stops before publication. The result never claims a held
batch is live.

The script currently requires the two production engines to use the same
previous image, and the configured runtime image to use a mutable tag. An
installation with separate per-market images or immutable image settings is
held for an explicit deployment plan.

## Tests

```bash
python3 -m unittest discover -s scripts -p test_publish_reviewed_batch.py
```

Gate tests use real temporary Git repositories. Release-flow tests mock
Docker, network and push operations and verify ordering, rollback and honest
counts. They are not presented as a real deployment or a financial result;
the actual queue must still pass its real data, engine and API checks.
