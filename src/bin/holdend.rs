// holdend — holden as a service (ADR-0003, holden RFC): the resident
// process is dispatch and bookkeeping; judgment stays ephemeral, one
// fresh judge per ruling through the same core the CLI drives. Serves
// holden.ruling.v1 over loopback HTTP with protojson bodies, on hahod's
// precedent: POST /rulings (SubmitRuling), GET /rulings/{id} (GetRuling),
// /health, /metrics. WatchRulings is polling GetRuling until a push
// surface lands, exactly as the contract header says.
//
// Strictness lives here, not in gen: the generated wire types are
// deliberately lenient, so the boundary refuses unknown fields itself —
// an off-contract request never arrives (ADR-0001's construction
// property, kept by hand where gen cannot keep it).
//
// In-memory state holds only in-flight lifecycle; the ledger is the
// durability, exactly as it is for the CLI. A holdend restart loses
// nothing but the in-flight states of rulings whose ledger rows never
// landed — and those judgments were ephemeral by doctrine.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use holden_contracts::holden::ruling::v1 as pb;
use judge::core::{self, Config, Decide, RunOpts, Stage};
use judge::{assemble, wire};
use tracing::{error, info, warn};

struct Row {
    state: pb::RulingState,
    ruling: Option<pb::Ruling>,
    failure_reason: String,
}

struct App {
    cfg: Config,
    rulings: Mutex<HashMap<String, Row>>,
    // idempotency: the same request_id is one fact, one ruling
    by_request: Mutex<HashMap<String, String>>,
    seq: AtomicU64,
    started: std::time::Instant,
    submitted: AtomicU64,
    published: AtomicU64,
    failed: AtomicU64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().json().init();

    // the same config boundary as the CLI: env over default (holdend has
    // no flags; it is a daemon, launchd carries its environment).
    let cfg = Config {
        delightd_url: core::pick(None, "JUDGE_DELIGHTD_URL", "http://127.0.0.1:8088"),
        sprints_root: core::pick_path(None, "JUDGE_SPRINTS_ROOT", "work/sprints")?,
        judge_cmd: core::pick(None, "JUDGE_CMD", "claude"),
        model: std::env::var("JUDGE_MODEL").ok(),
        home: std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("resolving the workstation home: HOME is unset"))?,
    };
    let addr = core::pick(None, "HOLDEND_ADDR", "127.0.0.1:8792");
    require_loopback(&addr)?;

    let app = Arc::new(App {
        cfg,
        rulings: Mutex::new(HashMap::new()),
        by_request: Mutex::new(HashMap::new()),
        seq: AtomicU64::new(0),
        started: std::time::Instant::now(),
        submitted: AtomicU64::new(0),
        published: AtomicU64::new(0),
        failed: AtomicU64::new(0),
    });

    let router = Router::new()
        .route("/rulings", post(submit))
        .route("/rulings/{id}", get(get_ruling))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr, "holdend serving");
    // Graceful stop: the listener closes on SIGTERM/ctrl-c; in-flight
    // rulings run to their landing on the blocking pool before the
    // runtime lets the process exit — the ledger write is the durability,
    // and a judgment mid-flight is never half-recorded by a shutdown.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("holdend stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("installing the SIGTERM handler");
    tokio::select! {
        _ = ctrl_c => {},
        _ = term.recv() => {},
    }
    info!("shutdown signal; draining");
}

// the loopback posture, enforced mechanically at startup like hahod's:
// a routable bind is a misconfiguration refused, not a surprise.
fn require_loopback(addr: &str) -> anyhow::Result<()> {
    let host = addr
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(addr)
        .trim_matches(['[', ']']);
    if !matches!(host, "127.0.0.1" | "::1" | "localhost") {
        anyhow::bail!(
            "addr {addr} is not loopback; holdend serves 127.0.0.1, ::1, or localhost only"
        );
    }
    Ok(())
}

// SubmitRuling. The body must be exactly a holden.ruling.v1.RulingRequest:
// unknown fields are refused loudly here because the generated types will
// not refuse them.
async fn submit(State(app): State<Arc<App>>, body: axum::body::Bytes) -> Response {
    let req = match strict_request(&body) {
        Ok(req) => req,
        Err(why) => {
            warn!(why, "submit refused");
            return loud(StatusCode::UNPROCESSABLE_ENTITY, &why);
        }
    };

    // idempotency: the same request_id is one fact — answer the handle it
    // already has, mint nothing.
    if let Some(existing) = app.by_request.lock().unwrap().get(&req.request_id) {
        return (
            StatusCode::OK,
            Json(pb::RulingHandle {
                ruling_id: existing.clone(),
            }),
        )
            .into_response();
    }

    let ruling_id = format!(
        "ruling-{}-{:05}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        app.seq.fetch_add(1, Ordering::Relaxed)
    );
    app.rulings.lock().unwrap().insert(
        ruling_id.clone(),
        Row {
            state: pb::RulingState::Received,
            ruling: None,
            failure_reason: String::new(),
        },
    );
    app.by_request
        .lock()
        .unwrap()
        .insert(req.request_id.clone(), ruling_id.clone());
    app.submitted.fetch_add(1, Ordering::Relaxed);
    info!(
        ruling_id,
        repo = req.repo,
        pr = req.pr_number,
        "ruling received"
    );

    let app2 = app.clone();
    let id2 = ruling_id.clone();
    tokio::task::spawn_blocking(move || execute(app2, id2, req));

    (StatusCode::ACCEPTED, Json(pb::RulingHandle { ruling_id })).into_response()
}

// one ruling to its landing, on the blocking pool: resolve the repo via
// the fleet's roster, then the same pipeline the CLI drives, with the
// lifecycle told into the row as it happens.
fn execute(app: Arc<App>, id: String, req: pb::RulingRequest) {
    let set_state = |st: pb::RulingState| {
        if let Some(row) = app.rulings.lock().unwrap().get_mut(&id) {
            row.state = st;
        }
    };

    let outcome = (|| -> anyhow::Result<core::RunOutcome> {
        let repo_path = assemble::resolve_repo_path(&app.cfg, &req.repo)?;
        core::run(
            &app.cfg,
            &repo_path,
            u64::from(req.pr_number),
            &RunOpts {
                decide: Decide::FreshJudge,
                includes: req.includes.clone(),
                skip_status: false,
                skip_lane: false,
            },
            &mut |stage| {
                set_state(match stage {
                    Stage::InputsAssembled => pb::RulingState::InputsAssembled,
                    Stage::JudgeSpawned => pb::RulingState::JudgeSpawned,
                    Stage::Verdict => pb::RulingState::Verdict,
                    Stage::Published => pb::RulingState::Published,
                })
            },
        )
    })();

    match outcome {
        Ok(outcome) => {
            if let Some((step, why)) = &outcome.lane_degraded {
                // loud, never a bail: the ruling already earned its merge
                warn!(ruling_id = id, step, why, "lane DEGRADED");
            }
            if let Some(row) = app.rulings.lock().unwrap().get_mut(&id) {
                row.ruling = Some(wire::to_wire(&outcome.doc));
                row.state = pb::RulingState::Published;
            }
            app.published.fetch_add(1, Ordering::Relaxed);
            info!(ruling_id = id, "ruling published");
        }
        Err(e) => {
            if let Some(row) = app.rulings.lock().unwrap().get_mut(&id) {
                row.state = pb::RulingState::Failed;
                row.failure_reason = e.to_string();
            }
            app.failed.fetch_add(1, Ordering::Relaxed);
            error!(ruling_id = id, error = %e, "ruling FAILED");
        }
    }
}

// strict_request refuses what gen cannot: any field the contract does not
// name, and the invariants shape cannot express (a non-empty request_id
// and repo, a real PR number).
fn strict_request(body: &[u8]) -> Result<pb::RulingRequest, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("body is not JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or("body is not a JSON object".to_string())?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "requestId" | "repo" | "prNumber" | "includes") {
            return Err(format!(
                "off-contract field {key:?}: not part of holden.ruling.v1.RulingRequest"
            ));
        }
    }
    let req: pb::RulingRequest =
        serde_json::from_value(value).map_err(|e| format!("body is not a RulingRequest: {e}"))?;
    if req.request_id.is_empty() {
        return Err("request_id is empty; the requestor assigns the idempotency key".into());
    }
    if req.repo.is_empty() {
        return Err("repo is empty".into());
    }
    if req.pr_number == 0 {
        return Err("pr_number is 0; there is no PR zero".into());
    }
    Ok(req)
}

async fn get_ruling(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let rulings = app.rulings.lock().unwrap();
    match rulings.get(&id) {
        None => loud(StatusCode::NOT_FOUND, &format!("no ruling {id}")),
        Some(row) => (
            StatusCode::OK,
            Json(pb::RulingStatus {
                ruling_id: id.clone(),
                state: row.state,
                ruling: row.ruling.clone(),
                failure_reason: row.failure_reason.clone(),
            }),
        )
            .into_response(),
    }
}

async fn health(State(app): State<Arc<App>>) -> Response {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "healthy": true,
            "uptime_seconds": app.started.elapsed().as_secs(),
        })),
    )
        .into_response()
}

async fn metrics(State(app): State<Arc<App>>) -> Response {
    let text = format!(
        "# TYPE holdend_rulings_submitted_total counter\n\
         holdend_rulings_submitted_total {}\n\
         # TYPE holdend_rulings_published_total counter\n\
         holdend_rulings_published_total {}\n\
         # TYPE holdend_rulings_failed_total counter\n\
         holdend_rulings_failed_total {}\n",
        app.submitted.load(Ordering::Relaxed),
        app.published.load(Ordering::Relaxed),
        app.failed.load(Ordering::Relaxed),
    );
    ([("content-type", "text/plain; version=0.0.4")], text).into_response()
}

fn loud(status: StatusCode, why: &str) -> Response {
    (status, Json(serde_json::json!({"error": why}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_request_refuses_off_contract_fields() {
        let err = strict_request(br#"{"requestId":"r1","repo":"x","prNumber":1,"vibes":"no"}"#)
            .unwrap_err();
        assert!(err.contains("off-contract field"), "{err}");
    }

    #[test]
    fn strict_request_enforces_shape_invariants() {
        assert!(strict_request(br#"{"repo":"x","prNumber":1}"#)
            .unwrap_err()
            .contains("request_id"));
        assert!(strict_request(br#"{"requestId":"r1","prNumber":1}"#)
            .unwrap_err()
            .contains("repo"));
        assert!(strict_request(br#"{"requestId":"r1","repo":"x"}"#)
            .unwrap_err()
            .contains("pr_number"));
    }

    #[test]
    fn strict_request_accepts_the_contract() {
        let req = strict_request(
            br#"{"requestId":"r1","repo":"janearc/big-little-mesh","prNumber":85,"includes":["docs/x.md"]}"#,
        )
        .unwrap();
        assert_eq!(req.repo, "janearc/big-little-mesh");
        assert_eq!(req.pr_number, 85);
        assert_eq!(req.includes, vec!["docs/x.md"]);
    }

    #[test]
    fn loopback_is_enforced() {
        assert!(require_loopback("127.0.0.1:8792").is_ok());
        assert!(require_loopback("[::1]:8792").is_ok());
        assert!(require_loopback("0.0.0.0:8792").is_err());
        assert!(require_loopback("192.168.1.5:8792").is_err());
    }
}
