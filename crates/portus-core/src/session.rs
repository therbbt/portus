use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

/// Every protocol (shell, ssh, telnet, serial, rdp) implements this trait in
/// its own crate. The app crate only ever talks to a `Box<dyn Session>` — it
/// never knows which protocol crate produced it.
#[async_trait]
pub trait Session: Send {
    /// Start the session's I/O loop. Implementations spawn their own tokio
    /// task(s) internally and must not block the caller. Output bytes and
    /// lifecycle notifications are pushed onto `events` as they occur.
    async fn start(&mut self, events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError>;

    /// Send raw input (keystrokes, pasted text) to the remote/local process.
    async fn write(&mut self, data: Bytes) -> Result<(), SessionError>;

    /// Notify the session of a terminal grid resize, in character cells.
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError>;

    /// Tear the session down and release its resources.
    async fn shutdown(&mut self) -> Result<(), SessionError>;
}

/// Events flow one direction: protocol crate -> event bridge -> frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// Raw output bytes to be written into the terminal/canvas.
    Data { data: Bytes },
    /// The session's own idea of a title changed (e.g. shell `cwd`, SSH banner).
    TitleChanged { title: String },
    /// Connection state changed (used to drive the status dot in the UI).
    StateChanged { state: SessionState },
    /// The session ended, cleanly or otherwise.
    Closed { reason: Option<String> },
    /// A non-fatal error the UI should surface (e.g. auth prompt failure).
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("session already closed")]
    Closed,
}

/// Which protocol crate should produce the `Session` for a given tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Shell,
    Ssh,
    Telnet,
    Serial,
    Rdp,
}
