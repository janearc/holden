// judge — ADR-0001 judgment-gate harness (sprint 2026-07-02-judge).
//
// full flow per the sprint's T0 design:
//   assemble (D6 inputs) -> spawn fresh judge -> refuse-or-accept (schema) ->
//   write the ledger entry (sprints repo, never the target repo) -> post the
//   ruling/ratify commit status to the head sha.

mod assemble;
mod lane;
mod publish;
mod ruling;
mod spawn;

use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "judge",
    about = "assemble inputs, spawn a fresh judge, refuse off-spec rulings, write the ledger, post the status"
)]
struct Args {
    /// validate a ruling YAML file and exit (0 = valid, 1 = refused); no PR needed
    #[arg(long, value_name = "FILE")]
    validate_ruling: Option<String>,
    /// path to the repo checkout under judgment
    #[arg(required_unless_present = "validate_ruling", default_value = "")]
    repo_path: String,
    /// pull request number to rule on
    #[arg(required_unless_present = "validate_ruling", default_value_t = 0)]
    pr_number: u64,
    /// operator overrule: record an overrule ruling before posting (an overrule is data)
    #[arg(long)]
    overrule: bool,
    /// reason for the overrule; required with --overrule, becomes ledger evidence
    #[arg(long, requires = "overrule")]
    reason: Option<String>,
    /// assemble and summarize the judge's inputs without spawning a judge
    #[arg(long)]
    dry_run: bool,
    /// include a file's head content in the bundle (repeatable); the supply
    /// side of a judge's needs-clarification evidence request
    #[arg(long = "include", value_name = "PATH")]
    include: Vec<String>,
    /// write the ledger but post no status (rehearsal / hostile-network mode)
    #[arg(long)]
    skip_status: bool,
    /// skip the durability lane (no commit/push of the ruling); for a truly
    /// local rehearsal — rehearsal mode otherwise keeps the lane on
    #[arg(long)]
    skip_lane: bool,
    /// judge executable (tests stub this); flag over env JUDGE_CMD over default claude
    #[arg(long)]
    judge_cmd: Option<String>,
    /// model override passed to the judge; flag over env JUDGE_MODEL over the CLI's configured model
    #[arg(long)]
    model: Option<String>,
    /// delightd control-port base URL for the roster; flag over env JUDGE_DELIGHTD_URL over default
    #[arg(long)]
    delightd_url: Option<String>,
    /// root of the sprints repo (ledger + ruling output); flag over env JUDGE_SPRINTS_ROOT over default
    #[arg(long)]
    sprints_root: Option<String>,
}

// the single config boundary (ratified 2026-07-03): every environment-derived
// fact the judge consumes resolves ONCE at startup, flag over env over default,
// into this struct — the container-readiness posture, and nothing more built.
// deliberately carries NO credential of any kind: the spawned `claude` CLI owns
// auth, so a secret never enters the judge's config, env handling, or docs.
struct Config {
    delightd_url: String,
    sprints_root: String,
    judge_cmd: String,
    model: Option<String>,
}

impl Config {
    fn resolve(args: &Args) -> anyhow::Result<Config> {
        Ok(Config {
            // the default is delightd's DefaultControlPort — the documented
            // single source of the default (pinned at the 2026-07-03 markup).
            delightd_url: pick(
                args.delightd_url.clone(),
                "JUDGE_DELIGHTD_URL",
                "http://127.0.0.1:8088",
            ),
            sprints_root: pick_path(
                args.sprints_root.clone(),
                "JUDGE_SPRINTS_ROOT",
                "work/sprints",
            )?,
            judge_cmd: pick(args.judge_cmd.clone(), "JUDGE_CMD", "claude"),
            // model has no default: absent means the CLI's own configured model.
            model: args
                .model
                .clone()
                .or_else(|| std::env::var("JUDGE_MODEL").ok()),
        })
    }
}

// one fact's resolution: the flag wins, else the env var, else the default.
fn pick(flag: Option<String>, env: &str, default: &str) -> String {
    flag.or_else(|| std::env::var(env).ok())
        .unwrap_or_else(|| default.to_string())
}

// a path fact whose default lives under HOME — no operator's absolute path is
// hard-coded ("work" is just the name this fleet uses for its checkout root).
// needing the default while HOME is unset is a loud error, never a guessed path.
fn pick_path(flag: Option<String>, env: &str, rel: &str) -> anyhow::Result<String> {
    if let Some(v) = flag.or_else(|| std::env::var(env).ok()) {
        return Ok(v);
    }
    let home = std::env::var("HOME").with_context(|| {
        format!("resolving the {env} default: HOME is unset and no flag or env supplied")
    })?;
    Ok(format!("{home}/{rel}"))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // standalone ruling validation: the local half of the ruling-present
    // check. validates LEDGER ENTRIES (id required) — judge output is
    // validated inside the spawn path, not here.
    if let Some(path) = &args.validate_ruling {
        let yaml = std::fs::read_to_string(path)?;
        match ruling::parse_ledger_entry(&yaml) {
            Ok(doc) => {
                println!(
                    "VALID: verdict={:?} shape={:?}",
                    doc.ruling.verdict, doc.ruling.shape_verdict
                );
                return Ok(());
            }
            Err(e) => {
                eprintln!("REFUSED: {e}");
                std::process::exit(1);
            }
        }
    }

    // resolve every environment-derived fact through the one config boundary.
    let cfg = Config::resolve(&args)?;
    let repo = std::path::Path::new(&args.repo_path);
    let sprints_root = std::path::Path::new(&cfg.sprints_root);

    let inputs = assemble::assemble(repo, args.pr_number, &cfg, &args.include)?;

    if args.dry_run {
        // a human-readable audit of exactly what the judge would receive.
        println!("repo:          {}", inputs.repo_name);
        println!(
            "pr:            {} (head {})",
            inputs.pr_number,
            &inputs.head_sha[..12.min(inputs.head_sha.len())]
        );
        println!("diff:          {} lines", inputs.diff.lines().count());
        println!(
            "design docs:   {}",
            inputs
                .design_docs
                .iter()
                .map(|(p, _)| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "contracts:     {}",
            if inputs.contracts_touched.is_empty() {
                "(none touched)".into()
            } else {
                inputs
                    .contracts_touched
                    .iter()
                    .map(|(p, _)| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        println!(
            "implicated:    {}",
            if inputs.implicated.is_empty() {
                "(none matched .docpairs)".into()
            } else {
                inputs
                    .implicated
                    .iter()
                    .map(|d| d.path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        println!("ledger:        {} prior ruling(s)", inputs.ledger.len());
        println!("consumer hits: {}", inputs.consumers.len());
        for c in &inputs.consumers {
            println!("  [{}] {}", c.message, c.citation);
        }
        return Ok(());
    }

    // the ruling: an operator overrule constructs one (an overrule is data,
    // ADR D6); otherwise a fresh judge is spawned and its output must survive
    // the schema or the ruling is absent.
    let (mut doc, _raw) = if args.overrule {
        let reason = args
            .reason
            .clone()
            .expect("clap enforces --reason with --overrule");
        (overrule_ruling(&inputs, &reason), String::new())
    } else {
        let spawn_cfg = spawn::SpawnCfg {
            judge_cmd: cfg.judge_cmd.clone(),
            model: cfg.model.clone(),
        };
        spawn::rule(&spawn_cfg, &inputs)?
    };

    // ledger first, lane second, status third (ratified ordering): the
    // status carries the ledger id, and by the time it posts the entry is
    // already durable off-laptop — or loudly known not to be. the lane
    // never blocks the gate: degraded is loud + exit 3, not a bail.
    let path = publish::write_ledger(sprints_root, &inputs.repo_name, inputs.pr_number, &mut doc)?;
    println!("ledger: {}", path.display());

    let mut lane_outcome = lane::LaneOutcome::Ok;
    if !args.skip_lane {
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
        lane_outcome = lane::lane_commit(sprints_root, sprint_dir, &path, ledger_id);
        match &lane_outcome {
            lane::LaneOutcome::Ok => println!("lane: {ledger_id} -> lane/{sprint_dir}"),
            lane::LaneOutcome::Degraded { step, why } => {
                eprintln!("lane: DEGRADED — {step}: {why}");
            }
        }
    }

    if args.skip_status {
        println!(
            "status: SKIPPED (--skip-status); verdict was {:?}",
            doc.ruling.verdict
        );
    } else {
        publish::post_status(repo, &inputs.head_sha, &doc)?;
        println!(
            "status: ruling/ratify -> {:?} on {} (pr {})",
            doc.ruling.verdict,
            &inputs.head_sha[..12.min(inputs.head_sha.len())],
            inputs.pr_number
        );
    }

    // exit 3: ruling written (and status posted, unless skipped) but the
    // lane is degraded — loud and machine-legible without stalling a merge
    // the ruling already earned.
    if !matches!(lane_outcome, lane::LaneOutcome::Ok) {
        std::process::exit(3);
    }
    Ok(())
}

// an operator overrule as a first-class ruling: verdict ratify, the overrule
// named as a divergence, the operator as the instance. it reads honestly in
// the ledger — nobody mistakes it for a judge's opinion.
fn overrule_ruling(inputs: &assemble::Inputs, reason: &str) -> ruling::RulingDoc {
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
        // a var name nothing else touches; set and removed inside this one
        // test, so parallel tests never race on it.
        let var = "JUDGE_TEST_PICK_VAR";
        std::env::remove_var(var);
        assert_eq!(pick(None, var, "dflt"), "dflt");
        std::env::set_var(var, "from-env");
        assert_eq!(pick(None, var, "dflt"), "from-env");
        assert_eq!(pick(Some("from-flag".into()), var, "dflt"), "from-flag");
        std::env::remove_var(var);
    }
}
