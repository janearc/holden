# ADR 0001: Enforcement gates for the coding process

- Status: DRAFT (pending markup) → Accepted on operator sign-off
- Date: 2026-07-02
- Deciders: Max (operator), Claude (agent)
- Evidence base: sprints `2026-07-01-completed/` (process definition + the Phase 0
  inventory, run against live repos and the GitHub API, not READMEs)

## Summary

The agent that writes code no longer decides when code is done. "Done" becomes
external: a set of deterministic mechanical gates that must be green, plus a ruling
from a judge agent that is never the writer, plus operator sign-off to land. The first
concrete action is not building new gates — it is making the checks that already exist
unbypassable, because today nothing is. The pilot proves the system on the one seam
that is live in code: service registration between magpie and delightd.

## Context

Two facts drive this design, one behavioral and one measured.

**Behavioral.** A code-writing agent grades its own work and always returns a passing
grade. Observed failure modes, all recurring: done-is-self-assessed ("I emit the event,
done" with no check against the design doc); design docs never corrected as commits
stack; locally-clean code of the wrong shape for the mesh; hand-written code where
generated code was mandated; review load collapsing onto a human at end-of-day, which
produces tired, superficial passes over large diffs. Instructing the writer to be more
careful does not fix this; the writer's self-assessment is structurally worthless
regardless of quality, because it is not independent.

**Measured (Phase 0, 2026-07-01).** Every roster repo has CI that triggers on
`pull_request` — and none of it gates anything. Branch protection returned 404 on
magpie, delightd, and big-little-mesh: `main` is directly pushable and a red PR merges.
Local hooks are a mixed bag (delightd's pre-push coverage hook is active; magpie's
pre-commit config is dead — never installed, binary off PATH) and are advisory anyway:
`--no-verify` walks past any hook. Meanwhile real gate machinery already exists in
places: big-little-mesh runs a `gen-drift` CI job (fails if committed generated code
differs from a fresh `buf generate`), and delightd's CI regenerates from proto and
builds with the race detector. Schema tooling is `buf` v2, local plugins, no schema
registry in the build path; the runtime Confluent Schema Registry lives in kafka-svc
with the broker.

The conclusion Phase 0 forces: the problem is not missing gates, it is that nothing
existing is *required*. Enforcement is mostly configuration, not construction.

## Decision 1 — Two kinds of gates, never blurred

**Mechanical gates** are deterministic, contain no model, and fire in CI or hooks. They
are authored once and can be read end-to-end. They do not get tired, and they do not
extend good faith.

**Judgment gates** are model-run, and the model is NEVER the instance that wrote the
diff. A judgment gate is spawned fresh per invocation, is single-purpose, and receives
its inputs as explicit required artifacts — it never relies on having "been there"
during the work.

The principle both share: no reasoner — human or model — is required to be smart in the
moment. The mechanical gate needs no intelligence; the judge needs intelligence but no
memory, history, or vigilance. Hostility lives in the *actions* (a failing check, a
bounce), not in exhortations to be careful.

Anything that blurs the two is a defect. A "mechanical" check that shells out to a
model is a judgment gate wearing a costume and gets reclassified. A judge asked to
also verify a checksum is doing a machine's job.

## Decision 2 — Four timepoints, T0–T3

| Time | Gate | Kind | Substrate |
|------|------|------|-----------|
| T0 design-time | Is the design/ADR/schema coherent and mesh-shaped, before code exists? | Judgment + operator | Session structure: design doc reviewed and signed off before implementation starts |
| T1 pre-write | Generate or hand-write? Anything enum- or state-shaped defaults to generated; hand-writing requires a stated justification | Procedural (writer answers it in the diff description; judge checks the answer at T3) | Diff description |
| T2 pre-land | Mechanical set (Decision 5) | Mechanical | CI, as REQUIRED status checks |
| T3 pre-land | Does the diff satisfy the design doc; is it on-mesh; consumer impact; doc content agreement | Judgment | Fresh judge, ruling recorded (Decision 6) |

T0 has no git substrate (there is no diff yet); it is enforced by the sprint process:
no implementation task enters a sprint without its design artifact. T1 is deliberately
procedural for now — automating a classifier is not worth its weight until the pilot
shows where hand-writing actually leaks in.

## Decision 3 — "Done" is external

A change is done when, and only when:

1. every mechanical gate is green (T2, required checks — not advisory),
2. a judge ruling with `verdict: ratify` exists for the head SHA (T3), and
3. the operator has signed off per the PR workflow.

The writer's own assessment appears nowhere in that list. This is the load-bearing
decision; everything else in this ADR is mechanism for it.

## Decision 4 — Enforcement before construction

The first concrete action is to make existing checks required: branch protection on
big-little-mesh, delightd, and magpie, with required status checks naming the CI jobs
that already run (big-little-mesh: `gen-drift` + build/test jobs; delightd: the `go`
regenerate/vet/race job; magpie: the python lint/test job). Only after that is any new
gate built — starting with the gaps Phase 0 named (magpie has no gen check at all).

Rationale: a gate that can be bypassed is a suggestion. Flipping existing checks to
required is hours of configuration; building new machinery while `main` remains
directly pushable would be decorating an open door.

Admin bypass: on a solo repo the admin can always force past protection. We enable
`enforce_admins` and treat any bypass as a break-glass event that MUST be recorded in
the sprint record (what, why, and what would have made it unnecessary). The bypass
capability is not pretended away; it is made loud.

## Decision 5 — The mechanical gate set (target state)

Per repo, as required checks:

- **Schema-breaking.** `buf breaking` against the base branch wherever a `buf.yaml`
  exists (big-little-mesh, delightd, kafka-svc). Catches the sanctioned-evolution
  class: the new enum value that parses fine downstream and falls through a consumer's
  default arm.
- **Gen-freshness.** The invariant: what the contract generates and what the repo
  ships are the same thing, for every target language, and it compiles. Two sanctioned
  postures satisfy it (Phase 0 found both in production, and both are sound):
  - *Posture A (big-little-mesh):* generated code is committed; CI regenerates and
    fails on drift. Required where consumers vendor or `go get` the generated code.
  - *Posture B (delightd):* generated code is gitignored; CI regenerates from proto and
    builds/tests every run. Acceptable where nothing external consumes the repo's
    generated code directly.
  Each repo declares its posture; magpie currently has neither and gets Posture B (it
  consumes frood's generated clients as a dependency; it ships none of its own).
  This resolves big-little-mesh issue 75 by sanctioning both variants with the
  invariant stated, rather than forcing one.
- **Doc-pairing (mechanical half).** A diff touching paths that have a declared paired
  doc fails unless the doc is touched in the same PR. Pairing is declared per repo in a
  committed map (`.docpairs`: glob → doc path); the check is pure path logic, no model.
  It proves the doc was touched, not that it is true — that second question is the
  judge's (T3) and stays advisory until it has a track record.
- **Coverage floor.** The existing 80% floor stays where wired — as a floor. Green
  coverage is not "tested" (see Decision 8).
- **Ruling-present.** A deterministic check that a judge ruling file exists for the
  head SHA with `verdict: ratify` (Decision 6). This is how a judgment gate becomes
  mechanically enforceable without putting a model in CI: the model produces an
  artifact; the machine verifies the artifact exists. During the pilot this check is
  configured on the pilot repos only.

## Decision 6 — The judge

**Invocation.** Fresh instance per ruling. Scope: the one repo (or the named seam)
under judgment. The judge receives, as required inputs: the diff, the design doc, the
contract(s) touched, the current ruling ledger, and the list of consumers of any
changed message type. It is never fed the writer's session context.

**Output.** No free-text verdicts. The ruling is a fixed-schema YAML document:

```yaml
ruling:
  diff_ref: <commit-sha-or-PR-url>
  judge_instance: <ephemeral-id, never reused>
  fired_at: <timestamp>
  verdict: ratify | bounce | needs-clarification      # enum
  divergences:
    - claim: <what diverged from the design doc>
      necessary: true | false
      justification: <why, with evidence>
  shape_verdict: on-mesh | wrong-shape                 # enum
  shape_justification: <text>
  consumer_impact:
    - consumer: <path>
      classification: additive | breaking | silent-drift
      evidence: <file:line — a citation, not an assertion>
  doc_content_agreement: agree | disagree | unclear    # advisory
  ledger_entry_id: <assigned on write>
```

Enum verdicts and file:line evidence are mandatory; a ruling without them is invalid
and the ruling-present check treats it as absent.

**The ledger.** The ruling IS the ledger entry — no separate write. Rulings are
committed in the repo whose contract they judge, under `docs/rulings/`, named by date
and diff ref. Rationale: the judge's only persistent memory is this ledger (fresh
instances + durable artifacts is the whole design — a resident judge was considered
and struck: long-lived sessions rot); keeping it in-repo makes it versioned, diffable,
and adjacent to the contract it interprets. A central cross-repo ledger was rejected:
it recreates the coordination-repo problem and detaches rulings from the code they
bind. For a cross-repo seam, rulings live with the contract owner (see Decision 7's
prerequisite).

**Authority during pilot.** The judge's ruling is required to land (via the
ruling-present check) but the judge itself is new and unproven; the operator reviews
bounces and can overrule with a recorded ratification (which becomes a ledger entry —
an overrule is data, not a shrug). If the judge's bounce quality is good after the
pilot, overrules should become rare; if it cries wolf, the calibration problem is
worked before its authority expands. The advisory field (`doc_content_agreement`)
earns blocking status only on track record, if ever.

## Decision 7 — Pilot scope: the registration seam

The pilot proves the full loop on the coupling that is live in code today (verified in
Phase 0, not from READMEs): magpie registers with delightd via the registration
contract (`registry.v1` / `frood.v1`), using generated clients. The Kafka/bus layer is
explicitly OUT of pilot scope — those contracts are owned by kafka-svc and can be a
later pilot of the same pattern; nothing in this ADR narrows to "the bus" or widens to
"the mesh."

**Prerequisite (pinned, with owner).** `registry.v1` proto sources currently exist in
both big-little-mesh and delightd, and `observability.v1` has the same floating-owner
problem (big-little-mesh issue 77). Before the pilot's schema-breaking gate can bind,
the pilot MUST pin single ownership of `registry.v1`/`frood.v1` and record it — the
gate needs one source of truth to diff against. Owner: operator decision, recorded as
a ledger-style note in the owning repo. This is a decision the pilot forces early, on
purpose; it is exactly the class of ambiguity the gates exist to make impossible.

**Success criteria — the pilot is done when all four have happened, not before:**

1. Branch protection with required checks is live on big-little-mesh, delightd, and
   magpie (Decision 4), `enforce_admins` on.
2. A deliberately bad diff (a schema break, or hand-edited generated code) is BOUNCED
   by a required mechanical check — observed as a real failing, merge-blocking status.
3. A real diff receives a real judge ruling, committed to the ledger, and the
   ruling-present check gates on it.
4. An end-to-end proof exists and runs: magpie registers with delightd in a test
   harness and the registration is observable on the delightd side ("service A talks
   to service B and the product comes out") — the e2e bar from Decision 8 applied to
   the seam.

## Decision 8 — The test goalpost is the design doc

Coverage percentage is a floor, not a goal, and it measures the wrong thing when used
as one: a repo can be 80% covered and never once demonstrate the behavior its design
doc promises. The standing question for every unit of work is: **with the software
actually committed, can we test what the design doc says this thing does — and if not,
why not, and why is anything else being worked on?**

Concretely:

- Every design doc names its e2e proof: the observable, cross-service behavior that
  demonstrates the design works ("A talks to B across the mesh and product X is
  generated"). If a design doc cannot name one, that is a T0 bounce — the design is
  not testable as stated.
- The judge's T3 conformance question is this same question applied to a diff: does
  the committed software still support testing what the design doc claims?
- Unit tests and the coverage floor remain — as hygiene, not as the definition of
  tested.

## Decision 9 — Full eventual scope, stated so it cannot be quietly narrowed

The mesh is an emergent state machine of generated FSMs; the contracts are the wire,
and the wire is the boundary-enforcer. The judgment gate's eventual jurisdiction is
shape-conformance across that whole mesh — every seam, not just the pilot seam. This
ADR deliberately builds the smallest complete instance of it (one seam, full loop) and
explicitly records that the pattern is meant to propagate seam-by-seam: bus contracts
(kafka-svc ↔ producers/consumers) next, then the remaining roster as each repo's
contract surface stabilizes. Narrowing the system to "the pilot repos, forever" would
be a failure of this ADR even with every pilot criterion met.

## Consequences

**Gained.** The writer cannot declare victory; the tired end-of-day human pass stops
being the last line of defense; doc drift becomes mechanically visible; enforcement
starts as configuration (required checks) rather than new machinery; every landing
leaves a durable, greppable record (rulings) of what was allowed and why.

**Paid.** Friction on every land — by design, and it will be felt; a new artifact type
(rulings) accumulates in repos; the judge costs one fresh model invocation per landing;
admin bypass remains physically possible and is handled by making it loud rather than
pretending it is impossible; the pilot forces an ownership decision (registry.v1) that
was comfortable to leave ambiguous.

**Trust bootstrapping.** Early on, the operator double-checks the gates — more work
than today, temporarily. The gates earn autonomy by their record: bounces that were
right, ratifications that held up. The payoff — the operator stops reading 4,000-line
diffs as a safety mechanism — arrives only after that record exists, and this is
accepted explicitly rather than promised for day one.

## Out of scope

Publication venue for this ADR (tone is held to publicly-readable; venue is a later,
separate question). Bus-layer gates (later pilot). Research-repo work (`~/work/
research` keeps its yolo-to-main discipline; gates bind production repos). The T1
classifier as automation. Resident/persistent judge processes (struck in Sprint 0 §8;
fresh-per-invocation + durable ledger is the design). Rewriting history anywhere.

## Alternatives considered and rejected

- **A resident judge/nanny process.** Long-lived sessions accumulate context and rot;
  judgment stored in a transcript is judgment lost. Fresh instances reading a durable
  ledger dissolve the fresh-vs-resident dilemma. (Decided in Sprint 0; recorded here
  because this ADR is where future readers will look.)
- **A schema registry (BSR or similar) in the build path.** Local `buf` + vendored
  generated code is working and matches the polyrepo design; the runtime Confluent SR
  in kafka-svc already governs the bus. A build-path registry adds a network
  dependency in a hostile-network environment for no current gain.
- **Coverage as the quality goalpost.** Measures execution, not conformance to design
  intent (Decision 8). Kept as a floor only.
- **One mandated gen-freshness posture.** Both observed postures are sound for their
  consumption patterns; mandating one would churn a working repo for symmetry's sake.
  The invariant is mandated instead (Decision 5).
- **A central rulings store.** Detaches rulings from the contracts they bind;
  recreates the monorepo-coordination problem the polyrepo design deliberately avoids.
- **Trusting hooks.** `--no-verify` exists. Hooks stay as the cheap fast layer; they
  are never the enforcement layer.
