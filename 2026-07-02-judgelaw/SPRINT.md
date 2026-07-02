# Sprint 7: Bring the judge under its own law

- Date cut: 2026-07-02 (seventh sprint of the day; HEAVY — the subject is the
  process's own enforcement tooling, and the writer is a fresh agent)
- Contract: ADR-0001 (ACCEPTED) + ADR-0002 (ACCEPTED); scope carried from the
  sprint 6 outcome, then cut at T0 review (see "Cut at T0 review" below)
- Stage: concept -> discussion (T0 below; no code until ratified)
- Session note: first sprint of the successor agent. The inbrief was the standing
  machinery (README, outcomes newest-first, ADRs, rulings) and it oriented cleanly;
  this doc is the first evidence of whether the handoff design works.

## Goal

The judge gates every merge on the pilot repos and is itself ungoverned: built
without operator review, documentation, or a maintenance path — named plainly in
the sprint 6 outcome as a governance hole. This sprint closes the governance hole
on the EXISTING artifact:

1. Operator line-review of the existing Rust (`tools/judge/`, ~1,200 lines, five
   modules).
2. Documentation and a maintenance path for the judge.

## Cut at T0 review (operator estimation call, PR 3)

Sprint 6's outcome carried five items. T0 review sized the full set at two
separate software surfaces (judge Rust + a delightd proof PR) with likely 2kloc+
of review — too much for one sprint, on top of a 1,200-line line-review that is
itself a full operator task. Conservative chosen today. Carried to the next
sprint, not dropped:

- The rulings durability lane (the README's open question; proposal drafted at
  this sprint's first T0 — the sprint branch is the lane, harness commits and
  pushes each ruling at write time — held for that sprint's ratification).
- The `.docpairs` -> implicated-documents wiring, full
  discussion -> pseudocode -> rough -> better cadence, with its named e2e proof.
- The wonderlib evaluation on merits (it lives inside the wiring design; it moves
  with it).

## T0 design — discussion level

**Standing constraint.** The judge is live and REQUIRED on the pilot repos. It
runs locally, so a broken working copy stalls merges (fail closed — annoying,
safe), but the discipline holds anyway: all changes on a branch; `main`'s binary
stays usable throughout. This sprint plans no behavior changes to the judge at
all — only review findings, if any are ratified as fixes.

**Phase A — line-review (item 1).** The operator reads the five modules as they
stand on `main` (`ruling.rs` 284, `assemble.rs` 348, `spawn.rs` 254, `publish.rs`
145, `main.rs` 150). The agent prepares an orientation brief first — what each
module does, where the risk lives, known seams — honest, not defensive; the code
was built fast and unreviewed, and the review exists to find what that cost.
Findings are disposed one at a time: fixed in-sprint or filed as issues assigned
to janearc. Review precedes documentation on purpose: documentation written
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
process tooling should not be the one unguarded artifact in the fleet. The CI is
a single workflow file; if even that reads as too much for today, it is the named
first diff of the follow-up sprint instead.

## Process discipline

- Heavy sprint: standup records in `2026-07-02-judgelaw/standups/` at phase
  boundaries; stage enum tracked in this header; divergence is drift, hard stop.
- Sprints-repo artifacts (this doc, standups, the README, any judge fixes) land
  branch -> PR -> operator review here; a diff to any other repo goes through
  that repo's own gate stack. This sprint plans none.
- One phase in flight at a time; ordered small diffs.

## Definition of done

- Line-review held; every finding disposed (fixed and cited, or filed assigned to
  janearc).
- `tools/judge/README.md` landed through review.
- Maintenance path ratified and recorded; if the CI half is ratified: live and
  REQUIRED on this repo's `main`, read-back verified.
- Standups at boundaries crossed; outcome recorded; sprint closes at least as
  many issues as it opens; promoted via `git mv` + chmod 400.
