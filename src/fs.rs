//! Filesystem helpers: workspace tree, read/write, and cross-file search.
//! Ported from the original Tauri backend (`src-tauri/src/lib.rs`) — pure Rust,
//! no GTK dependency, so it stays easy to test in isolation.

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A node in the workspace file tree (directories + markdown files only).
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown") | Some("mdx")
    )
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules" || name == "target"
}

/// Build a tree of directories and markdown files rooted at `path`.
/// Empty folders that contain no markdown anywhere below are pruned.
pub fn build_tree(path: &Path) -> Option<TreeNode> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    if path.is_dir() {
        let mut children: Vec<TreeNode> = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| {
                let p = e.path();
                (!p.is_dir(), e.file_name().to_string_lossy().to_lowercase())
            });
            for entry in entries {
                let p = entry.path();
                let fname = entry.file_name().to_string_lossy().to_string();
                if is_hidden(&fname) {
                    continue;
                }
                if p.is_dir() {
                    if let Some(node) = build_tree(&p) {
                        // Drop empty folders that contain no markdown.
                        if !node.children.is_empty() {
                            children.push(node);
                        }
                    }
                } else if is_markdown(&p) {
                    children.push(TreeNode {
                        name: fname,
                        path: p,
                        is_dir: false,
                        children: Vec::new(),
                    });
                }
            }
        }
        Some(TreeNode {
            name,
            path: path.to_path_buf(),
            is_dir: true,
            children,
        })
    } else {
        Some(TreeNode {
            name,
            path: path.to_path_buf(),
            is_dir: false,
            children: Vec::new(),
        })
    }
}

pub fn read_file(path: &Path) -> std::io::Result<String> {
    fs::read_to_string(path)
}

pub fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    fs::write(path, content)
}

/// A single matched line from a workspace search.
pub struct SearchHit {
    pub path: PathBuf,
    pub name: String,
    pub line: usize,
    pub text: String,
}

/// Search all markdown files under `root` for `query` (case-insensitive).
pub fn search_workspace(root: &Path, query: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return hits;
    }
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !is_hidden(&name)
    });
    for entry in walker.filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_markdown(p) {
            continue;
        }
        if let Ok(content) = fs::read_to_string(p) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&q) {
                    hits.push(SearchHit {
                        path: p.to_path_buf(),
                        name: p
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        line: i + 1,
                        text: line.trim().chars().take(200).collect(),
                    });
                    if hits.len() >= 500 {
                        return hits;
                    }
                }
            }
        }
    }
    hits
}
