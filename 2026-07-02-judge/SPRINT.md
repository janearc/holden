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

**Home: `sprints/tools/judge/`** (this repo, private, laptop-local — process tooling
lives with the process records; a public repo would drag in tone/hygiene overhead
with zero consumers).

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
