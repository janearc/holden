# Sprint 6: The doc-pairing gate, v1 (ADR-0001 D5's last unbuilt gate)

- Date cut: 2026-07-02 (sixth sprint of the day; HEAVY by the when-in-doubt rule)
- Contract: `2026-07-02-pm-completed/ADR-0001-coding-process-gates.md` (ACCEPTED)
- Stage: concept -> pseudocode (T0 below; code after ratify)
- Session note: last sprint before the planned agent handoff.

## Goal

The mechanical half of doc-pairing, live and REQUIRED on delightd, with delightd
issue 66 as the proof case: fix the api.md drift the judge found, pair the files so
that class of drift fails mechanically forever, prove a bounce, flip it required.
`Fixes #66`; the sprint closes more than it opens.

## Standing observation duty (operator brief, this week)

delightd is the fleet's most mature repo and still fails the whole-picture test:
clean, gofmt'd, "appears mature" — yet emotionally empty, lacking the refinement of
code that lived in production. This code represents the operator professionally.
While building in delightd this sprint: COLLECT whole-picture/maturity observations
as standup notes for the week's conversation about how the coding system and the
code-as-body-of-work interact. Notes, never unrequested refactor diffs.

## T0 design — the gate

**`.docpairs` (repo root, committed, plain text).** One pairing per line:
`<glob> -> <doc path>`; `#` comments. A diff that touches a path matching a glob
MUST also touch the paired doc, or the check fails. Pure path logic, no model —
it proves the doc was TOUCHED, not that it is true (truth is the judge's
whole-document question, already live).

**Initial map (small and obviously true; expand later):**
```
pkg/httpapi/**  -> docs/api.md
Taskfile.yml    -> docs/operations.md
config/**       -> docs/operations.md
```
(The Taskfile line is the operator's logged review item from sprint 5, made
mechanical at the touched level; description CONTENT stays with the judge.)

**The checker: `tools/docpairs`, a small Go program in delightd** (house language;
testable; copyable to other repos later — promotion to a shared home is a later
decision, not smuggled). Reads `.docpairs`, diffs base...HEAD names, exits nonzero
with a plain message naming each unpaired change. CI job `docpairs` runs it on
pull_request; joins the required contexts after the bounce proof.

**Order of operations (each PR through the full gate stack, judge included):**
1. PR A: fix docs/api.md (add the missing /register + /registrations routes) +
   add `.docpairs` + `tools/docpairs` + the CI job (non-required). `Fixes #66`.
2. Bounce proof: a throwaway PR touching pkg/httpapi without api.md — watch the
   docpairs check FAIL — close unmerged.
3. Flip `docpairs` into delightd's required contexts; read-back verified.

## Definition of done

- delightd 66 closed by a landed PR; `.docpairs` + checker + CI live.
- One observed docpairs bounce, closed unmerged.
- `docpairs` REQUIRED on delightd main; read-back verified.
- Maturity observations (>= whatever honestly surfaced) in standup notes.
- Outcome recorded; promoted; then the session handoff.
