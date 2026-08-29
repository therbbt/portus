//! Disk-backed scrollback for saved sessions — currently only wired up for
//! local-shell presets (see `app/portus/src/adapter.rs`), but keyed
//! generically by saved session id so any protocol could opt in later.
//!
//! Each saved session gets its own capped file at `<config_dir>/scrollback/<id>.log`
//! holding the raw output bytes (ANSI sequences included) most recently
//! written to its terminal. On reconnect, `read_tail` hands those bytes
//! back so the frontend can replay them into a fresh xterm.js instance
//! before live output resumes — xterm.js reconstructs the prior screen
//! state the same way it would have if it had been open the whole time.
//!
//! Truncation is a plain byte-count cutoff, not ANSI-aware, so the very
//! first thing replayed after a truncation can occasionally be a partial
//! escape sequence — a minor, self-correcting cosmetic glitch (the next
//! real escape sequence resyncs it), not a correctness issue.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

use uuid::Uuid;

use crate::config::config_dir;

/// Cap a scrollback file is truncated back down to once it crosses
/// [`TRUNCATE_THRESHOLD`].
const MAX_BYTES: u64 = 256 * 1024;
/// Truncation only fires once the file has grown 50% past `MAX_BYTES`,
/// rather than on every single append once at the cap — turns an O(n)
/// rewrite-the-whole-file operation into an occasional one instead of a
/// per-write one.
const TRUNCATE_THRESHOLD: u64 = MAX_BYTES + MAX_BYTES / 2;

fn scrollback_path(saved_session_id: Uuid) -> io::Result<PathBuf> {
    let dir = config_dir()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .join("scrollback");
    Ok(dir.join(format!("{saved_session_id}.log")))
}

/// Appends freshly-arrived output bytes for `saved_session_id`'s scrollback,
/// truncating the file back to `MAX_BYTES` (keeping the tail) if it's
/// grown well past that.
pub fn append(saved_session_id: Uuid, data: &[u8]) -> io::Result<()> {
    let path = scrollback_path(saved_session_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(data)?;
    drop(file);

    if std::fs::metadata(&path)?.len() > TRUNCATE_THRESHOLD {
        let contents = std::fs::read(&path)?;
        let start = contents.len().saturating_sub(MAX_BYTES as usize);
        std::fs::write(&path, &contents[start..])?;
    }

    Ok(())
}

/// The full saved scrollback for `saved_session_id`, or an empty vec if it
/// has none yet (first connect, or scrollback was cleared).
pub fn read_tail(saved_session_id: Uuid) -> io::Result<Vec<u8>> {
    match std::fs::read(scrollback_path(saved_session_id)?) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Removes a saved session's scrollback file entirely — called when the
/// saved session itself is deleted, so it doesn't leave an orphaned file
/// behind.
pub fn clear(saved_session_id: Uuid) -> io::Result<()> {
    match std::fs::remove_file(scrollback_path(saved_session_id)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_then_read_tail_round_trips() {
        let id = Uuid::new_v4();
        clear(id).unwrap();

        append(id, b"hello ").unwrap();
        append(id, b"world").unwrap();
        assert_eq!(read_tail(id).unwrap(), b"hello world");

        clear(id).unwrap();
        assert_eq!(read_tail(id).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn append_truncates_once_well_past_the_cap() {
        let id = Uuid::new_v4();
        clear(id).unwrap();

        // One append past TRUNCATE_THRESHOLD in a single write, so this
        // exercises the truncate path without needing thousands of calls.
        let chunk = vec![b'x'; (TRUNCATE_THRESHOLD + 1) as usize];
        append(id, &chunk).unwrap();

        let tail = read_tail(id).unwrap();
        assert_eq!(tail.len(), MAX_BYTES as usize, "should truncate back down to MAX_BYTES");
        assert!(tail.iter().all(|&b| b == b'x'));

        clear(id).unwrap();
    }
}
