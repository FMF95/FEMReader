use crate::cache::IncrementalCache;
use crate::ids::{leaf_name, stable_id};
use crate::model::{DirectoryNode, FileCategory, FileNode, SolverKind};
use crate::solver::is_model_file_path;
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
            let dir_source = current_dir.display().to_string();
            let dir_id = stable_id("dir", &dir_source);
            directories.push(DirectoryNode {
                id: dir_id,
                name: leaf_name(&current_dir),
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
                    if is_internal_femreader_file(&path) {
                        continue;
                    }

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
                        id: stable_id("file", &canonical_file.display().to_string()),
                        name: leaf_name(&canonical_file),
                        path: canonical_file,
                        extension,
                        size,
                        modified_unix_secs,
                        changed_since_last_scan: changed,
                        category: if is_model_file_path(&path) {
                            FileCategory::Model
                        } else {
                            FileCategory::NonModel
                        },
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

fn is_internal_femreader_file(path: &Path) -> bool {
    path.file_name()
        .map(|name| {
            name.to_string_lossy()
                .eq_ignore_ascii_case(".femreader-cache")
        })
        .unwrap_or(false)
}
