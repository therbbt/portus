//! Local shell `Session`, backed by `portable-pty`. This is the byte source
//! for Milestone 2 — everything downstream (grid rendering, reflow,
//! scrollback) is xterm.js's job, not this crate's.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent, SessionState};

pub struct ShellSession {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    child: Option<Box<dyn Child + Send + Sync>>,
}

impl Default for ShellSession {
    fn default() -> Self {
        Self { master: None, writer: None, child: None }
    }
}

#[async_trait]
impl Session for ShellSession {
    async fn start(&mut self, events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connecting });

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| SessionError::Protocol(format!("failed to open pty: {e}")))?;

        let mut cmd = CommandBuilder::new(default_shell());
        if let Ok(home) = std::env::var("HOME") {
            cmd.cwd(home);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SessionError::Protocol(format!("failed to spawn shell: {e}")))?;
        // Drop our copy of the slave end now that the child has it, so the
        // master's reader sees EOF when the child actually exits.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SessionError::Protocol(format!("failed to clone pty reader: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SessionError::Protocol(format!("failed to take pty writer: {e}")))?;

        self.master = Some(pair.master);
        self.writer = Some(Arc::new(Mutex::new(writer)));
        self.child = Some(child);

        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connected });

        // Blocking reads live on a dedicated OS thread; only the resulting
        // bytes cross back into async land, so an idle shell costs nothing.
        tokio::task::spawn_blocking(move || read_loop(reader, events));

        Ok(())
    }

    async fn write(&mut self, data: Bytes) -> Result<(), SessionError> {
        let writer = self.writer.clone().ok_or(SessionError::Closed)?;
        tokio::task::spawn_blocking(move || {
            let mut w = writer.lock().expect("pty writer mutex poisoned");
            w.write_all(&data)?;
            w.flush()
        })
        .await
        .map_err(|e| SessionError::Protocol(format!("write task panicked: {e}")))??;
        Ok(())
    }

    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        let Some(master) = &self.master else {
            return Err(SessionError::Closed);
        };
        master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| SessionError::Protocol(format!("resize failed: {e}")))?;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), SessionError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        self.writer = None;
        self.master = None;
        Ok(())
    }
}

fn read_loop(mut reader: Box<dyn Read + Send>, events: mpsc::UnboundedSender<SessionEvent>) {
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if events
                    .send(SessionEvent::Data { data: Bytes::copy_from_slice(&buf[..n]) })
                    .is_err()
                {
                    break; // frontend/adapter is gone, stop reading
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    let _ = events.send(SessionEvent::StateChanged { state: SessionState::Disconnected });
    let _ = events.send(SessionEvent::Closed { reason: None });
}

fn default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    }
}
