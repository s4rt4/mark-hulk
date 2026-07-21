// Central reactive app state (Svelte 5 runes).
// Multi-document: `tabs` holds open files; `active` is the focused one.
// `content`/`currentPath`/`dirty` are convenience accessors for the active tab.

export const SAMPLE = `---
title: "Welcome to Mark-Hulk"
keywords:
  - Markdown
  - Tauri
  - SvelteKit
---

# Markdown, redefined. [^1]

Mark-Hulk **supercharges your reading experience** and stays out of your way.
It renders standard Markdown directly — pull in your existing notes and start
working immediately. It uses a clean, focused approach and renders elements
like [links](https://tauri.app) for your convenience.

A native, lightweight viewer & editor aimed at people who live in \`.md\` files.
No playful clutter — just pure efficiency. It supports:

- [x] Live preview & split editing
- [x] Syntax highlighting
- [x] YAML frontmatter
- [x] Math & diagrams
- [ ] ...and much, much more!

## Code blocks

\`\`\`js
function greet(name) {
  return \`Hello, \${name}! You're reading in Mark-Hulk.\`;
}
\`\`\`

## Math

Inline $E = mc^2$, and a display block:

$$
\\int_0^\\infty e^{-x}\\,dx = 1
$$

## Diagram

\`\`\`mermaid
flowchart LR
  A[Write .md] --> B{Mark-Hulk}
  B --> C[Preview]
  B --> D[Edit]
\`\`\`

## Blockquote

> Writing is thinking. Mark-Hulk just gets out of the way.

[^1]: Of course, Mark-Hulk also supports footnotes!
`;

function baseName(path) {
  return path ? path.split(/[\\/]/).pop() : "Untitled.md";
}

class AppState {
  // ui
  theme = $state("sage"); // "sage" | "emerald"
  view = $state("preview"); // "preview" | "split" | "edit"
  sidebarOpen = $state(true);
  searchOpen = $state(false);

  // documents
  tabs = $state([]); // {id, path, name, content, savedContent, dirty}
  activeId = $state(null);

  // workspace
  rootPath = $state(null);
  tree = $state([]);

  // recent files (shown on the Home screen when no tab is open)
  recent = $state([]); // {path, name, at}
  recentView = $state("grid"); // "grid" | "list"

  _seq = 0;

  constructor() {
    // Restore the last-used theme, recent files, and view mode, if any.
    try {
      const saved = localStorage.getItem("mh-theme");
      if (saved && ["sage", "emerald", "light"].includes(saved)) {
        this.theme = saved;
      }
      const recent = JSON.parse(localStorage.getItem("mh-recent") || "[]");
      if (Array.isArray(recent)) {
        this.recent = recent.filter((r) => r && typeof r.path === "string");
      }
      const rv = localStorage.getItem("mh-recent-view");
      if (rv === "grid" || rv === "list") this.recentView = rv;
    } catch (_) {
      /* localStorage unavailable */
    }
  }

  // ---- active-tab accessors ----
  get active() {
    return this.tabs.find((t) => t.id === this.activeId) || null;
  }
  get content() {
    return this.active ? this.active.content : "";
  }
  set content(v) {
    const t = this.active;
    if (t) {
      t.content = v;
      t.dirty = v !== t.savedContent;
    }
  }
  get currentPath() {
    return this.active ? this.active.path : null;
  }
  get dirty() {
    return this.active ? this.active.dirty : false;
  }
  get fileName() {
    return this.active ? this.active.name : "No file";
  }
  get wordCount() {
    const m = this.content.trim().match(/\S+/g);
    return m ? m.length : 0;
  }
  get charCount() {
    return this.content.length;
  }

  // ---- tab management ----
  /** Open a document; if its path is already open, focus that tab. */
  openDoc({ path = null, name = null, content = "" }) {
    if (path) {
      this.addRecent(path, name);
      const existing = this.tabs.find((t) => t.path === path);
      if (existing) {
        existing.content = content;
        existing.savedContent = content;
        existing.dirty = false;
        this.activeId = existing.id;
        return existing;
      }
    }
    const id = ++this._seq;
    const tab = {
      id,
      path,
      name: name || baseName(path),
      content,
      savedContent: content,
      dirty: false,
    };
    this.tabs.push(tab);
    this.activeId = id;
    return tab;
  }

  newFile() {
    this.openDoc({ content: "", name: "Untitled.md" });
  }

  closeTab(id) {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    this.tabs.splice(idx, 1);
    if (this.activeId === id) {
      const next = this.tabs[idx] || this.tabs[idx - 1] || null;
      this.activeId = next ? next.id : null;
    }
    // No auto "Untitled" tab: an empty tab list shows the Home screen.
  }

  activate(id) {
    this.activeId = id;
  }

  /** Mark the active tab as saved, optionally assigning a path (save-as). */
  markSaved(path = null) {
    const t = this.active;
    if (!t) return;
    if (path) {
      t.path = path;
      t.name = baseName(path);
      this.addRecent(path);
    }
    t.savedContent = t.content;
    t.dirty = false;
  }

  // ---- recent files ----
  addRecent(path, name = null) {
    if (!path) return;
    const entry = { path, name: name || baseName(path), at: Date.now() };
    this.recent = [
      entry,
      ...this.recent.filter((r) => r.path !== path),
    ].slice(0, 30);
    this._persistRecent();
  }

  removeRecent(path) {
    this.recent = this.recent.filter((r) => r.path !== path);
    this._persistRecent();
  }

  clearRecent() {
    this.recent = [];
    this._persistRecent();
  }

  setRecentView(v) {
    if (v !== "grid" && v !== "list") return;
    this.recentView = v;
    try {
      localStorage.setItem("mh-recent-view", v);
    } catch (_) {
      /* localStorage unavailable */
    }
  }

  _persistRecent() {
    try {
      localStorage.setItem("mh-recent", JSON.stringify(this.recent));
    } catch (_) {
      /* localStorage unavailable */
    }
  }

  setTheme(id) {
    if (AppState.THEMES.some((t) => t.id === id)) this.theme = id;
  }

  toggleTheme() {
    const ids = AppState.THEMES.map((t) => t.id);
    const next = (ids.indexOf(this.theme) + 1) % ids.length;
    this.theme = ids[next];
  }

  setView(v) {
    this.view = v;
  }
}

export const THEMES = [
  { id: "sage", label: "Forest Sage", swatch: "#95d1af" },
  { id: "emerald", label: "Dark Emerald", swatch: "#2ecc71" },
  { id: "light", label: "Daylight", swatch: "#2f8a5b" },
];
AppState.THEMES = THEMES;

export const app = new AppState();
