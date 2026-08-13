use std::time::Duration;

use bytes::Bytes;
use portus_core::session::{Session, SessionEvent, SessionState};
use portus_ssh::{SshAuth, SshConnectOptions, SshSession};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// These tests expect a local sshd reachable at 127.0.0.1:2222 with pubkey
/// auth configured against the key path in `PORTUS_TEST_SSH_KEY`, and a
/// username in `PORTUS_TEST_SSH_USER`. Skipped (not failed) if unset, since
/// no such server exists in a default checkout/CI run.
fn test_env() -> Option<(String, String)> {
    let key = std::env::var("PORTUS_TEST_SSH_KEY").ok()?;
    let user = std::env::var("PORTUS_TEST_SSH_USER").ok()?;
    Some((key, user))
}

async fn next_data(rx: &mut mpsc::UnboundedReceiver<SessionEvent>) -> String {
    loop {
        let event = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for session event")
            .expect("session event stream ended unexpectedly");
        if let SessionEvent::Data { data } = event {
            return String::from_utf8_lossy(&data).to_string();
        }
    }
}

async fn wait_for_state(rx: &mut mpsc::UnboundedReceiver<SessionEvent>, want: SessionState) {
    loop {
        let event = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for state")
            .expect("session event stream ended unexpectedly");
        if let SessionEvent::StateChanged { state } = event {
            if state == want {
                return;
            }
        }
    }
}

#[tokio::test]
async fn ssh_session_echoes_written_command_over_real_sshd() {
    let Some((key_path, username)) = test_env() else {
        eprintln!("skipping: PORTUS_TEST_SSH_KEY / PORTUS_TEST_SSH_USER not set");
        return;
    };

    let options = SshConnectOptions {
        host: "127.0.0.1".to_string(),
        port: 2222,
        username,
        auth: SshAuth::PrivateKey { path: key_path, passphrase: None },
    };

    let mut session = SshSession::new(options);
    let (tx, mut rx) = mpsc::unbounded_channel();

    session.start(tx).await.expect("session failed to start");
    wait_for_state(&mut rx, SessionState::Connected).await;

    session
        .write(Bytes::from_static(b"echo PORTUS_SSH_OK\n"))
        .await
        .expect("write failed");

    let mut seen = String::new();
    let saw_marker = timeout(Duration::from_secs(5), async {
        loop {
            seen.push_str(&next_data(&mut rx).await);
            if seen.contains("PORTUS_SSH_OK") {
                return;
            }
        }
    })
    .await;
    assert!(saw_marker.is_ok(), "never saw echoed marker in ssh output: {seen:?}");

    session.resize(120, 40).await.expect("resize failed");
    session.shutdown().await.expect("shutdown failed");
}

#[tokio::test]
async fn ssh_session_rejects_bad_password() {
    let Some((_key_path, username)) = test_env() else {
        eprintln!("skipping: PORTUS_TEST_SSH_KEY / PORTUS_TEST_SSH_USER not set");
        return;
    };

    let options = SshConnectOptions {
        host: "127.0.0.1".to_string(),
        port: 2222,
        username,
        auth: SshAuth::Password { password: "definitely-wrong".to_string() },
    };

    let mut session = SshSession::new(options);
    let (tx, _rx) = mpsc::unbounded_channel();
    let result = session.start(tx).await;
    assert!(result.is_err(), "expected auth failure to be reported as an error");
}
