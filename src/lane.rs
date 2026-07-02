// lane.rs — the rulings durability lane (sprint 2026-07-02-durability,
// issue 14; design + pseudocode ratified in rulings-lane-pseudocode.md).
//
// a ruling that exists only in the working tree dies with the laptop. this
// module commits each accepted ruling onto a dedicated ref
// (lane/<sprint-dir>) and pushes it, using plumbing against a temporary
// index so the operator's checkout — index, worktree, checked-out branch —
// is never touched.
//
// the lane NEVER blocks the gate: every failure here is data (Degraded),
// reported loud by the caller and mapped to exit 3, never a bail. a failed
// push self-heals: the commit sits on the local lane ref and the next push
// (next ruling, or sprint close) carries it.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

// per-invocation sequence for the temporary index name: invocations in one
// process (parallel tests) must not share an index file.
static LANE_INDEX_SEQ: AtomicU64 = AtomicU64::new(0);

// degradation is data the caller reports, not an error it propagates.
#[derive(Debug, PartialEq)]
pub enum LaneOutcome {
    Ok,
    Degraded { step: &'static str, why: String },
}

// one git call, -C root, argv only — no shell anywhere in the chain, so the
// string-building upstream constructs arguments, never commands.
fn git(root: &Path, index_file: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    if let Some(idx) = index_file {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn lane_commit(
    sprints_root: &Path,
    sprint_dir: &str,
    ruling_path: &Path,
    ledger_id: &str,
) -> LaneOutcome {
    let lane = format!("lane/{sprint_dir}");
    let lane_ref = format!("refs/heads/{lane}");

    // degrade on the first failed step, naming it; never bail.
    macro_rules! step {
        ($name:literal, $call:expr) => {
            match $call {
                Ok(v) => v,
                Err(why) => return LaneOutcome::Degraded { step: $name, why },
            }
        };
    }

    // parent: the lane tip if the ref exists, else main (first ruling of
    // the sprint forks the lane off main).
    let parent = match git(
        sprints_root,
        None,
        &["rev-parse", "--verify", "--quiet", &lane_ref],
    ) {
        Ok(sha) => sha,
        Err(_) => step!(
            "rev-parse",
            git(sprints_root, None, &["rev-parse", "--verify", "main"])
        ),
    };

    // temporary index: the operator's real index is never touched. a stale
    // leftover from a crashed run is removed, not reused.
    let tmp_index = std::env::temp_dir().join(format!(
        "judge-lane-index-{}-{}",
        std::process::id(),
        LANE_INDEX_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&tmp_index);
    let idx = Some(tmp_index.as_path());

    // in-repo path of the ruling: <sprint>/rulings/<fname>. both halves are
    // harness-constructed (no spaces by construction); no user text enters
    // the chain.
    let fname = match ruling_path.file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => {
            return LaneOutcome::Degraded {
                step: "ruling-path",
                why: format!("no file name in {}", ruling_path.display()),
            }
        }
    };
    let in_repo = format!("{sprint_dir}/rulings/{fname}");
    let ruling_abs = ruling_path.to_string_lossy();

    let blob = step!(
        "hash-object",
        git(sprints_root, None, &["hash-object", "-w", &ruling_abs])
    );
    step!("read-tree", git(sprints_root, idx, &["read-tree", &parent]));
    step!(
        "update-index",
        git(
            sprints_root,
            idx,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob},{in_repo}"),
            ],
        )
    );
    let tree = step!("write-tree", git(sprints_root, idx, &["write-tree"]));
    let commit = step!(
        "commit-tree",
        git(
            sprints_root,
            None,
            &[
                "commit-tree",
                &tree,
                "-p",
                &parent,
                "-m",
                &format!("ruling: {ledger_id}")
            ],
        )
    );
    step!(
        "update-ref",
        git(sprints_root, None, &["update-ref", &lane_ref, &commit])
    );
    let _ = std::fs::remove_file(&tmp_index);

    // durable locally from here: a push failure below still degrades, but
    // the next push carries this commit (a ref tip carries its history).
    step!("push", git(sprints_root, None, &["push", "origin", &lane]));

    LaneOutcome::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SPRINT: &str = "2026-01-01-test";

    // run a git command in the fixture, asserting success.
    fn run(repo: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    // a fake sprints repo with one commit on main, a sprint rulings dir,
    // and (optionally) a local bare remote — no network anywhere.
    fn fixture(tag: &str, with_remote: bool) -> PathBuf {
        let root = std::env::temp_dir().join(format!("judge-lane-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("sprints");
        std::fs::create_dir_all(repo.join(SPRINT).join("rulings")).unwrap();
        run(&repo, &["init", "-q", "-b", "main"]);
        run(&repo, &["config", "user.email", "judge-test@invalid"]);
        run(&repo, &["config", "user.name", "judge-test"]);
        std::fs::write(repo.join("README.md"), "fixture").unwrap();
        run(&repo, &["add", "README.md"]);
        run(&repo, &["commit", "-q", "-m", "init"]);
        if with_remote {
            let bare = root.join("remote.git");
            let out = Command::new("git")
                .args(["init", "-q", "--bare"])
                .arg(&bare)
                .output()
                .unwrap();
            assert!(out.status.success());
            run(&repo, &["remote", "add", "origin", bare.to_str().unwrap()]);
        }
        repo
    }

    fn write_ruling(repo: &Path, name: &str) -> PathBuf {
        let p = repo.join(SPRINT).join("rulings").join(name);
        std::fs::write(&p, format!("ruling: {name}\n")).unwrap();
        p
    }

    #[test]
    fn lands_on_the_lane_ref_and_pushes() {
        let repo = fixture("lands", true);
        let ruling = write_ruling(&repo, "r1.yaml");
        let got = lane_commit(&repo, SPRINT, &ruling, "test/rulings/r1.yaml");
        assert_eq!(got, LaneOutcome::Ok);
        // the ref exists and its tree holds the ruling at the in-repo path
        let tip = run(&repo, &["rev-parse", &format!("refs/heads/lane/{SPRINT}")]);
        let content = run(
            &repo,
            &["show", &format!("lane/{SPRINT}:{SPRINT}/rulings/r1.yaml")],
        );
        assert_eq!(content, "ruling: r1.yaml");
        // and the push happened: remote tip matches local tip
        let remote = run(
            &repo,
            &["ls-remote", "origin", &format!("refs/heads/lane/{SPRINT}")],
        );
        assert!(remote.starts_with(&tip), "remote {remote} != local {tip}");
    }

    #[test]
    fn parent_chains_across_two_rulings() {
        let repo = fixture("chain", true);
        lane_commit(&repo, SPRINT, &write_ruling(&repo, "r1.yaml"), "id-1");
        let first = run(&repo, &["rev-parse", &format!("lane/{SPRINT}")]);
        lane_commit(&repo, SPRINT, &write_ruling(&repo, "r2.yaml"), "id-2");
        let parent = run(&repo, &["rev-parse", &format!("lane/{SPRINT}^")]);
        assert_eq!(parent, first, "second ruling must chain, not reset");
    }

    #[test]
    fn degraded_push_still_commits_locally() {
        let repo = fixture("degraded", false); // no remote: the hostile network
        let ruling = write_ruling(&repo, "r1.yaml");
        match lane_commit(&repo, SPRINT, &ruling, "id-1") {
            LaneOutcome::Degraded { step: "push", .. } => {}
            other => panic!("expected push degradation, got {other:?}"),
        }
        // nothing lost: the local lane ref advanced and the file is on disk
        run(
            &repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/lane/{SPRINT}"),
            ],
        );
        assert!(ruling.is_file());
    }

    #[test]
    fn operator_state_untouched() {
        let repo = fixture("untouched", true);
        // the ruling exists first so it reads identically (untracked) in
        // both snapshots; then dirty the worktree and stage something in
        // the REAL index
        let ruling = write_ruling(&repo, "r1.yaml");
        std::fs::write(repo.join("README.md"), "operator edit in flight").unwrap();
        std::fs::write(repo.join("staged.txt"), "staged").unwrap();
        run(&repo, &["add", "staged.txt"]);
        let before = run(&repo, &["status", "--porcelain"]);
        let branch_before = run(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
        lane_commit(&repo, SPRINT, &ruling, "id-1");
        let after = run(&repo, &["status", "--porcelain"]);
        let branch_after = run(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
        // everything the operator had in flight is byte-identical
        assert_eq!(before, after);
        assert_eq!(branch_before, branch_after);
    }
}
