use portus_sftp::SftpClient;
use portus_ssh::{SshAuth, SshConnectOptions};

/// Expects a local sshd reachable at 127.0.0.1:2222 with pubkey auth and an
/// `internal-sftp` subsystem configured, matching portus-ssh's own test
/// harness. Skipped (not failed) if unset, since no such server exists in a
/// default checkout/CI run.
fn test_env() -> Option<(String, String)> {
    let key = std::env::var("PORTUS_TEST_SSH_KEY").ok()?;
    let user = std::env::var("PORTUS_TEST_SSH_USER").ok()?;
    Some((key, user))
}

async fn connect() -> Option<SftpClient> {
    let (key_path, username) = test_env()?;
    let options = SshConnectOptions {
        host: "127.0.0.1".to_string(),
        port: 2222,
        username,
        auth: SshAuth::PrivateKey { path: key_path, passphrase: None },
    };
    Some(SftpClient::connect(&options).await.expect("sftp connect failed"))
}

#[tokio::test]
async fn sftp_round_trips_a_directory_and_a_file() {
    let Some(sftp) = connect().await else {
        eprintln!("skipping: PORTUS_TEST_SSH_KEY / PORTUS_TEST_SSH_USER not set");
        return;
    };

    // A fresh scratch directory under the test account's home, named
    // uniquely so repeated runs don't collide with a stale leftover.
    let dir = format!("portus-sftp-test-{}", uuid_like());
    sftp.create_dir(&dir).await.expect("create_dir failed");

    let before = sftp.list(&dir).await.expect("list (empty) failed");
    assert!(before.is_empty(), "freshly created dir should be empty, got {before:?}");

    let file_path = format!("{dir}/hello.txt");
    sftp.write_file(&file_path, b"PORTUS_SFTP_OK").await.expect("write_file failed");

    let listed = sftp.list(&dir).await.expect("list (with file) failed");
    assert_eq!(listed.len(), 1, "expected exactly one entry, got {listed:?}");
    assert_eq!(listed[0].name, "hello.txt");
    assert!(!listed[0].is_dir);
    assert_eq!(listed[0].size, "PORTUS_SFTP_OK".len() as u64);

    let contents = sftp.read_file(&file_path).await.expect("read_file failed");
    assert_eq!(contents, b"PORTUS_SFTP_OK");

    sftp.remove_file(&file_path).await.expect("remove_file failed");
    let after_remove = sftp.list(&dir).await.expect("list (after remove) failed");
    assert!(after_remove.is_empty(), "dir should be empty again, got {after_remove:?}");

    sftp.remove_dir(&dir).await.expect("remove_dir failed");
}

/// Cheap unique-enough suffix without pulling in the `uuid` crate here.
fn uuid_like() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
