//! `POST /api/ctf/start`, `/api/ctf/puzzle`, `/api/ctf/flag` (docs/CONTRACTS.md §8.3).
//!
//! Order of checks: method → rate limit (per network and endpoint) → size → parse → run token.
//! Answers are never logged; the access log has the route name and status like everything else.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::response::Response;
use http::StatusCode;
use http::header::CONTENT_LENGTH;
use http_body_util::{BodyExt, LengthLimitError, Limited};
use proof::{CTF_BODY_LIMIT, CtfError, CtfRequest, CtfState};

use super::{RouteName, ctx, internal_error, json, mark, method_not_allowed};
use crate::app::AppState;
use crate::ctf::{Limiter, Run};

fn error(status: StatusCode, code: &str) -> Response {
    json(status, &CtfError { error: code.to_owned() })
}

fn state(app: &AppState, run: &Run, correct: Option<bool>) -> Response {
    json(StatusCode::OK, &CtfState { correct, run: app.ctf.token(run), stage: run.stage, puzzle: app.ctf.puzzle(run) })
}

/// Method and rate limit, then the handler.
async fn guarded(
    app: &AppState,
    req: Request,
    limiter: impl Fn(&AppState) -> &Limiter,
    handle: impl AsyncFnOnce(Request) -> Response,
) -> Response {
    if req.method() != http::Method::POST {
        return method_not_allowed("POST");
    }
    let Some(ctx) = ctx(&req) else { return internal_error() };
    if !limiter(app).admit(ctx.conn.client_net, Instant::now()) {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate-limited");
    }
    handle(req).await
}

/// The JSON body of `puzzle` and `flag`, and the run its token carries; or the status and error
/// code to answer with.
async fn read(app: &AppState, req: Request) -> Result<(CtfRequest, Run), (StatusCode, &'static str)> {
    const TOO_LARGE: (StatusCode, &str) = (StatusCode::PAYLOAD_TOO_LARGE, "too-large");
    const BAD_REQUEST: (StatusCode, &str) = (StatusCode::BAD_REQUEST, "bad-request");
    let declared = req.headers().get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|n| n > CTF_BODY_LIMIT as u64) {
        return Err(TOO_LARGE);
    }
    let body = match Limited::new(req.into_body(), CTF_BODY_LIMIT).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) if e.downcast_ref::<LengthLimitError>().is_some() => return Err(TOO_LARGE),
        Err(_) => return Err(BAD_REQUEST),
    };
    let request: CtfRequest = serde_json::from_slice(&body).map_err(|_| BAD_REQUEST)?;
    let run = app.ctf.parse(&request.run).ok_or((StatusCode::BAD_REQUEST, "bad-run"))?;
    Ok((request, run))
}

/// `POST /api/ctf/start`: a new run at stage one.
pub(crate) async fn start(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = guarded(&app, req, |a| &a.ctf.starts, async |_| state(&app, &app.ctf.start(), None)).await;
    mark(response, RouteName::Named("/api/ctf/start"), routed)
}

/// `POST /api/ctf/puzzle`: where a run stands, and its puzzle.
pub(crate) async fn puzzle(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = guarded(&app, req, |a| &a.ctf.reads, async |req| match read(&app, req).await {
        Ok((_, run)) => state(&app, &run, None),
        Err((status, code)) => error(status, code),
    })
    .await;
    mark(response, RouteName::Named("/api/ctf/puzzle"), routed)
}

/// `POST /api/ctf/flag`: checks an answer. Right moves the run on (a new token); wrong keeps it.
pub(crate) async fn flag(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = guarded(&app, req, |a| &a.ctf.flags, async |req| match read(&app, req).await {
        Ok((request, run)) => match app.ctf.check(&run, request.flag.as_deref().unwrap_or("")) {
            Some(next) => {
                tracing::info!(target: "ctf", stage = run.stage, "stage solved");
                state(&app, &next, Some(true))
            }
            None => state(&app, &run, Some(false)),
        },
        Err((status, code)) => error(status, code),
    })
    .await;
    mark(response, RouteName::Named("/api/ctf/flag"), routed)
}
