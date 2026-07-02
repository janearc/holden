# Sprint 5: Pilot, part 3 — the registration e2e proof (ADR-0001 criterion 4)

- Date cut: 2026-07-02 (fifth sprint of the day; HEAVY — cross-service code)
- Contract: `2026-07-02-pm-completed/ADR-0001-coding-process-gates.md` (ACCEPTED)
- Stage: concept -> pseudocode (T0 below; no code until ratified)

## Goal

The last pilot criterion, run for real: **magpie registers with delightd in a test
harness, and the registration is observable on the delightd side** — service A talks
to service B and the product comes out. This is also the first executable instance of
the design-doc-names-its-e2e-proof doctrine (ADR D8) applied to a seam.

Meta-goal, free of charge: the PRs this sprint produces are the first ORDINARY diffs
to pass through the full gate stack (required checks + required judge ruling + operator
review) as plain workflow rather than as the thing being built. The process eats its
own dogfood end to end.

## T0 design — the proof harness

**Home: delightd, `tests/e2e/registration/`.** The registration contract's owner
(delightd owns `registry.v1`, per the pinned ownership) owns the seam's proof. magpie
is exercised as a real client, not mocked — a mocked client would prove nothing about
the seam.

**Shape (one script-driven Go test + a make/task entry):**

1. **Arrange:** build/start delightd from the working tree with a test config —
   ephemeral state dir, ephemeral port, Kafka publisher nil'd (best-effort bus is out
   of pilot scope, and delightd's events are already best-effort by design).
2. **Act:** invoke magpie's real registration path against the local delightd —
   `uv run` in the magpie checkout, calling the same `register_once()` the daemon
   uses, pointed at the test port. Local-first assumption stated plainly: the harness
   expects `~/work/magpie` checked out (laptop reality; this is a fleet-local proof,
   not a cloud CI artifact).
3. **Assert (the product):** delightd's HTTP API reports magpie registered — the
   project appears with its declared emit contracts (`ServiceHealthHeartbeat`,
   `BentoLifecycleEvent`), matching what `register.py` declares.
4. **Teardown:** kill the daemon, remove the ephemeral dir. Idempotent re-runs.

**Explicitly out:** wiring this into GitHub CI (cross-repo checkout + Go+Python
toolchains in one runner — real, but its own decision; recorded as a follow-up issue,
not smuggled in); the Kafka/bus leg (kafka-svc's seam, later pilot); the frood sidecar
(the r2/r4 ledger tripwire stays armed for whoever builds it).

**Judge note:** the diffs land in delightd (and magpie only if a test seam requires
a flag — none expected). Each PR gets `judge` run against it before landing, like any
other PR now.

## Definition of done

- The harness exists in delightd, runs green locally, re-runs idempotently.
- The assertion is on the DELIGHTD side (observed registration + contracts), not on
  magpie's exit code.
- Its PR(s) landed through the full gate stack: required checks + ruling/ratify +
  operator review.
- Follow-up issue filed for CI wiring (assigned janearc), per the out-of-scope note.
- Standup records at boundaries; outcome recorded; promoted via git mv + chmod 400.
