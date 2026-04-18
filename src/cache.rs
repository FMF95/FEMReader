use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct CacheEntry {
    pub modified_unix_secs: u64,
    pub size: u64,
}

#[derive(Debug, Default)]
pub struct IncrementalCache {
    entries: HashMap<String, CacheEntry>,
}

impl IncrementalCache {
    pub fn load(path: &Path) -> io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut entries = HashMap::new();
        let content = fs::read_to_string(path)?;
        for line in content.lines() {
            let mut parts = line.splitn(3, '\t');
            let Some(key) = parts.next() else { continue };
            let Some(modified) = parts.next() else {
                continue;
            };
            let Some(size) = parts.next() else { continue };

            if let (Ok(modified_unix_secs), Ok(size)) =
                (modified.parse::<u64>(), size.parse::<u64>())
            {
                entries.insert(
                    key.to_string(),
                    CacheEntry {
                        modified_unix_secs,
                        size,
                    },
                );
            }
        }

        Ok(Self { entries })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut lines = Vec::with_capacity(self.entries.len());
        for (key, entry) in &self.entries {
            lines.push(format!(
                "{key}\t{}\t{}",
                entry.modified_unix_secs, entry.size
            ));
        }
        lines.sort();
        fs::write(path, lines.join("\n"))?;
        mark_hidden_if_supported(path)
    }

    pub fn remove_file(path: &Path) -> io::Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn is_changed(&self, key: &str, modified_unix_secs: u64, size: u64) -> bool {
        match self.entries.get(key) {
            Some(entry) => entry.modified_unix_secs != modified_unix_secs || entry.size != size,
            None => true,
        }
    }

    pub fn update(&mut self, key: String, modified_unix_secs: u64, size: u64) {
        self.entries.insert(
            key,
            CacheEntry {
                modified_unix_secs,
                size,
            },
        );
    }
}

#[cfg(windows)]
fn mark_hidden_if_supported(path: &Path) -> io::Result<()> {
    let status = std::process::Command::new("attrib")
        .arg("+H")
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "Unable to mark cache as hidden: {}",
            path.display()
        )))
    }
}

#[cfg(not(windows))]
fn mark_hidden_if_supported(_: &Path) -> io::Result<()> {
    Ok(())
}
