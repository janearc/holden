# Sprint 1: Instruction surface (Phase 1 of fix-coding-process)

- Date cut: 2026-07-02  (a birth-name/label, NOT a liveness signal — this sprint is
  resolved only by the explicit `git mv ... -completed`, never by the calendar rolling)
- Predecessor: `2026-07-01-completed/SPRINT.md` (Sprint 0 — defines the gate model,
  the sprint process, and the Phase 0 findings; read it for the frame, it is not
  duplicated here)
- Stage: draft-diff

## Goal

Recompose what governs the agent into ONE authoritative, legible surface, with the
durable memory internally consistent and free of dead paths. This is Phase 1 (§5 of
Sprint 0): the hard prerequisite before any gate machinery.

## Scope (small on purpose — one phase)

Three things that all point the same direction (recompose the surface):

1. **Reconcile dead-path memory.** ~15 memory entries still cite `~/work/agents` /
   `agents/standards` / `AGENTS.md`. That dir is retired to
   `~/work/archaea/agents-compressed-to-avoid-ingestion.tbz` (archived 2026-07-01,
   deliberately compressed so it is out of the search path). Point refs at that reality.
2. **Recompose memory, no loss.** 95 entry files vs 79 index lines (~16 unindexed).
   Dedupe overlaps, index everything kept, delete only with a recorded reason. Method:
   a producer Haiku recomposes + reports; a fresh verifier Haiku checks the before-state
   for ANY lost fact; agreement = accept. Claude is the final gate; Max signs off before
   it goes live. Hard constraint: nothing is lost.
3. **Establish the single authoritative instructions file (§0a).** It does not exist yet
   — governance lives implicitly across the memory entries. Create a tight
   `~/.claude/CLAUDE.md` as the binding surface (working-in-prod discipline, contracts-
   first, the gate model pointing to this repo, register/tone), and demote memory to
   *supporting learned facts* that inform but do not compete.

## Definition of done

- Exactly one authoritative, auto-loaded instructions file exists and says so.
- Durable memory agrees with itself: index == kept files; zero dead-path refs.
- Recomposition verified loss-free (producer+verifier agree) AND accepted by Max.
- This doc promoted: `git mv 2026-07-02 2026-07-02-completed`, then chmod 400.

## Sizing / standup note

Deliberately ONE phase (Sprint 0 over-scoped 0->1->2 into a full day — see its §12 retro).
This first task tends to shake out more than expected; if it does, that is the estimation
signal, and it gets said out loud, not absorbed. Phase 2 (the gate ADR) is a LATER sprint.

## Outcome (2026-07-02, on resolution) — DONE, accepted by Max

All three scope items landed; the sizing held (roughly a half day including two review
rounds with Max). Verified mechanically, not asserted:

1. **Dead-path reconciliation: 14/14 files fixed** (the plan said ~15; ground truth by
   rg was 14). Zero refs now treat `~/work/agents` as live; every mention reads as
   archived-to-archaea. The producer/verifier Haiku pair worked as designed — the
   verifier caught 4 real gaps in the producer's plan (3 missed dead-paths, 7 files
   dropped from the proposed index, 2 miscounts), so the plan was NOT accepted;
   the recomposition was applied deterministically instead and verified by count.
2. **Memory recomposed, loss-free: index 95/95**, every ref resolves, zero unindexed,
   zero deletions/merges (the sprawl was index drift, not redundancy). One real memory
   CONFLICT caught and resolved with recorded supersession (public-repos' "omit the
   commit trailer" vs the newer denote-authorship "keep it" — newer wins).
3. **The single authoritative surface exists: `~/.claude/CLAUDE.md`** (constitution,
   v2 accepted by Max after a full markup round). Voice/standards reference re-homed to
   `~/.claude/reference/docs-voice/` as load-at-use material, mandated by the
   constitution for voice-heavy work. The retired agents/ tarball was excavated by a
   disposable agent (12 standards extracted, privates recorded path-only, full
   expansion deleted); triaged into constitution rules / loaded reference / retired
   mechanism.

Shaken out along the way (the predicted standup signal, handled not absorbed):
- The tarball was root-locked (Max's tripwire pattern); the excavation agent correctly
  BLOCKED rather than working around it; Max unlocked, agent completed, Max re-locks.
- Max's constitution markup produced two doctrine upgrades (wire-as-boundary-enforcer
  replacing per-language validation ceremony; design-doc-as-test-goalpost over coverage
  numbers), one new hazard rule (surprise transitive dependency pulls), and one hard
  privacy rule (the voice-calibration author's name never appears in any artifact).
- Fleet scan for the escaped name: exactly one public commit (obs-svc 70f3cf2).
  DECISION (Max): leave as immutable history; the hard rule prevents recurrence.
- taco doubt (built but unused, daemon not deployed) -> filed as taco#18, assigned.
- Net issue delta this sprint: +1 (taco#18). Ordinary-sprint ceiling formally starts
  next sprint; delta stated per the Sprint 0 §10 discovery-exemption pattern.

Carried forward: Phase 2 (gate architecture ADR) — the next sprint on the board.

Resolved per §4.4: `git mv 2026-07-02 2026-07-02-completed`, SPRINT.md chmod 400.
