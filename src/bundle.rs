// bundle.rs — the bundle bento (holden RFC section 5). everything the judge
// sees is assembled by holden; that bundle is exactly the shape of a bento,
// and holden records it as one — inert, manifested, replayable — so a
// disputed ruling can be re-run as an experiment: same bundle, fresh judge.
// the bundle is a record, not a transport: it rides no bus and no frood
// consumes it.
//
// on disk: <root>/<repo>/pr<N>/<bundle_id>/ holding manifest.json (the
// bento's manifest: identity plus the banchan list), inputs.json (the one
// banchan every bundle carries: assemble::Inputs, verbatim), ruling_ref.json
// (linked after the ledger write — its presence distinguishes "the inputs a
// ruling was actually rendered from" from a bare recording), and replays/
// (experiment output, one file per fresh judge; the recorded banchans are
// never touched after the recording run finishes).

use crate::assemble::Inputs;
use crate::ruling::RulingDoc;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// manifest kinds are versioned independently of the crate: a reader of the
// corpus dispatches on these strings, never on file layout guesses.
pub const BUNDLE_KIND: &str = "t3.bundle.v1";
pub const INPUTS_KIND: &str = "t3.judge-inputs.v1";
pub const RULING_REF_KIND: &str = "t3.ruling-ref.v1";

#[derive(Debug, Serialize, Deserialize)]
pub struct Banchan {
    pub name: String,
    pub kind: String,
    // bundle-dir-relative; the manifest never names an absolute path, so a
    // corpus survives being moved wholesale.
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub bundle_id: String,
    pub kind: String,
    pub repo_name: String,
    pub pr_number: u64,
    pub head_sha: String,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub banchans: Vec<Banchan>,
}

// the ruling a bundle's live run produced, linked after the ledger write.
// enough to find the ledger row and to compare a replay's verdict against
// the original without re-parsing the ledger.
#[derive(Debug, Serialize, Deserialize)]
pub struct RulingRef {
    pub ledger_entry_id: String,
    pub judge_instance: String,
    pub verdict: String,
    pub fired_at: chrono::DateTime<chrono::Utc>,
}

// unique without a clock-trust assumption, same construction as
// spawn::instance_id: process-unique counter + pid + timestamp.
static BUNDLE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn bundle_id() -> String {
    let seq = BUNDLE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "bundle-{}-p{}-s{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        std::process::id(),
        seq
    )
}

// record one assembled bundle under root. the inputs banchan is written
// before the manifest, so a directory bearing a manifest is complete by
// construction; a manifest-less directory is a crashed recording and reads
// as such (load refuses it).
pub fn record(root: &Path, inputs: &Inputs) -> Result<PathBuf> {
    let id = bundle_id();
    let dir = root
        .join(&inputs.repo_name)
        .join(format!("pr{}", inputs.pr_number))
        .join(&id);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let inputs_json =
        serde_json::to_string_pretty(inputs).context("serializing the inputs banchan")?;
    std::fs::write(dir.join("inputs.json"), inputs_json)
        .with_context(|| format!("writing {}", dir.join("inputs.json").display()))?;

    let manifest = Manifest {
        bundle_id: id,
        kind: BUNDLE_KIND.to_string(),
        repo_name: inputs.repo_name.clone(),
        pr_number: inputs.pr_number,
        head_sha: inputs.head_sha.clone(),
        recorded_at: chrono::Utc::now(),
        banchans: vec![Banchan {
            name: "inputs".into(),
            kind: INPUTS_KIND.into(),
            location: "inputs.json".into(),
        }],
    };
    write_manifest(&dir, &manifest)?;
    Ok(dir)
}

// link the ruling a live run produced: write the ruling-ref banchan and
// re-manifest to name it. only the run that recorded the bundle calls this;
// after it returns the bundle is inert.
pub fn link_ruling(dir: &Path, doc: &RulingDoc) -> Result<()> {
    let mut manifest = read_manifest(dir)?;
    let r = &doc.ruling;
    let ruling_ref = RulingRef {
        ledger_entry_id: r
            .ledger_entry_id
            .clone()
            .context("linking a ruling with no ledger id; link_ruling runs after write_ledger")?,
        judge_instance: r.judge_instance.clone(),
        verdict: format!("{:?}", r.verdict).to_lowercase(),
        fired_at: r.fired_at,
    };
    let json = serde_json::to_string_pretty(&ruling_ref).context("serializing the ruling ref")?;
    std::fs::write(dir.join("ruling_ref.json"), json)
        .with_context(|| format!("writing {}", dir.join("ruling_ref.json").display()))?;

    if !manifest.banchans.iter().any(|b| b.name == "ruling-ref") {
        manifest.banchans.push(Banchan {
            name: "ruling-ref".into(),
            kind: RULING_REF_KIND.into(),
            location: "ruling_ref.json".into(),
        });
    }
    write_manifest(dir, &manifest)
}

// load a bundle for replay: manifest first (identity and kind checked), then
// the inputs banchan it names. a coherence break between manifest and inputs
// is a loud refusal — a corpus entry that lies about itself is not evidence.
pub fn load(dir: &Path) -> Result<Inputs> {
    let manifest = read_manifest(dir)?;
    if manifest.kind != BUNDLE_KIND {
        bail!(
            "{} is a {:?}, not a {BUNDLE_KIND}; refusing to replay it",
            dir.display(),
            manifest.kind
        );
    }
    let banchan = manifest
        .banchans
        .iter()
        .find(|b| b.name == "inputs")
        .with_context(|| format!("{} manifests no inputs banchan", dir.display()))?;
    let path = dir.join(&banchan.location);
    let body =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let inputs: Inputs =
        serde_json::from_str(&body).with_context(|| format!("parsing {}", path.display()))?;
    if inputs.head_sha != manifest.head_sha
        || inputs.repo_name != manifest.repo_name
        || inputs.pr_number != manifest.pr_number
    {
        bail!(
            "{}: manifest and inputs disagree ({} pr {} @ {} vs {} pr {} @ {}); \
             a bundle that lies about itself is not evidence",
            dir.display(),
            manifest.repo_name,
            manifest.pr_number,
            manifest.head_sha,
            inputs.repo_name,
            inputs.pr_number,
            inputs.head_sha
        );
    }
    Ok(inputs)
}

// record one replay's output beside the bundle, named by the fresh judge's
// instance id (unique by construction). the recorded banchans are not
// touched; replays accumulate as experiment evidence.
pub fn record_replay(dir: &Path, yaml: &str, judge_instance: &str) -> Result<PathBuf> {
    let replays = dir.join("replays");
    std::fs::create_dir_all(&replays).with_context(|| format!("creating {}", replays.display()))?;
    let path = replays.join(format!("{judge_instance}.yaml"));
    std::fs::write(&path, yaml).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

fn write_manifest(dir: &Path, manifest: &Manifest) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest).context("serializing the manifest")?;
    std::fs::write(dir.join("manifest.json"), json)
        .with_context(|| format!("writing {}", dir.join("manifest.json").display()))
}

fn read_manifest(dir: &Path) -> Result<Manifest> {
    let path = dir.join("manifest.json");
    let body =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&body).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assemble::{ConsumerHit, ImplicatedDoc, Inputs};
    use std::path::PathBuf;

    fn fake_inputs() -> Inputs {
        Inputs {
            repo_name: "magpie".into(),
            pr_number: 42,
            head_sha: "abc123def456".into(),
            diff: "+++ b/magpie/register.py\n+x = 1\n".into(),
            head_tree: vec!["magpie/register.py".into()],
            head_files: vec![(
                "magpie/pipeline.py".into(),
                "from frood import model".into(),
            )],
            design_docs: vec![(PathBuf::from("docs/design.md"), "the design".into())],
            contracts_touched: vec![],
            ledger: vec![(
                PathBuf::from("/s/2026-07-08/rulings/x.yaml"),
                "ruling: {}".into(),
            )],
            consumers: vec![ConsumerHit {
                message: "Registration".into(),
                citation: "delightd/pkg/httpapi/register.go:15: uses Registration".into(),
            }],
            implicated: vec![ImplicatedDoc {
                path: PathBuf::from("docs/api.md"),
                content: Some("the api contract".into()),
            }],
        }
    }

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "judge-bundle-test-{}-{}",
            std::process::id(),
            BUNDLE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn record_then_load_roundtrips_the_inputs() {
        let root = scratch();
        let original = fake_inputs();
        let dir = record(&root, &original).unwrap();
        // layout: <root>/<repo>/pr<N>/<bundle-id>
        assert!(dir.starts_with(root.join("magpie").join("pr42")));
        let loaded = load(&dir).unwrap();
        // the replay guarantee: the prompt is a pure function of Inputs, so
        // input equality IS prompt equality (up to the minted instance id).
        assert_eq!(loaded, original);
    }

    #[test]
    fn manifest_names_the_bundle_honestly() {
        let root = scratch();
        let dir = record(&root, &fake_inputs()).unwrap();
        let m = read_manifest(&dir).unwrap();
        assert_eq!(m.kind, BUNDLE_KIND);
        assert_eq!(m.repo_name, "magpie");
        assert_eq!(m.pr_number, 42);
        assert_eq!(m.head_sha, "abc123def456");
        assert_eq!(m.banchans.len(), 1);
        assert_eq!(m.banchans[0].name, "inputs");
        assert_eq!(m.banchans[0].kind, INPUTS_KIND);
    }

    #[test]
    fn link_ruling_adds_the_ref_banchan() {
        let root = scratch();
        let dir = record(&root, &fake_inputs()).unwrap();
        let mut doc = crate::core::overrule_ruling(&fake_inputs(), "test link");
        doc.ruling.ledger_entry_id = Some("2026-07-24/rulings/x.yaml".into());
        link_ruling(&dir, &doc).unwrap();
        let m = read_manifest(&dir).unwrap();
        assert!(m.banchans.iter().any(|b| b.name == "ruling-ref"));
        let body = std::fs::read_to_string(dir.join("ruling_ref.json")).unwrap();
        let r: RulingRef = serde_json::from_str(&body).unwrap();
        assert_eq!(r.ledger_entry_id, "2026-07-24/rulings/x.yaml");
        assert_eq!(r.verdict, "ratify");
    }

    #[test]
    fn link_ruling_without_ledger_id_bails() {
        let root = scratch();
        let dir = record(&root, &fake_inputs()).unwrap();
        let doc = crate::core::overrule_ruling(&fake_inputs(), "no id yet");
        let err = link_ruling(&dir, &doc).unwrap_err();
        assert!(err.to_string().contains("no ledger id"), "{err}");
    }

    #[test]
    fn load_refuses_a_manifest_inputs_disagreement() {
        let root = scratch();
        let dir = record(&root, &fake_inputs()).unwrap();
        // corrupt the inputs banchan: same shape, different sha
        let body = std::fs::read_to_string(dir.join("inputs.json")).unwrap();
        std::fs::write(
            dir.join("inputs.json"),
            body.replace("abc123def456", "fffffffff"),
        )
        .unwrap();
        let err = load(&dir).unwrap_err();
        assert!(err.to_string().contains("disagree"), "{err}");
    }

    #[test]
    fn load_refuses_a_manifestless_directory() {
        let dir = scratch();
        let err = load(&dir).unwrap_err();
        assert!(err.to_string().contains("manifest.json"), "{err}");
    }

    #[test]
    fn replays_accumulate_without_touching_the_record() {
        let root = scratch();
        let dir = record(&root, &fake_inputs()).unwrap();
        let before = std::fs::read_to_string(dir.join("inputs.json")).unwrap();
        let p1 = record_replay(&dir, "ruling: one", "judge-a").unwrap();
        let p2 = record_replay(&dir, "ruling: two", "judge-b").unwrap();
        assert_ne!(p1, p2);
        assert_eq!(std::fs::read_to_string(p1).unwrap(), "ruling: one");
        assert_eq!(
            std::fs::read_to_string(dir.join("inputs.json")).unwrap(),
            before,
            "a replay must never touch the recorded banchans"
        );
    }
}
