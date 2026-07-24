// haho.rs — holden as a haho requestor (holden RFC section 7 step 3): open
// a session, submit one job, take the completion. The JobSpec is the RFC's
// worked spec verbatim — api lane, WHOLE_OR_ERROR, allow_partial false, no
// synthesis, no cache — because a ruling is whole or it is not a ruling.
// Which model answers is pinned in the spec (discernment is external in
// haho v1); how the chute executes is haho's business and invisible here.
//
// HTTP is a curl subprocess through the same seam as gh/rg and the roster
// fetch: the crate's ratified no-new-http-client posture. hahod has no push
// surface yet, so completion is polled — the same posture as holdend's own
// WatchRulings-is-polling note.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

// the messages API's canonical address, as the RFC's worked spec pins it.
// not environment-derived: this is the contract-documented value for the
// ANTHROPIC_COMPATIBLE kind; the chute reads the credential from the env
// var the spec NAMES (token_env) — holden never touches the value.
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";

// completion poll cadence. a ruling runs minutes; 3s keeps the wait honest
// without hammering the loopback. no client deadline in v0, the shim's own
// posture: hahod's drain and orphan recovery land a dead flight as FAILED,
// which the poll then sees — a hang here is hahod's bug surfacing, loudly.
const POLL_SECONDS: u64 = 3;

pub struct HahodCfg {
    pub base_url: String,
    // the judge, named exactly as --model does today; REQUIRED on this path
    // (the API takes no "whatever the CLI is configured with").
    pub model: String,
    // ENV VAR NAME of the credential, never the value (haho section 14).
    pub token_env: String,
}

// the worked spec, as data. pure, so the test can hold it against the RFC
// field for field.
pub fn ruling_spec(job_id: &str, prompt: &str, cfg: &HahodCfg) -> serde_json::Value {
    serde_json::json!({
        "jobId": job_id,
        "payload": {"messages": {"messages": [
            {"role": "user", "content": prompt}]}},
        "backend": {"kind": "ANTHROPIC_COMPATIBLE", "model": cfg.model,
                    "baseUrl": ANTHROPIC_BASE_URL,
                    "tokenEnv": cfg.token_env},
        "framing": {"strategy": "WHOLE_OR_ERROR"},
        "synthesis": {"mode": "NONE"},
        "cache": {"kind": "NONE"},
        "allowPartial": false
    })
}

// one ruling job to its landing: session, task, poll, completion. returns
// the model's reply text; every other outcome is a loud, specific error.
pub fn submit(cfg: &HahodCfg, job_id: &str, prompt: &str) -> Result<String> {
    let base = cfg.base_url.trim_end_matches('/');

    let created = http(&format!("{base}/sessions"), Method::Post(None))?;
    let session = parse_session_created(&created)?;

    let spec = ruling_spec(job_id, prompt, cfg).to_string();
    let tasked = http(
        &format!("{base}/sessions/{session}/task"),
        Method::Post(Some(&spec)),
    )?;
    match parse_task_reply(&tasked)? {
        TaskReply::Accepted => {}
        TaskReply::Refused { code, reason } => bail!(
            "hahod REFUSED the ruling job ({code}): {reason} — a refusal is the \
             feasibility proof working; fix the lane before judging anything"
        ),
    }

    loop {
        let status = http(&format!("{base}/sessions/{session}"), Method::Get)?;
        if parse_in_flight(&status)? {
            std::thread::sleep(std::time::Duration::from_secs(POLL_SECONDS));
            continue;
        }
        let completion = http(
            &format!("{base}/sessions/{session}/completion"),
            Method::Get,
        )?;
        return extract_ruling_text(&completion.body);
    }
}

enum Method<'a> {
    Get,
    Post(Option<&'a str>),
}

struct Reply {
    status: u16,
    body: String,
}

// curl with the status captured: refusals arrive as 4xx WITH a body that
// names the reason, so -f (which discards it) is exactly wrong here. the
// status rides the last line of stdout via -w; transport failure is loud.
fn http(url: &str, method: Method) -> Result<Reply> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-o", "-", "-w", "\n%{http_code}"]);
    let stdin_body = match method {
        Method::Get => None,
        Method::Post(body) => {
            cmd.args(["-X", "POST"]);
            if let Some(b) = body {
                cmd.args([
                    "-H",
                    "content-type: application/json",
                    "--data-binary",
                    "@-",
                ]);
                Some(b.to_string())
            } else {
                None
            }
        }
    };
    cmd.arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawning curl for {url}"))?;
    if let Some(body) = stdin_body {
        child
            .stdin
            .as_mut()
            .context("curl stdin unavailable")?
            .write_all(body.as_bytes())?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "curl {url} failed ({}): {} — hahod unreachable is a fleet problem \
             to fix before judging anything",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    parse_reply(&stdout).with_context(|| format!("reading the reply from {url}"))
}

// pure: split curl's "<body>\n<status>" capture.
fn parse_reply(stdout: &str) -> Result<Reply> {
    let (body, status_line) = stdout
        .rsplit_once('\n')
        .context("curl output carries no status line")?;
    let status: u16 = status_line
        .trim()
        .parse()
        .with_context(|| format!("not an http status: {status_line:?}"))?;
    Ok(Reply {
        status,
        body: body.to_string(),
    })
}

// pure: POST /sessions → the session id, or loud.
fn parse_session_created(reply: &Reply) -> Result<String> {
    if reply.status != 201 {
        bail!(
            "hahod session create answered {}: {}",
            reply.status,
            reply.body.trim()
        );
    }
    let v: serde_json::Value =
        serde_json::from_str(&reply.body).context("session create body is not JSON")?;
    v.get("id")
        .and_then(|i| i.as_str())
        .map(str::to_string)
        .context("session create body has no `id`")
}

pub enum TaskReply {
    Accepted,
    Refused { code: String, reason: String },
}

// pure: POST task → accepted (202), refused (422/429 with code+reason), or
// loud on anything else.
fn parse_task_reply(reply: &Reply) -> Result<TaskReply> {
    match reply.status {
        202 => Ok(TaskReply::Accepted),
        422 | 429 => {
            let v: serde_json::Value =
                serde_json::from_str(&reply.body).context("refusal body is not JSON")?;
            Ok(TaskReply::Refused {
                code: v
                    .get("code")
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                reason: v
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("(no reason given)")
                    .to_string(),
            })
        }
        s => bail!("hahod task answered {s}: {}", reply.body.trim()),
    }
}

// pure: GET /sessions/{id} → still in flight?
fn parse_in_flight(reply: &Reply) -> Result<bool> {
    if reply.status != 200 {
        bail!(
            "hahod status answered {}: {}",
            reply.status,
            reply.body.trim()
        );
    }
    let v: serde_json::Value =
        serde_json::from_str(&reply.body).context("status body is not JSON")?;
    v.get("in_flight")
        .and_then(|f| f.as_bool())
        .context("status body has no `in_flight`")
}

// pure: the completion → the reply text. PRODUCED with text is the only
// acceptable landing: FAILED carries hahod's specific reason; PARTIAL with
// allow_partial pinned false is a contract violation, refused as such.
pub fn extract_ruling_text(completion_body: &str) -> Result<String> {
    let v: serde_json::Value =
        serde_json::from_str(completion_body).context("completion body is not JSON")?;
    let outcome = v
        .get("outcome")
        .and_then(|o| o.as_str())
        .context("completion has no `outcome`")?;
    match outcome {
        "PRODUCED" => {
            let text = v
                .get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .context("PRODUCED completion carries no result text")?;
            if text.trim().is_empty() {
                bail!("PRODUCED completion carries empty result text");
            }
            Ok(text.to_string())
        }
        "FAILED" => bail!(
            "the ruling job FAILED: {}",
            v.get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("(no reason)")
        ),
        "PARTIAL" => bail!(
            "hahod landed PARTIAL against allow_partial=false — a contract \
             violation, and a partial ruling is not a ruling"
        ),
        other => bail!("completion outcome {other:?} is off-contract"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> HahodCfg {
        HahodCfg {
            base_url: "http://127.0.0.1:8790".into(),
            model: "claude-fable-5".into(),
            token_env: "HOLDEN_ANTHROPIC_KEY".into(),
        }
    }

    #[test]
    fn ruling_spec_matches_the_worked_spec() {
        // haho RFC section 8, "Three worked specs", holden's assessment:
        // field for field, so a drift between this client and the RFC is a
        // failing test, not a surprise at admission.
        let spec = ruling_spec(
            "ruling-delightd-121",
            "<the assembled ruling prompt>",
            &cfg(),
        );
        assert_eq!(spec["jobId"], "ruling-delightd-121");
        assert_eq!(spec["payload"]["messages"]["messages"][0]["role"], "user");
        assert_eq!(
            spec["payload"]["messages"]["messages"][0]["content"],
            "<the assembled ruling prompt>"
        );
        assert_eq!(spec["backend"]["kind"], "ANTHROPIC_COMPATIBLE");
        assert_eq!(spec["backend"]["model"], "claude-fable-5");
        assert_eq!(spec["backend"]["baseUrl"], "https://api.anthropic.com");
        assert_eq!(spec["backend"]["tokenEnv"], "HOLDEN_ANTHROPIC_KEY");
        assert_eq!(spec["framing"]["strategy"], "WHOLE_OR_ERROR");
        assert_eq!(spec["synthesis"]["mode"], "NONE");
        assert_eq!(spec["cache"]["kind"], "NONE");
        assert_eq!(spec["allowPartial"], false);
    }

    #[test]
    fn reply_parse_splits_body_and_status() {
        let r = parse_reply("{\"id\":\"haho-1-abc\"}\n201").unwrap();
        assert_eq!(r.status, 201);
        assert_eq!(r.body, "{\"id\":\"haho-1-abc\"}");
        // a multi-line body keeps everything but the status line
        let r = parse_reply("line one\nline two\n200").unwrap();
        assert_eq!(r.body, "line one\nline two");
    }

    #[test]
    fn session_create_yields_the_id() {
        let id = parse_session_created(&Reply {
            status: 201,
            body: r#"{"id":"haho-71-0af3c","created":true}"#.into(),
        })
        .unwrap();
        assert_eq!(id, "haho-71-0af3c");
        let err = parse_session_created(&Reply {
            status: 500,
            body: r#"{"error":"boom"}"#.into(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("500"), "{err}");
    }

    #[test]
    fn refusal_carries_code_and_reason() {
        let got = parse_task_reply(&Reply {
            status: 422,
            body: r#"{"id":"s","accepted":false,"refused":true,"code":"secret_env_missing","reason":"HOLDEN_ANTHROPIC_KEY is unset"}"#.into(),
        })
        .unwrap();
        match got {
            TaskReply::Refused { code, reason } => {
                assert_eq!(code, "secret_env_missing");
                assert!(reason.contains("unset"));
            }
            TaskReply::Accepted => panic!("a refusal parsed as acceptance"),
        }
    }

    #[test]
    fn in_flight_reads_the_status_shape() {
        assert!(parse_in_flight(&Reply {
            status: 200,
            body: r#"{"id":"s","in_flight":true,"disposition":""}"#.into(),
        })
        .unwrap());
        assert!(!parse_in_flight(&Reply {
            status: 200,
            body: r#"{"id":"s","in_flight":false,"disposition":"PRODUCED"}"#.into(),
        })
        .unwrap());
    }

    #[test]
    fn produced_yields_text_and_failures_are_loud() {
        let text = extract_ruling_text(
            r#"{"jobId":"j","outcome":"PRODUCED","result":{"text":"ruling: ..."},"modelUsed":"claude-fable-5"}"#,
        )
        .unwrap();
        assert_eq!(text, "ruling: ...");

        let err =
            extract_ruling_text(r#"{"jobId":"j","outcome":"FAILED","reason":"the lane died"}"#)
                .unwrap_err();
        assert!(err.to_string().contains("the lane died"), "{err}");

        let err = extract_ruling_text(r#"{"jobId":"j","outcome":"PARTIAL"}"#).unwrap_err();
        assert!(err.to_string().contains("not a ruling"), "{err}");

        let err =
            extract_ruling_text(r#"{"jobId":"j","outcome":"PRODUCED","result":{}}"#).unwrap_err();
        assert!(err.to_string().contains("no result text"), "{err}");
    }
}
