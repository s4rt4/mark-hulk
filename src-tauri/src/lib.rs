use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use walkdir::WalkDir;

/// A node in the workspace file tree (directories + markdown files only).
#[derive(Serialize)]
struct TreeNode {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeNode>>,
}

fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown") | Some("mdx")
    )
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules" || name == "target"
}

fn build_tree(path: &Path) -> Option<TreeNode> {
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
                        if node
                            .children
                            .as_ref()
                            .map(|c| !c.is_empty())
                            .unwrap_or(false)
                        {
                            children.push(node);
                        }
                    }
                } else if is_markdown(&p) {
                    children.push(TreeNode {
                        name: fname,
                        path: p.to_string_lossy().to_string(),
                        is_dir: false,
                        children: None,
                    });
                }
            }
        }
        Some(TreeNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: true,
            children: Some(children),
        })
    } else {
        Some(TreeNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: false,
            children: None,
        })
    }
}

#[tauri::command]
fn read_dir_tree(path: String) -> Result<TreeNode, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    build_tree(p).ok_or_else(|| "Could not read directory".to_string())
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

/// A single matched line from a workspace search.
#[derive(Serialize)]
struct SearchHit {
    path: String,
    name: String,
    line: usize,
    text: String,
}

/// Search all markdown files under `root` for `query` (case-insensitive).
#[tauri::command]
fn search_workspace(root: String, query: String) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return hits;
    }
    let walker = WalkDir::new(&root).into_iter().filter_entry(|e| {
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
                        path: p.to_string_lossy().to_string(),
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

/// Holds the active filesystem watcher so it stays alive between calls.
struct WatcherState(Mutex<Option<RecommendedWatcher>>);

/// The file the app was launched with (e.g. double-clicked in Explorer).
struct LaunchFile(Mutex<Option<String>>);

/// Pick the first existing markdown path from the launch arguments.
fn launch_file_from_args() -> Option<String> {
    std::env::args().skip(1).find(|arg| {
        let p = Path::new(arg);
        p.is_file() && is_markdown(p)
    })
}

/// Returns the markdown file the app was opened with, if any (consumed once).
#[tauri::command]
fn get_launch_file(state: State<LaunchFile>) -> Option<String> {
    state.0.lock().ok().and_then(|mut g| g.take())
}

/// Watch a file or folder; emits `fs-change` with the changed paths.
#[tauri::command]
fn watch_path(
    path: String,
    app: AppHandle,
    state: State<WatcherState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Dropping the previous watcher stops the old subscription.
    *guard = None;

    let handle = app.clone();
    let mut watcher = notify::recommended_watcher(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                use notify::EventKind::*;
                if matches!(event.kind, Modify(_) | Create(_) | Remove(_)) {
                    let paths: Vec<String> = event
                        .paths
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    let _ = handle.emit("fs-change", paths);
                }
            }
        },
    )
    .map_err(|e| e.to_string())?;

    watcher
        .watch(Path::new(&path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    *guard = Some(watcher);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance must be registered first: a second launch (e.g. opening
        // another .md from Explorer) is routed into the running window instead.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
            if let Some(file) = argv.iter().skip(1).find(|a| {
                let p = Path::new(a);
                p.is_file() && is_markdown(p)
            }) {
                let _ = app.emit("open-file", file.clone());
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(WatcherState(Mutex::new(None)));
            app.manage(LaunchFile(Mutex::new(launch_file_from_args())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_dir_tree,
            read_file,
            write_file,
            watch_path,
            get_launch_file,
            search_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
