//! Serial port `Session` implementation (`serialport` crate). Same shape as
//! `portus-shell`'s PTY session — `serialport`'s API is blocking, so reads
//! live on a dedicated OS thread and only the resulting bytes cross into
//! async land.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serialport::SerialPort;
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent, SessionState};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialConnectOptions {
    pub port_name: String,
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
}

fn default_baud_rate() -> u32 {
    9600
}

/// Ports currently visible to the OS, for populating a picker in the
/// frontend rather than making the user type a device path from memory.
pub fn list_ports() -> Vec<String> {
    serialport::available_ports()
        .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

pub struct SerialSession {
    options: SerialConnectOptions,
    writer: Option<Arc<Mutex<Box<dyn SerialPort>>>>,
    shutdown: Option<Arc<AtomicBool>>,
}

impl SerialSession {
    pub fn new(options: SerialConnectOptions) -> Self {
        Self { options, writer: None, shutdown: None }
    }
}

#[async_trait]
impl Session for SerialSession {
    async fn start(&mut self, events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connecting });

        let opts = self.options.clone();
        let port = tokio::task::spawn_blocking(move || {
            serialport::new(opts.port_name, opts.baud_rate)
                .timeout(Duration::from_millis(200))
                .open()
        })
        .await
        .map_err(|e| SessionError::Protocol(format!("open task panicked: {e}")))?
        .map_err(|e| SessionError::Protocol(format!("failed to open serial port: {e}")))?;

        let reader = port
            .try_clone()
            .map_err(|e| SessionError::Protocol(format!("failed to clone serial port handle: {e}")))?;

        let shutdown = Arc::new(AtomicBool::new(false));
        self.writer = Some(Arc::new(Mutex::new(port)));
        self.shutdown = Some(shutdown.clone());

        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connected });

        tokio::task::spawn_blocking(move || read_loop(reader, events, shutdown));

        Ok(())
    }

    async fn write(&mut self, data: Bytes) -> Result<(), SessionError> {
        let writer = self.writer.clone().ok_or(SessionError::Closed)?;
        tokio::task::spawn_blocking(move || {
            let mut w = writer.lock().expect("serial port mutex poisoned");
            w.write_all(&data)?;
            w.flush()
        })
        .await
        .map_err(|e| SessionError::Protocol(format!("write task panicked: {e}")))??;
        Ok(())
    }

    /// Serial is a raw byte stream with no notion of a terminal grid — a
    /// resize is meaningful to xterm.js's local rendering only, there's
    /// nothing to forward to the device.
    async fn resize(&mut self, _cols: u16, _rows: u16) -> Result<(), SessionError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), SessionError> {
        if let Some(flag) = self.shutdown.take() {
            flag.store(true, Ordering::SeqCst);
        }
        self.writer = None;
        Ok(())
    }
}

fn read_loop(mut reader: Box<dyn SerialPort>, events: mpsc::UnboundedSender<SessionEvent>, shutdown: Arc<AtomicBool>) {
    let mut buf = [0u8; 4096];
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => continue,
            Ok(n) => {
                if events
                    .send(SessionEvent::Data { data: Bytes::copy_from_slice(&buf[..n]) })
                    .is_err()
                {
                    break; // frontend/adapter is gone, stop reading
                }
            }
            // The read timeout is how we get a chance to notice `shutdown`
            // without blocking forever on a quiet line — not an error.
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break, // device unplugged, port closed elsewhere, etc.
        }
    }
    let _ = events.send(SessionEvent::StateChanged { state: SessionState::Disconnected });
    let _ = events.send(SessionEvent::Closed { reason: None });
}
