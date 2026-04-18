use crate::cache::IncrementalCache;
use crate::model::{DirectoryNode, FileNode, SolverKind};
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub root_directory: PathBuf,
    pub directories: Vec<DirectoryNode>,
    pub files: Vec<FileNode>,
}

pub trait FileSystemScanner {
    fn scan(&self, root_directory: &Path, cache: &mut IncrementalCache) -> io::Result<ScanResult>;
}

#[derive(Debug, Default)]
pub struct DefaultFileSystemScanner;

impl FileSystemScanner for DefaultFileSystemScanner {
    fn scan(&self, root_directory: &Path, cache: &mut IncrementalCache) -> io::Result<ScanResult> {
        let canonical_root = fs::canonicalize(root_directory)?;
        let mut directories = Vec::new();
        let mut files = Vec::new();

        let mut queue = VecDeque::new();
        queue.push_back(canonical_root.clone());

        while let Some(current_dir) = queue.pop_front() {
            let dir_id = format!("dir:{}", current_dir.display());
            directories.push(DirectoryNode {
                id: dir_id,
                path: current_dir.clone(),
            });

            for entry in fs::read_dir(&current_dir)? {
                let entry = entry?;
                let path = entry.path();
                let metadata = entry.metadata()?;

                if metadata.is_dir() {
                    queue.push_back(path);
                    continue;
                }

                if metadata.is_file() {
                    let canonical_file = fs::canonicalize(&path).unwrap_or(path.clone());
                    let extension = canonical_file
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_ascii_lowercase());
                    let modified_unix_secs = metadata
                        .modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let size = metadata.len();
                    let cache_key = canonical_file.display().to_string();
                    let changed = cache.is_changed(&cache_key, modified_unix_secs, size);
                    cache.update(cache_key, modified_unix_secs, size);

                    files.push(FileNode {
                        id: format!("file:{}", canonical_file.display()),
                        path: canonical_file,
                        extension,
                        size,
                        modified_unix_secs,
                        changed_since_last_scan: changed,
                        solver_hint: SolverKind::Unknown,
                    });
                }
            }
        }

        files.sort_by(|a, b| a.path.cmp(&b.path));
        directories.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(ScanResult {
            root_directory: canonical_root,
            directories,
            files,
        })
    }
}
