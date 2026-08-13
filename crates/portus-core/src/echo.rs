//! A trivial [`Session`] that echoes back whatever it's written, plus a
//! greeting on start. Used to prove the command/event bridge end-to-end
//! before any real protocol crate exists.

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::session::{Session, SessionError, SessionEvent, SessionState};

pub struct EchoSession {
    events: Option<mpsc::UnboundedSender<SessionEvent>>,
}

impl Default for EchoSession {
    fn default() -> Self {
        Self { events: None }
    }
}

#[async_trait]
impl Session for EchoSession {
    async fn start(&mut self, events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connecting });
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connected });
        let _ = events.send(SessionEvent::Data {
            data: Bytes::from_static(b"portus echo session ready\r\n$ "),
        });
        self.events = Some(events);
        Ok(())
    }

    async fn write(&mut self, data: Bytes) -> Result<(), SessionError> {
        let Some(events) = &self.events else {
            return Err(SessionError::Closed);
        };
        let _ = events.send(SessionEvent::Data { data: data.clone() });
        if data.as_ref() == b"\r" {
            let _ = events.send(SessionEvent::Data { data: Bytes::from_static(b"\n$ ") });
        }
        Ok(())
    }

    async fn resize(&mut self, _cols: u16, _rows: u16) -> Result<(), SessionError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), SessionError> {
        if let Some(events) = self.events.take() {
            let _ = events.send(SessionEvent::StateChanged { state: SessionState::Disconnected });
            let _ = events.send(SessionEvent::Closed { reason: None });
        }
        Ok(())
    }
}
