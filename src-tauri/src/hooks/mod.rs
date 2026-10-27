//! Loopback endpoint that Radicale's `taskroot_hook` posts to.
//!
//! Radicale is the single writer of record, so every change — the app's own edits,
//! the agent CLI, a phone, a direct `PUT` — is announced here right after it is
//! stored. That lets the app sync immediately instead of waiting for the poll
//! interval, which stays as the backstop for edits made behind Radicale's back
//! (files edited on disk).
//!
//! The payload is ignored on purpose: a notification only means "something
//! changed", and `sync::sync_all` pulls every collection with its own token. That
//! also makes the endpoint harmless if anything else on the machine posts to it —
//! the worst case is a redundant pull.

use crate::sync;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use std::net::{Ipv4Addr, SocketAddr};
use tauri::AppHandle;

/// Port the Radicale hook posts to; keep in sync with `NOTIFY_URL` in
/// `C:\radicale\taskroot_hook.py`.
const NOTIFY_PORT: u16 = 5233;

/// Start the loopback listener, if it can bind. Another instance (or an
/// unrelated program) holding the port only costs instant sync; the poll remains.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, NOTIFY_PORT));
        let Ok(listener) = tokio::net::TcpListener::bind(address).await else {
            eprintln!("Change notifications disabled: cannot bind {address}");
            return;
        };
        let router = Router::new()
            .route("/notify", post(notify))
            .with_state(app);
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("Change notification listener stopped: {error}");
        }
    });
}

async fn notify(State(app): State<AppHandle>, _body: Bytes) -> StatusCode {
    sync::trigger_sync(&app);
    StatusCode::NO_CONTENT
}
