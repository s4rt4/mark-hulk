# Mark-Hulk Roadmap

Current release: **v0.1.0** — workspace explorer, tabs, Preview/Split/Edit,
CodeMirror editor, KaTeX + Mermaid, cross-file search, two themes, file watcher,
HTML/PDF export, `.md` association with a custom icon, single-instance, and an
NSIS/MSI installer.

This document lists planned work. Items are grouped by milestone; nothing here
is committed to a date.

## Quick wins

Small, high-value items that can land independently.

- Unsaved-changes guard when closing a tab or the window
- Scroll sync between editor and preview in Split mode
- Draggable divider to resize the split
- "Copy" button on code blocks

## v0.2 — Writing experience

Make authoring as comfortable as reading.

- Formatting toolbar and shortcuts (`Ctrl+B`, `Ctrl+I`, `Ctrl+K` for links)
- Smart editing: auto-pair `**`/`[]`, continue lists on Enter, toggle checkboxes
- Find & Replace within the document (`Ctrl+H`)
- Paste or drag an image, save it to an `assets/` folder, and insert the link

## v0.3 — Navigation & session

Move fast, and remember state across restarts.

- Outline / Table of Contents panel built from headings
- Quick Open fuzzy file finder (`Ctrl+P`)
- Session restore: reopen tabs and the last window size/position
- Settings panel (font, size, theme) that persists
- A Light theme (both current themes are dark)
- Recent files and folders

## v0.4 — Trustworthy distribution

Automate releases and remove platform warnings.

- GitHub Actions CI: build `.exe`/`.msi` and publish a release on each tag
- Auto-updater (Tauri updater)
- Code signing to remove the SmartScreen "Unknown publisher" warning
- Cross-platform builds: macOS (`.dmg`) and Linux (`.AppImage`/`.deb`)

## v0.5 — Knowledge base

Grow from a viewer into a note-taking tool.

- Wiki-links `[[...]]` with backlinks
- Tags and tag-based search
- Callouts / admonitions (`:::note`, `:::warning`)
- Command palette (`Ctrl+Shift+P`)
- Additional export: DOCX and custom export themes

## Notes

- Priorities depend on the goal: a personal daily tool leans on v0.2 + v0.3;
  a public product leans on v0.4; a PKM/notes app leans on v0.5.
- Contributions and suggestions are welcome via issues.
