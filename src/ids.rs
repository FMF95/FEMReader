use std::path::Path;

pub fn stable_id(prefix: &str, source: &str) -> String {
    let combined = format!("{prefix}|{source}");
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in combined.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
