//! SFTP file transfer, layered over its own authenticated SSH connection
//! (via `portus_ssh::connect`) rather than implementing `Session` itself —
//! this is a file panel, not a terminal stream, so it gets its own
//! request/response API instead of a byte pipe.

use portus_ssh::{HostKeyVerifier, SshConnectOptions};
use russh::client::Handle;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Error)]
pub enum SftpError {
    #[error(transparent)]
    Connect(#[from] portus_core::session::SessionError),
    #[error("ssh error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("sftp error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Owns both the SFTP session and the SSH connection it runs over — the
/// connection has to stay alive for exactly as long as the session does,
/// since dropping the `Handle` tears down the channel underneath it.
pub struct SftpClient {
    session: SftpSession,
    _handle: Handle<HostKeyVerifier>,
}

impl SftpClient {
    pub async fn connect(options: &SshConnectOptions) -> Result<Self, SftpError> {
        let handle = portus_ssh::connect(options, None).await?;
        let channel = handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;
        Ok(Self { session, _handle: handle })
    }

    pub async fn list(&self, path: &str) -> Result<Vec<DirEntry>, SftpError> {
        let mut entries: Vec<DirEntry> = self
            .session
            .read_dir(path)
            .await?
            .map(|entry| DirEntry {
                name: entry.file_name(),
                is_dir: entry.metadata().is_dir(),
                size: entry.metadata().len(),
            })
            .collect();
        // Directories first, then alphabetical within each group.
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
        Ok(entries)
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, SftpError> {
        let mut file = self.session.open(path).await?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    pub async fn write_file(&self, path: &str, data: &[u8]) -> Result<(), SftpError> {
        let mut file = self.session.create(path).await?;
        file.write_all(data).await?;
        file.shutdown().await?;
        Ok(())
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), SftpError> {
        self.session.remove_file(path).await?;
        Ok(())
    }

    pub async fn create_dir(&self, path: &str) -> Result<(), SftpError> {
        self.session.create_dir(path).await?;
        Ok(())
    }

    pub async fn remove_dir(&self, path: &str) -> Result<(), SftpError> {
        self.session.remove_dir(path).await?;
        Ok(())
    }
}
