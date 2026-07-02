// publish.rs — ledger write + commit status (ADR-0001 D5-D6, T0 flow 5-6).
//
// the ruling artifact stays home (the private sprints repo); the only trace
// on GitHub is the status on the head sha. the ledger id is assigned HERE —
// a judge that tries to assign one is refused upstream by the schema.

use crate::ruling::{RulingDoc, Verdict};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// the active sprint = lexicographically last dir with a SPRINT.md that is
// not yet promoted (-completed). resolution stays an explicit git mv; this
// just finds where in-flight artifacts belong.
pub fn active_sprint(sprints_root: &Path) -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(sprints_root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.join("SPRINT.md").is_file()
                && !p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with("-completed"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .context("no active (un-promoted) sprint dir found; cut a sprint first")
}

// write the ruling into <active-sprint>/rulings/, assigning the ledger id.
// returns the path written.
pub fn write_ledger(
    sprints_root: &Path,
    repo_name: &str,
    pr_number: u64,
    doc: &mut RulingDoc,
) -> Result<PathBuf> {
    let sprint = active_sprint(sprints_root)?;
    let dir = sprint.join("rulings");
    std::fs::create_dir_all(&dir)?;

    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H%M%SZ");
    let fname = format!("{stamp}-{repo_name}-pr{pr_number}.yaml");
    let path = dir.join(&fname);
    if path.exists() {
        // same repo+pr+second: astronomically unlikely by hand; refuse rather
        // than overwrite a ledger entry. ledger entries are append-only.
        bail!("ledger entry already exists: {}", path.display());
    }

    // the id IS the sprint-relative path: unique, greppable, human-legible.
    let sprint_name = sprint.file_name().unwrap().to_string_lossy();
    doc.ruling.ledger_entry_id = Some(format!("{sprint_name}/rulings/{fname}"));

    let yaml = serde_yaml::to_string(doc).context("serializing ruling for the ledger")?;
    std::fs::write(&path, yaml).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

// post the commit status (context ruling/ratify) to the head sha. state maps
// per the ratified T0 call: ratify=success, bounce=failure,
// needs-clarification=pending.
pub fn post_status(repo_path: &Path, head_sha: &str, doc: &RulingDoc) -> Result<()> {
    let nwo = String::from_utf8(
        Command::new("gh")
            .args([
                "repo",
                "view",
                "--json",
                "nameWithOwner",
                "-q",
                ".nameWithOwner",
            ])
            .current_dir(repo_path)
            .output()
            .context("resolving nameWithOwner")?
            .stdout,
    )?
    .trim()
    .to_string();
    if nwo.is_empty() {
        bail!("could not resolve owner/repo for {}", repo_path.display());
    }

    let (state, description) = match doc.ruling.verdict {
        Verdict::Ratify => ("success", "judge: ratified"),
        Verdict::Bounce => ("failure", "judge: bounced — see the sprint ledger"),
        Verdict::NeedsClarification => ("pending", "judge: needs clarification from the operator"),
    };

    let ledger_ref = doc
        .ruling
        .ledger_entry_id
        .as_deref()
        .unwrap_or("(unwritten)");

    let out = Command::new("gh")
        .args([
            "api",
            &format!("repos/{nwo}/statuses/{head_sha}"),
            "-f",
            &format!("state={state}"),
            "-f",
            "context=ruling/ratify",
            "-f",
            &format!("description={description} [{ledger_ref}]"),
        ])
        .current_dir(repo_path)
        .output()
        .context("posting commit status")?;
    if !out.status.success() {
        bail!(
            "status post failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_sprint_prefers_unpromoted_latest() {
        let root = std::env::temp_dir().join(format!("judge-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, has_doc) in [
            ("2026-07-01-completed", true),
            ("2026-07-02-completed", true),
            ("2026-07-02-judge", true),
            ("tools", false), // no SPRINT.md: never a candidate
        ] {
            let d = root.join(dir);
            std::fs::create_dir_all(&d).unwrap();
            if has_doc {
                std::fs::write(d.join("SPRINT.md"), "x").unwrap();
            }
        }
        let got = active_sprint(&root).unwrap();
        assert!(got.ends_with("2026-07-02-judge"), "got {}", got.display());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_active_sprint_is_loud() {
        let root = std::env::temp_dir().join(format!("judge-test-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("2026-07-01-completed")).unwrap();
        std::fs::write(root.join("2026-07-01-completed/SPRINT.md"), "x").unwrap();
        assert!(active_sprint(&root).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
