# Sprint: Fix the Coding Process

- Date: 2026-07-01
- Session: fix-coding-process
- State: DRAFT (for markup; not yet released to the sprints repo)
- Author: Max + Claude (Opus 4.8)

---

## 0. What this document is

This is two things at once, on purpose:

1. Today's sprint doc — the single artifact that drives this coding session.
2. The first instance of the sprint-doc *format* we are defining today.

We are dogfooding. If the format is any good, this document is proof of it. If it
is not, we find that out by living in it for one session.

This is a LOW-code, not a no-code, day. The bulk of the work is design, GitHub hygiene,
and cleaning up what governs the agent. The only "code" is wiring and YAML schemas (4.3);
we reuse `wonderlib` (already built and tested) rather than write new tooling, and there
is no protocol nanny (Section 8). Do not let the low-code framing invite a large-code
detour.

---

## 0a. Terms and conventions

- This document uses RFC 2119 keywords (MUST, MUST NOT, SHOULD, MAY) in caps; they carry
  their RFC 2119 meaning.
- We describe behavior in concrete terms, not personas. We do not say "write it like a
  senior engineer"; we name the artifact a given standard would have produced. Asking
  for a persona buys tone; forcing the artifact buys the work.

Definitions:

- **instructions** (n.): markdown content the agent is directed to treat as binding on
  its own behavior — not project documentation, not code comments, not design docs.
  Currently and exclusively the single authoritative file established in Section 5.
  Anything else, however rule-shaped it reads, is reference material.
- **the writer** (n.): the agent that produces an initial diff in response to a task —
  one file or several, whatever comes back with a diff attached. Never the same
  instance/context that judges its own output.
- **the judge** (n.): a fresh, single-purpose agent spawned solely to evaluate one diff
  against the design doc, the contract, and the ruling ledger. Never the writer's
  instance or context. Stateless across invocations — its only persistent memory is the
  ruling ledger (3.3), never a conversation.
- **schema** (n.): the machine-readable, registered definition of a message's shape
  (protobuf, here) — what a producer emits and a consumer parses against.
- **contract** (n.): a schema, PLUS the rule for how it is allowed to evolve, PLUS who is
  accountable when a change violates that rule. A schema is necessary, not sufficient —
  exactly why the schema-breaking gate (3.1) exists: "still parses" and "safe change" are
  different questions.
- **ADR** (n.): Architecture Decision Record — a short, durable document recording one
  design decision, its context, and its consequences. What Phase 2 produces; promoted and
  protected the same way a sprint doc is (4.4).
- **debrief / inbrief**: NOT a standalone manual handoff mechanism — that is the exact
  failure mode Section 1 names (state carried agent-to-agent by hand). They are defined
  ONLY as synonyms for the standup record: *debrief* = the most recent standup record;
  *inbrief* = a fresh agent reading that record plus the sprint doc plus current git
  state. There is no manual handoff path outside the standup artifact, under any name.

---

## 1. The problem we are fixing

The agent that WRITES the code is also the agent that JUDGES the code, and that is
the defect. Everything else is downstream of it.

Observed failure modes:

- **Done is self-assessed.** The writer emits the event it was told to emit and
  declares victory ("I write FooBarBaz to the wire, done") without ever climbing
  back to the design doc to ask: did I do the job as specified? Did I diverge?
  Where, why, and was it necessary?
- **Docs drift.** As commits stack, the design doc is never corrected to match the
  code. No competent engineer leaves stale docs on disk; the writer does, every time.
- **Off-mesh code.** The writer produces locally-clean Go/Python that is the wrong
  *shape* for how the mesh operates. It does not reason forward about what a change
  implies three services downstream.
- **Hand-coding past the gen.** The writer hand-writes code that should have been
  generated, and quietly makes contracts permeable.
- **Context balloon.** Churn burns tokens, context bloats, and the human review that
  is supposed to be the safety net becomes a tired, superficial pass over thousands
  of lines of diff.
- **Manual orchestration.** State is carried agent-to-agent by hand (debrief ->
  inbrief), and agents go stale mid-session and confabulate.
- **Instruction-surface pollution.** Instructions that govern the agent are scattered
  across ~/.claude, ~/work/agents (legacy gemini/agy executable markdown), and stray
  .claude dirs. The agent trips over stale docs and cannot say where a rule came from.

---

## 2. The move (the single principle)

We stop trying to give the writer expert-level judgment in the moment. We move the
definition of "done" OUT of the writer's own head, and we make surviving the gate
mechanical wherever it can be, and a separate reader fed the exact context wherever
it cannot.

The wire made instinct non-load-bearing, whoever held it: diverge from the schema and
your packet never arrives, no taste required. We do the same thing to the
writer's PROCESS. We build hostility around the *actions*, not around our description
of what the actions should be.

Corollary: **"Done" is external.** Done means the mechanical gates are green AND a
separate judge, holding the design doc, signed off. The writer cannot declare victory
about itself.

Corollary applied to T1/T2: never break schema, never hand-write what can be generated.
Same "done is external" principle, enforced mechanically rather than asked for.

Second axiom (load-bearing for the whole sprint model): **we do not scale by context
window; we scale by moving state out of the model.** If we ever find ourselves NEEDING
a bigger context window to hold a sprint, we have failed. State lives in durable
artifacts — the contract, the ruling ledger, the standup record — not in a transcript
we are trying to keep alive. This is the sentence that reconciles sizing-as-discipline
and handoff-as-architecture: the same strategy aimed at two different failure timings.

Corollary (self-describing repos): repositories and code MUST be self-describing — a
fresh agent (or human) should understand a repo from its own structure and durable
artifacts (contract, ADR, docs) without a large context dump to orient. This is the
concrete reason doc-pairing (3.1) and gen-freshness matter beyond drift-prevention:
they are what keep a repo legible without inflating anyone's context just to read it.

---

## 3. The model we are building toward

Two different animals, often blurred under the word "gate." Keep them separate.

### 3.1 Mechanical gates (NO LLM; deterministic; fire in hooks/CI)

These do not get tired, cheerful, or salute. Each is a check we author ONCE and can
read end-to-end ourselves.

- **Schema-breaking gate.** An incompatible protobuf change fails the build
  (`buf breaking` or equivalent). Catches the sanctioned-evolution class: a new enum
  value that parses fine and falls through a downstream consumer's default branch.
- **Gen-freshness gate.** Re-run the generator; diff its output against what is
  committed. If committed generated code does not equal what the contract produces,
  fail. This is the mechanical form of "the gen cannot drift / do not hand-code past
  the gen." One gate, all four target languages: regenerate the Rust, Go, Swift, and
  Python bindings, diff each against committed output, and fail if any language's
  generated output does not match or does not compile. Four languages, not four gates.
- **Doc-pairing gate (mechanical half only).** A diff touching a path with a paired
  doc that does not also touch that doc in the same commit fails. This is a pure
  path-touched check: deterministic, hard-fail, T2. It encodes the human rule "I
  refuse a diff without doc updates, and I check" so a human never has to be the check.
  It proves the doc was TOUCHED, not that its content is true. That second question is
  a judgment call and lives in 4.3, not here — do not smuggle an LLM into this gate.

### 3.2 Judgment gate (an LLM, but a DIFFERENT one from the writer)

Spawned fresh, single-purpose, fed durable artifacts as REQUIRED input, not optional
context. It answers what no linter can:

- Does this diff satisfy the design doc? Where did it diverge, and was that necessary?
- Is this on-mesh, or is it clean code of the wrong shape?
- Consumer-impact: enumerate every current consumer of a changed message type and
  classify each additive / breaking / silent-drift, with the consumer's actual code
  as evidence, not an assertion.
- (A fourth question — does the paired doc's content still agree with the code — lives in
  4.3's standup record now, not here.)

The judge does NOT free-text its verdict. A nondeterministic model narrating a prose
opinion is no more trustworthy than a tired human pass over a diff — the exact thing
this doc exists to eliminate. The judge emits a fixed-schema ruling, symmetric to the
standup YAML, and that record IS the 3.3 ledger entry (no separate write step):

```yaml
ruling:
  diff_ref: <commit-sha-or-PR-url>
  judge_instance: <ephemeral-id, never reused>
  fired_at: 2026-07-01T16:05:00Z
  verdict: ratify | bounce | needs-clarification      # enum, not free text
  divergences:
    - claim: "added retry logic not specified in design doc"
      necessary: true
      justification: "design doc silent on transient-failure handling; required for schema compat"
  shape_verdict: on-mesh | wrong-shape                 # answers 3.2's second bullet directly
  shape_justification: "..."
  consumer_impact:
    - consumer: magpie/internal/handler.go
      classification: additive | breaking | silent-drift
      evidence: "handler.go:142 - default case does not branch on new enum value FOO_BAR"
  ledger_entry_id: <assigned on write; appends to 3.3's ledger>
```

`verdict` and `shape_verdict` are enums, not free text — no path to Turkish or FORTRAN.
Every `evidence` field requires a file:line citation, applying the "not an assertion" bar
3.2 already sets for consumer-impact to the whole ruling.

Both halves obey the same rule: the reasoner never has to be smart in the moment. The
mechanical gate needs zero intelligence. The judge needs intelligence but is fresh
(no staleness), scoped (no context balloon), and handed the exact arrows (no holding
the graph in its head).

### 3.3 The judge's memory is durable, not conversational

This is what dissolves the fresh-vs-resident dilemma. Fresh-per-check forgets what it
refused yesterday; resident rots. Both fail because they store judgment in a
transcript. Instead the judge's state lives in artifacts:

- the contract (ADR + protocol + registered schema),
- a ledger of prior rulings ("bounced this, here is why"; "this divergence was
  approved on DATE because Y").

Then the judge is spawned FRESH every time and reads a small curated durable memory.
Fresh-per-check WITH a memory that is not a conversation.

### 3.4 The four gate timepoints

Do not conflate these; each fires at a different time on a different substrate.

- **T0 - design-time (judgment + human approval).** Is the design/ADR/schema coherent
  and mesh-shaped, before any code exists? Not a git hook; there is no diff yet. Lives
  in session structure (plan-mode / a design-review subagent / Max's sign-off).
- **T1 - pre-write (generate-vs-hand-write classifier).** For anything enum- or
  state-shaped: should this be generated or hand-written? A cheap pre-check.
- **T2 - pre-land mechanical.** Schema-breaking, gen-freshness, doc-pairing. Hooks/CI.
- **T3 - pre-land judgment.** Design-doc conformance + consumer-impact. Fresh subagent.

---

## 4. The sprint process (defined here; used from here on)

### 4.1 What a sprint is

A sprint is one coding session (roughly 4-12 hours) driven by exactly one document.
That document lives in the private `sprints` repo. Old-world sprints were cut by date
(two weeks); we cut by scope instead.

### 4.2 Sizing, and the real problem underneath it (handoff)

We do not assign points, and we do not try to predict where the wall sits. The
staleness wall is not fixed — it has shrunk as task complexity grew (24h+ -> ~4-5h and
trending shorter), and we explicitly refuse to move it by growing into a bigger context
window (Section 2 axiom: we scale by moving state out of the model, not by context
window). A sizing discipline that assumes you can predict the wall is building on sand:
the target moves, and we have foreclosed the easy way to move with it.

So the two mechanisms are ranked, not in tension:

- **Primary (discipline).** Bias hard toward sprints smaller than you think you need.
  Treat any overrun as an immediate stop-and-split decision, NOT a push-through. This is
  a discipline, not an estimate.
- **Secondary (permanent infrastructure, not a fallback).** Durable standup records
  (4.3) are what make a sprint survivable WHEN the discipline fails anyway — which, given
  the wall keeps moving, it will, sometimes unpredictably. This is not a hedge we hope
  not to need; it is permanent infrastructure.

The bias-setter before code is not a size scale but the work's position in a fixed
pipeline — which stage the task is entering and how far it has to travel:

> concept -> pseudocode -> draft-diff -> landing-candidate -> landed

This is a literal enum (`stage`, in the standup schema below) so "where are we" is
mechanically legible, not prose. Every stage transition is a standup-worthy event on its
own, independent of wall-clock cadence.

### 4.3 Standup (drift check AND handoff artifact)

Standup is missing and we are adding it. The general mechanism is the durable record;
"standup" is its most frequent, scheduled instance. Phase boundaries (0 -> 1 -> 2) and
T2/T3 gate firings are also natural checkpoints, and any of them MAY trigger writing a
record. Max likes standups; this is a feature, not a ceremony tax.

- A small subagent (Haiku is enough by default) reads this sprint doc plus recent
  commits/diffs and answers three questions: what did we say we would do; what have we
  actually done; where has it drifted.
- **Enough-ness check, computed WITHOUT calling a model, using `wonderlib` (already
  built and tested — no new code).** A hand-rolled `git diff --stat` plus directory-count
  one-liner is exactly the brittle, ad hoc thing this doc argues against everywhere else;
  we use the real tool instead. Surface CONFIRMED against source in Phase 0; wire against
  the published `janearc/wonderlib` repo pinned to a specific revision (as prod neal
  does), NOT the research/monorepo or paling copies — see the Pin note below.
  - `get_git_stats(path) -> GitStats` — real per-file commit/edit statistics (commits,
    additions, deletions) for churn.
  - `profile_document(text, title=...) -> TaxonometryProfile` — Zipf word-frequency and
    POS-based rarity scoring (`zipf_avg`, `rarity_pos`, `rare_terms`), heuristic by
    default — zero model execution when called with no model/tokenizer, which is exactly
    standup's call pattern. A better proxy than line count for how dense/unusual a diff or
    doc is — i.e. whether it needs a stronger reader than Haiku.
  **Pin note:** four copies of `wonderlib` exist — the published `janearc/wonderlib`
  repo, a `research/wonderlib` monorepo copy (ahead of published), a `tmp-migration`
  copy, and a `paling/wonderlib` copy that diverges behaviorally (eager `import torch`,
  unlike the others). Standup MUST wire against the **published repo, pinned to a specific
  revision** — not the monorepo or paling copies. The four-way split itself is tracked
  separately (issue 6 in the Phase 0 batch) and is not this sprint's problem to resolve,
  only to pin around.
  The check becomes: `get_git_stats` on touched files for churn, plus `profile_document`
  on the diff/doc text for rarity. Both mechanical, both already built.
- **Every firing MUST write a durable, structured record** — a file, not a transcript
  exchange, populated mechanically (the shell signals and wall clock fill themselves;
  the subagent writes the prose fields). It is the file a fresh agent reads cold to pick
  up mid-sprint, no lived context required. Schema (YAML, not code, per least-code):

```yaml
standup:
  sprint: fix-coding-process
  fired_at: 2026-07-01T14:32:00Z
  stage: concept | pseudocode | draft-diff | landing-candidate | landed   # enum (4.2)
  elapsed_since_sprint_start: 3h12m
  elapsed_since_last_standup: 47m
  plan_ref: sprints/fix-coding-process/SPRINT.md#section-6
  planned:
    - "Phase 0 inventory: audit CI/hooks across roster"
  done:
    - "CI audit complete: CI runs but no branch protection on any of the 3 repos"
  doc_content_agreement: agree | disagree | unclear   # advisory: semantic half of doc-pairing (3.1)
  drift: []            # non-empty list = drift found; see halted
  halted: false        # true the instant drift is non-empty; stays true until resolved
  next_task: "Phase 1: consolidate instruction surface"
```

- **Wall clock and token burn** (`elapsed_since_sprint_start`, `elapsed_since_last_standup`)
  are emitted the same shape we intend — but both need a REAL producer. Token burn today
  is a `.proto` with no producer and no consumer: nothing emits it, so nothing can read
  it — itself a live instance of the failure this sprint diagnoses (a contract on paper
  that nothing fulfills). `wonderlib.Benchmark` (timing + token-count telemetry) is that
  producer; wire it into the standup firing so both fields populate mechanically, not
  aspirationally. Framing: this is a spin-vs-churn detector, not a cost metric — watch for
  wall clock climbing without matching entries under `done`. [`Benchmark` surface
  CONFIRMED in Phase 0: it provides timing + token telemetry in the published
  `janearc/wonderlib`.]
- **What "drift" means** (so the hard stop does not cry wolf): drift is divergence from
  the plan that has NOT been examined and ratified. A divergence the standup surfaces and
  that Max ratifies — by amending the plan/design doc — is not drift; it is an amended
  plan, and `drift` stays empty. Only unexamined or unratified divergence populates
  `drift`. Resolution path: surface -> ratify (amend `plan_ref`) or revert -> a new
  standup record clears `halted`.
- **Drift is a HARD STOP.** The moment `drift` is non-empty, `halted: true`. Nothing
  lands until the drift is resolved and a new standup record clears it. Drift does not
  "become the next thing we discuss"; it halts the sprint.
- **Clearing a halt needs no new machinery.** "Hard stop" here just means standup.
  Ratify or revert the divergence, then fire another standup — that record itself clears
  `halted`. There is no separate "resolve drift" ceremony to build; do not build one.
- **Permission scope** (see Section 8; identical wording there):

  > The safe, default subagent scope is `~/work/{single-project}` — one project
  > directory, nothing else. Crossing from `~/work/{project}` up to `~/work` (root) is
  > the dangerous action, because that is what exposes everything at once. It MUST be a
  > deliberate, separately authorized capability, granted explicitly per use — never a
  > default, and never granted because it was convenient in the moment.

### 4.4 Definition of sprint done

A sprint is done when:

- Every task below is either landed or explicitly cut (cuts are recorded, not silent).
- Docs on disk match the code that landed.
- Open issues opened for this sprint are closed or carried forward with a reason.
- The sprint doc is promoted to the `sprints` repo as the record of what happened:
  `git mv <sprint> <sprint>-completed` (concrete, low-code, and it should feel good to
  do), then made read-only (chmod 400) on promotion — once it is the record, it is
  immutable.

---

## 5. Instruction surface (hard prerequisite)

The gates are built on top of what governs the agent. If that surface is polluted,
the gates inherit the pollution. This MUST land before we build gate machinery.

- There MUST be exactly one authoritative, auto-loaded instruction surface. Everything
  else is explicitly non-authoritative / reference-only.
- Non-authoritative material MUST be out of the agent's default search path so it
  cannot be tripped over by a broad find/grep.
- The agent's durable memory MUST agree with the one surface. Two current memory
  entries point INTO ~/work/agents (a "private, never-push" entry and an "engineering
  standards cite AGENTS.md / agents/standards" entry); when Max moves ~/work/agents,
  those memories MUST be reconciled or the agent will chase a dead path.
- Max moves ~/work/agents himself. Claude fixes the memories that reference it.

---

## 6. Today's scope (phases, in dependency order)

Today is **0 -> 1 -> 2**. Phase 3 is cut from today (see 6.1). A landed ADR is today's
win.

- **Phase 0 - Inventory (read-only), with a hard boundary.** Go look; do not guess.
  Establish: what CI/hooks exist across the roster; whether `buf` is present and where
  schemas register; where design docs live per project; and a full audit of what
  governs the agent (~/.claude vs ~/work/agents vs stray .claude dirs vs Claude's own
  memory dir). This pass WILL turn up a dozen things worth fixing. Those findings become
  ISSUES today (assigned to janearc, Section 10), NOT diffs. Phase 0 does not fix
  anything it finds; it records and moves on. Same task-drift discipline as Section 1,
  pointed at the inventory instead of at code.
  - HEADLINE finding (Phase 0 ANSWERED it): there is NO unbypassable CI. CI exists and
    runs on `pull_request`, but nothing is a required status check — branch protection is
    absent on magpie, delightd, and frood (GitHub API returned 404 "Branch not protected"
    on all three), so `main` is directly pushable and red PRs merge. A pre-push hook is
    likewise advisory (`--no-verify` walks past it). Phase 2's ADR is built on this: the
    first action is to MAKE the existing checks required, not to assume a gate exists.
  - Also audit for schema-defined-but-unemitted telemetry: contracts that exist in
    `.proto` with no producer and no consumer (token burn is the known instance — see
    4.3; there may be others). Record each as an issue; do not fix inline.
- **Phase 1 - Instruction surface.** Execute Section 5.
- **Phase 2 - Gate architecture as its own contract.** An ADR + design doc for the gate
  itself, written to the bar of being publicly read. We dogfood: the gate is a project
  and gets a contract. Nails down the mechanical-vs-judgment split, what fires at
  T0-T3, what "done" means, the pilot-repo choice (by the criterion in 7.1), and the
  doc-pairing split (mechanical blocking + semantic advisory). The ADR MUST name the
  full eventual scope of the judgment gate up front — the mesh as an emergent state
  machine of generated FSMs (Section 1) — so that scope is not quietly narrowed away
  later.

  Phase 0 found the CI premise was backwards: gates partially exist already (frood has
  a working gen-drift job in CI; delightd's build regenerates and tests) and enforce
  nothing anywhere — branch protection is absent on all three repos (magpie, delightd,
  frood), confirmed directly against the GitHub API, not inferred. `main` is directly
  pushable and red PRs merge on all three. The ADR's first concrete action is therefore
  NOT "build gates" — it's **require the gates that already exist**, via branch
  protection with required status checks, before any new gate-building is prioritized.
  Where a gate genuinely doesn't exist yet (magpie has no gen-drift check at all), build
  it — but enforcement-of-what-exists comes first, since it's cheaper and already half
  built.

### 6.1 Why Phase 3 is cut from today

Phase 3 (one live mechanical gate) is sprint two, not an "if inventory is kind" stretch
goal. "If inventory is kind" is the one soft, in-the-moment judgment call left in a
document whose whole thesis is to stop making judgment calls in the moment — and it
sits exactly where taste is least reliable: end of a long session, a green checkmark
right there. 0 -> 1 -> 2 at the bar above is a full sprint on its own. We let it be the
sprint, bank what it tells us about sizing (4.4), and decide Phase 3 fresh at the START
of the next sprint, not mid-Phase-2 under a clock.

Phase 3 is NOT cancelled. It is filed as an issue today, per the Phase 0 discipline
(findings become issues, not diffs), and carries into the sprint backlog for scheduling
in a subsequent sprint. Consistent with 4.4 ("cuts are recorded, not silent") and the
"sprints should close more issues than they open" axiom: Phase 3 is not a cut, it is a
scheduled item, and it reads that way.

---

## 7. Open questions / assumptions to verify in Phase 0

- HEADLINE (ANSWERED by Phase 0): NO unbypassable CI anywhere — CI runs on PRs but no
  branch protection / required checks on any of the three repos (API 404). See Section 6.
- Where does each gate physically fire: pre-commit / pre-push / GitHub Action /
  subagent step?
- Is `buf` installed; where do "registered" schemas actually register to?
- Where do design docs live per project; is there a uniform convention (there probably
  is not)?

### 7.1 Pilot repo — revised per Phase 0 evidence: the registration seam

The original decision (magpie + delightd, "because coupled") was made from README text
and recollection, neither checked against code. Phase 0 checked. The real picture has
three seams, not one pairing:

- **magpie → delightd: LIVE**, via the registration contract. magpie imports
  frood-generated clients (`frood_pb2`, `register_pb2`); `register.py` implements
  delightd registration on `main` right now. This is the one seam confirmed in code,
  not in a README.
- **magpie → Kafka: NOT wired.** No producer, no sidecar. (This part of magpie's
  README was accurate — the registration part wasn't.)
- **delightd → Kafka (`delight.events`): LIVE**, via a franz-go producer. `delight/v1`
  is vendored from **kafka-svc**, which is the actual source of truth for the bus
  layer — not frood, and not part of this pilot.

Decision: Phase 3's gate targets **the registration contract surface**
(`registry.v1` / `frood.v1`) between magpie and delightd — the seam that's actually
live. The Kafka/bus layer is a separate concern owned by kafka-svc and is explicitly
OUT of scope for this pilot; it can be its own pilot later if warranted.

The original criterion still governs the ADR's framing and doesn't need to change:

1. Fewest current consumers — smallest blast radius if the gate itself has a bug.
2. Schema already registered — so Phase 3 is not also debugging registration.
3. Known cold enough to spot a false positive instantly.

Scoping note the ADR MUST state: this pilot proves the gate against the registration
seam specifically. It does not validate the gate against the bus/Kafka layer, and the
ADR should say so explicitly rather than let "pilot" imply broader coverage than it has.

---

## 8. Struck: the semi-persistent "protocol nanny"

STRUCK, not deferred. The earlier candidate was a wrapper spinning up semi-persistent
Claude sessions (tmux) sitting resident as a "protocol nanny" / judge. Section 3.3
already kills it: if the judge's memory is just files (the ADR and the ruling ledger),
nothing needs to stay resident, and a long-lived session only re-introduces the
staleness we are trying to eliminate. The judge is spawned FRESH on demand against
those files. No tmux, no nanny. "Defer" would imply something is left to revisit; once
3.3 is settled as file-based, there is nothing.

Parked (genuinely later, not struck): the subagent permission boundary (identical
wording in 4.3):

> The safe, default subagent scope is `~/work/{single-project}` — one project
> directory, nothing else. Crossing from `~/work/{project}` up to `~/work` (root) is
> the dangerous action, because that is what exposes everything at once. It MUST be a
> deliberate, separately authorized capability, granted explicitly per use — never a
> default, and never granted because it was convenient in the moment.

---

## 9. Definition of done for THIS sprint

- The gate model (Sections 2-3) is written down and agreed, including the doc-pairing
  split (mechanical blocking + semantic advisory).
- The sprint process (Section 4), including standup as the durable handoff artifact, is
  defined and in use.
- The instruction surface (Section 5) is single, authoritative, and memory agrees.
- Phase 0 inventory is complete; findings are recorded and filed as issues, not fixed.
- The Phase 2 ADR is landed, to the bar of being publicly read, with the pilot-repo
  choice made and the judgment gate's full eventual scope named.
- This document is promoted into the `sprints` repo as today's record and made
  read-only (chmod 400) on promotion.

Phase 3 is explicitly NOT in this sprint's definition of done. It is sprint two.

---

## 10. GitHub hygiene

- Issues opened for this sprint are assigned to `janearc` at creation.
- Issue ceiling (ordinary sprints, from sprint two): the count of open tasks assigned to
  `janearc`, per repo and overall, MUST NOT exceed the count at sprint start; if it does,
  the sprint runs a rectification pass before it is done. Issues stay short-lived and
  tightly scoped.
- Exemption for discovery/inventory sprints (like today's Phase 0, whose whole mechanism
  is "findings become issues"): exempt from the ceiling itself, NOT from accounting. Such
  a sprint MUST state its net issue delta as an explicit number in its own record (same
  "cuts are recorded, not silent" pattern as 4.4), so the exemption is visible and bounded
  — never a silent escape hatch for a future sprint that would rather open issues than
  close them.
- Commits that close work use `Fixes #N`.
- Claude authorship is marked on commits (Co-Authored-By trailer naming the model) and
  on any PR/issue/comment prose.

---

## 11. Changelog / resolved in markup

Round 1 — resolved in the 2026-07-01 markup pass (Max + Sonnet 5):

- Phase 3 CUT from today; today is 0 -> 1 -> 2. Phase 3 is sprint two. (6.1, 9)
- Phase 0 gets a hard boundary: findings become issues, not diffs. (6)
- "Is there real, unbypassable CI?" promoted to the headline Phase 0 finding. (6, 7)
- Doc-pairing gate split: mechanical path-touched (blocking, T2) vs semantic
  content-agreement (advisory, day one). (3.1, 3.2)
- Protocol nanny STRUCK, not deferred. (8)
- Sizing demoted; the real fix is durable standup handoff. (4.2, 4.3)
- ~/work permission boundary named and parked. (4.3, 8)
- Pilot repo decided by criterion, not pre-picked; chosen today in the ADR. (7.1)
- chmod-400 read-only promotion flow for the sprint doc recorded. (4.4, 9)

Round 2 — resolved (Max + Sonnet 5):

- 0a added: definitions for "instructions" and "debrief/inbrief" (= standup record only,
  no manual handoff path); RFC 2119 + descriptive-terms conventions stated. (0a)
- 4.2 sizing/handoff resolved as primary (discipline) / secondary (permanent
  infrastructure), with the new Section 2 axiom "we scale by moving state out of the
  model, not by context window." (2, 4.2)
- 4.3 standup: durable YAML record schema added; wall-clock fields (spin-vs-churn, not
  cost); enough-ness decided by two free shell signals (diff-stat + top-level-dir span),
  no new tooling; drift is now a HARD STOP (halted:true), not "the next thing we
  discuss." (4.3)
- 4.3 addition (Claude, flag for veto): "drift" defined as UNEXAMINED/unratified
  divergence, so the hard stop does not cry wolf; ratified divergence = amended plan. (4.3)
- 6.1: Phase 3 is not cancelled — filed as an issue today, carried in the backlog. (6.1)
- 7.1: pilot decided — magpie + delightd, because coupled; gate scoped to their SHARED
  contract surface, not "the mesh"; big-little-mesh is substrate only. (7.1)
- Permission boundary corrected to `~/work/{project}` (safe) vs `~/work` root
  (dangerous, separately authorized), identical wording in 4.3 and Section 8. (4.3, 8)
- 4.4 promotion mechanic recorded verbatim: `git mv <sprint> <sprint>-completed` then
  chmod 400. (4.4)
- Section 1 "senior engineer" -> "competent engineer" per 0a; Uber anecdote's "junior
  engineer" left intact as history (flagged). (1, 2)

Round 3 — resolved (Max + Sonnet 5):

- 0a: restored "the writer"; added "the judge", "schema", "contract", "ADR". (0a)
- Persona-word sweep: "senior judgment" and "junior engineer's instinct" (both Sec 2)
  removed; the 0a negative-example "senior engineer" intentionally kept. Grep clean. (2)
- Section 2: self-describing-repos corollary added. (2)
- 3.1 gen-freshness extended to regenerate + diff + compile-check all four bindings
  (Rust/Go/Swift/Python) — one gate, four languages. (3.1)
- 3.2: judge now emits a fixed-schema RULING (enums for verdict/shape_verdict, file:line
  evidence); the ruling IS the 3.3 ledger entry. (3.2)
- 4.2: small/medium/at-the-edge replaced by the stage pipeline enum (concept ->
  pseudocode -> draft-diff -> landing-candidate -> landed); added as `stage` in the
  standup schema. (4.2, 4.3)
- 4.3 enough-ness: shell heuristic struck; replaced with `wonderlib` (get_git_stats +
  profile_document). Signatures marked UNVERIFIED pending Phase 0 source check. (4.3)
- 4.3 telemetry: token burn is a contract with no producer; reworded to require a real
  producer (`wonderlib.Benchmark`), plus a Phase 0 audit for unemitted telemetry. (4.3, 6)
- 4.3: clearing a halt = just fire another standup; no "resolve drift" ceremony. (4.3)
- 10: issue ceiling scoped to ordinary sprints (sprint two+); discovery sprints exempt
  from the ceiling but MUST state net issue delta. (10)

Open flag for Max: RESOLVED in Round 5 — the `wonderlib` surface is now CONFIRMED against
source (Phase 0). Remaining care: four divergent copies exist; standup pins to the
published `janearc/wonderlib` at a specific revision (see 4.3 Pin note, and issue 6).

Kept unchanged: T0-T3 taxonomy; Section 2 corollary (done is external); ledger-over-
transcript judge memory; the mesh-as-emergent-state-machine framing (named in the ADR
per Phase 2); git mv promotion mechanic (4.4).

No blocking decisions remain. Today's scope is unchanged: 0 -> 1 -> 2, Phase 3 tracked
as its own backlog issue.

Round 5 — resolved (Max + Sonnet 5, from Phase 0 execution):

- 7.1 rewritten: pilot is the registration seam (magpie <-> delightd via frood-generated
  clients), not a magpie+delightd pairing generically. Kafka/bus layer owned by kafka-svc,
  out of scope for this pilot. (7.1)
- 6 (Phase 2 bullet): CI premise corrected — gates partially exist (frood gen-drift,
  delightd regen+test) and enforce nothing (zero branch protection on all three repos,
  confirmed via GitHub API). First Phase 2 action = require existing gates, not build new
  ones. (6)
- 4.3: wonderlib surface CONFIRMED against source; UNVERIFIED markers flipped throughout;
  Pin note added — wire against the published repo pinned to a revision, not the four
  divergent copies. heuristic-only -> heuristic-by-default. (0/4.3/6)
- Phase 0 delta: +7 issues FILED (assigned janearc) — big-little-mesh #74/#75/#76/#77,
  magpie #20/#21, wonderlib #1. Record: 2026-07-01/phase0-issues.md. Net +7 per Section
  10's discovery-sprint exemption; none fixed inline, per Section 6.

Merge notes (Claude, honesty): applied against the on-disk canonical copy
(~/.claude/sprint-2026-07-01-fix-coding-process.md). Sonnet's Round 5 OLD strings targeted
a Bear/claude.ai "consolidated" copy that has drifted from this file; three OLD blocks
(Section 0 UNVERIFIED, the 4.3 header phrasing, the Section 6 wonderlib checklist bullet)
do not exist here and were applied BY INTENT to the actual text instead. The two-copy split
is itself the handoff disease this sprint fights — recommend collapsing to one SOT.

Round 6 (final — Max + Sonnet 5) — two small fixes, then STOP (no more rounds):

- 3.2's doc-content-agreement bullet moved to 4.3's standup record (`doc_content_agreement`
  field, advisory); 3.1 cross-ref updated 3.2 -> 4.3. (3.1, 3.2, 4.3)
- Section 6 HEADLINE + Section 7 HEADLINE rephrased from open question to the Phase 0
  finding (no unbypassable CI; branch-protection 404 on all three repos). (6, 7)

Process verdict: this doc survived contact with real code five times and drifted once
(caught, not silent). The planning phase is over; next is running Phase 0 for real. Cut
tomorrow's sprint from THIS file — the on-disk copy is canonical; there is no parallel
version.
