//! The Tauri command + event bridge. This module is deliberately dumb: it
//! owns each live `Session`, forwards commands down into it, and forwards
//! its events back out onto the Tauri event bus. It never decides protocol
//! behavior — that all lives in the `Session` implementations themselves.

use std::collections::HashMap;
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
}

impl AppState {
    /// Construct a `Session` for `protocol`, hand it its own tokio task, and
    /// return the id the frontend will use to address it from now on.
    /// `options` carries protocol-specific connect parameters (e.g. host/
    /// auth for SSH) as JSON — this function is the only place that knows
    /// how to turn that JSON into a concrete `Session`.
    pub fn open(&self, protocol: &str, options: serde_json::Value, app: AppHandle) -> Result<String, String> {
        let session: Box<dyn Session> = match protocol {
            "echo" => Box::new(portus_core::echo::EchoSession::default()),
            "shell" => Box::new(portus_shell::ShellSession::default()),
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

        let id = Uuid::new_v4().to_string();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        self.sessions.lock().unwrap().insert(id.clone(), cmd_tx);

        let sessions = self.sessions.clone();
        let task_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_session(session, cmd_rx, &app, &task_id).await;
            sessions.lock().unwrap().remove(&task_id);
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
) {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    if let Err(e) = session.start(event_tx).await {
        emit(app, id, &SessionEvent::Error { message: e.to_string() });
        emit(app, id, &SessionEvent::Closed { reason: Some(e.to_string()) });
        return;
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
