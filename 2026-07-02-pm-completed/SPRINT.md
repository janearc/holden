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

## Outcome (2026-07-02, on resolution) — DONE, ADR-0001 ACCEPTED

The full-strength-draft bet paid: ONE markup round (three items: name the models in
the deciders line; rulings live in the sprint dir, never the target repo, with a
required `ruling/ratify` commit status as the on-GitHub trace; Goodhart's law adopted
as doctrine in Decision 8 and echoed into the constitution). Accepted same morning.

**Calibration note (the parking-lot comment a humans-only sprint would file):** we
named this sprint "-pm" expecting it to consume the afternoon; it was cut, drafted,
marked up, and accepted before 09:00. Complexity was over-estimated — partly because
the ADR was 90% transcription of doctrine already settled and evidenced in Sprints
0-1 (the real design work had already been paid for), and partly a model upgrade
mid-arc (drafting moved from Opus 4.8 to Fable 5 this morning; markup rounds needed
dropped accordingly). Sizing lesson banked for §4.2: a document whose decisions are
already made is a SMALL artifact regardless of length; size by open decisions, not by
page count. The day has ~9h of runway remaining; the pilot build starts now as its
own sprint rather than padding this one.

Resolved per §4.4: `git mv 2026-07-02-pm 2026-07-02-pm-completed`; SPRINT.md and the
accepted ADR both chmod 400 (the ADR is a record now; amendments come as new ADRs).
