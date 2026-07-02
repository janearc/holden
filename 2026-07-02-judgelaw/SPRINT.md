# Sprint 7: Bring the judge under its own law

- Date cut: 2026-07-02 (seventh sprint of the day; HEAVY — the subject is the
  process's own enforcement tooling, and the writer is a fresh agent)
- Contract: ADR-0001 (ACCEPTED) + ADR-0002 (ACCEPTED); scope carried verbatim from
  the sprint 6 outcome
- Stage: concept -> discussion (T0 below; no code until ratified)
- Session note: first sprint of the successor agent. The inbrief was the standing
  machinery (README, outcomes newest-first, ADRs, rulings) and it oriented cleanly;
  this doc is the first evidence of whether the handoff design works.

## Goal

The judge gates every merge on the pilot repos and is itself ungoverned: built
without operator review, documentation, or a maintenance path — named plainly in
the sprint 6 outcome as a governance hole. Close the hole without wedging the gate,
and land the one piece of judge capability ADR-0002 deferred to this sprint: the
`.docpairs` implicated-documents wiring.

Five items, carried from sprint 6:

1. Operator line-review of the existing Rust (`tools/judge/`, ~1,200 lines, five
   modules).
2. Documentation and a maintenance path for the judge.
3. The `.docpairs` -> implicated-documents wiring, through the full
   discussion -> pseudocode -> rough -> better cadence.
4. The rulings-automation lane for sprints hygiene (README open question).
5. The wonderlib evaluation on merits.

## T0 design — discussion level; pseudocode follows ratification

**Standing constraint.** The judge is live and REQUIRED on the pilot repos. It runs
locally, so a broken working copy stalls merges (fail closed — annoying, safe), but
the discipline holds anyway: all changes on a branch, `main`'s binary stays usable
until its replacement is proven.

**Phase A — line-review (item 1).** The operator reads the five modules as they
stand on `main` (`ruling.rs` 284, `assemble.rs` 348, `spawn.rs` 254, `publish.rs`
145, `main.rs` 150). The agent prepares an orientation brief first — what each
module does, where the risk lives, known seams — honest, not defensive; the code
was built fast and unreviewed, and the review exists to find what that cost.
Findings are disposed one at a time: fixed in-sprint or filed as issues assigned
to janearc. Review precedes everything else on purpose: documentation written
before review would document unratified behavior.

**Phase B — documentation and maintenance path (item 2).**
`tools/judge/README.md`: what the judge is (ADR-0001 D6, amended by ADR-0002),
invocation, the bundle (what a judge receives and why each input is required),
the ruling schema and the invalid-equals-absent rule, statuses, the overrule flag,
failure modes, and how to change the tool. Written for the 03:20 reader.

Maintenance path, proposed: judge changes land branch -> PR -> operator
line-review, like everything else here — writer-is-not-judge holds because the
operator is the judge's judge; the tool never rules on its own diffs. Mechanical
floor: a small CI on this repo (cargo fmt --check, clippy -D warnings, cargo test)
flipped REQUIRED on `main` once green. This repo currently has no CI at all; the
process tooling should not be the one unguarded artifact in the fleet.

**Phase C — rulings durability lane (item 4).** The README names the tension: the
harness writes rulings at judgment time, the merge they gate cannot wait on a
sprints PR, and nothing lands on `main` unreviewed. Sprint 6's rulings sat
uncommitted until the close PR — which also fails the push-after-every-landed-
commit durability rule.

Proposed design: **the sprint branch is the lane.** A sprint runs on its own
branch from cut to close (this sprint already does). The harness, after writing a
ruling, commits exactly that path (explicit staging — the `git add -A` reflex is
retired) and pushes the sprint branch. Durability is immediate; the close PR
reviews everything, rulings included, where review is of provenance rather than
prose — the serde schema already gates content shape. No carve-out, no bot lane,
no weakening: `main` still takes nothing unreviewed. Sequenced before Phase D so
the wiring proof's own rulings ride the new lane.

**Phase D — `.docpairs` wiring (item 3), the full cadence.** Discussion here;
pseudocode as a ratified artifact after T0; rough; better. Design intent, per
ADR-0002 §3: `assemble` reads `.docpairs` at the target repo's head; every changed
path is matched against the globs; each implicated document's head content enters
the bundle tagged as an implicated document — one the diff did not touch but may
have falsified. The judge's standing instruction extends: a diff that falsifies an
implicated document without updating it MUST be bounced. Edges named now: a paired
doc the diff DID touch is an ordinary changed file, not implicated; a missing
`.docpairs` is a no-op; a glob whose paired doc is absent at head is reported
loudly in the bundle, and the judge weighs it.

Named e2e proof (ADR-0001 D8): a real delightd PR touching `pkg/httpapi/` without
touching `docs/api.md`, judged by the wired harness, produces a ruling that cites
`docs/api.md` as implicated — bounce or divergence according to whether the doc
is actually falsified. A throwaway falsifying PR (sprint 6's planned bounce proof,
reborn at the judgment layer) is acceptable as the vehicle if no ordinary diff
arrives first.

**Item 5 — wonderlib, on merits, inside Phase D.** ADR-0002 designates wonderlib
as the candidate when implicated-document detection or diff-weight heuristics grow
past trivial. The evaluation is written into the wiring design at pseudocode time:
what the matching actually requires today (three globs, prefix semantics), where
the trivial/non-trivial line sits, and what signal-generation work (churn, rarity,
diff-weight as bundle inputs) would clear it. The conclusion is recorded either
way — an honest "not yet, and here is the threshold" is a valid outcome; a
reflexive deflection is not.

## Process discipline

- Heavy sprint: standup records in `2026-07-02-judgelaw/standups/` at phase
  boundaries; stage enum tracked in this header; divergence is drift, hard stop.
- Everything lands via PR on this repo, this doc included.
- One phase in flight at a time; ordered small diffs.

## Definition of done

- Line-review held; every finding disposed (fixed and cited, or filed assigned to
  janearc).
- `tools/judge/README.md` landed through review; CI live and REQUIRED on this
  repo's `main`, read-back verified.
- Rulings lane ratified and implemented; at least one real ruling committed and
  pushed by the harness at write time.
- Wiring landed through discussion -> pseudocode -> rough -> better; the named
  proof observed in a real ruling.
- wonderlib evaluation recorded.
- Standups at boundaries crossed; outcome recorded; sprint closes at least as many
  issues as it opens; promoted via `git mv` + chmod 400.
