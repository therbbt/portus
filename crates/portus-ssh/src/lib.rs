//! SSH `Session` implementation (russh). Auth flows, known-hosts checking,
//! and resize land in Milestone 3; this crate currently exposes the shape
//! so `app/portus` can depend on it without knowing russh exists.

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent};

pub struct SshConnectOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
}

pub struct SshSession {
    #[allow(dead_code)]
    options: SshConnectOptions,
}

impl SshSession {
    pub fn new(options: SshConnectOptions) -> Self {
        Self { options }
    }
}

#[async_trait]
impl Session for SshSession {
    async fn start(&mut self, _events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        Err(SessionError::Protocol("ssh session not yet implemented (Milestone 3)".into()))
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
