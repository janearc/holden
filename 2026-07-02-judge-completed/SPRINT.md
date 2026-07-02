# Sprint 4: Pilot, part 2 — the judge harness (ADR-0001 criterion 3)

- Date cut: 2026-07-02 (fourth sprint of the day; the heavy one — first real code)
- Contract: `2026-07-02-pm-completed/ADR-0001-coding-process-gates.md` (ACCEPTED)
- Stage: concept -> pseudocode (design section below is T0; no code until it is agreed)

## Goal

ADR pilot criterion 3, end to end: a real diff receives a real ruling from a fresh
judge, the ruling lands in this sprint's `rulings/` directory, a `ruling/ratify`
commit status appears on the head SHA, and that status becomes REQUIRED on the pilot
repos — in that order, proven at each step.

Explicitly NOT this sprint: the magpie->delightd e2e registration proof (criterion 4,
next sprint), and any wonderlib integration (wonderlib belongs to standup's advisory
enough-ness check, never to the enforcement path).

## T0 design — the judge harness

**Language: Rust.** Decision, not vibe: the harness's core is a validator — the
ruling schema is a serde type (`deny_unknown_fields`, real enums for
`verdict`/`shape_verdict`/`classification`), so the ADR's "invalid ruling = absent"
rule is enforced by deserialization, not by remembered checks. The enforcement layer
is where pedantic-by-construction is pure virtue; speed is irrelevant (the judge
invocation dwarfs all else). Recorded counterweight: fleet language policy says Go
for I/O-bound control-plane glue and Go would be ~25% less lift — overruled HERE
because that policy targets recovery-critical tooling (failure mode: dead host,
interpreter debugging); the harness fails CLOSED (merges stall — annoying, safe), a
risk profile where compile-time pedantry costs nothing we care about. Operator
concurred ("fast and pedantic").

**Home: `sprints/tools/judge/`** (this repo — private on GitHub, not laptop-only:
process tooling lives with the process records; a public repo would drag in
tone/hygiene overhead with zero consumers). Durability rule, stated because it
matters: this repo MUST be pushed to its private GitHub remote after every landed
commit — a laptop-in-canal or stolen-machine event must cost zero work. Private ≠
unpushed.

**Flow (one invocation = one ruling):**

1. `judge <repo-path> <pr-number>` — everything else is derived.
2. Assemble required inputs (ADR D6): the diff (`gh pr diff`), the design doc(s)
   (repo docs/ per its convention), the contract files touched, the ruling ledger
   (this sprint's + prior sprints' `rulings/`), and the consumer list for any changed
   message type (rg over the roster for the generated type names).
3. Spawn a FRESH judge: `claude -p` (headless), strongest available model,
   single-purpose prompt built from the inputs; the writer's session context is never
   included. The prompt demands the ruling YAML and nothing else.
4. Parse + validate the output against the serde schema. Invalid or missing
   file:line evidence -> ONE retry with the validation error appended; still invalid
   -> the ruling is ABSENT (no status posted, exit nonzero, loud).
5. Write the ruling to `sprints/<active-sprint>/rulings/<date>-<repo>-pr<N>.yaml`.
6. Post the commit status to the head SHA via the GitHub API: context
   `ruling/ratify`, state `success` only when `verdict: ratify`; `failure` for
   `bounce`; `pending`+description for `needs-clarification`. Overrules (operator
   ratifies over a bounce) are a harness flag that writes an overrule ruling to the
   ledger first — an overrule is data (ADR D6).

**Named e2e proof (D8 discipline — this is the test that counts):** a real PR on a
pilot repo receives a `ruling/ratify` status posted by the harness after a genuine
fresh-judge invocation, and with the status required, the PR is mergeable exactly
when the ruling says so. Unit tests cover schema refusal (off-spec YAML cannot
deserialize); the e2e is the goalpost.

**Rollout order (so nothing wedges):** build -> first real ruling on a real PR
(status posted but NOT yet required) -> verify -> flip `ruling/ratify` into the
required contexts on the pilot repos. Required-flip is the LAST act, after the loop
is proven.

**Ratified amendment (operator review on magpie PR 22, 2026-07-02):** doc-content
agreement is judged on the WHOLE post-image document, never the delta alone. The
delta-scoped question had a gaming vector: append two locally-true sentences and the
rest of the document can be a squirrel — minimum-truth-to-pass, Goodhart at the gate.
The judge's standard is now the most truthful and descriptive document; locally-true
additions beside stale surroundings are `disagree` with the stale passages cited.
Proven necessary on the very first judged PR: its diff fixed the Status section while
the Pipeline section's "(Interim: ... once that lands)" claim sat stale one section up
(pipeline.py:19 imports `from frood import model`; it landed).

## Process discipline for this sprint

- Heavy sprint => standup records are REAL here, first exercise of §4.3: YAML records
  in `2026-07-02-judge/standups/` at phase boundaries (design agreed / harness builds
  / first ruling / status required), spin-vs-churn watched.
- The stage enum tracks in this doc's header; every transition is standup-worthy.
- Divergence from this design section is drift: surface, ratify, or revert.

## Definition of done

- Harness builds and its schema-refusal tests pass.
- One real ruling on a real PR, ledger entry in `rulings/`, status visible.
- `ruling/ratify` REQUIRED on big-little-mesh, delightd, magpie; read-back verified.
- Standup records exist for the phase boundaries actually crossed.
- Outcome recorded; promoted via `git mv` + chmod 400.

## Outcome (2026-07-02, on resolution) — DONE, every line of the definition met

The harness: Rust, 16/16 refusal tests, 0 warnings, at `tools/judge/`. Schema-first
build order held (types -> assembly -> spawn -> publish), and the validator caught its
own author once (judge-output vs ledger-entry are distinct validation contexts).

The first ruling became FOUR rulings on one 20-line docs PR (magpie 22, closing magpie
issue 20), and that sequence is the sprint's real product — the judge earning
authority exactly as ADR-0001 prescribed:

- r1 clarify: refused to certify existence claims it could not cite -> head tree
  became a bundle input.
- r2 ratify with tree citations; left a forward tripwire for the future sidecar PR.
- r3 clarify under the operator-expanded standard (below): paths cannot evidence
  routing behavior; named the exact files it needed -> --include became the demand
  side of the clarification dialogue.
- r4 RATIFY: every doc clause cited to code lines (pipeline.py:19/27-30/72/74-76,
  pyproject.toml:13-18); closed r2's tripwire positively; consistent with all three
  predecessors by ledger id.

Operator review supplied the sprint's most important correction: the delta-scoped
doc-agreement question had a minimum-truth gaming vector (locally-true sentences
beside a stale document pass the gate — Goodhart at the gate). Ratified amendment:
the judge rules on the WHOLE post-image document; the standard is the most truthful
and descriptive document, not the minimum change that passes. Proven necessary on the
first judged PR — its own diff had fixed Status while a stale "(Interim: ... once
that lands)" claim sat one section up.

Enforcement: `ruling/ratify` is now a REQUIRED status on big-little-mesh, delightd,
and magpie (read-back verified; enforce_admins stays on). Every merge on the pilot
repos now needs a judge ruling. magpie 22 landed through the full gate stack (python
+ ruling/ratify green, operator sign-off) as the proof.

Issues: closed magpie 20 (via the landed PR); opened none. Net -1.

Carried forward: ADR pilot criterion 4 (magpie -> delightd e2e registration proof) —
the next sprint, as scoped from the start. Also carried: the sidecar-PR tripwire
recorded in r2/r4, which a future ruling must check.

Resolved per §4.4: `git mv 2026-07-02-judge 2026-07-02-judge-completed`, chmod 400.
