# judge

Every merge on the pilot repos (big-little-mesh, delightd, magpie) needs a ruling
from a judge that did not write the diff. This tool is how a ruling happens: it
gathers everything a fresh judge is allowed to consider, spawns one, refuses any
reply that does not survive the ruling schema, writes the accepted ruling into
the active sprint's ledger, and posts the `ruling/ratify` commit status that
branch protection requires. One invocation, one judge, one ruling.

It is the T3 judgment gate of ADR-0001 (Decision 6), amended by ADR-0002
(doc pairing binds through the judge). Doctrine follows the ADR chain; where a
later ADR conflicts with an earlier one, the later ADR governs.

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
judge <repo-path> <pr-number> [flags]
judge --validate-ruling <file>
```

| Flag | What it does |
|------|--------------|
| `--validate-ruling <file>` | Validate a LEDGER ENTRY and exit (0 valid, 1 refused). No PR needed. |
| `--dry-run` | Assemble and summarize the bundle; spawn nothing. Audit what a judge would see. |
| `--include <path>` | Add a file's head content to the bundle. Repeatable. The supply side of a `needs-clarification` ruling: the judge names the evidence it needs, the re-fire provides it. |
| `--overrule --reason <text>` | Operator overrule: write a ratify ruling that names itself an overrule, then post the status. An overrule is data, never a shrug. |
| `--skip-status` | Write the ledger, post nothing. Rehearsal / hostile-network mode. |
| `--skip-lane` | Skip the durability lane (no commit/push of the ruling). For a truly local rehearsal; rehearsal mode otherwise keeps the lane on. |
| `--judge-cmd <bin>` | Judge executable. Default `claude`; tests stub it. |
| `--model <name>` | Model override. Default: the CLI's configured model. |
| `--work-root <dir>` | Fleet checkouts root for the consumer scan. Default `/Users/jane/work`. |
| `--sprints-root <dir>` | Sprints repo root (ledger home). Default `/Users/jane/work/sprints`. |

## The bundle

The judge is never fed the writer's session. It receives exactly these inputs,
assembled fresh per invocation, and is instructed to cite only from them:

| Input | Source | Why it is required |
|-------|--------|--------------------|
| The diff | `gh pr diff` | The thing under judgment. Empty diff = refuse to run. |
| Head sha + tree | `gh` / `git ls-tree` | Existence evidence. A doc claim like "register.py exists" is citable against the tree; "trust me" is not. |
| Design docs | repo `docs/*.md`, `DESIGN.md`, `VISION.md`, `README.md` | The goalpost (ADR-0001 D8). A repo with no design artifact CANNOT be judged — that is a loud error, not a default pass. |
| Implicated docs | repo `.docpairs` (literal `<path-prefix> -> <doc>`) | ADR-0002: a doc paired to a changed path-prefix rides in, and a diff that falsifies it without updating it is a bounce. A fired pair naming a missing doc is a loud error; a glob metacharacter in a prefix is refused (the map must migrate to literal prefixes). Absent `.docpairs` = no pairings. |
| Contracts touched | `.proto` files named in the diff | The wire is the boundary-enforcer; the judge reads what changed on it. |
| The ruling ledger | every `rulings/*.yaml` across all sprint dirs | The judge's only persistent memory. Fresh instances + a durable ledger replace a resident judge (struck in Sprint 0: long-lived sessions rot). |
| Consumer scan | `rg` for changed proto message names across the roster | Consumer impact must be cited, not asserted. |
| `--include` files | `git show HEAD_SHA:path` | Judge-requested evidence. A named file missing at head is a loud error — wrong evidence supplied silently would corrupt the ruling. |

## The ruling

The reply must be a single YAML document matching the schema in
`src/ruling.rs` — enum verdicts (`ratify` / `bounce` / `needs-clarification`),
divergences with justifications, a shape verdict, consumer impact with
file:line citations. `deny_unknown_fields` and real enums mean an off-spec
ruling does not deserialize; `validate()` adds what shape cannot express (a
bounce must state why; the judge MUST NOT assign its own ledger id).

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

- CLOSED: the spawned judge runs with `--tools ""` — no tools at all. Every
  input it may consider is already in the prompt and its only output is YAML,
  so bundle text cannot steer it into executing anything as the operator.
  Verified empirically (2026-07-02): a locked-down judge cannot read a nonce
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
- `ruling ABSENT: refused twice` — the judge could not produce a schema-valid
  ruling. Read the two refusals in the error; the schema is not negotiable.
- A wedged judge hangs forever: there is NO timeout in v0, by decision — runs
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
  repo. The tool never rules on its own diffs: the operator is the judge's
  judge, and that is the writer-is-not-judge rule applied to the judge itself.
- Before push: `cargo fmt --check`, `cargo clippy` clean, `cargo test` green.
  A required CI enforcing exactly that is proposed and awaiting the operator's
  call (Sprint 7, Phase B).
- Doctrine lives in two places that MUST move together: the ADRs, and the
  prompt constant in `src/spawn.rs` (`build_prompt`). An ADR that amends judge
  doctrine includes the prompt edit in its own diff. Generating the prompt's
  schema section from the serde types is sized-but-not-built (Sprint 7 review,
  spawn.rs:27 thread).

## Known debts (recorded, owned, not hidden)

Named at the Sprint 7 line-review; each is carried deliberately, not forgotten:

- Ledger growth: every prior ruling enters every bundle, unbounded. Needs a
  policy (wiring sprint), not a quiet truncation.
- Roster: `PILOT_ROSTER` is a hard-coded constant scanning checkout DIRECTORY
  names (`kafka-logging` is the checkout of janearc/kafka-svc — correct today,
  fragile forever); a missing dir is skipped silently. The fix is roster from
  delightd (`GET /projects`), fail-loud, kicked to the wiring sprint. The
  repo-name vs dir-name split needs a mapping when that lands.
- Deleted files are invisible to `changed_paths` (`+++ /dev/null`), so a
  deleted contract never enters the bundle.
- The file:line citation mandate is schema-enforced only for consumer impact;
  divergence justifications are checked non-empty, not cited.
- Consumer hits truncate at 20 per repo with no marker.
- `fired_at` and the echoed instance id are the judge's word; the harness
  could stamp both, as it already does the ledger id.
- Status descriptions embed the ledger path; GitHub caps descriptions at 140
  characters and long names will brush it.
- Two un-promoted sprint dirs resolve to the lexicographic last, silently —
  the process forbids the situation; the tool should be loud about it anyway.
- `serde_yaml` is archived upstream. Fine today; a migration candidate when
  the crate is next touched in anger.
