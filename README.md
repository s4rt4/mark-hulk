# Mark-Hulk

A native, lightweight Markdown **viewer and editor** for Windows — modern UI,
tiny footprint, deeply integrated with your `.md` files.

Built with **Tauri 2** (Rust), **SvelteKit** (Svelte 5), and **Tailwind v4**.
It runs on the system WebView2, so the installer stays a few megabytes and
memory use stays low.

![Mark-Hulk — Forest Sage and Dark Emerald themes](screenshot.jpg)

## Features

- **Workspace explorer** — open a folder and browse a recursive `.md` file tree.
- **Tabbed editing** — open multiple documents at once; each tab tracks its own
  unsaved state.
- **Three view modes** — Preview, Split (live), and Edit.
- **CodeMirror 6 editor** — markdown syntax highlighting, line numbers, and
  soft-wrapping.
- **Rich rendering** — headings, tables, task lists, footnotes, code
  highlighting, and YAML frontmatter.
- **Math and diagrams** — inline and block math with KaTeX, flowcharts and more
  with Mermaid (both themed to match the UI).
- **Cross-file search** — search the whole workspace and jump straight to a
  result.
- **Two themes** — Forest Sage (default) and Dark Emerald, switched live.
- **File watcher** — the open file auto-reloads when it changes on disk.
- **Export** — save a self-contained HTML file, or print to PDF.
- **File association** — markdown files get their own document icon in Explorer
  and open in Mark-Hulk on double-click.
- **Single instance** — opening another file reuses the running window in a new
  tab instead of launching a second copy.

## Keyboard shortcuts

| Shortcut       | Action            |
| -------------- | ----------------- |
| `Ctrl+S`       | Save              |
| `Ctrl+N`       | New file          |
| `Ctrl+B`       | Toggle sidebar    |
| `Ctrl+Shift+F` | Search workspace  |
| `Ctrl+1/2/3`   | Preview/Split/Edit |

## Themes

| Theme        | Base      | Accent    | Source            |
| ------------ | --------- | --------- | ----------------- |
| Forest Sage  | `#0b231a` | `#95d1af` | derived from logo |
| Dark Emerald | `#0a0c0a` | `#2ecc71` | palette 1         |

## Development

```bash
pnpm install
pnpm tauri dev      # native window (compiles Rust on first run)
pnpm dev            # browser-only UI preview (no file system)
```

## Build

```bash
pnpm tauri build    # produces an .msi and an .exe (NSIS) installer
```

The NSIS `setup.exe` is recommended: it is the smallest bundle and the one that
installs the custom `.md` document icon.

## Project layout

```
src/                        SvelteKit frontend
  lib/components/           Sidebar, TreeNode, Toolbar, TabBar,
                            CodeEditor, Preview, Search
  lib/markdown/            markdown-it pipeline (KaTeX, Mermaid) + themed styles
  lib/services/fs.js       bridge to the Rust file commands and dialogs
  lib/services/export.js   HTML and PDF export
  lib/stores/              reactive app state (runes, multi-tab)
src-tauri/src/lib.rs        Rust commands: read tree, read/write, watch, search
src-tauri/installer/        NSIS hook + branded installer images
scripts/build-assets.ps1    regenerates the file icon and installer art
```

## Tech stack

| Layer    | Choice                                            |
| -------- | ------------------------------------------------- |
| Shell    | Tauri 2 (Rust, WebView2)                           |
| UI       | SvelteKit / Svelte 5 runes, Tailwind v4           |
| Editor   | CodeMirror 6                                       |
| Markdown | markdown-it, highlight.js, KaTeX, Mermaid          |

## License

MIT
