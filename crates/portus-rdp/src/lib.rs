//! RDP `Session` implementation (IronRDP). Hardest protocol, fully isolated
//! here per the architecture — the framebuffer/input plumbing and the
//! IronRDP wasm/canvas frontend view land together in Milestone 6.

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent};

pub struct RdpSession {
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    port: u16,
}

impl RdpSession {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

#[async_trait]
impl Session for RdpSession {
    async fn start(&mut self, _events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        Err(SessionError::Protocol("rdp session not yet implemented (Milestone 6)".into()))
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
