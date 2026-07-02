// judge — ADR-0001 judgment-gate harness (sprint 2026-07-02-judge).
//
// full flow per the sprint's T0 design:
//   assemble (D6 inputs) -> spawn fresh judge -> refuse-or-accept (schema) ->
//   write the ledger entry (sprints repo, never the target repo) -> post the
//   ruling/ratify commit status to the head sha.

mod assemble;
mod publish;
mod ruling;
mod spawn;

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
    /// judge executable (tests stub this; production default is claude)
    #[arg(long, default_value = "claude")]
    judge_cmd: String,
    /// model override passed to the judge; default = the CLI's configured model
    #[arg(long)]
    model: Option<String>,
    /// root containing the fleet checkouts (consumer scan)
    #[arg(long, default_value = "/Users/jane/work")]
    work_root: String,
    /// root of the sprints repo (ledger + ruling output)
    #[arg(long, default_value = "/Users/jane/work/sprints")]
    sprints_root: String,
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
                println!("VALID: verdict={:?} shape={:?}", doc.ruling.verdict, doc.ruling.shape_verdict);
                return Ok(());
            }
            Err(e) => {
                eprintln!("REFUSED: {e}");
                std::process::exit(1);
            }
        }
    }

    let repo = std::path::Path::new(&args.repo_path);
    let work_root = std::path::Path::new(&args.work_root);
    let sprints_root = std::path::Path::new(&args.sprints_root);

    let inputs = assemble::assemble(repo, args.pr_number, work_root, sprints_root, &args.include)?;

    if args.dry_run {
        // a human-readable audit of exactly what the judge would receive.
        println!("repo:          {}", inputs.repo_name);
        println!("pr:            {} (head {})", inputs.pr_number, &inputs.head_sha[..12.min(inputs.head_sha.len())]);
        println!("diff:          {} lines", inputs.diff.lines().count());
        println!("design docs:   {}", inputs.design_docs.iter().map(|(p, _)| p.display().to_string()).collect::<Vec<_>>().join(", "));
        println!("contracts:     {}", if inputs.contracts_touched.is_empty() { "(none touched)".into() } else { inputs.contracts_touched.iter().map(|(p, _)| p.display().to_string()).collect::<Vec<_>>().join(", ") });
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
        let reason = args.reason.clone().expect("clap enforces --reason with --overrule");
        (overrule_ruling(&inputs, &reason), String::new())
    } else {
        let cfg = spawn::SpawnCfg { judge_cmd: args.judge_cmd.clone(), model: args.model.clone() };
        spawn::rule(&cfg, &inputs)?
    };

    // ledger first, status second: the status carries the ledger id, and a
    // status without a ledger entry would be an untraceable claim.
    let path = publish::write_ledger(sprints_root, &inputs.repo_name, inputs.pr_number, &mut doc)?;
    println!("ledger: {}", path.display());
    if args.skip_status {
        println!("status: SKIPPED (--skip-status); verdict was {:?}", doc.ruling.verdict);
        return Ok(());
    }
    publish::post_status(repo, &inputs.head_sha, &doc)?;
    println!(
        "status: ruling/ratify -> {:?} on {} (pr {})",
        doc.ruling.verdict, &inputs.head_sha[..12.min(inputs.head_sha.len())], inputs.pr_number
    );
    Ok(())
}

// an operator overrule as a first-class ruling: verdict ratify, the overrule
// named as a divergence, the operator as the instance. it reads honestly in
// the ledger — nobody mistakes it for a judge's opinion.
fn overrule_ruling(inputs: &assemble::Inputs, reason: &str) -> ruling::RulingDoc {
    ruling::RulingDoc {
        ruling: ruling::Ruling {
            diff_ref: format!("{} pr {} @ {}", inputs.repo_name, inputs.pr_number, inputs.head_sha),
            judge_instance: format!("operator-overrule-{}", chrono::Utc::now().format("%Y%m%dT%H%M%SZ")),
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
