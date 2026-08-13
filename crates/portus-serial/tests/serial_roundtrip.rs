use std::io::{Read, Write};
use std::time::Duration;

use bytes::Bytes;
use portus_core::session::{Session, SessionEvent, SessionState};
use portus_serial::{SerialConnectOptions, SerialSession};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Expects a virtual null-modem pair (e.g. via `socat -d -d pty,raw,echo=0,link=$A
/// pty,raw,echo=0,link=$B`) with `PORTUS_TEST_SERIAL_PORT_A` / `_B` pointing at the
/// two ends. Skipped (not failed) if unset, since no such device exists by default.
fn test_ports() -> Option<(String, String)> {
    let a = std::env::var("PORTUS_TEST_SERIAL_PORT_A").ok()?;
    let b = std::env::var("PORTUS_TEST_SERIAL_PORT_B").ok()?;
    Some((a, b))
}

async fn next_data(rx: &mut mpsc::UnboundedReceiver<SessionEvent>) -> Vec<u8> {
    loop {
        let event = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for session event")
            .expect("session event stream ended unexpectedly");
        if let SessionEvent::Data { data } = event {
            return data.to_vec();
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
async fn serial_session_write_reaches_the_other_end_of_the_line() {
    let Some((port_a, port_b)) = test_ports() else {
        eprintln!("skipping: PORTUS_TEST_SERIAL_PORT_A / _B not set");
        return;
    };

    let mut session = SerialSession::new(SerialConnectOptions { port_name: port_a, baud_rate: 9600 });
    let (tx, mut rx) = mpsc::unbounded_channel();
    session.start(tx).await.expect("session failed to start");
    wait_for_state(&mut rx, SessionState::Connected).await;

    // portus-serial -> the other end of the virtual line.
    session.write(Bytes::from_static(b"PORTUS_SERIAL_OK\n")).await.expect("write failed");

    let mut other_end = std::fs::OpenOptions::new()
        .read(true)
        .open(&port_b)
        .expect("failed to open other end of virtual serial line");
    let mut buf = [0u8; 256];
    let n = other_end.read(&mut buf).expect("read from other end failed");
    assert_eq!(&buf[..n], b"PORTUS_SERIAL_OK\n");

    session.shutdown().await.expect("shutdown failed");
}

#[tokio::test]
async fn serial_session_receives_bytes_written_from_the_other_end() {
    let Some((port_a, port_b)) = test_ports() else {
        eprintln!("skipping: PORTUS_TEST_SERIAL_PORT_A / _B not set");
        return;
    };

    let mut session = SerialSession::new(SerialConnectOptions { port_name: port_a, baud_rate: 9600 });
    let (tx, mut rx) = mpsc::unbounded_channel();
    session.start(tx).await.expect("session failed to start");
    wait_for_state(&mut rx, SessionState::Connected).await;

    // The other end of the virtual line -> portus-serial.
    let mut other_end = std::fs::OpenOptions::new()
        .write(true)
        .open(&port_b)
        .expect("failed to open other end of virtual serial line");
    other_end.write_all(b"HELLO_FROM_DEVICE\n").expect("write to other end failed");
    other_end.flush().ok();

    let mut seen = Vec::new();
    let saw_marker = timeout(Duration::from_secs(5), async {
        loop {
            seen.extend(next_data(&mut rx).await);
            if seen.windows(17).any(|w| w == b"HELLO_FROM_DEVICE") {
                return;
            }
        }
    })
    .await;
    assert!(saw_marker.is_ok(), "never saw device bytes: {:?}", String::from_utf8_lossy(&seen));

    session.shutdown().await.expect("shutdown failed");
}
