# Sprint 2: The gate architecture ADR (Phase 2 of fix-coding-process)

- Date cut: 2026-07-02 (second sprint of the day, per the README suffix convention)
- Predecessors: `2026-07-01-completed/SPRINT.md` (process + Phase 0 evidence),
  `2026-07-02-completed/SPRINT.md` (instruction surface)
- Stage: draft-diff

## Goal

One artifact: `ADR-0001-coding-process-gates.md`, in this directory. It is the
contract for the gate system — the mechanical/judgment split, the T0-T3 timepoints,
"done is external", enforcement-before-construction, the pilot scope, and the judge's
ruling schema + ledger — written to the bar of being publicly readable.

## Scope discipline

- The ADR is committed HERE as a sprint artifact. Where it might eventually be
  published is explicitly OUT of scope — "can be public" is a tone-and-content bar,
  not a venue decision.
- The ADR DECIDES; it does not defer. Every section states a decision, its rationale,
  and the alternatives rejected. Open questions are named as pilot pre-requisites with
  owners, not left as shrugs.
- Building/enabling anything (branch protection flips, CI jobs, the judge harness) is
  the NEXT sprint. This sprint produces the contract those builds are wired against.

## Definition of done

- ADR drafted at full strength (one muscular draft, minimal markup rounds by design).
- Max's markup applied; ADR marked Accepted.
- Committed in this directory; sprint promoted per the standard mechanic
  (`git mv <dir> <dir>-completed`, chmod 400).
