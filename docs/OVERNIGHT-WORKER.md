# Unattended source review and verification

`scripts/overnight_worker.py` executes a reviewed private manifest sequentially.
It can supervise source extraction, independent reference backtests, ledger
comparisons and noninteractive Codex conversion tasks. It does not discover
rules, enable catalogue methods or deploy results merely because a command
succeeded. Pending interpretations remain pending.

The worker and all prompts, manifests, datasets, logs and generated evidence
belong in separate places: the worker is tracked source; everything else lives
outside Git, for example
`/home/server/.cache/strategy-vault-webapp/overnight-2026-10-13`.

## Run, inspect and stop

Use an existing reviewed manifest, not an invented placeholder batch:

```bash
python3 scripts/overnight_worker.py run \
  --manifest /private/overnight/manifest.json \
  --state-dir /private/overnight \
  --memory-mb 2048 --cpu-cores 2 --max-runtime-hours 24

python3 scripts/overnight_worker.py status --summary --state-dir /private/overnight
python3 scripts/overnight_worker.py stop --state-dir /private/overnight
```

`stop` creates `STOP`; the running worker terminates the current process group
and retains its evidence and journal. Remove that file deliberately before
resuming. Completed jobs are skipped. Interrupted jobs may retry only within
their manifest attempt limit. One file lock prevents two workers sharing a
state directory.

`--watch` allows reviewed jobs to be appended to the same manifest while the
worker runs. The worker also waits when available memory is below 2 GiB.
`--continuous` removes the worker's overall wall-clock deadline and implies
`--watch`; individual job timeouts and resource limits still apply. Without
that explicit option the default deadline remains 24 hours.
Idle polling checks every five seconds and avoids rewriting unchanged status artifacts.
Changing an existing job's commands, provenance or results path is rejected;
use a new ID for a changed specification. The explicit `approved` boolean may
change from false to true without changing a job's meaning.

## Durable Linux service

A user systemd service survives the end of a chat or terminal. On this host,
tools may need the two user bus environment variables below. Start one
reviewed queue only, using absolute paths to the actual script and manifest:

```bash
XDG_RUNTIME_DIR=/run/user/1000 \
DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
systemd-run --user --unit=strategy-vault-overnight \
  --property=Nice=10 --property=CPUQuota=150% \
  --property=MemoryMax=3G --property=MemorySwapMax=0 \
  --property=IOWeight=10 --property=KillMode=control-group \
  --property=TimeoutStopSec=15 --property=RuntimeMaxSec=24h \
  --property=WorkingDirectory=/home/server/.cache/strategy-vault-development \
  /usr/bin/python3 \
  /home/server/.cache/strategy-vault-development/scripts/overnight_worker.py run \
  --manifest /private/overnight/manifest.json \
  --state-dir /private/overnight --watch --max-runtime-hours 24
```

The operating system limits all descendants to the service's memory and CPU
budget. Every subprocess also has low scheduling priority, a maximum two-core
affinity, one-thread numeric library settings, CPU time and wall time limits,
and a bounded log. Python tasks receive an address-space limit. Node and Codex
tasks use resident-memory observation because virtual address reservations
are larger than their real memory use. `MemoryMax` is the firm bound for those
process trees. External engine APIs retain their own container limits.

Service settings do not survive a PC reboot automatically; this example is a
transient unit. Check the service after starting it and verify the first real
job in `status.json`, rather than treating successful launch as completed work.

## Continuous conversion and separate review

`scripts/continuous_conversion.py` refills the worker from an owner-authorized
private backlog. It creates an isolated worktree, runs a bounded conversion,
freezes the proposed file hashes, then starts a separate Codex review session.
Reviewers must read the actual source and perform their own checks without
editing the frozen candidate. Only after that stage finishes does the next
source task begin. This coordinator **never publishes** candidates: its best
outcome is `reviewed_pending_integration`, requiring the normal integration
and release checks. Its `live_admissions` counter is always zero.

The private backlog has `schema_version: 1`, `approved_by`, and `tasks`. Each
task supplies a stable `id`, `author`, `book`, concrete `scope`, `instrument`,
absolute `source_paths` and `data_paths`, source SHA-256 bindings in
`source_hashes`, `adaptations`, and exact `method_ids` when already identified.
Optional `data_hashes` bind the price inputs. Empty method IDs mean source
discovery, not permission to invent strategies. Source files are checked lazily
for the current task; startup does not scan or backtest the whole library.
Changing a started specification requires a new task ID.

```bash
python3 scripts/continuous_conversion.py \
  --backlog /private/continuous/backlog.json \
  --state-dir /private/continuous \
  --repo /home/server/.cache/strategy-vault-development \
  --worktree-root /private/continuous-worktrees \
  --codex /absolute/path/to/codex --watch
```

For owner-authorized continuous operation, install a persistent user unit in
`~/.config/systemd/user/strategy-vault-conversion.service`, with the absolute
command above as `ExecStart`, `WantedBy=default.target`, and the same CPU,
memory, swap and process-group limits as the transient example. Set
`RuntimeMaxSec=infinity`, `Restart=on-failure`, `RestartSec=20`, and bounded
start retries; enable the unit with `systemctl --user enable --now`. User
lingering must be enabled if it is to survive logout. These settings remove
the old daily service expiry without removing per-job limits. A powered-off
machine or unavailable account/network cannot perform research.

Inspect `pipeline.json` for the current source and conversion/review stage;
`status.json` records the underlying jobs. `waiting_for_backlog` explicitly
means idle. The state directory lock prevents duplicate coordinators.
The coordinator waits before starting another source when less than 20 GiB
remains on its state volume; preserved evidence and live data are not deleted
to make room.

An actual CLI usage/quota/rate-limit error stops the process, retains its
evidence, and writes `LIMIT_REACHED.json`. That marker prevents automatic
restart or progression to other tasks. Error-shaped events are checked while
the CLI runs, so its internal retries do not keep consuming work after a
detected limit. Ordinary agent prose mentioning limits is not an error event.
Never change accounts or API credentials to evade a limit. Resume deliberately
only after the account is usable again; preserve the marker's audit history.
The regular `STOP` mechanism also stops the coordinator.

Unique PDF hashes are source-candidate counts. A PDF may be a chapter, article,
edition or non-rule-based reading; it is not necessarily a distinct book or a
convertible strategy. Keep candidate, family, variant, checked-run and deployed
counts separate in progress reports.

## Reviewed manifest contract

The JSON object contains `schema_version: 1`, an `approved_by` description and
at most 200 jobs. Every job has:

| Field | Meaning |
| --- | --- |
| `id` | Stable unique identifier; a changed task needs a new ID |
| `approved` | Explicit boolean; false jobs are held |
| `kind` | `source_review`, `reference_backtest`, `ledger_check`, `data_audit` or `agent_conversion` |
| `argv` | Argument array, executed without a shell |
| `cwd` | Existing local working directory; never an original GVFS share |
| `timeout_seconds` | Wall time, at most 7,200 seconds; default 600 |
| `max_attempts` | One to three attempts; default one |
| `result_file` | Absolute JSON output path inside the private state directory |
| `provenance` | Author, book, method, source paths, instrument, data paths and adaptations |

`provenance` requires `author`, `book`, `method`, `instrument`, `source_paths`,
`data_paths` and `adaptations`. The last three fields are arrays. An instrument
description should include market, instrument class, actual symbols and data
coverage. Empty source/data arrays are valid for operational test jobs; actual
strategy jobs must retain their real inputs. The worker records hashes of
source/input files up to 32 MiB; the data verifier must hash larger inputs.

No command can depend on an interactive answer: ordinary commands receive
closed stdin. Commands are explicitly approved code, not a general security
sandbox. They must respect AGENTS.md, preserve original shares, and write
results only to their private output location or isolated development tree.

## Codex conversion jobs

An `agent_conversion` job additionally requires `isolated_workspace: true`,
an existing Git worktree at `cwd`, and `stdin_file` pointing to its reviewed
private prompt. It runs an explicit `codex exec` command with:

- `-c 'approval_policy="never"'` so failed commands return without a prompt;
- `--sandbox workspace-write` or `read-only`;
- `--json`, `--output-schema` pointing to a real schema, and
  `--output-last-message` pointing to the job's `result_file`;
- an explicit `--add-dir` for the job's private artifact directory when it is
  outside the worktree.

Use `-` as the final prompt argument so Codex reads the reviewed `stdin_file`.
The executable must be named `codex`. Do not bypass the sandbox, alter global
configuration, launch nested workers or recursively spawn CLI sessions.
The [official noninteractive documentation](https://learn.chatgpt.com/docs/non-interactive-mode)
describes structured output and automation. Local `codex exec --help` must
also confirm flags for the installed version.

Each prompt should define a small book/method scope, exact source paths,
permitted data and instruments, declared-default policy, required tests,
output evidence and allowed files. It must prohibit writes to original shares,
credentials, deployment and automatic source-policy admission. Missing ATR
periods or execution conventions can be researched or declared as adaptations;
they cannot be described as the exact book rule.

## Honest result and progress contract

The result artifact is a JSON object. Optional fields are:

```json
{
  "source_reviewed": false,
  "implementation_ready": false,
  "admitted": false,
  "ledger_checked": false,
  "checked_trades": 0,
  "evidence": [],
  "summary": "Describe actual completed work and unresolved items."
}
```

Missing flags mean false. Admission requires reviewed source rules and an
implementation ready for review. A checked ledger requires reviewed source,
at least one checked trade and existing evidence files. A successful command
without a valid result is a failed job. An older result is archived before a
retry, so it cannot turn a failed attempt into success.

These booleans are assertions made by the approved verifier or reviewer;
the supervisor checks the contract, not the financial correctness of a
strategy. An agent's final response alone does not certify its implementation.
Keep independent ledger comparison, source evidence and human review separate.

`status.json` counts **jobs**, including succeeded tasks and each separate
review flag. It does not count new method families, parameter variants or
instrument runs interchangeably. A batch result may retain detailed
per-method reports and actual counts, but those reports need their own audit
before the application availability counters change.

## Verification

```bash
python3 -m unittest discover -s scripts -p test_overnight_worker.py
python3 -m unittest discover -s scripts -p test_continuous_conversion.py
```

The tests use temporary queue artifacts and small subprocesses. They check
timeouts, stop behavior, retries, immutable provenance, explicit admission
evidence, resume behavior, resource settings and the status/stop CLI. They do
not generate fictional market data or publish financial results.
