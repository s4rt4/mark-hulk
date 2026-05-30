import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { app } from "$lib/stores/app.svelte.js";

/** True only inside the native Tauri window (not the browser preview). */
export function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Open a folder picker and load it as the workspace. */
export async function openFolder() {
  if (!isTauri()) return;
  const dir = await open({ directory: true, multiple: false });
  if (!dir) return;
  await loadFolder(dir);
}

export async function loadFolder(dir) {
  const root = await invoke("read_dir_tree", { path: dir });
  app.rootPath = dir;
  app.tree = [normalize(root)];
  await watch(dir);
}

/** Open a single .md file via the file picker. */
export async function openFile() {
  if (!isTauri()) return;
  const file = await open({
    multiple: false,
    filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdx"] }],
  });
  if (!file) return;
  await openPath(file);
}

/** Load a file's content into a tab (focusing it if already open). */
export async function openPath(path) {
  if (!isTauri()) return;
  const content = await invoke("read_file", { path });
  app.openDoc({ path, content });
  await watch(path);
}

/** Persist the active document to disk (prompts for a path if untitled). */
export async function saveFile() {
  if (!isTauri()) return false;
  let path = app.currentPath;
  if (!path) {
    path = await save({
      defaultPath: app.fileName,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (!path) return false;
  }
  await invoke("write_file", { path, content: app.content });
  app.markSaved(path);
  await watch(path);
  return true;
}

let watching = null;
async function watch(path) {
  if (!isTauri()) return;
  watching = path;
  await invoke("watch_path", { path });
}

/** Search the workspace for a query; returns [{path, name, line, text}]. */
export async function searchWorkspace(query) {
  const q = query.trim();
  if (!q) return [];
  if (isTauri()) {
    if (!app.rootPath) return [];
    return await invoke("search_workspace", { root: app.rootPath, query: q });
  }
  // Browser fallback: search inline demo content.
  const hits = [];
  const lower = q.toLowerCase();
  const walk = (nodes) => {
    for (const n of nodes || []) {
      if (n.isDir) walk(n.children);
      else if (n.content) {
        n.content.split("\n").forEach((line, i) => {
          if (line.toLowerCase().includes(lower)) {
            hits.push({ path: n.path, name: n.name, line: i + 1, text: line.trim().slice(0, 200) });
          }
        });
      }
    }
  };
  walk(app.tree);
  return hits.slice(0, 500);
}

/** Convert Rust snake_case tree into the frontend shape. */
function normalize(node) {
  return {
    name: node.name,
    path: node.path,
    isDir: node.is_dir,
    children: node.children ? node.children.map(normalize) : undefined,
  };
}

/** If the app was launched by opening a .md file, load it. */
export async function loadLaunchFile() {
  if (!isTauri()) return;
  try {
    const path = await invoke("get_launch_file");
    if (path) await openPath(path);
  } catch (_) {
    /* no launch file */
  }
}

/** Wire app-level event listeners once, at app start. */
export async function initWatcher() {
  if (!isTauri()) return;

  // A second instance (e.g. double-clicking another .md) routes the file here.
  await listen("open-file", async (event) => {
    if (event.payload) await openPath(event.payload);
  });

  await listen("fs-change", async (event) => {
    const changed = event.payload || [];
    // Reload any open, clean tab whose file changed on disk.
    for (const tab of app.tabs) {
      if (tab.path && changed.includes(tab.path) && !tab.dirty) {
        try {
          const fresh = await invoke("read_file", { path: tab.path });
          tab.content = fresh;
          tab.savedContent = fresh;
        } catch (_) {
          /* file may have been removed */
        }
      }
    }
  });
}
