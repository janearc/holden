# Sprint 3: Pilot, part 1 — enforcement live (ADR-0001 Decisions 4 + 7.1-7.2)

- Date cut: 2026-07-02 (third sprint of the day; suffix per README convention)
- Contract: `2026-07-02-pm-completed/ADR-0001-coding-process-gates.md` (ACCEPTED)
- Stage: draft-diff

## Goal

The first two pilot success criteria, live and observed:

1. **Branch protection with required status checks** on big-little-mesh, delightd,
   and magpie — the checks that already exist (big-little-mesh: gen-drift + build/test
   jobs; delightd: the go job; magpie: the python job), `enforce_admins` on.
2. **A deliberately bad diff is BOUNCED** by a required mechanical check — a real
   failing, merge-blocking status observed on a real PR (schema break or hand-edited
   generated code), then closed unmerged.

Plus the pilot prerequisite the ADR pins:

3. **Contract ownership recorded**: `registry.v1` and `frood.v1` single owners named
   by the operator, recorded in the owning repos' docs. Evidence for the
   recommendation is in the sprint notes below; the DECISION is Max's.

## Deliberately out of this sprint

The judge harness, the `ruling/ratify` status, and the e2e registration proof (pilot
criteria 3-4) are the NEXT sprint. Sizing lesson from Sprint 2 applies: size by open
decisions and moving parts, not ambition. This sprint's parts: GitHub configuration,
one throwaway PR per bounce proof, two docs notes.

## Workflow consequence, stated up front

Once protection is on with `enforce_admins`: NOBODY pushes those three repos' main
directly — including Max, including Claude. Everything goes through a PR with green
required checks. Any break-glass bypass MUST be recorded in the active sprint doc
(what, why, what would have made it unnecessary), per ADR Decision 4.

## Ownership evidence (for decision item 3)

De facto ownership is already visible in commit history:

- big-little-mesh `8863196`: "feat(frood): Python register-client + **vendor
  registry.v1 from delightd**" — big-little-mesh treats delightd as registry.v1's
  source.
- delightd `49b05d5`: "rename: **re-vendor frood.v1**, blm -> big-little-mesh" —
  delightd treats big-little-mesh as frood.v1's source.

Recommendation (Max decides): record the de facto state — **delightd owns
registry.v1; big-little-mesh owns frood.v1**; the other side vendors. No code moves;
the pin is documentation + which repo's CI runs `buf breaking` as the authoritative
check for each package.

## Definition of done

- Protection live on all three repos, verified by API read-back (not assumed).
- One bounce proof per gate class exercised (at minimum: gen-drift bounce on
  big-little-mesh), PR visibly blocked, then closed unmerged.
- Ownership notes committed to delightd and big-little-mesh docs after Max's call.
- Outcome recorded; promoted via `git mv 2026-07-02-pilot 2026-07-02-pilot-completed`
  + chmod 400.
