# ADR 0002: Doc pairing binds through the judge, not a mechanical touch check

- Status: ACCEPTED (operator review of delightd PR 68, 2026-07-02)
- Deciders: Max (operator), Claude (agent — Fable 5)
- Supersedes: the "Doc-pairing gate (mechanical half only)" bullet of ADR-0001
  Decision 5. ADR-0001 is otherwise unchanged.

## The problem, restated

ADR-0001 named the disease correctly — documentation drifts from code — and
prescribed a mechanizable proxy: a diff touching a paired path must touch the
paired doc. Built and reviewed, the proxy failed on inspection, and the operator's
review comments are the record:

- It measures TOUCH, not TRUTH. `echo >> docs/api.md` satisfies the pairing
  forever — a gate that invites the gaming it exists to prevent.
- It could not have caught either real drift found this week. Both api.md
  staleness findings PRE-DATED any paired change; the touch gate only fires on
  future diffs, and then accepts any touch.
- It had to disclaim itself to be honest. The checker's own header explained
  that it proves touching, never truth — "we are building a car without
  propulsion, but the test said 'there must be a car'." A gate that must
  narrate its own weakness is confessing the design.
- Its mechanics leaked inscrutability outward: an undocumented CI conditional,
  a bespoke glob matcher, base-ref plumbing — machinery whose weight bought a
  proxy, not the property.

Meanwhile the judgment gate caught both real drifts — twice, at increasing
depth — because it judges the whole post-image document against the code.
The strong tool already existed; the weak one duplicated it badly.

## Decision

1. **The mechanical doc-pairing gate is withdrawn.** No checker binary, no CI
   job, no required `docpairs` status. The T2 mechanical set is: schema-breaking,
   gen-freshness, coverage floor, ruling-present.
2. **`.docpairs` survives as a MAP, not a gate.** Committed at repo root, glob →
   doc pairs, exactly as designed — it encodes the one thing that was genuinely
   missing: which documents are IMPLICATED by which code paths.
3. **The judge consumes the map.** The harness reads the target repo's
   `.docpairs`; when a diff matches a glob, the paired document's head content
   enters the judge's bundle as an IMPLICATED DOC — a document the diff did not
   touch but may have falsified. The standing instruction extends accordingly:
   *an implicated document that the diff falsifies without updating is a
   bounce.* Truth is judged where truth can be judged; nothing counts touches.
4. **Enforcement is already real.** `ruling/ratify` is a required status on the
   pilot repos; doc truth therefore gates merges today, through the judge,
   with no new machinery.

## Consequences

- Doc conformance costs a judge invocation instead of 14 CI seconds. Accepted:
  the deterministic version was deterministically wrong, and the invocation is
  already mandatory per landing.
- The pairing map stays cheap, legible, per-repo data — copyable across the
  fleet the way delightd proves things out.
- The harness wiring (map → implicated docs in the bundle) is NOT built at the
  time of this ADR. It is the successor sprint's work, deliberately: the judge
  harness itself was built without operator review, documentation, or a
  maintenance path — a governance hole this week exposed — and it takes no new
  unreviewed surgery. "Bring the judge under its own law" precedes and includes
  this wiring.

## Future consideration, recorded at operator direction

wonderlib exists for mechanical text analysis (git churn statistics, rarity and
density profiling) and was offered for this class of problem earlier in the
week; the agent repeatedly deflected it on categorical grounds ("no Python in
the enforcement path") that do not apply to ANALYSIS feeding the judge.
Analysis is not enforcement: signals that inform a judgment gate no one can
game into a pass. When implicated-doc detection or diff-weight heuristics grow
past trivial, wonderlib (pinned to the published repo, per the standing rule)
is the designated candidate — evaluated on its merits, not on the agent's
carried bias.
