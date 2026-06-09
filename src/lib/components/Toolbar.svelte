<script>
  import {
    PanelLeft,
    Eye,
    Columns2,
    Pencil,
    Palette,
    Search,
    FolderOpen,
    Save,
    Download,
    Check,
  } from "lucide-svelte";
  import { app, THEMES } from "$lib/stores/app.svelte.js";
  import { openFolder, saveFile } from "$lib/services/fs.js";
  import { exportHtml, exportPdf } from "$lib/services/export.js";

  const views = [
    { id: "preview", icon: Eye, label: "Preview" },
    { id: "split", icon: Columns2, label: "Split" },
    { id: "edit", icon: Pencil, label: "Edit" },
  ];

  let exportMenu = $state(false);
  let themeMenu = $state(false);

  function runExport(fn) {
    exportMenu = false;
    fn();
  }

  function pickTheme(id) {
    themeMenu = false;
    app.setTheme(id);
  }

  function closeMenus() {
    exportMenu = false;
    themeMenu = false;
  }
</script>

<svelte:window onclick={closeMenus} />

<header class="toolbar themed">
  <div class="left">
    <button
      class="icon-btn"
      title="Toggle sidebar"
      onclick={() => (app.sidebarOpen = !app.sidebarOpen)}
    >
      <PanelLeft size={18} />
    </button>
    <button class="icon-btn" title="Open folder" onclick={openFolder}>
      <FolderOpen size={18} />
    </button>
    <button
      class="icon-btn"
      title="Save (Ctrl+S)"
      disabled={!app.dirty}
      onclick={saveFile}
    >
      <Save size={18} />
    </button>

    <div class="export-wrap">
      <button
        class="icon-btn"
        title="Export"
        onclick={(e) => {
          e.stopPropagation();
          exportMenu = !exportMenu;
        }}
      >
        <Download size={18} />
      </button>
      {#if exportMenu}
        <div class="menu" role="menu">
          <button onclick={() => runExport(exportHtml)}>Export as HTML…</button>
          <button onclick={() => runExport(exportPdf)}>Print / Save as PDF…</button>
        </div>
      {/if}
    </div>
  </div>

  <div class="center">
    <div class="segmented">
      {#each views as v (v.id)}
        <button
          class="seg"
          class:active={app.view === v.id}
          title={v.label}
          onclick={() => app.setView(v.id)}
        >
          <v.icon size={16} />
          <span>{v.label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="right">
    <span class="words">{app.wordCount} words · {app.charCount} chars</span>
    <button
      class="icon-btn"
      class:on={app.searchOpen}
      title="Search across files (Ctrl+Shift+F)"
      onclick={() => (app.searchOpen = !app.searchOpen)}
    >
      <Search size={18} />
    </button>
    <div class="theme-wrap">
      <button
        class="icon-btn"
        title="Theme"
        onclick={(e) => {
          e.stopPropagation();
          themeMenu = !themeMenu;
        }}
      >
        <Palette size={18} />
      </button>
      {#if themeMenu}
        <div class="menu right" role="menu">
          {#each THEMES as t (t.id)}
            <button class="theme-item" onclick={() => pickTheme(t.id)}>
              <span class="swatch" style="background: {t.swatch}"></span>
              <span class="theme-label">{t.label}</span>
              {#if app.theme === t.id}<Check size={15} class="check" />{/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    height: 48px;
    padding: 0 0.75rem;
    background: var(--bg-sidebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .left {
    flex: 1;
    min-width: 0;
  }
  .right {
    flex: 1;
    justify-content: flex-end;
  }
  .center {
    flex-shrink: 0;
  }
  .export-wrap,
  .theme-wrap {
    position: relative;
  }
  .menu {
    position: absolute;
    top: 38px;
    left: 0;
    z-index: 50;
    min-width: 190px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.3rem;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.25);
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  .theme-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.6rem;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .theme-item:hover {
    background: var(--bg-hover);
  }
  .theme-label {
    flex: 1;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .theme-item :global(.check) {
    color: var(--accent-soft);
  }
  .menu button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.7rem;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .menu button:hover {
    background: var(--accent);
    color: var(--accent-text);
  }
  .icon-btn.on {
    background: var(--bg-hover);
    color: var(--accent-soft);
  }
  .words {
    font-size: 0.75rem;
    color: var(--text-faint);
    margin-right: 0.4rem;
    white-space: nowrap;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
    background: transparent;
    color: var(--text-muted);
  }
  .segmented {
    display: flex;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 2px;
    gap: 2px;
  }
  .seg {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.7rem;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .seg:hover {
    color: var(--text);
  }
  .seg.active {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
</style>
