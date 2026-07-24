// judge — ADR-0001 judgment-gate harness (sprint 2026-07-02-judge), now a
// thin client of the crate's core (ADR-0003 step 2: the CLI remains the
// rehearsal and hostile-network escape hatch while holdend serves).
//
// full flow per the sprint's T0 design:
//   assemble (D6 inputs) -> spawn fresh judge -> refuse-or-accept (schema) ->
//   write the ledger entry (sprints repo, never the target repo) -> post the
//   ruling/ratify commit status to the head sha.

use anyhow::Context;
use clap::Parser;

use judge::core::{self, Config, Decide, RunOpts};
use judge::{assemble, bundle, ruling, spawn};

#[derive(Parser, Debug)]
#[command(
    name = "judge",
    about = "assemble inputs, spawn a fresh judge, refuse off-spec rulings, write the ledger, post the status"
)]
struct Args {
    /// validate a ruling YAML file and exit (0 = valid, 1 = refused); no PR needed
    #[arg(long, value_name = "FILE")]
    validate_ruling: Option<String>,
    /// replay a recorded bundle through a fresh judge: same bundle, fresh
    /// judge, no GitHub, no delightd; the ruling lands beside the bundle
    /// under replays/, never in the ledger, never as a status
    #[arg(long, value_name = "BUNDLE_DIR", conflicts_with = "validate_ruling")]
    replay: Option<String>,
    /// path to the repo checkout under judgment
    #[arg(required_unless_present_any = ["validate_ruling", "replay"], default_value = "")]
    repo_path: String,
    /// pull request number to rule on
    #[arg(required_unless_present_any = ["validate_ruling", "replay"], default_value_t = 0)]
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
    /// assemble and record the bundle bento, then exit without judging —
    /// the corpus seeder (a dry run that persists)
    #[arg(long, conflicts_with_all = ["dry_run", "overrule", "replay"])]
    record_only: bool,
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
    /// how a prompt reaches a judge: "shim" (claude CLI subprocess) or
    /// "hahod" (the haho client, RFC section 7 step 3); flag over env
    /// JUDGE_VIA over default shim until the rehearsal proof flips it
    #[arg(long)]
    via: Option<String>,
    /// hahod loopback base URL; flag over env JUDGE_HAHOD_URL over default
    #[arg(long)]
    hahod_url: Option<String>,
    /// ENV VAR NAME the JobSpec names for the chute's credential (never a
    /// value); flag over env JUDGE_TOKEN_ENV over default HOLDEN_ANTHROPIC_KEY
    #[arg(long)]
    token_env: Option<String>,
    /// model override passed to the judge; flag over env JUDGE_MODEL over the CLI's configured model
    #[arg(long)]
    model: Option<String>,
    /// delightd control-port base URL for the roster; flag over env JUDGE_DELIGHTD_URL over default
    #[arg(long)]
    delightd_url: Option<String>,
    /// root of the sprints repo (ledger + ruling output); flag over env JUDGE_SPRINTS_ROOT over default
    #[arg(long)]
    sprints_root: Option<String>,
    /// root of the bundle corpus (recorded bundle bentos); flag over env JUDGE_BUNDLE_ROOT over default
    #[arg(long)]
    bundle_root: Option<String>,
}

fn resolve(args: &Args) -> anyhow::Result<Config> {
    Ok(Config {
        // the default is delightd's DefaultControlPort — the documented
        // single source of the default (pinned at the 2026-07-03 markup).
        delightd_url: core::pick(
            args.delightd_url.clone(),
            "JUDGE_DELIGHTD_URL",
            "http://127.0.0.1:8088",
        ),
        sprints_root: core::pick_path(
            args.sprints_root.clone(),
            "JUDGE_SPRINTS_ROOT",
            "work/sprints",
        )?,
        bundle_root: core::pick_path(
            args.bundle_root.clone(),
            "JUDGE_BUNDLE_ROOT",
            ".holden/bundles",
        )?,
        judge_cmd: core::pick(args.judge_cmd.clone(), "JUDGE_CMD", "claude"),
        // model has no default: absent means the CLI's own configured model
        // (the hahod path refuses absence loudly at spawn).
        model: args
            .model
            .clone()
            .or_else(|| std::env::var("JUDGE_MODEL").ok()),
        judge_via: core::pick(args.via.clone(), "JUDGE_VIA", "shim"),
        hahod_url: core::pick(
            args.hahod_url.clone(),
            "JUDGE_HAHOD_URL",
            "http://127.0.0.1:8790",
        ),
        judge_token_env: core::pick(
            args.token_env.clone(),
            "JUDGE_TOKEN_ENV",
            "HOLDEN_ANTHROPIC_KEY",
        ),
        home: std::env::var("HOME").context("resolving the workstation home: HOME is unset")?,
    })
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

    let cfg = resolve(&args)?;
    let repo = std::path::Path::new(&args.repo_path);

    // replay: same bundle, fresh judge (RFC section 5). the bundle carries
    // every input, so no GitHub and no delightd are reached; the verdict
    // lands beside the bundle as experiment evidence — never in the ledger,
    // never as a status. a replay cannot gate anything.
    if let Some(dir) = &args.replay {
        let dir = std::path::Path::new(dir);
        let inputs = bundle::load(dir)?;
        let spawn_cfg = spawn::SpawnCfg {
            judge_cmd: cfg.judge_cmd.clone(),
            model: cfg.model.clone(),
            via: spawn::Via::parse(&cfg.judge_via)?,
            hahod_url: cfg.hahod_url.clone(),
            token_env: cfg.judge_token_env.clone(),
        };
        let (doc, yaml) = spawn::rule(&spawn_cfg, &inputs)?;
        let out = bundle::record_replay(dir, &yaml, &doc.ruling.judge_instance)?;
        println!(
            "replay: verdict={:?} shape={:?} ({} pr {} @ {})",
            doc.ruling.verdict,
            doc.ruling.shape_verdict,
            inputs.repo_name,
            inputs.pr_number,
            &inputs.head_sha[..12.min(inputs.head_sha.len())]
        );
        println!("recorded: {}", out.display());
        return Ok(());
    }

    if args.dry_run {
        // a human-readable audit of exactly what the judge would receive.
        let inputs = assemble::assemble(repo, args.pr_number, &cfg, &args.include)?;
        print_dry_run(&inputs);
        return Ok(());
    }

    // the corpus seeder: assemble and record, judge nothing. the bundle has
    // no ruling-ref, which is exactly how a reconstruction reads.
    if args.record_only {
        let inputs = assemble::assemble(repo, args.pr_number, &cfg, &args.include)?;
        let dir = bundle::record(std::path::Path::new(&cfg.bundle_root), &inputs)?;
        println!("bundle: {}", dir.display());
        return Ok(());
    }

    let opts = RunOpts {
        decide: if args.overrule {
            Decide::Overrule {
                reason: args
                    .reason
                    .clone()
                    .expect("clap enforces --reason with --overrule"),
            }
        } else {
            Decide::FreshJudge
        },
        includes: args.include.clone(),
        skip_status: args.skip_status,
        skip_lane: args.skip_lane,
    };

    let outcome = core::run(&cfg, repo, args.pr_number, &opts, &mut |_stage| {})?;

    println!("ledger: {}", outcome.ledger_path.display());
    if let Some(dir) = &outcome.bundle_path {
        println!("bundle: {}", dir.display());
    }
    if let Some(why) = &outcome.bundle_degraded {
        eprintln!("bundle: DEGRADED — {why}");
    }
    if let Some((step, why)) = &outcome.lane_degraded {
        eprintln!("lane: DEGRADED — {step}: {why}");
    } else if !args.skip_lane {
        let ledger_id = outcome
            .doc
            .ruling
            .ledger_entry_id
            .as_deref()
            .expect("write_ledger assigns the ledger id");
        let sprint_dir = ledger_id.split('/').next().unwrap_or_default();
        println!("lane: {ledger_id} -> lane/{sprint_dir}");
    }
    if outcome.status_posted {
        let head = &outcome.doc.ruling.diff_ref;
        println!(
            "status: ruling/ratify -> {:?} ({head}, pr {})",
            outcome.doc.ruling.verdict, args.pr_number
        );
    } else {
        println!(
            "status: SKIPPED (--skip-status); verdict was {:?}",
            outcome.doc.ruling.verdict
        );
    }

    // exit 3: ruling written (and status posted, unless skipped) but a
    // durability surface degraded — the lane, or the bundle corpus — loud
    // and machine-legible without stalling a merge the ruling already earned.
    if outcome.lane_degraded.is_some() || outcome.bundle_degraded.is_some() {
        std::process::exit(3);
    }
    Ok(())
}

fn print_dry_run(inputs: &assemble::Inputs) {
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
}
