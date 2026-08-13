use portus_core::session::Session;
use portus_ssh::{SshAuth, SshConnectOptions, SshSession};
use tokio::sync::mpsc;

/// Companion to ssh_roundtrip.rs's TOFU test: run this AFTER a successful
/// connect to 127.0.0.1:2222 has already recorded that host's key, then the
/// server's host key changed (simulated MITM / server reinstall). Expects
/// `start()` to fail rather than silently connecting.
#[tokio::test]
async fn ssh_session_rejects_changed_host_key() {
    let Ok(key_path) = std::env::var("PORTUS_TEST_SSH_KEY") else {
        eprintln!("skipping: PORTUS_TEST_SSH_KEY not set");
        return;
    };
    let username = std::env::var("PORTUS_TEST_SSH_USER").unwrap_or_else(|_| "test".to_string());

    let options = SshConnectOptions {
        host: "127.0.0.1".to_string(),
        port: 2222,
        username,
        auth: SshAuth::PrivateKey { path: key_path, passphrase: None },
    };

    let mut session = SshSession::new(options);
    let (tx, _rx) = mpsc::unbounded_channel();
    let result = session.start(tx).await;
    assert!(result.is_err(), "expected connection to be rejected after host key changed");
}
