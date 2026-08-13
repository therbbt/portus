//! Telnet `Session` implementation. Lands in Milestone 4, once the
//! stream -> session -> xterm path proven by the shell/SSH milestones
//! makes this a cheap addition.

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent};

pub struct TelnetSession {
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    port: u16,
}

impl TelnetSession {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

#[async_trait]
impl Session for TelnetSession {
    async fn start(&mut self, _events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        Err(SessionError::Protocol("telnet session not yet implemented (Milestone 4)".into()))
    }

    async fn write(&mut self, _data: Bytes) -> Result<(), SessionError> {
        Err(SessionError::Closed)
    }

    async fn resize(&mut self, _cols: u16, _rows: u16) -> Result<(), SessionError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), SessionError> {
        Ok(())
    }
}
