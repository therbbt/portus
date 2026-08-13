use std::time::Duration;

use bytes::Bytes;
use portus_core::session::{Session, SessionEvent, SessionState};
use portus_shell::ShellSession;
use tokio::sync::mpsc;
use tokio::time::timeout;

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

/// Proves the byte pipe end to end without any GUI involved: spawn a real
/// shell behind the pty, round-trip a command through write() -> stdout ->
/// Data events, and confirm resize() doesn't error (SIGWINCH plumbing).
#[tokio::test]
async fn shell_session_echoes_written_command() {
    let mut session = ShellSession::default();
    let (tx, mut rx) = mpsc::unbounded_channel();

    session.start(tx).await.expect("session failed to start");
    wait_for_state(&mut rx, SessionState::Connected).await;

    session
        .write(Bytes::from_static(b"echo PORTUS_PTY_OK\n"))
        .await
        .expect("write failed");

    let mut seen = String::new();
    let saw_marker = timeout(Duration::from_secs(5), async {
        loop {
            seen.push_str(&next_data(&mut rx).await);
            if seen.contains("PORTUS_PTY_OK") {
                return;
            }
        }
    })
    .await;
    assert!(saw_marker.is_ok(), "never saw echoed marker in pty output: {seen:?}");

    session.resize(120, 40).await.expect("resize failed");

    session.shutdown().await.expect("shutdown failed");
}
