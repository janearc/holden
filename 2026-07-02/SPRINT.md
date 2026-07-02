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
