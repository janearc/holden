// the ledger form to the wire form: ruling::RulingDoc (the YAML schema,
// kebab-case values, ADR-0001 D6 as serde) into holden_contracts (the
// protobuf twin, declared-name values). the correspondence is mechanical
// and stated in the contract's header; this module is the one place it
// is written down as code, so the two renderings can never drift apart
// silently — a new enum value without a mapping here is a compile error.

use holden_contracts::holden::ruling::v1 as pb;

use crate::ruling;

pub fn to_wire(doc: &ruling::RulingDoc) -> pb::Ruling {
    let r = &doc.ruling;
    pb::Ruling {
        diff_ref: r.diff_ref.clone(),
        judge_instance: r.judge_instance.clone(),
        // protojson Timestamp form: RFC3339 UTC
        fired_at: Some(
            r.fired_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ),
        verdict: verdict(r.verdict),
        divergences: r.divergences.iter().map(divergence).collect(),
        shape_verdict: shape(r.shape_verdict),
        shape_justification: r.shape_justification.clone(),
        consumer_impact: r.consumer_impact.iter().map(impact).collect(),
        doc_content_agreement: agreement(r.doc_content_agreement),
        ledger_entry_id: r.ledger_entry_id.clone().unwrap_or_default(),
    }
}

fn verdict(v: ruling::Verdict) -> pb::Verdict {
    match v {
        ruling::Verdict::Ratify => pb::Verdict::Ratify,
        ruling::Verdict::Bounce => pb::Verdict::Bounce,
        ruling::Verdict::NeedsClarification => pb::Verdict::NeedsClarification,
    }
}

fn shape(v: ruling::ShapeVerdict) -> pb::ShapeVerdict {
    match v {
        ruling::ShapeVerdict::OnMesh => pb::ShapeVerdict::OnMesh,
        ruling::ShapeVerdict::WrongShape => pb::ShapeVerdict::WrongShape,
    }
}

fn agreement(v: ruling::DocContentAgreement) -> pb::DocContentAgreement {
    match v {
        ruling::DocContentAgreement::Agree => pb::DocContentAgreement::Agree,
        ruling::DocContentAgreement::Disagree => pb::DocContentAgreement::Disagree,
        ruling::DocContentAgreement::Unclear => pb::DocContentAgreement::Unclear,
    }
}

fn divergence(d: &ruling::Divergence) -> pb::Divergence {
    pb::Divergence {
        claim: d.claim.clone(),
        necessary: d.necessary,
        justification: d.justification.clone(),
    }
}

fn impact(c: &ruling::ConsumerImpact) -> pb::ConsumerImpact {
    pb::ConsumerImpact {
        consumer: c.consumer.clone(),
        classification: match c.classification {
            ruling::Classification::Additive => pb::Classification::Additive,
            ruling::Classification::Breaking => pb::Classification::Breaking,
            ruling::Classification::SilentDrift => pb::Classification::SilentDrift,
        },
        evidence: c.evidence.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_form_speaks_declared_names() {
        let doc = crate::core::overrule_ruling(
            &crate::assemble::Inputs {
                repo_name: "big-little-mesh".into(),
                pr_number: 85,
                head_sha: "abc123".into(),
                diff: String::new(),
                head_tree: vec![],
                head_files: vec![],
                design_docs: vec![],
                contracts_touched: vec![],
                ledger: vec![],
                implicated: vec![],
                consumers: vec![],
            },
            "test overrule",
        );
        let wire = to_wire(&doc);
        // the wire speaks the contract's declared names, kebab-case stays
        // in the YAML ledger — the correspondence the contract header pins.
        let json = serde_json::to_value(&wire).expect("wire serializes");
        assert_eq!(json["verdict"], "RATIFY");
        assert_eq!(json["shapeVerdict"], "ON_MESH");
        assert_eq!(json["docContentAgreement"], "UNCLEAR");
        assert!(json["firedAt"].as_str().unwrap().ends_with('Z'));
        // empty ledger id is omitted-as-default on the wire until assigned
        assert_eq!(wire.ledger_entry_id, "");
    }
}
