// assemble.rs — input assembly (ADR-0001 D6): everything the judge receives,
// gathered as explicit artifacts. the judge is never fed the writer's session;
// it gets exactly what this module returns and nothing else.
//
// subprocess seams (gh, rg) are isolated in run(); the diff-parsing logic is
// pure and unit-tested. network/API failures are loud errors — the harness
// fails closed, it never rules on partial inputs.

use crate::Config;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// pilot-scope roster for the consumer scan: the registration-seam repos plus
// the fleet repos that vendor generated contract types. post-pilot this reads
// the fleet's WorkstationConfig instead of a constant — recorded as a pilot
// boundary in the sprint doc, not a shortcut: the pilot binds one seam.
const PILOT_ROSTER: &[&str] = &[
    "big-little-mesh",
    "delightd",
    "magpie",
    "kafka-logging",
    "obs-svc",
    "taco",
    "paling",
];

#[derive(Debug)]
pub struct Inputs {
    pub repo_name: String,
    pub pr_number: u64,
    pub head_sha: String,
    pub diff: String,
    // repo file paths at the PR head, so the judge can CITE the existence of
    // code that docs claim (added after the first real ruling asked for it).
    pub head_tree: Vec<String>,
    // full head-content of operator-included files: the demand side of the
    // needs-clarification dialogue (a judge names the evidence it needs; the
    // re-fire supplies it via --include).
    pub head_files: Vec<(String, String)>,
    // (path, contents) pairs; paths are repo-relative where possible.
    pub design_docs: Vec<(PathBuf, String)>,
    pub contracts_touched: Vec<(PathBuf, String)>,
    // prior rulings, oldest first: the judge's only persistent memory.
    pub ledger: Vec<(PathBuf, String)>,
    pub consumers: Vec<ConsumerHit>,
    // documents implicated by the diff's paths via .docpairs (ADR-0002 as
    // amended by the 2026-07-03 markup: literal path-prefix matching).
    pub implicated: Vec<ImplicatedDoc>,
}

#[derive(Debug)]
pub struct ConsumerHit {
    pub message: String,
    // "<repo>/<path>:<line>: <text>" — already citation-shaped for the ruling.
    pub citation: String,
}

#[derive(Debug)]
pub struct ImplicatedDoc {
    pub path: PathBuf,
    // None => the doc already rides above among the design docs; it is marked
    // implicated, not carried twice. Some(_) => its full content, included here.
    pub content: Option<String>,
}

// a .docpairs pairing: a literal path-prefix and the document it implicates.
#[derive(Debug)]
struct DocPair {
    prefix: String,
    doc: String,
}

// one roster entry from delightd's GET /projects: the fleet's own answer to
// "who is a consumer, and where does its checkout live".
// stack note: dead_code allow until the integration commit of this stack
// wires the roster into assemble().
#[allow(dead_code)]
#[derive(Debug)]
pub struct RosterEntry {
    pub name: String,
    pub path: String,
}

// run a subprocess and capture stdout; stderr becomes the error context.
fn run(cmd: &mut Command) -> Result<String> {
    let out = cmd
        .output()
        .with_context(|| format!("spawning {:?}", cmd))?;
    if !out.status.success() {
        bail!(
            "{:?} failed ({}): {}",
            cmd,
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// pure: parse delightd's GET /projects envelope into roster entries. the wire
// (delightd pkg/httpapi/httpapi.go, rosterResponse) is {status, projects[]},
// each entry a protojson registry.v1.Project with snake_case fields; only name
// and path are consumed here. a body without the envelope, or an entry missing
// name or path, is a loud error — a roster the judge cannot read is not a roster.
// stack note: dead_code allow until the integration commit of this stack.
#[allow(dead_code)]
fn parse_roster(body: &str) -> Result<Vec<RosterEntry>> {
    let v: serde_json::Value =
        serde_json::from_str(body).context("GET /projects body is not JSON")?;
    let projects = v
        .get("projects")
        .and_then(|p| p.as_array())
        .context("GET /projects body has no `projects` array")?;
    let mut out = Vec::with_capacity(projects.len());
    for p in projects {
        let name = p
            .get("name")
            .and_then(|n| n.as_str())
            .context("roster entry has no `name`")?;
        let path = p
            .get("path")
            .and_then(|n| n.as_str())
            .context("roster entry has no `path`")?;
        out.push(RosterEntry {
            name: name.to_string(),
            path: path.to_string(),
        });
    }
    Ok(out)
}

pub fn assemble(
    repo_path: &Path,
    pr_number: u64,
    cfg: &Config,
    include: &[String],
) -> Result<Inputs> {
    let work_root = Path::new(&cfg.work_root);
    let sprints_root = Path::new(&cfg.sprints_root);
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("repo path has no basename")?
        .to_string();

    // head sha + diff come from gh, scoped to the repo checkout.
    let head_sha = run(Command::new("gh")
        .args([
            "pr",
            "view",
            &pr_number.to_string(),
            "--json",
            "headRefOid",
            "-q",
            ".headRefOid",
        ])
        .current_dir(repo_path))?
    .trim()
    .to_string();
    if head_sha.is_empty() {
        bail!("could not resolve head sha for PR {pr_number}");
    }

    let diff = run(Command::new("gh")
        .args(["pr", "diff", &pr_number.to_string()])
        .current_dir(repo_path))?;
    if diff.trim().is_empty() {
        bail!("PR {pr_number} has an empty diff; nothing to rule on");
    }

    // the tree at the PR head, paths only: existence-evidence for doc claims
    // ("register.py exists at head" is citable; "trust me" is not). the head
    // commit is local — branches are pushed from this checkout — but fall
    // back to the remote API if it is not.
    let head_tree: Vec<String> = match run(Command::new("git")
        .args(["ls-tree", "-r", "--name-only", &head_sha])
        .current_dir(repo_path))
    {
        Ok(s) => s.lines().map(str::to_string).collect(),
        Err(_) => {
            let nwo = run(Command::new("gh")
                .args([
                    "repo",
                    "view",
                    "--json",
                    "nameWithOwner",
                    "-q",
                    ".nameWithOwner",
                ])
                .current_dir(repo_path))?
            .trim()
            .to_string();
            run(Command::new("gh")
                .args([
                    "api",
                    &format!("repos/{nwo}/git/trees/{head_sha}?recursive=1"),
                    "-q",
                    ".tree[] | select(.type==\"blob\") | .path",
                ])
                .current_dir(repo_path))?
            .lines()
            .map(str::to_string)
            .collect()
        }
    };
    if head_tree.is_empty() {
        bail!(
            "could not list the tree at head {head_sha}; the judge cannot verify existence claims"
        );
    }

    // operator-included file contents at head: judge-requested evidence.
    // a named file that does not exist at head is a loud error, not a skip —
    // supplying wrong evidence silently would corrupt the ruling.
    let mut head_files = Vec::new();
    for path in include {
        let content = run(Command::new("git")
            .args(["show", &format!("{head_sha}:{path}")])
            .current_dir(repo_path))
        .with_context(|| format!("--include {path}: not readable at head {head_sha}"))?;
        head_files.push((path.clone(), content));
    }

    // design docs: the repo's docs/ tree plus root-level DESIGN/VISION files,
    // plus the README — for small repos the README IS the design doc by
    // convention (magpie), and for the rest it is legitimate context. the
    // judge rules against the design as committed, not as remembered.
    let mut design_docs = Vec::new();
    for name in ["DESIGN.md", "VISION.md", "README.md"] {
        let p = repo_path.join(name);
        if let Ok(s) = std::fs::read_to_string(&p) {
            design_docs.push((PathBuf::from(name), s));
        }
    }
    let docs_dir = repo_path.join("docs");
    if docs_dir.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(&docs_dir)
            .with_context(|| format!("reading {}", docs_dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        entries.sort();
        for p in entries {
            let rel = PathBuf::from("docs").join(p.file_name().unwrap());
            let s =
                std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
            design_docs.push((rel, s));
        }
    }
    if design_docs.is_empty() {
        // a repo with no design artifact cannot be judged for design
        // conformance; that is a finding, not a default-pass.
        bail!("{repo_name} has no design docs (docs/*.md, DESIGN.md, VISION.md); T3 cannot run without a design artifact");
    }

    // implicated documents (ADR-0002, amended at the 2026-07-03 markup to
    // literal path-prefix matching): .docpairs pairs a path-prefix with a
    // document; a changed path that string-prefix-matches implicates the
    // document. read from the working copy on disk, the same ingestion as
    // design docs. an absent file is a valid "no pairings" declaration.
    let mut implicated = Vec::new();
    let docpairs_path = repo_path.join(".docpairs");
    match std::fs::read_to_string(&docpairs_path) {
        Ok(body) => {
            let pairs = parse_docpairs(&body)
                .with_context(|| format!("parsing {}", docpairs_path.display()))?;
            let changed = changed_paths(&diff);
            let design_paths: Vec<PathBuf> = design_docs.iter().map(|(p, _)| p.clone()).collect();
            implicated = resolve_implicated(
                &pairs,
                &changed,
                |doc| repo_path.join(doc).is_file(),
                |doc| {
                    let p = repo_path.join(doc);
                    std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))
                },
                &design_paths,
            )?;
        }
        // absent is the valid "no pairings" declaration; any OTHER read error
        // (permissions, IO) must not silently disarm doc pairing — fail closed.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "reading {} (present but unreadable)",
                    docpairs_path.display()
                )
            })
        }
    }

    // contract files touched by this diff, with their full current contents.
    let mut contracts_touched = Vec::new();
    for rel in changed_paths(&diff) {
        if rel.ends_with(".proto") {
            let p = repo_path.join(&rel);
            // a deleted proto still matters; record its absence explicitly.
            let contents = std::fs::read_to_string(&p)
                .unwrap_or_else(|_| "(file deleted in this diff)".to_string());
            contracts_touched.push((PathBuf::from(rel), contents));
        }
    }

    // the ledger: every prior ruling across all sprint dirs, oldest first.
    let mut ledger = Vec::new();
    let mut sprint_dirs: Vec<_> = std::fs::read_dir(sprints_root)
        .with_context(|| format!("reading {}", sprints_root.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("rulings").is_dir())
        .collect();
    sprint_dirs.sort();
    for sd in sprint_dirs {
        let mut rulings: Vec<_> = std::fs::read_dir(sd.join("rulings"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
            .collect();
        rulings.sort();
        for p in rulings {
            let s = std::fs::read_to_string(&p)?;
            ledger.push((p, s));
        }
    }

    // consumer scan: for every message type named in touched proto hunks,
    // rg the roster (excluding the repo under judgment) for uses. hits come
    // back citation-shaped so the judge can cite rather than assert.
    let mut consumers = Vec::new();
    for msg in changed_proto_messages(&diff) {
        for other in PILOT_ROSTER.iter().filter(|r| **r != repo_name) {
            let dir = work_root.join(other);
            if !dir.is_dir() {
                continue;
            }
            // rg exits 1 on no-matches; that is not an error here.
            let out = Command::new("rg")
                .args(["-n", "--no-heading", "-w", &msg])
                .args([
                    "-g",
                    "!*.pb.go",
                    "-g",
                    "!*_pb2.py*",
                    "-g",
                    "!gen/**",
                    "-g",
                    "!vendor/**",
                    "-g",
                    "!.venv/**",
                ])
                .arg(".")
                .current_dir(&dir)
                .output()
                .with_context(|| format!("rg in {}", dir.display()))?;
            if out.status.code() == Some(0) {
                for line in String::from_utf8_lossy(&out.stdout).lines().take(20) {
                    consumers.push(ConsumerHit {
                        message: msg.clone(),
                        citation: format!("{other}/{line}"),
                    });
                }
            }
        }
    }

    Ok(Inputs {
        repo_name,
        pr_number,
        head_sha,
        diff,
        head_tree,
        head_files,
        design_docs,
        contracts_touched,
        ledger,
        consumers,
        implicated,
    })
}

// pure: parse a .docpairs body into pairings. `#` comments and blank lines are
// ignored. a prefix bearing a glob metacharacter is a loud error: the map must
// migrate to literal path-prefixes (the 2026-07-03 markup ruling amending
// ADR-0002). silently never-matching a `**` suffix would defeat the pairing —
// the judge forces the migration instead.
fn parse_docpairs(body: &str) -> Result<Vec<DocPair>> {
    let mut pairs = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (prefix, doc) = line
            .split_once("->")
            .with_context(|| format!(".docpairs line is not '<prefix> -> <doc>': {line}"))?;
        let prefix = prefix.trim();
        let doc = doc.trim();
        if prefix.is_empty() || doc.is_empty() {
            bail!(".docpairs line is not '<prefix> -> <doc>': {line}");
        }
        if prefix.contains('*') || prefix.contains('?') {
            bail!(
                ".docpairs prefix '{prefix}' contains a glob metacharacter; the map must \
                 migrate to literal path-prefixes (matching is literal string-prefix, \
                 no glob semantics)"
            );
        }
        pairs.push(DocPair {
            prefix: prefix.to_string(),
            doc: doc.to_string(),
        });
    }
    Ok(pairs)
}

// pure: resolve which documents a diff implicates. matching is literal
// string-prefix (an exact file path is its own prefix); map order is preserved
// and a document implicated by two prefixes enters once. `doc_exists`/`read_doc`
// are the filesystem seam — real reads in assemble, fixtures in tests. a fired
// pair naming a missing document is a loud, fail-closed error. a document
// already carried among the design docs is marked (content None), not duplicated.
fn resolve_implicated(
    pairs: &[DocPair],
    changed: &[String],
    doc_exists: impl Fn(&str) -> bool,
    read_doc: impl Fn(&str) -> Result<String>,
    design_doc_paths: &[PathBuf],
) -> Result<Vec<ImplicatedDoc>> {
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for pair in pairs {
        if !changed.iter().any(|c| c.starts_with(&pair.prefix)) {
            continue;
        }
        if seen.contains(&pair.doc) {
            continue;
        }
        if !doc_exists(&pair.doc) {
            bail!("docpairs names missing doc: {}", pair.doc);
        }
        seen.push(pair.doc.clone());
        let already = design_doc_paths
            .iter()
            .any(|p| p.as_path() == Path::new(&pair.doc));
        let content = if already {
            None
        } else {
            Some(read_doc(&pair.doc)?)
        };
        out.push(ImplicatedDoc {
            path: PathBuf::from(&pair.doc),
            content,
        });
    }
    Ok(out)
}

// pure: repo-relative paths this unified diff touches ("+++ b/<path>").
pub fn changed_paths(diff: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            out.push(rest.trim().to_string());
        }
    }
    out
}

// pure: protobuf message names appearing in the diff's proto hunks. scans
// added/removed/context lines for `message <Name> {` — cheap and sufficient
// for the consumer scan; the authoritative breaking check is buf's job (T2),
// not this scan's.
pub fn changed_proto_messages(diff: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_proto = false;
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            in_proto = rest.trim().ends_with(".proto");
            continue;
        }
        if !in_proto {
            continue;
        }
        let body = line
            .strip_prefix('+')
            .or_else(|| line.strip_prefix('-'))
            .or_else(|| line.strip_prefix(' '))
            .unwrap_or(line)
            .trim_start();
        if let Some(rest) = body.strip_prefix("message ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_DIFF: &str = r#"diff --git a/proto/registry/v1/register.proto b/proto/registry/v1/register.proto
--- a/proto/registry/v1/register.proto
+++ b/proto/registry/v1/register.proto
@@ -10,6 +10,8 @@
 message Registration {
   string project_name = 1;
+  string flavor = 2;
 }
+message NotRegistered {
+  string reason = 1;
+}
diff --git a/pkg/httpapi/register.go b/pkg/httpapi/register.go
--- a/pkg/httpapi/register.go
+++ b/pkg/httpapi/register.go
@@ -1,3 +1,4 @@
+// message handling for registration
 package httpapi
"#;

    #[test]
    fn changed_paths_finds_both_files() {
        let paths = changed_paths(FAKE_DIFF);
        assert_eq!(
            paths,
            vec![
                "proto/registry/v1/register.proto".to_string(),
                "pkg/httpapi/register.go".to_string()
            ]
        );
    }

    #[test]
    fn proto_messages_found_only_in_proto_hunks() {
        let msgs = changed_proto_messages(FAKE_DIFF);
        // Registration appears as context, NotRegistered as an addition; the
        // go file's "// message handling" comment must NOT match.
        assert_eq!(
            msgs,
            vec!["Registration".to_string(), "NotRegistered".to_string()]
        );
    }

    #[test]
    fn no_proto_no_messages() {
        let diff = "+++ b/main.go\n+message Fake {\n";
        assert!(changed_proto_messages(diff).is_empty());
    }

    #[test]
    fn docpairs_ignores_comments_and_blanks() {
        let body = "# header\n\npkg/httpapi/ -> docs/api.md\n  # indented\nTaskfile.yml    -> docs/operations.md\n";
        let pairs = parse_docpairs(body).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].prefix, "pkg/httpapi/");
        assert_eq!(pairs[0].doc, "docs/api.md");
        assert_eq!(pairs[1].prefix, "Taskfile.yml");
    }

    #[test]
    fn docpairs_wildcard_prefix_bails() {
        // the delightd map still carries `**`; the judge must force migration.
        let err = parse_docpairs("pkg/httpapi/** -> docs/api.md\n").unwrap_err();
        assert!(
            err.to_string().contains("migrate to literal"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn implicated_prefix_and_exact_match() {
        // "pkg/httpapi/" fires by prefix; "Taskfile.yml" is its own prefix (an
        // exact-file match). neither is a design doc, so content is carried.
        let pairs =
            parse_docpairs("pkg/httpapi/ -> docs/api.md\nTaskfile.yml -> docs/operations.md\n")
                .unwrap();
        let changed = vec![
            "pkg/httpapi/register.go".to_string(),
            "Taskfile.yml".to_string(),
        ];
        let got = resolve_implicated(
            &pairs,
            &changed,
            |_| true,
            |d| Ok(format!("body of {d}")),
            &[],
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].path, PathBuf::from("docs/api.md"));
        assert_eq!(got[0].content.as_deref(), Some("body of docs/api.md"));
        assert_eq!(got[1].path, PathBuf::from("docs/operations.md"));
    }

    #[test]
    fn implicated_empty_when_nothing_matches() {
        // an unmatched map (and, equivalently, an absent .docpairs) yields no
        // implicated documents: a valid "no pairings" state, not an error.
        let pairs = parse_docpairs("config/ -> docs/operations.md\n").unwrap();
        let changed = vec!["pkg/httpapi/register.go".to_string()];
        let got =
            resolve_implicated(&pairs, &changed, |_| true, |_| Ok(String::new()), &[]).unwrap();
        assert!(got.is_empty());
        assert!(parse_docpairs("# only a comment\n").unwrap().is_empty());
    }

    #[test]
    fn implicated_missing_doc_bails() {
        let pairs = parse_docpairs("config/ -> docs/operations.md\n").unwrap();
        let changed = vec!["config/broker.yml".to_string()];
        let err = resolve_implicated(&pairs, &changed, |_| false, |_| Ok(String::new()), &[])
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "docpairs names missing doc: docs/operations.md"
        );
    }

    #[test]
    fn implicated_dedups_and_marks_existing_design_doc() {
        // two prefixes implicate the SAME doc, which already rides as a design
        // doc: one entry, marked (content None), read_doc never called.
        let pairs =
            parse_docpairs("config/ -> docs/operations.md\nTaskfile.yml -> docs/operations.md\n")
                .unwrap();
        let changed = vec!["config/broker.yml".to_string(), "Taskfile.yml".to_string()];
        let design = vec![PathBuf::from("docs/operations.md")];
        let got = resolve_implicated(
            &pairs,
            &changed,
            |_| true,
            |_| -> Result<String> { panic!("a marked design doc must not be re-read") },
            &design,
        )
        .unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, PathBuf::from("docs/operations.md"));
        assert!(got[0].content.is_none());
    }

    // delightd's GET /projects wire: {status, projects[]}, each entry a
    // protojson registry.v1.Project (snake_case, sparse). the fixture carries
    // fields beyond name/path to prove the parser reads the real shape, not a
    // trimmed one.
    const ROSTER_BODY: &str = r#"{"status":"ok","projects":[
        {"name":"delightd","path":"/w/delightd","essential":true,"deploy":{},"remote_url":"git@github.com:janearc/delightd"},
        {"name":"magpie","path":"/w/magpie","essential":false,"deploy":{}}
    ]}"#;

    #[test]
    fn roster_parses_the_projects_envelope() {
        let roster = parse_roster(ROSTER_BODY).unwrap();
        assert_eq!(roster.len(), 2);
        assert_eq!(roster[0].name, "delightd");
        assert_eq!(roster[0].path, "/w/delightd");
        assert_eq!(roster[1].name, "magpie");
        assert_eq!(roster[1].path, "/w/magpie");
    }
}
