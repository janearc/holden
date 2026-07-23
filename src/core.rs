// the pipeline, extracted from the CLI's main so the service can call it:
// assemble -> rule (fresh judge, or operator overrule) -> ledger -> lane ->
// status. ordering is ratified: ledger first, lane second, status third —
// the status carries the ledger id, and by the time it posts the entry is
// durable off-laptop or loudly known not to be. the lane never blocks the
// gate.

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::{assemble, lane, publish, ruling, spawn};

// the single config boundary (ratified 2026-07-03): every environment-
// derived fact resolves ONCE, flag over env over default, into this
// struct. deliberately carries NO credential of any kind: the spawned
// judge CLI owns auth, so a secret never enters config, env handling, or
// docs.
pub struct Config {
    pub delightd_url: String,
    pub sprints_root: String,
    pub judge_cmd: String,
    pub model: Option<String>,
    // the workstation home, resolved once here per the boundary above: the
    // roster's ~-prefixed paths are workstation-home-relative by contract.
    pub home: String,
}

// one fact's resolution: the flag wins, else the env var, else the default.
pub fn pick(flag: Option<String>, env: &str, default: &str) -> String {
    flag.or_else(|| std::env::var(env).ok())
        .unwrap_or_else(|| default.to_string())
}

// a path fact whose default lives under HOME — no operator's absolute path
// is hard-coded. needing the default while HOME is unset is a loud error,
// never a guessed path.
pub fn pick_path(flag: Option<String>, env: &str, rel: &str) -> anyhow::Result<String> {
    if let Some(v) = flag.or_else(|| std::env::var(env).ok()) {
        return Ok(v);
    }
    let home = std::env::var("HOME").with_context(|| {
        format!("resolving the {env} default: HOME is unset and no flag or env supplied")
    })?;
    Ok(format!("{home}/{rel}"))
}

// how a ruling is decided: a fresh judge, or the operator's overrule with
// its reason (an overrule is data, ADR D6).
pub enum Decide {
    FreshJudge,
    Overrule { reason: String },
}

pub struct RunOpts {
    pub decide: Decide,
    pub includes: Vec<String>,
    pub skip_status: bool,
    pub skip_lane: bool,
}

// the lifecycle, told as it happens. the service maps these onto
// holden.ruling.v1 RulingState; the CLI ignores them (its prints live at
// the call sites it always had).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    InputsAssembled,
    JudgeSpawned,
    Verdict,
    Published,
}

pub struct RunOutcome {
    pub doc: ruling::RulingDoc,
    pub ledger_path: PathBuf,
    // Some((step, why)) when the durability lane degraded — loud, never a
    // bail; the ruling already earned its merge.
    pub lane_degraded: Option<(String, String)>,
    pub status_posted: bool,
}

// run one ruling to its landing. `progress` is told each stage as it
// begins to matter; errors are loud and specific, and an absent ruling
// (refused twice) is an Err, exactly as the CLI has always treated it.
pub fn run(
    cfg: &Config,
    repo_path: &Path,
    pr_number: u64,
    opts: &RunOpts,
    progress: &mut dyn FnMut(Stage),
) -> anyhow::Result<RunOutcome> {
    let sprints_root = Path::new(&cfg.sprints_root);

    let inputs = assemble::assemble(repo_path, pr_number, cfg, &opts.includes)?;
    progress(Stage::InputsAssembled);

    let mut doc = match &opts.decide {
        Decide::Overrule { reason } => overrule_ruling(&inputs, reason),
        Decide::FreshJudge => {
            progress(Stage::JudgeSpawned);
            let spawn_cfg = spawn::SpawnCfg {
                judge_cmd: cfg.judge_cmd.clone(),
                model: cfg.model.clone(),
            };
            spawn::rule(&spawn_cfg, &inputs)?.0
        }
    };
    progress(Stage::Verdict);

    let ledger_path =
        publish::write_ledger(sprints_root, &inputs.repo_name, inputs.pr_number, &mut doc)?;

    let mut lane_degraded = None;
    if !opts.skip_lane {
        let ledger_id = doc
            .ruling
            .ledger_entry_id
            .as_deref()
            .expect("write_ledger assigns the ledger id");
        // the ledger id IS the sprint-relative path; its first component is
        // the sprint dir the lane ref is named after
        let sprint_dir = ledger_id
            .split('/')
            .next()
            .expect("ledger id starts with the sprint dir");
        if let lane::LaneOutcome::Degraded { step, why } =
            lane::lane_commit(sprints_root, sprint_dir, &ledger_path, ledger_id)
        {
            lane_degraded = Some((step.to_string(), why.to_string()));
        }
    }

    let mut status_posted = false;
    if !opts.skip_status {
        publish::post_status(repo_path, &inputs.head_sha, &doc)?;
        status_posted = true;
    }
    progress(Stage::Published);

    Ok(RunOutcome {
        doc,
        ledger_path,
        lane_degraded,
        status_posted,
    })
}

// an operator overrule as a first-class ruling: verdict ratify, the
// overrule named as a divergence, the operator as the instance. it reads
// honestly in the ledger — nobody mistakes it for a judge's opinion.
pub fn overrule_ruling(inputs: &assemble::Inputs, reason: &str) -> ruling::RulingDoc {
    ruling::RulingDoc {
        ruling: ruling::Ruling {
            diff_ref: format!(
                "{} pr {} @ {}",
                inputs.repo_name, inputs.pr_number, inputs.head_sha
            ),
            judge_instance: format!(
                "operator-overrule-{}",
                chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
            ),
            fired_at: chrono::Utc::now(),
            verdict: ruling::Verdict::Ratify,
            divergences: vec![ruling::Divergence {
                claim: "operator overrule of the judge's bounce".into(),
                necessary: true,
                justification: reason.to_string(),
            }],
            shape_verdict: ruling::ShapeVerdict::OnMesh,
            shape_justification: format!("operator overrule: {reason}"),
            consumer_impact: vec![],
            doc_content_agreement: ruling::DocContentAgreement::Unclear,
            ledger_entry_id: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{pick, pick_path};

    #[test]
    fn pick_path_defaults_under_home() {
        // a var name nothing else touches; set and removed inside this test.
        let var = "JUDGE_TEST_PICK_PATH_VAR";
        std::env::remove_var(var);
        let home = std::env::var("HOME").expect("test env has HOME");
        assert_eq!(
            pick_path(None, var, "work").unwrap(),
            format!("{home}/work")
        );
        std::env::set_var(var, "/elsewhere");
        assert_eq!(pick_path(None, var, "work").unwrap(), "/elsewhere");
        std::env::remove_var(var);
    }

    #[test]
    fn pick_resolves_flag_over_env_over_default() {
        let var = "JUDGE_TEST_PICK_VAR";
        std::env::remove_var(var);
        assert_eq!(pick(None, var, "dflt"), "dflt");
        std::env::set_var(var, "from-env");
        assert_eq!(pick(None, var, "dflt"), "from-env");
        assert_eq!(pick(Some("from-flag".into()), var, "dflt"), "from-flag");
        std::env::remove_var(var);
    }
}
