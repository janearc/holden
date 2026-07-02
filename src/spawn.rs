// spawn.rs — the fresh-judge seam (ADR-0001 D6 + sprint T0 flow steps 3-4).
//
// one invocation = one judge = one ruling. the judge is a brand-new headless
// model process fed ONLY what assemble.rs gathered; the writer's session is
// structurally absent. tolerant on the output envelope (a fenced yaml block
// is fine), pedantic on the payload (ruling::parse or it does not exist).

use crate::assemble::Inputs;
use crate::ruling::{self, RulingDoc};
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub struct SpawnCfg {
    // the judge executable; overridable so tests can stub a judge without
    // burning a model call ("claude" in production).
    pub judge_cmd: String,
    // optional model override; None = the CLI's configured default.
    pub model: Option<String>,
}

// the single-purpose prompt. everything the judge may consider is IN the
// prompt; it is instructed to cite only from these materials.
pub fn build_prompt(inputs: &Inputs) -> String {
    let mut p = String::new();
    p.push_str(
        "You are the judgment gate (T3) defined by ADR-0001. You are NOT the writer \
         of this diff and you have no memory of its authoring. Rule on the diff \
         strictly against the materials below. Cite file:line from the provided \
         materials only; an uncited claim is invalid.\n\n\
         Your ENTIRE reply must be a single YAML document matching exactly this \
         schema — no prose before or after (a ```yaml fence is permitted):\n\n\
         ruling:\n\
           diff_ref: <the PR url or head sha given below>\n\
           judge_instance: <the instance id given below>\n\
           fired_at: <UTC now, RFC3339>\n\
           verdict: ratify | bounce | needs-clarification\n\
           divergences:            # every departure from the design docs, even ratified ones\n\
             - claim: <what diverged>\n\
               necessary: true|false\n\
               justification: <why, citing file:line>\n\
           shape_verdict: on-mesh | wrong-shape\n\
           shape_justification: <one paragraph, concrete>\n\
           consumer_impact:        # one entry per consumer hit provided below\n\
             - consumer: <path>\n\
               classification: additive | breaking | silent-drift\n\
               evidence: <file:line citation>\n\
           doc_content_agreement: agree | disagree | unclear\n\n\
         Rules: a bounce MUST carry divergences or wrong-shape. Do NOT set \
         ledger_entry_id (the harness assigns it). Unknown fields are refused.\n\n\
         Doc-content agreement is judged on the WHOLE post-image document, never the \
         delta alone: after this diff, every touched document must be a truthful, \
         coherent description of the system at head, in its entirety. A diff that adds \
         locally-true sentences while surrounding claims remain stale is NOT agreement \
         — mark disagree and cite the stale passages. The standard is the most truthful \
         and descriptive document, not the minimum change that passes the gate.\n\n",
    );

    p.push_str(&format!(
        "== UNDER JUDGMENT ==\nrepo: {}\npr: {}\nhead sha: {}\njudge instance id: {}\n\n",
        inputs.repo_name,
        inputs.pr_number,
        inputs.head_sha,
        instance_id()
    ));

    p.push_str("== REPO TREE AT HEAD (paths only; cite these for existence claims) ==\n");
    for path in &inputs.head_tree {
        p.push_str(path);
        p.push('\n');
    }
    p.push('\n');

    if !inputs.head_files.is_empty() {
        p.push_str("== REQUESTED FILE CONTENTS AT HEAD (evidence you asked for) ==\n");
        for (path, body) in &inputs.head_files {
            p.push_str(&format!("--- {path} ---\n{body}\n"));
        }
        p.push('\n');
    }

    p.push_str("== DESIGN DOCS (the goalpost; rule against THESE) ==\n");
    for (path, body) in &inputs.design_docs {
        p.push_str(&format!("--- {} ---\n{}\n", path.display(), body));
    }

    if inputs.contracts_touched.is_empty() {
        p.push_str("\n== CONTRACTS TOUCHED ==\n(none)\n");
    } else {
        p.push_str("\n== CONTRACTS TOUCHED (current contents) ==\n");
        for (path, body) in &inputs.contracts_touched {
            p.push_str(&format!("--- {} ---\n{}\n", path.display(), body));
        }
    }

    p.push_str("\n== PRIOR RULINGS (your only memory; stay consistent or say why not) ==\n");
    if inputs.ledger.is_empty() {
        p.push_str("(none yet)\n");
    } else {
        for (path, body) in &inputs.ledger {
            p.push_str(&format!("--- {} ---\n{}\n", path.display(), body));
        }
    }

    p.push_str("\n== CONSUMERS OF CHANGED MESSAGE TYPES ==\n");
    if inputs.consumers.is_empty() {
        p.push_str("(no changed message types, or no external consumers found)\n");
    } else {
        for c in &inputs.consumers {
            p.push_str(&format!("[{}] {}\n", c.message, c.citation));
        }
    }

    p.push_str("\n== THE DIFF ==\n");
    p.push_str(&inputs.diff);
    p
}

// ephemeral, never reused: utc stamp + pid is unique enough for a laptop.
pub fn instance_id() -> String {
    format!(
        "judge-{}-p{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        std::process::id()
    )
}

// tolerate a fenced envelope; return the yaml payload.
pub fn strip_fences(s: &str) -> &str {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```yaml").or_else(|| t.strip_prefix("```")) {
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
    }
    t
}

fn spawn_once(cfg: &SpawnCfg, prompt: &str) -> Result<String> {
    let mut cmd = Command::new(&cfg.judge_cmd);
    cmd.arg("-p"); // headless: read prompt, reply, exit
    if let Some(m) = &cfg.model {
        cmd.args(["--model", m]);
    }
    // prompt via stdin: real diffs blow argv limits.
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().with_context(|| format!("spawning judge {:?}", cfg.judge_cmd))?;
    child
        .stdin
        .as_mut()
        .context("judge stdin unavailable")?
        .write_all(prompt.as_bytes())?;
    // no timeout in v0: first runs are operator-watched; a wedged judge is
    // ctrl-c'd by a human, not silently killed into a half-ruling.
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "judge process failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// spawn -> parse; on refusal, ONE retry with the refusal appended (ratified
// T0 call); still refused -> absent, loud, nonzero. never a third try.
pub fn rule(cfg: &SpawnCfg, inputs: &Inputs) -> Result<(RulingDoc, String)> {
    let prompt = build_prompt(inputs);

    let first = spawn_once(cfg, &prompt)?;
    let yaml = strip_fences(&first).to_string();
    match ruling::parse(&yaml) {
        Ok(doc) => return Ok((doc, yaml)),
        Err(first_err) => {
            let retry_prompt = format!(
                "{prompt}\n\n== YOUR PREVIOUS REPLY WAS REFUSED ==\n{first_err}\n\
                 Reply again with ONLY the corrected YAML document."
            );
            let second = spawn_once(cfg, &retry_prompt)?;
            let yaml2 = strip_fences(&second).to_string();
            match ruling::parse(&yaml2) {
                Ok(doc) => Ok((doc, yaml2)),
                Err(second_err) => bail!(
                    "ruling ABSENT: refused twice.\n first: {first_err}\n second: {second_err}"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assemble::{ConsumerHit, Inputs};
    use std::path::PathBuf;

    fn fake_inputs() -> Inputs {
        Inputs {
            repo_name: "magpie".into(),
            pr_number: 42,
            head_sha: "abc123def456".into(),
            diff: "+++ b/magpie/register.py\n+x = 1\n".into(),
            head_tree: vec!["magpie/register.py".into(), "kube/service.yaml".into()],
            head_files: vec![("magpie/pipeline.py".into(), "from frood import model".into())],
            design_docs: vec![(PathBuf::from("docs/design.md"), "the design".into())],
            contracts_touched: vec![],
            ledger: vec![],
            consumers: vec![ConsumerHit {
                message: "Registration".into(),
                citation: "delightd/pkg/httpapi/register.go:15: uses Registration".into(),
            }],
        }
    }

    #[test]
    fn prompt_carries_every_required_section() {
        let p = build_prompt(&fake_inputs());
        for needle in [
            "UNDER JUDGMENT",
            "REPO TREE AT HEAD",
            "REQUESTED FILE CONTENTS AT HEAD",
            "from frood import model",
            "DESIGN DOCS",
            "CONTRACTS TOUCHED",
            "PRIOR RULINGS",
            "CONSUMERS OF CHANGED MESSAGE TYPES",
            "THE DIFF",
            "repo: magpie",
            "head sha: abc123def456",
            "register.go:15",
            "kube/service.yaml",
        ] {
            assert!(p.contains(needle), "prompt missing: {needle}");
        }
    }

    #[test]
    fn fences_stripped_and_bare_passthrough() {
        assert_eq!(strip_fences("```yaml\nruling: x\n```"), "ruling: x");
        assert_eq!(strip_fences("```\nruling: x\n```"), "ruling: x");
        assert_eq!(strip_fences("  ruling: x  "), "ruling: x");
    }

    #[test]
    fn instance_ids_are_unique_enough() {
        // same process, same second is possible; the format at least pins
        // process + time. two sequential calls must not be identical only if
        // the clock ticks — so just assert the shape here.
        let id = instance_id();
        assert!(id.starts_with("judge-20"), "unexpected id shape: {id}");
        assert!(id.contains("-p"), "missing pid segment: {id}");
    }
}
