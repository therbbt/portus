//! A minimal TOFU (trust-on-first-use) known-hosts store, one JSON file
//! (`known_hosts.json`, sibling to `config.json` in Portus's config dir)
//! mapping `"host:port"` to the base64-encoded public key blob last seen
//! for it. Deliberately not OpenSSH's `known_hosts` format — we don't need
//! to interop with it, and JSON keeps this consistent with the rest of
//! Portus's on-disk state.

use std::collections::HashMap;
use std::path::PathBuf;

pub fn path() -> PathBuf {
    portus_core::config::config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("known_hosts.json")
}

fn load(path: &std::path::Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(path: &std::path::Path, entries: &HashMap<String, String>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(path, json);
    }
}

pub enum Verdict {
    /// First time we've seen this host — key was recorded, connection may proceed.
    TrustedOnFirstUse,
    /// Matches what we recorded last time.
    Known,
    /// The presented key does NOT match what we recorded — possible MITM.
    Mismatch,
}

pub fn verify(host_id: &str, presented_key_base64: &str) -> Verdict {
    let path = path();
    let mut entries = load(&path);
    match entries.get(host_id) {
        Some(stored) if stored == presented_key_base64 => Verdict::Known,
        Some(_) => Verdict::Mismatch,
        None => {
            entries.insert(host_id.to_string(), presented_key_base64.to_string());
            save(&path, &entries);
            Verdict::TrustedOnFirstUse
        }
    }
}
