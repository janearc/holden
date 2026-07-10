# ADR 0003: holden is a service

- Status: DRAFT (awaiting operator ratification)
- Date: 2026-07-10
- Deciders: Max (operator), Claude (agent — drafted by Fable 5)
- Revises: ADR-0001 (coding-process gates), Decision 6 mechanics. ADR-0001's
  principles are not touched; this document changes how the judge is invoked and
  where it lives, because we outgrew the original mechanics, not the ideas.
- Naming: holden catches diffs before they go over the cliff. That is the whole
  job description, and the repo is named for it.

## Summary

The judge grew up in the sprints repo as a hand-run CLI: an operator types a
command, a `claude` subprocess renders a verdict, a YAML ruling lands in the
sprint ledger, a commit status gates the merge. ADR-0001's design has held —
writer is never judge, done is external, rulings are schema'd artifacts — and
that spine carries forward unchanged. What we outgrew is the mechanics around
it: invocation only by a human at this keyboard, a model reached only as a
hardwired subprocess, no way for anything else on the mesh to request a ruling
or observe one happening. holden becomes a service with two protobuf contract
surfaces: an inbound ruling surface that holden owns, and an outbound work
surface that the harness (haho) owns and holden merely consumes.

## What ADR-0001 got right, kept verbatim

- **Fresh judgment per ruling.** Every ruling is a fresh model instance with no
  memory of the diff's authoring. Unchanged, and load-bearing (see the
  resident-judge reconciliation below).
- **Done is external** (ADR-0001 Decision 3). Mechanical gates green, a ratify
  ruling on the head SHA, operator sign-off. Nothing here relaxes that.
- **Rulings are fixed-schema artifacts** in a durable ledger, enforced on GitHub
  as the `ruling/ratify` commit status. The schema is unchanged; it gains a
  protobuf representation so it can travel a wire as well as sit in a file.
- **The test goalpost is the design doc** (Decision 8). This ADR names its own
  e2e proof below.

## What we outgrew

- **Invocation is a human at one keyboard.** Nothing on the mesh can request a
  ruling; delightd cannot ask for one, a CI event cannot trigger one, a second
  operator machine does not exist as a concept.
- **The model is a hardwired subprocess.** `spawn.rs` shells out to the `claude`
  CLI. That was the right first cut; it is also an implementation detail wearing
  a contract's clothes. Which model or harness answers should be invisible to
  holden.
- **Nothing is observable.** A ruling in flight is a silent subprocess. The only
  evidence it ran is the artifact afterward. A service that gates every merge on
  three repos has health, and we cannot see it.
- **Residual laptop assumptions.** Defaults like `~/work/sprints` survived one
  filesystem relayout by luck. A service resolves its facts through config, once,
  loudly.

## Decision 1 — What "service" means here (the resident-judge reconciliation)

ADR-0001 explicitly struck resident judge processes: long-lived sessions rot,
and judgment stored in a transcript is judgment lost. That ruling stands.

holden-the-service holds **no judgment context**. The resident process is
dispatch and bookkeeping: it accepts ruling requests, assembles inputs (as
`assemble.rs` does today), spawns a *fresh* judgment per request through the
harness contract, validates the verdict against the ruling schema, writes the
ledger, posts the status, and emits lifecycle events. Every ruling remains a
fresh model instance. What stays resident is plumbing; what stays ephemeral is
judgment. A holden that has been up for a month rules exactly as a holden
started this morning.

## Decision 2 — Two contract surfaces, two owners

**Inbound — `holden.ruling.v1`, owned by holden.** The requestor-facing surface:

- `SubmitRuling(RulingRequest) returns (RulingHandle)` — repo, PR number,
  includes; returns a ruling id immediately.
- `GetRuling(RulingHandle) returns (Ruling)` — the ruling document, or its
  in-flight state.
- `WatchRulings(WatchRequest) returns (stream RulingEvent)` — holden emits;
  requestors and health-watchers observe instead of polling.

`Ruling` is the protobuf twin of the ADR-0001 YAML schema — `diff_ref`,
`judge_instance`, `fired_at`, verdict enum, divergences, `shape_verdict`,
consumer impact with file:line evidence, `doc_content_agreement` — one schema,
two serializations, the proto authoritative once this lands. `RulingEvent`
covers the lifecycle: RECEIVED, INPUTS_ASSEMBLED, JUDGE_SPAWNED, VERDICT,
REFUSED_RETRY, PUBLISHED, FAILED. Consumers vendor-generate, per mesh practice.
Bus emission (Kafka) is explicitly a later seam — kafka-svc owns bus contracts,
and wiring holden into the bus is a separate decision this ADR names rather
than smuggles.

**Outbound — `haho.harness.v1.HarnessService`, owned by haho.** holden consumes
a generated client of haho's existing contract (`Submit`, `StreamSubmit`,
`GetHealth`). Which harness answers — the `claude` CLI, mistral, mapesis, a
future haho with surrealdb caching and RAG — is haho's business and invisible
to holden. The current `spawn.rs` subprocess becomes the first implementation
*behind* that client interface, so nothing blocks on haho maturing.

holden never defines a model contract, and haho never learns what a ruling is.
That is the whole division of labor, and it is enforced by the wire.

## Decision 3 — Resources and health

Per fleet rules, holden gets `/health` and `/metrics`, structured JSON logging,
and a readiness check that means something:

| Check | Green means |
|-------|-------------|
| `harness_reachable` | haho `GetHealth` answers (or the subprocess shim reports its command on PATH) |
| `delightd_reachable` | the roster endpoint answers; holden can resolve consumers |
| `ledger_writable` | the rulings directory accepts a write |
| `publisher_ready` | GitHub API reachable for status posting; if not, holden serves but reports DEGRADED, loudly — rulings queue rather than vanish |

Config resolves once at startup, flag over env over default (the existing
`Config` boundary), with the stale `~/work/sprints` default corrected as part
of this work. holden carries no credentials; the harness owns model auth, the
`gh` layer owns GitHub auth, exactly as today.

## Decision 4 — Migration, ordered

1. Contracts first: `holden.ruling.v1` proto lands with this ADR ratified;
   generated code follows the gen-freshness invariant (ADR-0001 Decision 5).
2. Core refactor: the existing crate's assemble/rule/publish path is wrapped by
   the service; the CLI remains as a thin client of the same core (rehearsal
   and hostile-network modes keep working).
3. `spawn.rs` moves behind a `HarnessService` client trait; subprocess shim is
   implementation one.
4. haho graduates dev to prod. This is a **precondition for holden serving in
   production** — a prod service does not take a hard dependency on a dev
   repo's contract. Until then, holden-as-a-service runs against the shim.

## The e2e proof (Decision 8 compliance)

Submit a ruling for a real PR through `SubmitRuling`, watch the lifecycle on
`WatchRulings`, and observe the `ruling/ratify` status appear on the PR's head
SHA and the ruling row appear in the ledger. Requestor talks to holden, holden
talks to the harness, the gate comes out. If that flow cannot be demonstrated,
this design is not done, whatever the coverage number says.

## Carried, not solved here

- sprints issue 52: judge non-determinism (a required field dropped twice on a
  1686-line diff, clean on respawn). Service-ization makes retries and
  validation observable, which should make this bug diagnosable; it does not
  fix it by itself.
- Bus emission of ruling events (kafka-svc seam, later pilot).
- Where the ledger lives long-term. Today it stays in the sprints repo; a
  surrealdb-backed ledger is plausible once the RAG layer exists, and is a
  separate ADR.

## Consequences

**Gained.** Anything on the mesh can request and observe a ruling; the harness
is swappable without touching holden; the gate that guards three repos finally
has a health surface of its own; the laptop-path era ends.

**Paid.** Two contract surfaces to keep gen-fresh; a daemon to run and watch
where a CLI used to suffice; a dev-to-prod graduation (haho) now sits on
holden's critical path; the CLI-only escape hatch must keep working through the
transition, which is a real maintenance tax.
