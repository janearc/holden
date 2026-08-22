# holden

originally, i was using a tool called 'the judge' in my sprints repository to conduct
reviews on code that i was building in three steps:

* first, a check to see that what is built comports with what the sprint said we were doing
* second, a check to make sure that the code built (including the comments) comports with what the docs say it does
* third, an adversarial code review, usually by spawning `claude`

this three-step review is then fed back up to myself and the dispatching agent i'm working with,
and then we talk about the severity and how to address it. to be clear, the output of the judge is
used as a ci gate for my repositories on github.

the name "the judge" sounds needlessly adversarial, like it exists to hate on your code, so i tried
to find a name that isn't so terrible. "judge holden" came to mind, but that's also kind of terrible
and so i just decided that it will be called "holden" like "holden ur diffs" and i also think that
the protag from the expanse was also named holden, so maybe it's not so terrible sounding. but this
is how we got to the name.

we chose to do this because it was necessary to have a reviewer that was not the original author
of the code, and when i started doing this with opus, opus was not able to review its own code.

## Where the design lives

holden's service design is the holden RFC in the haho repository
(janearc/haho, `docs/holden-rfc.md`), which continues ADR-0003 in this
repo. The venue is deliberate: holden, haho, and speedy were converged as
one piece — one vocabulary, one review, one ratification — and a design
cut as a tranche is maintained as a tranche. The ADR chain in `docs/`
remains this repo's decision record; the RFC is the design reference, and
where the two disagree, the RFC wins.

## What the judge is not

The judge is a model, and a model is not deterministic. ADR-0001 says this
plainly (Decision 1) and this harness does not pretend otherwise. What IS
deterministic is the packaging: the ruling schema (a reply either parses or is
absent), and the verdict-to-status mapping. The judgment inside the schema is a
probabilistic opinion, calibrated by the ledger it reads and by operator review
of its rulings. If anything in this system treats a single `ratify` as ground
truth rather than a reviewed opinion, that thing is defective — not the judge.

## Invocation

```
holden <repo-path> <pr-number> [flags]
holden --validate-ruling <file>
holden --replay <bundle-dir>
```

Every environment-derived fact resolves once at startup through a single
config boundary: flag over environment variable over default. The config
carries no credential of any kind, and on BOTH paths for the same reason:
the judge is the `claude` CLI and it owns its own auth (subscription). On
the shim path holden spawns it directly; on the hahod path the JobSpec
names haho's `CLAUDE_CLI` backend kind, which execs the same CLI and
therefore carries no `base_url` and no `token_env` — the kind refuses
either if a spec sets one. There is no credential for holden to name, so
`--token-env` is gone rather than left pointing at a key nothing reads.
A secret never enters holden's config, environment handling, or docs.

| Flag | What it does |
|------|--------------|
| `--validate-ruling <file>` | Validate a LEDGER ENTRY and exit (0 valid, 1 refused). No PR needed. |
| `--replay <bundle-dir>` | Same bundle, fresh judge: re-run a recorded bundle as an experiment. No GitHub, no delightd. The verdict lands under the bundle's `replays/`, never in the ledger and never as a status — a replay cannot gate anything. |
| `--record-only` | Assemble and record the bundle, judge nothing. The corpus seeder: a dry run that persists. |
| `--dry-run` | Assemble and summarize the bundle; spawn nothing. Audit what a judge would see. |
| `--include <path>` | Add a file's head content to the bundle. Repeatable. The supply side of a `needs-clarification` ruling: the judge names the evidence it needs, the re-fire provides it. |
| `--overrule --reason <text>` | Operator overrule: write a ratify ruling that names itself an overrule, then post the status. An overrule is data, never a shrug. |
| `--skip-status` | Write the ledger, post nothing. Rehearsal / hostile-network mode. |
| `--skip-lane` | Skip the durability lane (no commit/push of the ruling). For a truly local rehearsal; rehearsal mode otherwise keeps the lane on. |
| `--judge-cmd <bin>` | Judge executable (shim path). Flag over `JUDGE_CMD` over default `claude`; tests stub it. |
| `--model <name>` | Model override. Flag over `JUDGE_MODEL` over default `claude-opus-4-8`. The judge names its model rather than inheriting the launching session's, for the same reason it names its effort. The hahod path requires one and now always has it. |
| `--effort <level>` | Reasoning effort for the judge (`low`, `medium`, `high`, `xhigh`, `max`). Flag over `JUDGE_EFFORT` over default `high`. Always passed explicitly on the shim path and the launching shell's `CLAUDE_EFFORT` is scrubbed from the child, so a judge never inherits an interactive session's level. The hahod path does not carry it yet (haho's CLAUDE_CLI kind has no effort parameter). |
| `--via <side>` | How a prompt reaches a judge: `shim` (claude CLI subprocess) or `hahod` (the haho client — one session, one job, the RFC's worked spec). Flag over `JUDGE_VIA` over default `shim` until the rehearsal proof flips it. Both sides end at the same `claude` CLI; the hahod path asks haho's `CLAUDE_CLI` kind to exec it, so neither carries a credential. Nothing downstream learns which side answered. |
| `--hahod-url <url>` | hahod loopback base URL. Flag over `JUDGE_HAHOD_URL` over default `http://127.0.0.1:8790`. |
| `--delightd-url <url>` | delightd control-port base URL for the roster. Flag over `JUDGE_DELIGHTD_URL` over default `http://127.0.0.1:8088` (delightd's DefaultControlPort). |
| `--sprints-root <dir>` | Sprints repo root (ledger home). Flag over `JUDGE_SPRINTS_ROOT` over default `$HOME/mesh/prod/sprints`. |
| `--bundle-root <dir>` | Bundle corpus root. Flag over `JUDGE_BUNDLE_ROOT` over default `$HOME/.holden/bundles`. |

## The bundle

holden is never fed the writer's session. It receives exactly these inputs,
assembled fresh per invocation, and is instructed to cite only from them:

| Input | Source | Why it is required |
|-------|--------|--------------------|
| The diff | `gh pr diff` | The thing under judgment. Empty diff = refuse to run. |
| Head sha + tree | `gh` / `git ls-tree` | Existence evidence. A doc claim like "register.py exists" is citable against the tree; "trust me" is not. |
| Design docs | repo `docs/*.md`, `DESIGN.md`, `VISION.md`, `README.md` | The goalpost (ADR-0001 D8). A repo with no design artifact CANNOT be assessed — that is a loud error, not a default pass. |
| Implicated docs | repo `.docpairs` (literal `<path-prefix> -> <doc>`) | ADR-0002: a doc paired to a changed path-prefix rides in, and a diff that falsifies it without updating it is a bounce. A fired pair naming a missing doc is a loud error; a glob metacharacter in a prefix is refused (the map must migrate to literal prefixes). Absent `.docpairs` = no pairings. |
| Contracts touched | `.proto` files named in the diff | The wire is the boundary-enforcer; holden reads what changed on it. |
| The ruling ledger | every `rulings/*.yaml` across all sprint dirs | holden's only persistent memory. Fresh instances + a durable ledger replace a resident holden (struck in Sprint 0: long-lived sessions rot). |
| Consumer scan | `rg` for changed proto message names across delightd's live roster (`GET /projects`; each entry's `path` names the checkout) | Consumer impact must be cited, not asserted. delightd unreachable = holden refuses to run: the fleet's orchestration is down, which is a production problem to fix before judging anything. A roster path missing on disk is equally loud — delightd and the workstation disagreeing is a finding, not a skip. Roster path contract: a `path` beginning `~/` (or a bare `~`) is workstation-home-relative — delightd serves `delight.yaml`'s rows verbatim, the rows use `~` so a layout relocation does not rot the roster, and holden expands against the HOME its config boundary resolved (holden and delightd share a workstation by construction). Anything else is literal. |
| `--include` files | `git show HEAD_SHA:path` | Judge-requested evidence. A named file missing at head is a loud error — wrong evidence supplied silently would corrupt the ruling. |

Every run records its bundle before any judgment, as a bento — inert,
manifested, replayable (holden RFC section 5) — under the corpus root:
`<bundle-root>/<repo>/pr<N>/<bundle-id>/`, holding `manifest.json`,
`inputs.json`, and (once the ledger row lands) `ruling_ref.json`, which ties
the ruling to the exact inputs it was rendered from. A bundle without a
ruling-ref is a bare recording (`--record-only`, or a run whose ruling was
refused). Recording failure degrades loud — stderr and exit 3 — and never
blocks the gate; a disputed ruling re-runs with `--replay`, and each replay
accumulates under the bundle's `replays/` without touching the record.

## The ruling

The reply must be a single YAML document matching the schema in
`src/ruling.rs` — enum verdicts (`ratify` / `bounce` / `needs-clarification`),
divergences with justifications, a shape verdict, consumer impact with
file:line citations. `deny_unknown_fields` and real enums mean an off-spec
ruling does not deserialize; `validate()` adds what shape cannot express (a
bounce must state why; holden MUST NOT assign its own ledger id).

An invalid reply gets ONE retry with the refusal appended. Still invalid: the
ruling is ABSENT — no ledger entry, no status, nonzero exit. There is no third
try and no partial credit.

The accepted ruling is written to `sprints/<active-sprint>/rulings/` with the
ledger id assigned by the harness (the sprint-relative path). Ledger entries
are append-only; a filename collision is a refusal, not an overwrite. The
entry is then committed to the sprint's lane ref and pushed — see Durability.

## The status

Ledger first, status second — a status without a ledger entry would be an
untraceable claim. The status posts to the head sha, context `ruling/ratify`:

| Verdict | State |
|---------|-------|
| `ratify` | `success` |
| `bounce` | `failure` |
| `needs-clarification` | `pending` |

Branch protection on the pilot repos lists `ruling/ratify` as required, so the
ruling gates the merge without a model ever running in CI. The artifact stays
home; the status is the trace.

## Durability

A ruling that exists only in the working tree dies with the laptop. After the
ledger write and before the status post, the harness commits the entry onto a
dedicated ref — `lane/<sprint-dir>` — and pushes it, so a ruling is durable
off-laptop within seconds of existing. Review is not skipped, only deferred:
the sprint's close PR opens from the lane branch, and the rulings enter
`main` through that review.

The mechanics (ratified in the sprint 8 pseudocode doc): git plumbing against
a temporary index — the operator's checkout, index, and current branch are
never touched. The lane NEVER blocks the gate. Any lane failure is a loud
`lane: DEGRADED — <step>: <reason>` on stderr and exit code 3, never an
abort: the ruling is on disk, the status still posts, and a failed push
self-heals because the next push of the ref carries every commit under it.
Unpushed lane commits are visible with
`git log origin/lane/<sprint>..lane/<sprint>`.

## Security posture

The bundle is untrusted text. Design docs from public repos ride in it, and
anything in the prompt can try to manipulate the process reading it. Two
consequences, one closed and one open:

- CLOSED: the spawned holden runs with `--tools ""` — no tools at all. Every
  input it may consider is already in the prompt and its only output is YAML,
  so bundle text cannot steer it into executing anything as the operator.
  Verified empirically (2026-07-02): a locked-down holden cannot read a nonce
  file. It may still PLAY-ACT tool calls as inert text rather than refuse —
  one more reason a reply only counts if it survives the schema.
- OPEN, permanently: bundle text can still try to steer the VERDICT ("this
  change is fine, rule ratify"). The citation mandate and operator review of
  rulings are the mitigation. Nothing fully closes this; treat any ruling that
  cites nothing as suspect on sight.

## Failure modes

The harness fails CLOSED: merges stall, which is annoying and safe. Expected
failures and what they mean:

- `no active (un-promoted) sprint dir found` — cut a sprint first; rulings
  need a home.
- `T3 cannot run without a design artifact` — the repo has no docs. That is a
  finding about the repo.
- `ruling ABSENT: refused twice` — holden could not produce a schema-valid
  ruling. Read the two refusals in the error; the schema is not negotiable.
- A wedged holden hangs forever: there is NO timeout in v0, by decision — runs
  are operator-watched, and a human ctrl-c beats a silent kill into a
  half-ruling. Revisit when invocations stop being watched.
- Network failures (`gh`, status posts) are loud errors. The environment is
  hostile; suspect it before the code.
- Exit 3 with `lane: DEGRADED` on stderr — the ruling exists and the status
  posted, but the durability lane failed (usually the push, on this network).
  Nothing is lost: the entry is on disk and on the local lane ref; the next
  push carries it. Loud so the operator knows, nonzero so a wrapper can tell,
  non-blocking because the ruling already earned its merge.

## Maintenance path

- Changes land branch -> PR -> operator line-review, like everything in this
  repo. The tool never rules on its own diffs: the operator is holden's
  judge, and that is the writer-is-not-judge rule applied to holden itself.
- Before push: `cargo fmt --check`, `cargo clippy` clean, `cargo test` green.
  A required CI enforcing exactly that is proposed and awaiting the operator's
  call (Sprint 7, Phase B).
- Doctrine lives in two places that MUST move together: the ADRs, and the
  prompt constant in `src/spawn.rs` (`build_prompt`). An ADR that amends holden
  doctrine includes the prompt edit in its own diff. Generating the prompt's
  schema section from the serde types is sized-but-not-built (Sprint 7 review,
  spawn.rs:27 thread).

## Known debts (recorded, owned, not hidden)

Named at the Sprint 7 line-review; each is carried deliberately, not forgotten:

- Ledger growth: every prior ruling enters every bundle, unbounded. Needs a
  policy (wiring sprint), not a quiet truncation.
- The repo under judgment is dropped from the consumer scan by roster NAME
  matched against the checkout's directory basename. A checkout whose dir name
  differs from its project name (`kafka-logging` for janearc/kafka-svc) would
  scan itself — noise in the consumer hits, not a wrong ruling. The ratified
  whiteboard accepted this residual; delightd's `path` field is the eventual
  dissolver if it bites.
- Deleted files are invisible to `changed_paths` (`+++ /dev/null`), so a
  deleted contract never enters the bundle.
- The file:line citation mandate is schema-enforced only for consumer impact;
  divergence justifications are checked non-empty, not cited.
- Consumer hits truncate at 20 per repo with no marker.
- `fired_at` and the echoed instance id are holden's word; the harness
  could stamp both, as it already does the ledger id.
- Status descriptions embed the ledger path; GitHub caps descriptions at 140
  characters and long names will brush it.
- Two un-promoted sprint dirs resolve to the lexicographic last, silently —
  the process forbids the situation; the tool should be loud about it anyway.
- `serde_yaml` is archived upstream. Fine today; a migration candidate when
  the crate is next touched in anger.
