//! Secrets (passwords, key passphrases, RDP credentials) never touch the
//! config JSON. They live in the OS keychain (macOS Keychain, Windows
//! Credential Manager, Secret Service on Linux) and the config only stores
//! an opaque handle string pointing at one.

use thiserror::Error;
use uuid::Uuid;

const SERVICE: &str = "com.portus.app";

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain backend error: {0}")]
    Backend(#[from] keyring::Error),
}

/// Generates a fresh opaque handle and stores `secret` under it. The
/// returned handle is what gets written into `Host::credential_handle`.
pub fn store(secret: &str) -> Result<String, KeychainError> {
    let handle = Uuid::new_v4().to_string();
    let entry = keyring::Entry::new(SERVICE, &handle)?;
    entry.set_password(secret)?;
    Ok(handle)
}

pub fn retrieve(handle: &str) -> Result<String, KeychainError> {
    let entry = keyring::Entry::new(SERVICE, handle)?;
    Ok(entry.get_password()?)
}

pub fn delete(handle: &str) -> Result<(), KeychainError> {
    let entry = keyring::Entry::new(SERVICE, handle)?;
    entry.delete_credential()?;
    Ok(())
}
