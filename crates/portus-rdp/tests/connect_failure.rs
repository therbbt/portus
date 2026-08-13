use portus_rdp::{RdpClient, RdpConnectOptions, RdpEvent};
use tokio::sync::mpsc;

/// There's no RDP server available to test the actual protocol path
/// (connect/auth/framebuffer decode) against in this environment — unlike
/// every other protocol crate here, that path is unverified beyond
/// compiling and matching the upstream reference client's API usage. This
/// at least proves the async plumbing around it works: connecting to a
/// port nothing is listening on should fail promptly and cleanly, not
/// hang or panic.
#[tokio::test]
async fn connect_to_a_closed_port_fails_promptly() {
    let options = RdpConnectOptions {
        host: "127.0.0.1".to_string(),
        port: 1, // reserved, nothing listens here
        username: "test".to_string(),
        password: "test".to_string(),
        domain: None,
    };

    let (tx, _rx) = mpsc::unbounded_channel::<RdpEvent>();
    let mut client = RdpClient::default();

    let result = tokio::time::timeout(std::time::Duration::from_secs(5), client.connect(options, tx))
        .await
        .expect("connect() hung instead of failing promptly");

    assert!(result.is_err(), "expected connect to a closed port to fail");
}
