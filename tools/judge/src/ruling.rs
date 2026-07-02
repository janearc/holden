// the ruling schema from ADR-0001 Decision 6, as types.
//
// the load-bearing property: `deny_unknown_fields` + real enums mean an
// off-spec ruling does not deserialize, so "invalid ruling = absent" is a
// fact of construction, not a check someone remembered to write. anything
// that parses here is shape-valid; validate() then enforces the invariants
// shape alone cannot express (file:line evidence, non-empty justifications,
// a bounce must carry its reasons).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// the YAML document nests everything under a single `ruling:` key.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulingDoc {
    pub ruling: Ruling,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ruling {
    // commit sha or PR url this ruling binds to.
    pub diff_ref: String,
    // ephemeral judge id; never reused across invocations.
    pub judge_instance: String,
    pub fired_at: DateTime<Utc>,
    pub verdict: Verdict,
    // every divergence from the design doc, ratified-or-not, with evidence.
    #[serde(default)]
    pub divergences: Vec<Divergence>,
    pub shape_verdict: ShapeVerdict,
    pub shape_justification: String,
    #[serde(default)]
    pub consumer_impact: Vec<ConsumerImpact>,
    // advisory field (ADR D6): never blocks on its own.
    pub doc_content_agreement: DocContentAgreement,
    // assigned by the harness on ledger write; the judge MUST NOT set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_entry_id: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Ratify,
    Bounce,
    NeedsClarification,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeVerdict {
    OnMesh,
    WrongShape,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocContentAgreement {
    Agree,
    Disagree,
    Unclear,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Additive,
    Breaking,
    SilentDrift,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    pub claim: String,
    pub necessary: bool,
    pub justification: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerImpact {
    pub consumer: String,
    pub classification: Classification,
    // a citation, not an assertion: must name file:line.
    pub evidence: String,
}

// invariants that shape alone cannot express. a ruling failing any of these
// is treated exactly like one that failed to parse: absent.
pub fn validate(doc: &RulingDoc) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    let r = &doc.ruling;

    if r.diff_ref.trim().is_empty() {
        errs.push("diff_ref is empty".into());
    }
    if r.judge_instance.trim().is_empty() {
        errs.push("judge_instance is empty".into());
    }
    if r.shape_justification.trim().is_empty() {
        errs.push("shape_justification is empty".into());
    }
    // the judge must not assign ledger ids; the harness does, on write.
    if r.ledger_entry_id.is_some() {
        errs.push("ledger_entry_id must not be set by the judge".into());
    }
    // a bounce must carry its reasons: at least one divergence, or wrong-shape.
    if r.verdict == Verdict::Bounce
        && r.divergences.is_empty()
        && r.shape_verdict == ShapeVerdict::OnMesh
    {
        errs.push("bounce with no divergences and shape on-mesh: a bounce must state why".into());
    }
    for (i, d) in r.divergences.iter().enumerate() {
        if d.claim.trim().is_empty() {
            errs.push(format!("divergences[{i}].claim is empty"));
        }
        if d.justification.trim().is_empty() {
            errs.push(format!("divergences[{i}].justification is empty"));
        }
    }
    for (i, c) in r.consumer_impact.iter().enumerate() {
        if !is_file_line_citation(&c.evidence) {
            errs.push(format!(
                "consumer_impact[{i}].evidence is not a file:line citation: {:?}",
                c.evidence
            ));
        }
    }

    if errs.is_empty() { Ok(()) } else { Err(errs) }
}

// evidence must contain at least one `path:NNN` citation somewhere in it.
fn is_file_line_citation(s: &str) -> bool {
    // scan for ':' followed by digits, preceded by a plausible path char.
    let b = s.as_bytes();
    for (i, &ch) in b.iter().enumerate() {
        if ch == b':' && i > 0 {
            let prev_ok = b[i - 1].is_ascii_alphanumeric() || matches!(b[i - 1], b'.' | b'/' | b'_' | b'-');
            let next_digit = b.get(i + 1).is_some_and(|c| c.is_ascii_digit());
            if prev_ok && next_digit {
                return true;
            }
        }
    }
    false
}

// parse-and-validate: the single entry point the harness uses. anything short
// of Ok(..) is, by ADR rule, an ABSENT ruling.
pub fn parse(yaml: &str) -> Result<RulingDoc, String> {
    let doc: RulingDoc =
        serde_yaml::from_str(yaml).map_err(|e| format!("ruling does not parse: {e}"))?;
    validate(&doc).map_err(|errs| format!("ruling is invalid: {}", errs.join("; ")))?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_yaml() -> String {
        r#"
ruling:
  diff_ref: "https://github.com/janearc/big-little-mesh/pull/79"
  judge_instance: "judge-2026-07-02-a1b2c3"
  fired_at: "2026-07-02T10:00:00Z"
  verdict: ratify
  divergences:
    - claim: "added retry not named in the design doc"
      necessary: true
      justification: "design doc silent on transient failures; register.py:61 shows the caller expects it"
  shape_verdict: on-mesh
  shape_justification: "uses the generated registry.v1 client; no hand-rolled wire types"
  consumer_impact:
    - consumer: "magpie/magpie/register.py"
      classification: additive
      evidence: "register.py:54 - existing contract fields untouched"
  doc_content_agreement: agree
"#
        .to_string()
    }

    #[test]
    fn valid_ruling_parses() {
        let doc = parse(&valid_yaml()).expect("valid ruling must parse");
        assert_eq!(doc.ruling.verdict, Verdict::Ratify);
        assert_eq!(doc.ruling.shape_verdict, ShapeVerdict::OnMesh);
    }

    #[test]
    fn unknown_field_is_refused() {
        let y = valid_yaml().replace("verdict: ratify", "verdict: ratify\n  vibes: immaculate");
        assert!(parse(&y).is_err(), "unknown field must refuse to deserialize");
    }

    #[test]
    fn off_enum_verdict_is_refused() {
        let y = valid_yaml().replace("verdict: ratify", "verdict: lgtm");
        assert!(parse(&y).is_err(), "free-text verdict must refuse");
    }

    #[test]
    fn evidence_without_file_line_is_refused() {
        let y = valid_yaml().replace(
            "register.py:54 - existing contract fields untouched",
            "trust me, it is additive",
        );
        assert!(parse(&y).is_err(), "assertion without citation must refuse");
    }

    #[test]
    fn bounce_without_reasons_is_refused() {
        let y = valid_yaml()
            .replace("verdict: ratify", "verdict: bounce")
            .replace(
                r#"  divergences:
    - claim: "added retry not named in the design doc"
      necessary: true
      justification: "design doc silent on transient failures; register.py:61 shows the caller expects it"
"#,
                "  divergences: []\n",
            );
        assert!(parse(&y).is_err(), "a bounce must state why");
    }

    #[test]
    fn judge_setting_ledger_id_is_refused() {
        let y = valid_yaml().replace(
            "doc_content_agreement: agree",
            "doc_content_agreement: agree\n  ledger_entry_id: \"sneaky\"",
        );
        assert!(parse(&y).is_err(), "ledger ids are the harness's to assign");
    }

    #[test]
    fn empty_justification_is_refused() {
        let y = valid_yaml().replace(
            "justification: \"design doc silent on transient failures; register.py:61 shows the caller expects it\"",
            "justification: \"\"",
        );
        assert!(parse(&y).is_err(), "empty justification must refuse");
    }
}
