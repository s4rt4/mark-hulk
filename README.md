# Mark-Hulk

A native, lightweight Markdown **viewer & editor** for Windows — modern UI,
tiny footprint, deeply integrated with your `.md` files.

Built with **Tauri 2** (Rust) + **SvelteKit** (Svelte 5) + **Tailwind v4**.
Uses the system WebView2, so the binary stays small and memory use stays low.

## Features

- 📁 **Workspace explorer** — open a folder, browse a recursive `.md` file tree
- 👁️ **Three view modes** — Preview · Split (live) · Edit
- ✍️ **CodeMirror 6 editor** — markdown syntax highlighting, line numbers
- 🎨 **Two themes** — Forest Sage (default) & Dark Emerald, toggled live
- 🧩 **Rich rendering** — headings, tables, task lists, footnotes, code
  highlighting, YAML frontmatter
- 🔄 **File watcher** — auto-reloads the open file when it changes on disk
- 🔗 **`.md` file association** — double-click a markdown file to open it here
- ⌨️ **Shortcuts** — `Ctrl+S` save · `Ctrl+B` sidebar · `Ctrl+1/2/3` views

## Themes

| Theme            | Base       | Accent     | Source            |
| ---------------- | ---------- | ---------- | ----------------- |
| 🌲 Forest Sage   | `#0b231a`  | `#95d1af`  | derived from logo |
| ⬛ Dark Emerald   | `#0a0c0a`  | `#2ecc71`  | palette 1         |

## Development

```bash
pnpm install
pnpm tauri dev      # native window (compiles Rust on first run)
pnpm dev            # browser-only UI preview (no file system)
```

## Build

```bash
pnpm tauri build    # produces an .msi / .exe installer
```

## Project layout

```
src/                      SvelteKit frontend
  lib/components/         Sidebar, Toolbar, CodeEditor, Preview, TreeNode
  lib/markdown/           markdown-it pipeline + themed styles
  lib/services/fs.js      bridge to the Rust file commands + dialogs
  lib/stores/             reactive app state (runes)
src-tauri/src/lib.rs      Rust commands: read tree, read/write, watch
```
