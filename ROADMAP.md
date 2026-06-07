# Mark-Hulk Roadmap

Mark-Hulk was rewritten from a Tauri/WebView app into a **native GTK4 + Rust**
app, targeting Linux, to be as light and fast as possible (idle ~45 MB vs the
old ~330 MB).

Current state (**v0.2**): native GTK4/libadwaita shell, GtkSourceView editor,
tabs, Preview/Split/Edit modes, native preview with syntax-highlighted code and
tables, workspace explorer, cross-file search, file watcher (auto-reload),
`.md` file association, and the green "Forest Sage" theme.

## Quick wins

- Unsaved-changes guard when closing a tab or the window
- Scroll sync between editor and preview in Split mode
- Draggable divider remembers its position
- "Copy" button on rendered code blocks

## Next — writing & navigation

- Formatting shortcuts (`Ctrl+B`/`Ctrl+I`, links)
- Smart editing: auto-pair, continue lists, toggle checkboxes
- Find & Replace within the document
- Outline / Table of Contents panel from headings
- Quick Open fuzzy file finder (`Ctrl+P`)
- Session restore: reopen tabs and last window size

## Preview & rendering

- Real GtkGrid tables instead of monospace text
- A second (light) theme and a theme switcher
- Settings panel (font, size, theme) that persists
- Optional math rendering (pre-rendered images) for the rare math-heavy file

## Distribution

- `.rpm` packaging (Fedora) and a Flatpak
- Desktop integration polish (icon themes, app metadata)

## Notes

- Native rendering means no embedded browser: Mermaid diagrams and live KaTeX
  math are intentionally out of scope. The priority is speed and footprint.
- Contributions and suggestions are welcome via issues.
