use std::{fs, path::PathBuf};

fn store_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("spn").join("peers.json")
}

/// Return all persisted peer multiaddr strings.
pub fn load() -> Vec<String> {
    let path = store_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Overwrite the store with the given list.
pub fn save(addrs: &[String]) {
    let path = store_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(addrs) {
        let _ = fs::write(path, s);
    }
}

/// Append a multiaddr string to the store (no-op if already present).
pub fn add(addr: &str) {
    let mut addrs = load();
    if !addrs.iter().any(|a| a == addr) {
        addrs.push(addr.to_string());
        save(&addrs);
    }
}
