# Phase 0 Issues — Staged, Net +7 (Section 10 discovery-sprint exemption)

Status: STAGED, not yet filed to GitHub. All to be assigned to `janearc` per Section 10.
None fixed inline — Phase 0 records, does not fix, per Section 6.

Repo name mapping (on disk -> GitHub): `kafka-logging` = `janearc/kafka-svc`.

---

## Issue 1 — BLOCKING for Phase 2 ADR

Title: No branch protection on magpie/delightd/frood — CI exists but doesn't gate

Body:
`gh api repos/janearc/{magpie,delightd,big-little-mesh}/branches/main/protection`
returns 404 ("Branch not protected") on all three repos. `pull_request` triggers exist
and run real jobs, but nothing is a required status check — `main` is directly pushable
and a red PR can merge.

This is not a routine backlog item: Phase 2's ADR assumes gates will enforce something.
Right now, none of them do, anywhere in the roster. First concrete action before any new
gate is built: turn on branch protection with required status checks for the CI that
already exists (frood's `gen-drift` job, delightd's `go` regen+test job).

---

## Issue 2

Title: magpie README stale — delightd registration, skill wrapper, kube manifests already exist on main

Body:
`magpie/README.md` lines 39-41 list "delightd registration," the agent skill wrapper,
and kube manifests as "still to wire." All three exist on `main` — `register.py`
implements delightd registration now, using frood-generated `frood_pb2`/`register_pb2`
clients.

(Once the doc-pairing gate exists, this class of drift should be mechanical going
forward — see 3.1.)

---

## Issue 3

Title: Gen-freshness discipline inconsistent across the roster

Body:
Three different postures for the same problem, found in one Phase 0 pass:

- frood: commits generated code, has a `gen-drift` CI job that diffs it against fresh
  generation.
- delightd: gitignores generated code, regenerates and tests on every CI run instead.
- magpie: no gen-drift check at all.

Neither frood's nor delightd's approach is wrong on its own, but the roster has no single
stated convention, and magpie has none. Pick one pattern for 3.1's gen-freshness gate and
apply it uniformly, or explicitly document why the two existing approaches both stay.

---

## Issue 4

Title: frood: observability.v1 has two unemitted telemetry contracts

Body:
`observability.v1.TokenBurnEvent` — schema-registry-seeded, no non-test producer or
consumer anywhere in the roster. Already known (this is the instance the sprint doc's 4.3
originally flagged).

Second, newly found: `observability.v1.QuotaMetrics` — zero references anywhere. Same
failure shape: a contract that exists in the schema with nothing on either end.

---

## Issue 5

Title: observability.v1 package boundary floats between frood and kafka-svc

Body:
frood's `observability.proto` sets `go_package` to a kafka-svc import path — the package
is defined in frood but its generated Go code target is kafka-svc's module, not frood's
own. Worth a deliberate call in the Phase 2 ADR: is `observability.v1` meant to be owned
by kafka-svc with frood just holding the `.proto` source, or is this drift? Either is fine
— but it should be a stated decision, not an accident of where a file happened to get
generated.

---

## Issue 6

Title: wonderlib has four divergent copies with no reconciliation policy

Body:
Four sources found, no documented relationship between them:

- `janearc/wonderlib` — the published repo, treated as consumer source of truth, pushed
  2026-06-27. (Not cloned locally; prod neal pins it at rev `06ce8e9`.)
- `research/wonderlib` — a monorepo dev copy (in `janearc/blm-research`), commit
  2026-06-28, ahead of published.
- `tmp-migration/wonderlib` — an import/migration copy, no remote.
- `paling/wonderlib` — diverges behaviorally: eager `import torch` at module load, unlike
  the published repo's lazy/heuristic-by-default behavior.

Not blocking for this sprint (4.3's standup wiring pins to the published repo at a specific
revision, which sidesteps this), but the four-way split is real technical debt and should
get a sync/ownership policy before more consumers accumulate.

---

## Issue 7

Title: magpie's .pre-commit-config.yaml is dead

Body:
The config file is present and looks correct, but the hook was never installed
(`pre-commit install` was never run) and `pre-commit` itself is off PATH in the dev
environment. Right now this file does nothing — it's not enforcing, it's not even running.
Either install it for real or remove the file; a config that looks active but isn't is
worse than no config, since it reads as coverage that doesn't exist.
