// judge — ADR-0001 judgment-gate harness (sprint 2026-07-02-judge).
//
// schema-first build: the ruling schema and its refusal semantics (ruling.rs)
// land before any orchestration, so the enforcement core exists and is tested
// independent of everything that talks to git, gh, or a model. the flow below
// is wired in the next diffs, per the sprint doc's T0 design.

mod ruling;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "judge",
    about = "assemble inputs, spawn a fresh judge, refuse off-spec rulings, write the ledger, post the status"
)]
struct Args {
    /// path to the repo checkout under judgment
    repo_path: String,
    /// pull request number to rule on
    pr_number: u64,
    /// operator overrule: record an overrule ruling before posting (an overrule is data)
    #[arg(long)]
    overrule: bool,
}

fn main() {
    let args = Args::parse();
    // T0 design is in the sprint doc; orchestration lands as its own diffs:
    //   assemble inputs -> spawn fresh judge -> ruling::parse -> ledger write -> status post
    eprintln!(
        "judge: schema core only (not yet wired). repo={} pr={} overrule={}",
        args.repo_path, args.pr_number, args.overrule
    );
    std::process::exit(2);
}
