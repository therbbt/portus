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
    let local_path = std::env::temp_dir().join(format!("portus-sftp-test-upload-{}", uuid_like()));
    std::fs::write(&local_path, b"PORTUS_SFTP_OK").expect("write local temp file failed");
    sftp.upload_from_file(&local_path, &file_path).await.expect("upload_from_file failed");
    std::fs::remove_file(&local_path).ok();

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

#[tokio::test]
async fn sftp_downloads_a_nested_directory_tree() {
    let Some(sftp) = connect().await else {
        eprintln!("skipping: PORTUS_TEST_SSH_KEY / PORTUS_TEST_SSH_USER not set");
        return;
    };

    // dir/
    //   top.txt
    //   sub/
    //     nested.txt
    let dir = format!("portus-sftp-test-dirdl-{}", uuid_like());
    let sub = format!("{dir}/sub");
    sftp.create_dir(&dir).await.expect("create_dir (top) failed");
    sftp.create_dir(&sub).await.expect("create_dir (sub) failed");

    let local_src = std::env::temp_dir().join(format!("portus-sftp-test-dirdl-src-{}", uuid_like()));
    std::fs::write(&local_src, b"TOP").expect("write local temp file failed");
    sftp.upload_from_file(&local_src, &format!("{dir}/top.txt")).await.expect("upload (top) failed");
    std::fs::write(&local_src, b"NESTED").expect("overwrite local temp file failed");
    sftp.upload_from_file(&local_src, &format!("{sub}/nested.txt")).await.expect("upload (nested) failed");
    std::fs::remove_file(&local_src).ok();

    let local_dest = std::env::temp_dir().join(format!("portus-sftp-test-dirdl-dest-{}", uuid_like()));
    sftp.download_dir_to(&dir, &local_dest).await.expect("download_dir_to failed");

    assert_eq!(std::fs::read(local_dest.join("top.txt")).expect("top.txt missing"), b"TOP");
    assert_eq!(std::fs::read(local_dest.join("sub").join("nested.txt")).expect("sub/nested.txt missing"), b"NESTED");

    std::fs::remove_dir_all(&local_dest).ok();

    sftp.remove_file(&format!("{sub}/nested.txt")).await.expect("remove_file (nested) failed");
    sftp.remove_dir(&sub).await.expect("remove_dir (sub) failed");
    sftp.remove_file(&format!("{dir}/top.txt")).await.expect("remove_file (top) failed");
    sftp.remove_dir(&dir).await.expect("remove_dir (top) failed");
}

/// Cheap unique-enough suffix without pulling in the `uuid` crate here.
fn uuid_like() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
