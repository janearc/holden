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

## Lifecycle

- While a sprint runs, `SPRINT.md` is the single source of truth. It is edited in place;
  there is no parallel copy anywhere else (a second copy is the drift disease the process
  exists to kill).
- On completion: `git mv YYYY-MM-DD YYYY-MM-DD-completed`, then `chmod 400` the SPRINT.md.
  Once it is the record, it is immutable.

The process itself is defined in the first sprint here (`2026-07-01/SPRINT.md`).
