//! Watch the directory of the open file and notify the UI thread when that file
//! changes on disk. We watch the parent directory (not the file itself) so that
//! atomic saves — write-to-temp + rename, as many editors do — are still caught.

use std::path::{Path, PathBuf};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Start watching `file`'s parent directory. Every change to `file` is sent
/// down `tx` as the changed path. The returned watcher must be kept alive.
pub fn watch_file(
    file: &Path,
    tx: async_channel::Sender<PathBuf>,
) -> notify::Result<RecommendedWatcher> {
    let target = file.to_path_buf();
    let dir = file.parent().unwrap_or(file).to_path_buf();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            use notify::EventKind::*;
            if matches!(event.kind, Modify(_) | Create(_) | Remove(_))
                && event.paths.iter().any(|p| p == &target)
            {
                let _ = tx.send_blocking(target.clone());
            }
        }
    })?;

    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
