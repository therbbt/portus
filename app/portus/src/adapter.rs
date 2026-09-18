//! The Tauri command + event bridge. This module is deliberately dumb: it
//! owns each live `Session`, forwards commands down into it, and forwards
//! its events back out onto the Tauri event bus. It never decides protocol
//! behavior — that all lives in the `Session` implementations themselves.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use uuid::Uuid;

use portus_core::session::{Session, SessionEvent};

pub enum SessionCommand {
    Write(Bytes),
    Resize(u16, u16),
    Shutdown,
}

#[derive(Clone, Default)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SessionCommand>>>>,
    /// Saved-session ids that currently have a live session reading/writing
    /// scrollback. Guards against two concurrently-open tabs for the same
    /// saved shell preset interleaving their output into one shared
    /// scrollback file — without this, opening a second tab for the same
    /// preset replayed everything the first tab had already appended, and a
    /// third replayed everything both the first and second had appended, so
    /// the visible prompt count grew by one with every additional
    /// concurrently-open tab of the same preset.
    active_scrollback_sessions: Arc<Mutex<HashSet<Uuid>>>,
}

impl AppState {
    /// Construct a `Session` for `protocol`, hand it its own tokio task, and
    /// return the id the frontend will use to address it from now on.
    /// `options` carries protocol-specific connect parameters (e.g. host/
    /// auth for SSH) as JSON — this function is the only place that knows
    /// how to turn that JSON into a concrete `Session`. `saved_session_id`
    /// is set only when opening a saved session, and only actually used for
    /// a saved shell preset (scrollback persistence) — see `run_session`.
    pub fn open(
        &self,
        protocol: &str,
        options: serde_json::Value,
        saved_session_id: Option<Uuid>,
        app: AppHandle,
    ) -> Result<String, String> {
        let session: Box<dyn Session> = match protocol {
            "echo" => Box::new(portus_core::echo::EchoSession::default()),
            "shell" => {
                let opts: portus_shell::ShellConnectOptions = match options {
                    serde_json::Value::Null => portus_shell::ShellConnectOptions::default(),
                    other => serde_json::from_value(other).map_err(|e| format!("invalid shell options: {e}"))?,
                };
                Box::new(portus_shell::ShellSession::new(opts))
            }
            "ssh" => {
                let opts: portus_ssh::SshConnectOptions =
                    serde_json::from_value(options).map_err(|e| format!("invalid ssh options: {e}"))?;
                Box::new(portus_ssh::SshSession::new(opts))
            }
            "serial" => {
                let opts: portus_serial::SerialConnectOptions =
                    serde_json::from_value(options).map_err(|e| format!("invalid serial options: {e}"))?;
                Box::new(portus_serial::SerialSession::new(opts))
            }
            other => return Err(format!("unknown protocol: {other}")),
        };

        // Ad-hoc "Local shell" tabs have no stable identity to persist
        // scrollback under, and every other protocol hasn't opted in yet —
        // scoped to saved shell presets specifically for now. If this saved
        // session already has another tab open (claimed the id below), this
        // one connects as a plain ad-hoc shell instead of a second writer
        // fighting over the same scrollback file.
        let scrollback_saved_session_id = if protocol == "shell" {
            saved_session_id.filter(|id| self.active_scrollback_sessions.lock().unwrap().insert(*id))
        } else {
            None
        };

        let id = Uuid::new_v4().to_string();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        self.sessions.lock().unwrap().insert(id.clone(), cmd_tx);

        let sessions = self.sessions.clone();
        let active_scrollback_sessions = self.active_scrollback_sessions.clone();
        let task_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_session(session, cmd_rx, &app, &task_id, scrollback_saved_session_id).await;
            sessions.lock().unwrap().remove(&task_id);
            if let Some(saved_session_id) = scrollback_saved_session_id {
                active_scrollback_sessions.lock().unwrap().remove(&saved_session_id);
            }
        });

        Ok(id)
    }

    pub fn send(&self, session_id: &str, cmd: SessionCommand) -> Result<(), String> {
        let sessions = self.sessions.lock().unwrap();
        let tx = sessions.get(session_id).ok_or_else(|| "unknown session".to_string())?;
        tx.send(cmd).map_err(|_| "session already closed".to_string())
    }
}

async fn run_session(
    mut session: Box<dyn Session>,
    mut cmd_rx: mpsc::UnboundedReceiver<SessionCommand>,
    app: &AppHandle,
    id: &str,
    scrollback_saved_session_id: Option<Uuid>,
) {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    if let Err(e) = session.start(event_tx).await {
        emit(app, id, &SessionEvent::Error { message: e.to_string() });
        emit(app, id, &SessionEvent::Closed { reason: Some(e.to_string()) });
        return;
    }

    // Replay whatever was captured last time, before any live output, so it
    // reads as "the terminal picks up where it left off" rather than a
    // separate, clearly-stale block of text.
    if let Some(saved_session_id) = scrollback_saved_session_id {
        match tokio::task::spawn_blocking(move || portus_core::scrollback::read_tail(saved_session_id)).await {
            Ok(Ok(bytes)) if !bytes.is_empty() => {
                emit(app, id, &SessionEvent::Data { data: Bytes::from(bytes) });
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => tracing::warn!("failed to read scrollback for {saved_session_id}: {e}"),
            Err(e) => tracing::warn!("scrollback read task panicked for {saved_session_id}: {e}"),
        }
    }

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(SessionCommand::Write(data)) => {
                        if let Err(e) = session.write(data).await {
                            emit(app, id, &SessionEvent::Error { message: e.to_string() });
                        }
                    }
                    Some(SessionCommand::Resize(cols, rows)) => {
                        if let Err(e) = session.resize(cols, rows).await {
                            emit(app, id, &SessionEvent::Error { message: e.to_string() });
                        }
                    }
                    Some(SessionCommand::Shutdown) | None => {
                        let _ = session.shutdown().await;
                        break;
                    }
                }
            }
            event = event_rx.recv() => {
                match event {
                    Some(event) => {
                        if let (Some(saved_session_id), SessionEvent::Data { data }) = (scrollback_saved_session_id, &event) {
                            // Detached (not awaited here) so a disk write never adds
                            // latency to the live data path the terminal is waiting on.
                            let data = data.clone();
                            tokio::task::spawn(async move {
                                match tokio::task::spawn_blocking(move || portus_core::scrollback::append(saved_session_id, &data)).await {
                                    Ok(Err(e)) => tracing::warn!("failed to append scrollback for {saved_session_id}: {e}"),
                                    Err(e) => tracing::warn!("scrollback append task panicked for {saved_session_id}: {e}"),
                                    Ok(Ok(())) => {}
                                }
                            });
                        }
                        let closed = matches!(event, SessionEvent::Closed { .. });
                        emit(app, id, &event);
                        if closed {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }
}

/// Mirrors each `SessionEvent` variant onto its own `session:<id>:<kind>`
/// channel so the frontend's `listen("session:<id>:data", ...)` matches the
/// architecture doc literally, while lifecycle events get their own channels.
fn emit(app: &AppHandle, id: &str, event: &SessionEvent) {
    let kind = match event {
        SessionEvent::Data { .. } => "data",
        SessionEvent::TitleChanged { .. } => "title",
        SessionEvent::StateChanged { .. } => "state",
        SessionEvent::Closed { .. } => "closed",
        SessionEvent::Error { .. } => "error",
    };
    let _ = app.emit(&format!("session:{id}:{kind}"), event);
}
