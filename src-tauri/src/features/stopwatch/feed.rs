//! A localhost HTTP feed for the current focus task.
//!
//! Other applications (for example a distraction blocker) can read the task the
//! user is tracking without talking to Tauri. The feed is opt-in and bound to
//! `127.0.0.1` only. `GET /focus` returns one JSON snapshot; `GET /focus/events`
//! is a server-sent-events stream that pushes a new snapshot when it changes.

use std::io;
use std::net::Ipv4Addr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

use crate::features::calendar::{AppTask, StoreState};

use super::state::{
    current_epoch_millis, StopwatchLabelSource, StopwatchManager, StopwatchPhase, StopwatchState,
};

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const MIN_PORT: u16 = 1024;

/// One read of the focus state. `title` is the task's current title when the
/// store still knows the task, so renaming it is reflected without reselecting.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FocusSnapshot {
    pub(super) task_id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) label: Option<String>,
    pub(super) label_source: StopwatchLabelSource,
    pub(super) phase: StopwatchPhase,
    pub(super) active: bool,
    pub(super) tags: Vec<String>,
    pub(super) list: Option<String>,
}

impl FocusSnapshot {
    fn from_state(
        state: &StopwatchState,
        task: Option<&AppTask>,
        list: Option<String>,
        now: u64,
    ) -> Self {
        let paused = state.paused_until.is_some_and(|until| until > now);
        let title = task
            .map(|task| task.title.clone())
            .or_else(|| state.label.clone());
        let tags = task
            .and_then(|task| task.tags.as_ref())
            .map_or_else(Vec::new, |tags| {
                tags.iter().map(|tag| tag.name.clone()).collect()
            });
        Self {
            task_id: state.task_id.clone(),
            title,
            label: state.label.clone(),
            label_source: state.label_source,
            phase: state.phase,
            active: state.phase == StopwatchPhase::Running && !paused,
            tags,
            list,
        }
    }
}

/// Start the feed on `port`, unless the configured port is unusable.
pub fn start(app: tauri::AppHandle, port: i32) {
    let Ok(port) = u16::try_from(port) else {
        eprintln!("Focus feed disabled: port {port} is out of range");
        return;
    };
    if port < MIN_PORT {
        eprintln!("Focus feed disabled: port {port} is below {MIN_PORT}");
        return;
    }
    tauri::async_runtime::spawn(async move {
        if let Err(error) = serve(app, port).await {
            eprintln!("Focus feed stopped: {error}");
        }
    });
}

async fn serve(app: tauri::AppHandle, port: u16) -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    eprintln!("Focus feed listening on http://127.0.0.1:{port}/focus");

    let (updates_tx, updates_rx) = watch::channel(String::new());
    tauri::async_runtime::spawn(publish_loop(app.clone(), updates_tx));

    loop {
        let (socket, _) = listener.accept().await?;
        let app = app.clone();
        let updates = updates_rx.clone();
        tauri::async_runtime::spawn(async move {
            handle_connection(socket, &app, updates).await;
        });
    }
}

async fn publish_loop(app: tauri::AppHandle, updates: watch::Sender<String>) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last = String::new();
    loop {
        interval.tick().await;
        let json = serde_json::to_string(&current_snapshot(&app).await).unwrap_or_default();
        if json != last {
            last.clone_from(&json);
            let _ = updates.send(json);
        }
    }
}

async fn current_snapshot(app: &tauri::AppHandle) -> FocusSnapshot {
    let state = app
        .try_state::<StopwatchManager>()
        .and_then(|manager| manager.snapshot().ok())
        .unwrap_or_default();
    let now = current_epoch_millis().unwrap_or_default();
    let (task, list) = match app.try_state::<StoreState>() {
        Some(store) => {
            store
                .read(|store| {
                    let task = state
                        .task_id
                        .as_deref()
                        .and_then(|id| store.task(id))
                        .cloned();
                    let list = task
                        .as_ref()
                        .and_then(|task| task.remote_collection_id.as_deref())
                        .and_then(|href| store.calendar(href))
                        .map(|calendar| calendar.display_name.clone());
                    (task, list)
                })
                .await
        }
        None => (None, None),
    };
    FocusSnapshot::from_state(&state, task.as_ref(), list, now)
}

async fn handle_connection(
    mut socket: TcpStream,
    app: &tauri::AppHandle,
    updates: watch::Receiver<String>,
) {
    let Ok(head) = read_request(&mut socket).await else {
        return;
    };
    let Some((method, path)) = request_target(&head) else {
        let _ = respond_json(&mut socket, "400 Bad Request", r#"{"error":"bad request"}"#).await;
        return;
    };

    match (method, path) {
        ("GET", "/focus") => {
            let json = serde_json::to_string(&current_snapshot(app).await).unwrap_or_default();
            let _ = respond_json(&mut socket, "200 OK", &json).await;
        }
        ("GET", "/focus/events") => {
            let _ = stream_events(&mut socket, app, updates).await;
        }
        ("GET", "/health") => {
            let _ = respond_json(&mut socket, "200 OK", r#"{"ok":true}"#).await;
        }
        _ => {
            let _ = respond_json(&mut socket, "404 Not Found", r#"{"error":"not found"}"#).await;
        }
    }
}

async fn read_request(socket: &mut TcpStream) -> io::Result<String> {
    let mut buffer = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    loop {
        let read = tokio::time::timeout(READ_TIMEOUT, socket.read(&mut chunk))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "request timed out"))??;
        if read == 0 {
            break;
        }
        let bytes = chunk
            .get(..read)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid read length"))?;
        buffer.extend_from_slice(bytes);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n")
            || buffer.len() >= MAX_REQUEST_BYTES
        {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn request_target(head: &str) -> Option<(&str, &str)> {
    let mut parts = head.lines().next()?.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    let path = target.split('?').next()?;
    Some((method, path))
}

async fn respond_json(socket: &mut TcpStream, status: &str, body: &str) -> io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(header.as_bytes()).await?;
    socket.write_all(body.as_bytes()).await?;
    socket.flush().await
}

async fn stream_events(
    socket: &mut TcpStream,
    app: &tauri::AppHandle,
    updates: watch::Receiver<String>,
) -> io::Result<()> {
    let initial = serde_json::to_string(&current_snapshot(app).await).unwrap_or_default();
    stream_events_with_initial(socket, &initial, updates).await
}

async fn stream_events_with_initial(
    socket: &mut TcpStream,
    initial: &str,
    mut updates: watch::Receiver<String>,
) -> io::Result<()> {
    socket
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n",
        )
        .await?;

    let mut last_sent = initial.to_string();
    send_event(socket, &last_sent).await?;
    let current = updates.borrow_and_update().clone();
    if !current.is_empty() && current != last_sent {
        send_event(socket, &current).await?;
        last_sent = current;
    }

    let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    keepalive.tick().await;
    loop {
        tokio::select! {
            changed = updates.changed() => {
                if changed.is_err() {
                    break;
                }
                let json = updates.borrow_and_update().clone();
                if !json.is_empty() && json != last_sent {
                    send_event(socket, &json).await?;
                    last_sent = json;
                }
            }
            _ = keepalive.tick() => {
                socket.write_all(b": keepalive\n\n").await?;
                socket.flush().await?;
            }
        }
    }
    Ok(())
}

async fn send_event(socket: &mut TcpStream, json: &str) -> io::Result<()> {
    socket.write_all(b"event: focus\ndata: ").await?;
    socket.write_all(json.as_bytes()).await?;
    socket.write_all(b"\n\n").await?;
    socket.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::calendar::Tag;

    fn running_state() -> StopwatchState {
        StopwatchState {
            phase: StopwatchPhase::Running,
            work_running_since: Some(1_000),
            task_id: Some("task-1".to_string()),
            label: Some("stale label".to_string()),
            label_source: StopwatchLabelSource::Task,
            ..StopwatchState::default()
        }
    }

    fn task() -> AppTask {
        AppTask {
            id: "task-1".into(),
            title: "Learn Python".into(),
            status: None,
            priority: None,
            tags: Some(vec![Tag {
                id: "tag-1".into(),
                name: "learning".into(),
                color: None,
            }]),
            parent_task: None,
            dependencies: None,
            est: None,
            time_spent: None,
            added: None,
            remote_id: None,
            remote_collection_id: Some("/lists/1/".into()),
            notes: None,
            start: None,
            due: None,
            completed_at: None,
            rrule: None,
            geo: None,
            alarms: None,
            timezone: None,
            updated_at: None,
            etag: None,
        }
    }

    #[test]
    fn snapshot_prefers_the_live_task_title_and_tags() {
        let state = running_state();
        let task = task();

        let snapshot =
            FocusSnapshot::from_state(&state, Some(&task), Some("Tasks".to_string()), 2_000);

        assert_eq!(snapshot.task_id.as_deref(), Some("task-1"));
        assert_eq!(snapshot.title.as_deref(), Some("Learn Python"));
        assert_eq!(snapshot.label.as_deref(), Some("stale label"));
        assert_eq!(snapshot.tags, vec!["learning".to_string()]);
        assert_eq!(snapshot.list.as_deref(), Some("Tasks"));
        assert!(snapshot.active);
    }

    #[test]
    fn snapshot_falls_back_to_the_label_when_the_task_is_missing() {
        let state = running_state();

        let snapshot = FocusSnapshot::from_state(&state, None, None, 2_000);

        assert_eq!(snapshot.title.as_deref(), Some("stale label"));
        assert_eq!(snapshot.tags, Vec::<String>::new());
        assert!(snapshot.list.is_none());
    }

    #[test]
    fn a_break_or_pause_is_not_active() {
        let mut state = running_state();
        state.phase = StopwatchPhase::Breaking;
        assert!(!FocusSnapshot::from_state(&state, None, None, 2_000).active);

        let mut paused = running_state();
        paused.paused_until = Some(3_000);
        assert!(!FocusSnapshot::from_state(&paused, None, None, 2_000).active);
        assert!(FocusSnapshot::from_state(&paused, None, None, 3_001).active);
    }

    #[test]
    fn request_target_ignores_the_query_and_the_rest_of_the_head() {
        assert_eq!(
            request_target("GET /focus?verbose=1 HTTP/1.1\r\nHost: localhost\r\n\r\n"),
            Some(("GET", "/focus"))
        );
        assert_eq!(
            request_target("GET /focus/events HTTP/1.1\r\n\r\n"),
            Some(("GET", "/focus/events"))
        );
        assert_eq!(request_target("nonsense"), None);
    }

    #[tokio::test]
    async fn sse_stream_does_not_duplicate_unchanged_initial_snapshot() {
        let (tx, rx) = watch::channel(String::new());
        let initial = r#"{"phase":"paused","active":false}"#.to_string();
        // Simulate publish_loop having already populated the channel before the client connects.
        tx.send(initial.clone()).unwrap();

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client_rx = rx.clone();
        let server_initial = initial.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = stream_events_with_initial(&mut socket, &server_initial, client_rx).await;
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        drop(tx);
        let mut received = String::new();
        client.read_to_string(&mut received).await.unwrap();
        server.await.unwrap();

        assert_eq!(
            received.matches("event: focus\n").count(),
            1,
            "initial SSE snapshot must not be sent twice: {received:?}"
        );
    }
}
