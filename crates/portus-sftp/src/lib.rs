//! SFTP file transfer, layered over an existing `portus-ssh` connection
//! rather than implementing `Session` itself (it's a file panel, not a
//! terminal stream). Lands in Milestone 5.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SftpError {
    #[error("sftp not yet implemented (Milestone 5)")]
    NotImplemented,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct SftpClient;

impl SftpClient {
    pub async fn list(&self, _path: &str) -> Result<Vec<DirEntry>, SftpError> {
        Err(SftpError::NotImplemented)
    }
}
