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

## Outcome (2026-07-02, on resolution) — DONE, all three items

1. **Protection live and verified.** Branch protection with required status checks on
   big-little-mesh (gen-drift/go/python/rust), delightd (go/rust), magpie (python);
   `enforce_admins: true` on all three; `strict: false` for now (green required,
   up-to-date-branch not — avoids rebase churn on stacked diffs; tightenable). No
   GitHub-review requirement: janearc-authored PRs cannot be janearc-approved, so
   review stays in-conversation per the standing PR workflow. Verified by API
   read-back.
2. **Bounce proven.** blm PR 78: deliberate hand-edit to `gen/go/auth/v1/auth.pb.go`;
   gen-drift FAILED (22s), go FAILED (11s), `mergeStateStatus: BLOCKED`; closed
   unmerged, branch deleted. Yesterday's headline finding (404 on branch protection,
   nothing unbypassable) is dead; blm issue 74 closed with both halves of evidence
   (config read-back + observed bounce).
3. **Ownership pinned** (operator decision): delightd owns `registry.v1` (+
   `resolve.v1`); big-little-mesh owns `frood.v1` (+ substrate packages); each vendors
   the other's. `observability.v1` deliberately left OPEN pending blm issue 77 — the
   note declines to claim it so a docs PR cannot silently resolve a contested
   ownership question (that decision is the operator's, via its own `Fixes #77` PR).
   Landed as blm#79 (merge 7308102c) and delightd#64 (merge 67e3db81), through the new
   required checks, on operator sign-off.

Texture worth keeping:
- delightd's own pre-push hook bounced the first push of PR 64 — legitimately (fresh
  Posture-B checkout, no `gen/`); fixed by regenerating, NOT by `--no-verify`. The
  gates creating friction that gets resolved correctly is the system working.
- Environment: SSH :22 blocked at sprint start (VPN bounce healed it); the 1Password
  signing agent locked twice (commits fell back to `--no-gpg-sign` per standing rule;
  local main syncs done via one-off HTTPS fetch, remote config untouched throughout).
- Net issues this sprint: -1 (closed blm 74; opened none).

Pilot criteria scoreboard after this sprint: 1 DONE, 2 DONE, prerequisite DONE;
remaining for the pilot: criterion 3 (judge ruling + required `ruling/ratify` status)
and criterion 4 (magpie->delightd e2e registration proof) — the next sprint.

Resolved per §4.4: `git mv 2026-07-02-pilot 2026-07-02-pilot-completed`, chmod 400.
