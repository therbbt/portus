//! SSH `Session` implementation (russh). Primary use case per the
//! architecture doc — a new byte source behind the same `Session` trait and
//! the same xterm.js frontend the local shell already uses.
//!
//! [`connect`] is also the entry point `portus-sftp` uses to get an
//! authenticated connection to run the SFTP subsystem over, so host-key
//! verification and auth only live in one place.

mod known_hosts;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use russh::client::{self, Handle};
use russh::keys::key::PublicKey;
use russh::keys::{load_secret_key, PublicKeyBase64};
use russh::{Channel, ChannelMsg, Disconnect};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use portus_core::session::{Session, SessionError, SessionEvent, SessionState};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SshAuth {
    Password { password: String },
    PrivateKey { path: String, passphrase: Option<String> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SshConnectOptions {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
}

fn default_port() -> u16 {
    22
}

/// Connects and authenticates, verifying the host key via the TOFU
/// known-hosts store along the way. `events`, if given, gets a colored
/// inline notice for a first-seen host key — pass `None` when there's no
/// terminal to write into (e.g. an SFTP-only connection).
pub async fn connect(
    options: &SshConnectOptions,
    events: Option<mpsc::UnboundedSender<SessionEvent>>,
) -> Result<Handle<HostKeyVerifier>, SessionError> {
    let host_id = format!("{}:{}", options.host, options.port);
    let rejection_reason = Arc::new(Mutex::new(None));
    let handler = HostKeyVerifier { host_id, events, rejection_reason: rejection_reason.clone() };

    let config = Arc::new(client::Config::default());
    let mut handle = client::connect(config, (options.host.as_str(), options.port), handler)
        .await
        .map_err(|e| {
            // A rejected host key surfaces from russh as a generic
            // "unknown key" error — prefer the detailed MITM-warning
            // message the handler stashed, when there is one.
            let reason = rejection_reason.lock().expect("poisoned").take();
            SessionError::Protocol(reason.unwrap_or_else(|| format!("connect failed: {e}")))
        })?;

    authenticate(&mut handle, options).await?;
    Ok(handle)
}

enum SshCommand {
    Write(Bytes),
    Resize(u16, u16),
    Shutdown,
}

pub struct SshSession {
    options: SshConnectOptions,
    cmd_tx: Option<mpsc::UnboundedSender<SshCommand>>,
}

impl SshSession {
    pub fn new(options: SshConnectOptions) -> Self {
        Self { options, cmd_tx: None }
    }
}

#[async_trait]
impl Session for SshSession {
    async fn start(&mut self, events: mpsc::UnboundedSender<SessionEvent>) -> Result<(), SessionError> {
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connecting });

        let opts = self.options.clone();
        let handle = connect(&opts, Some(events.clone())).await?;

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SessionError::Protocol(format!("failed to open channel: {e}")))?;

        channel
            .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
            .await
            .map_err(|e| SessionError::Protocol(format!("pty request failed: {e}")))?;
        channel
            .request_shell(true)
            .await
            .map_err(|e| SessionError::Protocol(format!("shell request failed: {e}")))?;

        let _ = events.send(SessionEvent::TitleChanged { title: format!("{}@{}", opts.username, opts.host) });
        let _ = events.send(SessionEvent::StateChanged { state: SessionState::Connected });

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        self.cmd_tx = Some(cmd_tx);

        // `handle` is moved in here too: dropping it would tear the
        // connection down, so it has to live exactly as long as the channel
        // does, on the one task that owns both.
        tokio::spawn(run_channel(handle, channel, cmd_rx, events));

        Ok(())
    }

    async fn write(&mut self, data: Bytes) -> Result<(), SessionError> {
        let tx = self.cmd_tx.as_ref().ok_or(SessionError::Closed)?;
        tx.send(SshCommand::Write(data)).map_err(|_| SessionError::Closed)
    }

    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        let tx = self.cmd_tx.as_ref().ok_or(SessionError::Closed)?;
        tx.send(SshCommand::Resize(cols, rows)).map_err(|_| SessionError::Closed)
    }

    async fn shutdown(&mut self) -> Result<(), SessionError> {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(SshCommand::Shutdown);
        }
        Ok(())
    }
}

async fn authenticate(handle: &mut Handle<HostKeyVerifier>, opts: &SshConnectOptions) -> Result<(), SessionError> {
    let ok = match &opts.auth {
        SshAuth::Password { password } => handle
            .authenticate_password(&opts.username, password)
            .await
            .map_err(|e| SessionError::Protocol(format!("password auth failed: {e}")))?,
        SshAuth::PrivateKey { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|e| SessionError::Protocol(format!("failed to load private key: {e}")))?;
            handle
                .authenticate_publickey(&opts.username, Arc::new(key))
                .await
                .map_err(|e| SessionError::Protocol(format!("public key auth failed: {e}")))?
        }
    };
    if !ok {
        return Err(SessionError::Protocol("authentication rejected by server".into()));
    }
    Ok(())
}

/// Owns the channel and the connection handle for the session's lifetime,
/// alternating between commands coming down from `Session::write`/`resize`
/// and messages coming up from the server — same pattern as the top-level
/// Tauri adapter, kept local here so the app crate never has to know a
/// channel exists.
async fn run_channel(
    handle: Handle<HostKeyVerifier>,
    mut channel: Channel<client::Msg>,
    mut cmd_rx: mpsc::UnboundedReceiver<SshCommand>,
    events: mpsc::UnboundedSender<SessionEvent>,
) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(SshCommand::Write(data)) => {
                        if channel.data(&data[..]).await.is_err() {
                            break;
                        }
                    }
                    Some(SshCommand::Resize(cols, rows)) => {
                        let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                    }
                    Some(SshCommand::Shutdown) | None => {
                        let _ = channel.eof().await;
                        let _ = channel.close().await;
                        break;
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        let _ = events.send(SessionEvent::Data { data: Bytes::copy_from_slice(&data) });
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        let _ = events.send(SessionEvent::Data { data: Bytes::copy_from_slice(&data) });
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }

    let _ = handle.disconnect(Disconnect::ByApplication, "", "en").await;
    let _ = events.send(SessionEvent::StateChanged { state: SessionState::Disconnected });
    let _ = events.send(SessionEvent::Closed { reason: None });
}

pub struct HostKeyVerifier {
    host_id: String,
    events: Option<mpsc::UnboundedSender<SessionEvent>>,
    rejection_reason: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl client::Handler for HostKeyVerifier {
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        let presented = server_public_key.public_key_base64();
        match known_hosts::verify(&self.host_id, &presented) {
            known_hosts::Verdict::Known => Ok(true),
            known_hosts::Verdict::TrustedOnFirstUse => {
                let msg = format!(
                    "trusting new SSH host key for {} (fingerprint SHA256:{})",
                    self.host_id,
                    server_public_key.fingerprint()
                );
                tracing::info!("{msg}");
                if let Some(events) = &self.events {
                    let _ = events.send(SessionEvent::Data {
                        data: Bytes::from(format!("\r\n\x1b[33m[portus] {msg}\x1b[0m\r\n")),
                    });
                }
                Ok(true)
            }
            known_hosts::Verdict::Mismatch => {
                let msg = format!(
                    "SSH host key for {} has changed (fingerprint now SHA256:{}) — refusing to connect, possible MITM",
                    self.host_id,
                    server_public_key.fingerprint()
                );
                tracing::warn!("{msg}");
                *self.rejection_reason.lock().expect("poisoned") = Some(msg.clone());
                if let Some(events) = &self.events {
                    let _ = events.send(SessionEvent::Error { message: msg });
                }
                Ok(false)
            }
        }
    }
}
