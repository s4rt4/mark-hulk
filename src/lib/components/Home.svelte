<script>
  import {
    FileText,
    FilePlus,
    FolderOpen,
    LayoutGrid,
    List,
    Trash2,
    X,
    Clock,
  } from "lucide-svelte";
  import { app } from "$lib/stores/app.svelte.js";
  import { isTauri, openFile, openFolder, openPath } from "$lib/services/fs.js";

  function dirName(path) {
    const dir = path.replace(/[\\/][^\\/]*$/, "");
    return dir === path ? "" : dir;
  }

  function timeAgo(ts) {
    if (!ts) return "";
    const s = (Date.now() - ts) / 1000;
    if (s < 60) return "just now";
    const m = Math.floor(s / 60);
    if (m < 60) return `${m} min ago`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h} h ago`;
    const d = Math.floor(h / 24);
    if (d < 7) return `${d} d ago`;
    return new Date(ts).toLocaleDateString();
  }

  function findNode(nodes, path) {
    for (const n of nodes || []) {
      if (!n.isDir && n.path === path) return n;
      if (n.isDir) {
        const hit = findNode(n.children, path);
        if (hit) return hit;
      }
    }
    return null;
  }

  async function openRecent(entry) {
    if (isTauri()) {
      try {
        await openPath(entry.path);
      } catch (_) {
        // File moved or deleted — drop the stale entry.
        app.removeRecent(entry.path);
      }
    } else {
      // Browser preview: reopen from the inline demo tree if possible.
      const node = findNode(app.tree, entry.path);
      if (node) app.openDoc({ path: node.path, content: node.content ?? "" });
      else app.removeRecent(entry.path);
    }
  }

  function remove(e, path) {
    e.stopPropagation();
    app.removeRecent(path);
  }
</script>

<div class="home themed">
  <div class="inner">
    <div class="hero">
      <img src="/mark-hulk.svg" alt="" class="logo" />
      <div>
        <h1>Mark-Hulk</h1>
        <p class="tagline">A native, lightweight Markdown viewer &amp; editor.</p>
      </div>
    </div>

    <div class="quick">
      <button class="action" onclick={() => app.newFile()}>
        <FilePlus size={16} />
        <span>New file</span>
      </button>
      <button class="action" onclick={openFile}>
        <FileText size={16} />
        <span>Open file…</span>
      </button>
      <button class="action" onclick={openFolder}>
        <FolderOpen size={16} />
        <span>Open folder…</span>
      </button>
    </div>

    <div class="section-head">
      <h2>Recent files</h2>
      <div class="controls">
        {#if app.recent.length > 0}
          <button
            class="mini"
            title="Clear recent files"
            onclick={() => app.clearRecent()}
          >
            <Trash2 size={14} />
          </button>
        {/if}
        <div class="segmented">
          <button
            class="seg"
            class:active={app.recentView === "grid"}
            title="Grid view"
            onclick={() => app.setRecentView("grid")}
          >
            <LayoutGrid size={15} />
          </button>
          <button
            class="seg"
            class:active={app.recentView === "list"}
            title="List view"
            onclick={() => app.setRecentView("list")}
          >
            <List size={15} />
          </button>
        </div>
      </div>
    </div>

    {#if app.recent.length === 0}
      <div class="empty">
        <Clock size={30} strokeWidth={1.4} />
        <p>No recent files yet</p>
        <span>Files you open will show up here.</span>
      </div>
    {:else if app.recentView === "grid"}
      <div class="grid">
        {#each app.recent as entry (entry.path)}
          <button class="card" title={entry.path} onclick={() => openRecent(entry)}>
            <span class="card-icon"><FileText size={22} strokeWidth={1.6} /></span>
            <span class="name">{entry.name}</span>
            <span class="path">{dirName(entry.path)}</span>
            <span class="time">{timeAgo(entry.at)}</span>
            <span
              class="remove"
              role="button"
              tabindex="-1"
              aria-label="Remove from recent files"
              onclick={(e) => remove(e, entry.path)}
              onkeydown={() => {}}
            >
              <X size={13} />
            </span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="list">
        {#each app.recent as entry (entry.path)}
          <button class="row" title={entry.path} onclick={() => openRecent(entry)}>
            <FileText size={16} strokeWidth={1.8} />
            <span class="name">{entry.name}</span>
            <span class="path">{dirName(entry.path)}</span>
            <span class="time">{timeAgo(entry.at)}</span>
            <span
              class="remove"
              role="button"
              tabindex="-1"
              aria-label="Remove from recent files"
              onclick={(e) => remove(e, entry.path)}
              onkeydown={() => {}}
            >
              <X size={13} />
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .home {
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }
  .inner {
    max-width: 860px;
    margin: 0 auto;
    padding: 3rem 2rem 4rem;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.75rem;
  }
  .logo {
    width: 52px;
    height: 52px;
    border-radius: 12px;
  }
  h1 {
    margin: 0;
    font-size: 1.6rem;
    letter-spacing: -0.02em;
  }
  .tagline {
    margin: 0.15rem 0 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 2.25rem;
  }
  .action {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.5rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--bg-elevated);
    color: var(--text);
    font-size: 0.83rem;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }
  .action:hover {
    border-color: var(--accent);
    color: var(--accent-soft);
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.8rem;
  }
  h2 {
    margin: 0;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  .mini:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .segmented {
    display: flex;
    background: var(--bg-sidebar);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
  }
  .seg {
    display: grid;
    place-items: center;
    width: 28px;
    height: 24px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .seg:hover {
    color: var(--text);
  }
  .seg.active {
    background: var(--accent);
    color: var(--accent-text);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 3rem 1rem;
    border: 1px dashed var(--border);
    border-radius: 12px;
    color: var(--text-faint);
    text-align: center;
  }
  .empty p {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-muted);
  }
  .empty span {
    font-size: 0.78rem;
  }

  /* ---- grid mode ---- */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.7rem;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
    padding: 0.9rem 1rem;
    border: 1px solid var(--border);
    border-radius: 11px;
    background: var(--bg-sidebar);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .card:hover {
    border-color: var(--accent);
    background: var(--bg-elevated);
  }
  .card-icon {
    color: var(--accent-soft);
    margin-bottom: 0.2rem;
  }
  .card .name {
    max-width: 100%;
    font-size: 0.86rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card .path {
    max-width: 100%;
    font-size: 0.72rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl; /* keep the tail (deepest folder) visible */
    text-align: left;
  }
  .card .time {
    font-size: 0.7rem;
    color: var(--text-faint);
  }

  /* ---- list mode ---- */
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 0.7rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    text-align: left;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .row:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .row .name {
    flex-shrink: 0;
    max-width: 45%;
    font-size: 0.86rem;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row .path {
    flex: 1;
    font-size: 0.74rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .row .time {
    flex-shrink: 0;
    font-size: 0.72rem;
    color: var(--text-faint);
  }

  .remove {
    position: absolute;
    top: 0.45rem;
    right: 0.45rem;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    color: var(--text-faint);
    opacity: 0;
    transition: opacity 0.12s;
  }
  .row .remove {
    top: 50%;
    transform: translateY(-50%);
  }
  .card:hover .remove,
  .row:hover .remove {
    opacity: 1;
  }
  .remove:hover {
    background: color-mix(in srgb, var(--text-muted) 25%, transparent);
    color: var(--text);
  }
</style>
