// judge — ADR-0001 judgment-gate harness (sprint 2026-07-02-judge).
//
// schema-first build: the ruling schema and its refusal semantics (ruling.rs)
// land before any orchestration, so the enforcement core exists and is tested
// independent of everything that talks to git, gh, or a model. the flow below
// is wired in the next diffs, per the sprint doc's T0 design.

mod assemble;
mod ruling;

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
    /// assemble and summarize the judge's inputs without spawning a judge
    #[arg(long)]
    dry_run: bool,
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
    // check. an off-spec ruling is refused loudly, with the reasons.
    if let Some(path) = &args.validate_ruling {
        let yaml = std::fs::read_to_string(path)?;
        match ruling::parse(&yaml) {
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

    let inputs = assemble::assemble(repo, args.pr_number, work_root, sprints_root)?;

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

    // spawn -> parse -> ledger write -> status post land as the next diffs,
    // per the sprint T0 design. overrule flag is honored there.
    let _ = args.overrule;
    eprintln!("judge: spawn/ledger/status not yet wired; run with --dry-run to audit inputs");
    std::process::exit(2);
}
