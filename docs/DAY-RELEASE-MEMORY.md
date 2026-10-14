# Bounded memory for reviewed Rust publication

The first overnight publisher attempts passed independent source/ledger review
but failed in the focused Rust compiler stage. The Python supervisor applied
its 2 GiB address-space limit to the publisher, and Rust/LLVM inherited it.
Compiler virtual address reservations can exceed real resident memory, so this
limit caused allocation failure before a source batch reached publication.
Those attempts remain recorded as held; they are not counted as live methods.

New reviewed publication jobs may explicitly use:

```json
{
  "kind": "source_review",
  "memory_mode": "resident",
  "argv": [
    "/usr/bin/python3",
    "/home/server/.cache/strategy-vault-development/scripts/publish_reviewed_batch.py",
    "--candidate", "/private/isolated-candidate",
    "--repo", "/home/server/.cache/strategy-vault-development",
    "--review", "/private/batch/independent-review.json",
    "--result", "/private/batch/result.json",
    "--runtime-env", "/home/server/.cache/strategy-vault-webapp/.env",
    "--deploy-user-approved"
  ]
}
```

This mode is accepted only for the trusted publisher in the worker's own
checkout, through the worker's Python interpreter, with exactly one explicit
authoritative `--repo`. A generic script, shell wrapper or copied publisher
cannot opt out of the normal address-space limit.

Before accepting the manifest and again immediately before spawning, the
worker reads its effective cgroup-v2 limits, including tighter ancestors.
Resident publication requires a finite memory hard cap no greater than 3 GiB
and a zero swap cap. The existing systemd service retains `MemoryMax=3G`,
`MemorySwapMax=0` and its existing CPU quota; no global settings or resource
caps are increased.

The virtual address-space limit alone is omitted for these explicit jobs.
The supervisor still observes aggregate descendant RSS and terminates the
process group above its job memory budget, which cannot exceed 2 GiB for this
mode. Low priority, maximum two-core affinity, one-thread numeric libraries,
CPU time, wall time, stop handling and bounded logs remain active. The cgroup
hard cap bounds the complete service, including descendants outside the
original process group. Existing Docker engine validation containers retain
their own resource limits.

Publisher Cargo configuration separately uses one build job, disables
incremental compilation, and omits dev/test debug information. Those settings
reduce compiler memory; they do not change source rules or strategy parameters.
The publisher still runs focused tests, full real-data regression, temporary
engine/API checks and the independent publication gates.

`memory_mode` changes a job's immutable specification. Preserve old failures
and use a new job ID for the bounded retry. Restart an idle supervisor to load
the new implementation before adding the new jobs. Do not restart or change
the manifest while a source agent is actively working. Root owns these live
queue/service operations.

Tests verify restricted opt-in, missing/unbounded cgroup rejection, zero-swap
protection, effective ancestor caps, preserved generic address-space limits,
an actual child retaining CPU/core limits, and termination at the RSS budget.
Operational subprocess fixtures do not claim Rust source or trading accuracy;
the real reviewed publisher still has to pass its actual bounded compilation.
