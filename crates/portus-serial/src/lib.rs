//! Serial port `Session` implementation (serialport crate). Lands in
//! Milestone 4 alongside telnet.

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent};

pub struct SerialSession {
    #[allow(dead_code)]
    port_name: String,
    #[allow(dead_code)]
    baud_rate: u32,
}

impl SerialSession {
    pub fn new(port_name: String, baud_rate: u32) -> Self {
        Self { port_name, baud_rate }
    }
}

#[async_trait]
impl Session for SerialSession {
    async fn start(&mut self, _events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        Err(SessionError::Protocol("serial session not yet implemented (Milestone 4)".into()))
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
