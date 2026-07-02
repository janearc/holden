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

## Outcome (2026-07-02, on resolution) — DONE; ADR-0001 PILOT COMPLETE

The proof: a real delightd built from its tree accepted magpie's real
`register_once()` and the product was observed on the delightd side — magpie in the
live registry with both declared emit subjects. Green on the FIRST run (2.9s), all
five handleRegister gates exercised for real (roster, identity, fail-closed contract
verification against a live stub SR, the /health dial-back). Landed as delightd PR
65 (merge e63daa97) through the full gate stack: required checks + required judge
ruling + operator review — the first ORDINARY diff through the whole system.

The judge earned its keep on an ordinary PR: ratified at the first head with three
recorded divergences (one pre-existing api.md drift -> filed as delightd 66; one
debt this diff created in operations.md -> fixed in-diff; one public-hygiene catch,
private process artifacts cited from a public repo -> header made self-contained),
then ratified clean at the fixed head. Review-fix-rejudge as plain workflow.

Close-of-sprint items (operator review), both disposed:
1. Taskfile description fields are an ungated divergence surface — ACCEPTED-OPEN;
   named into the doc-pairing gate's scope for when that gate is built (Taskfile
   pairs with operations.md; the judge's whole-document standard already reads the
   ops doc). No machinery now.
2. The harness does not drive the test through our own libraries — RATIFIED AS
   DOCTRINE for seam proofs: an e2e proof approaches as a stranger, because driving
   it through our client conveniences tests our assumptions with our assumptions.
   Units cover the libraries; the seam is tested from outside.

Issues: closed none here (magpie 20 closed in sprint 4's ledger; this sprint filed
delightd 66 (judge-found drift) and 67 (CI wiring follow-up)). Net +2, both
tightly scoped.

ADR-0001 pilot scoreboard, final: criterion 1 (required checks live) DONE sprint 3;
criterion 2 (bad diff bounced) DONE sprint 3; criterion 3 (real ruling, ledger,
required status) DONE sprint 4; criterion 4 (seam proof observed) DONE here. The
fix-coding-process arc that began 2026-07-01 with "our coding process is broken" is
complete: gates live and required, judge seated and calibrated, seam proven,
process self-hosting.

Resolved per §4.4: `git mv 2026-07-02-e2e 2026-07-02-e2e-completed`, chmod 400.
