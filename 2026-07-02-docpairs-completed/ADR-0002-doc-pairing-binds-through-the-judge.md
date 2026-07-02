# ADR 0002: Doc pairing binds through the judge

- Status: ACCEPTED (operator review, sprints PR 1, 2026-07-02)
- Deciders: Max (operator), Claude (agent — Fable 5)
- Supersedes the mechanical doc-pairing bullet of ADR-0001 Decision 5. Where a
  later ADR conflicts with an earlier one, the later ADR governs; this note is a
  courtesy, not a mechanism.

## Problem

ADR-0001 identified documentation drift and prescribed a mechanical check: a diff
touching a paired path must also touch the paired document. Built and put in
front of review, the check did not survive contact:

- It measures whether a document changed, not whether it is true. Appending a
  blank line satisfies it forever.
- It would not have caught either drift actually found this week. Both predated
  any paired change; the check fires only on future diffs, and accepts any edit.
- It required disclaimers to stay honest. The checker's own header explained
  that it verifies touching rather than truth — a check that must qualify itself
  this way is reporting its own design problem.
- Its mechanics added opacity: an undocumented CI conditional, a bespoke glob
  matcher, base-ref plumbing. The weight bought a proxy for the property, not
  the property.

The judgment gate, meanwhile, caught both real drifts by reading whole documents
against the code. The stronger tool already existed.

## Decision

1. The mechanical doc-pairing check is withdrawn: no checker binary, no CI job,
   no required status. The mechanical set is schema-breaking, gen-freshness, the
   coverage floor, and ruling-present.
2. `.docpairs` survives as a map: glob → document pairs at the repo root. It
   records which documents are implicated by which code paths — the one piece of
   knowledge that was genuinely missing.
3. The judge consumes the map. When a diff matches a glob, the paired document's
   content at head enters the judge's bundle as an *implicated document*: one the
   diff did not touch but may have falsified. The judge's standing instruction
   extends accordingly: a diff that falsifies an implicated document without
   updating it MUST be bounced.
4. No new enforcement is needed. `ruling/ratify` is already a required status on
   the pilot repos, so document truth already gates merges through the judge.

## Consequences

- Document conformance costs a judge invocation rather than a CI job. The
  invocation is already mandatory for every landing, so the marginal cost is
  prompt size — and the deterministic alternative checked the wrong property.
- The map stays small, per-repo, human-readable data, and follows the usual
  path: proven in delightd, then carried to other projects as they move to
  production.
- The harness wiring (map → implicated documents in the bundle) is deliberately
  not built with this ADR. The judge harness was built without operator review,
  documentation, or a maintenance path; correcting that ("bring the judge under
  its own law") is the next sprint, and this wiring belongs inside that work
  rather than ahead of it.

## Parked

If a proposed ADR is rejected at review, the file is not deleted — the
no-deletion rule stands. The expected disposition is a REJECTED status line with
the review as the record. Low stakes today; noted here so it is not re-derived
later.

## wonderlib (recorded at operator direction)

wonderlib was built for exactly this class of mechanical text analysis: git
churn statistics, rarity and density profiling — run-local, model-free, and
fast, born of an environment where a model cannot always be trusted to answer
and where reading large volumes through a model is expensive. The agent
deflected it twice this week on grounds ("no Python in the enforcement path")
that do not apply to analysis feeding a judgment: signals that inform the judge
gate nothing and cannot be gamed into a pass.

Maturity invested in wonderlib pays fleet-wide — every resident project uses
it, and adaptation work done here improves the library for all of them. When
implicated-document detection or diff-weight heuristics grow past trivial,
wonderlib (pinned to the published repo) is the designated candidate, evaluated
on its merits.
