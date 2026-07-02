# sprints

One directory per sprint. A sprint is one coding session (~4-12h) driven by exactly one
document; this repo is that document's home and the durable record of what happened.

## Layout

```
sprints/
  YYYY-MM-DD/            # one sprint per day (preferred)
    SPRINT.md            # the doc that drives the sprint
    <artifacts>          # e.g. phase0-issues.md, standup records
```

- **One sprint per day** is the default; the directory is the ISO date (`2026-07-01`).
- If a day needs more than one, suffix later sprints with a short discriminator —
  `-pm`, or a descriptive tag like `-pilot` when that reads better. The suffix is a
  label, not a schedule; resolution is always the explicit git mv, never the clock.

## Sprint weight

Some sprints are **heavy**: first-code work, many moving parts, or anything where
drift is cheap to create and expensive to unwind. A sprint doc declares itself heavy
at cut time, and a heavy sprint runs the full discipline, not the abbreviated one:

- T0 design section written INTO the sprint doc and ratified before code;
- real standup records (YAML, in `<sprint>/standups/`) at every phase boundary;
- the stage enum tracked in the doc header; divergence = drift = hard stop.

Light sprints (docs, config, single-decision work) may skip the standup ceremony but
never the outcome record. When in doubt, call it heavy.

## Durability

This repo is private on GitHub — private, not laptop-only. Push after every landed
commit; a laptop-in-canal or stolen-machine event must cost zero work.

## Lifecycle

- While a sprint runs, `SPRINT.md` is the single source of truth. It is edited in place;
  there is no parallel copy anywhere else (a second copy is the drift disease the process
  exists to kill).
- On completion: `git mv YYYY-MM-DD YYYY-MM-DD-completed`, then `chmod 400` the SPRINT.md.
  Once it is the record, it is immutable.

The process itself is defined in the first sprint here (`2026-07-01/SPRINT.md`).

## Agent context and handoff

The working agent names its context usage when asked and unprompted at thresholds.
At **65% context**, handoff planning STARTS — not at 80%, where quality visibly
degrades: discussing the handoff itself consumes handoff budget, so the buffer is
part of the design. The inbrief for a successor is the standing machinery — the
active SPRINT.md, its standup records, and the rulings — never a hand-written
transcript summary.

## Git hygiene (this repo is the offline state repository)

This repo is the process's durable memory — unsummarized, canonical, and load-
bearing for agent handoffs. Nothing lands on `main` unreviewed:

- ALL changes go branch -> PR -> operator review -> land. Sprint docs, ADRs,
  standups, rulings, tooling — no exceptions by default.
- ADRs are reviewed AS PULL REQUESTS (a real diff, not pasted markdown) and
  become immutable (chmod 400) when their sprint promotes.
- Open question for the judge-governance sprint: rulings are written by the
  harness at judgment time and the merge they gate cannot wait on a sprints PR
  — the automation seam (auto-branch? post-hoc batch PR? a bot lane?) is that
  sprint's to design, not an excuse to pre-weaken this rule.
