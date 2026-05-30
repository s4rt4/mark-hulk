<script>
  import { FolderOpen, FilePlus } from "lucide-svelte";
  import { app } from "$lib/stores/app.svelte.js";
  import { openFolder } from "$lib/services/fs.js";
  import TreeNode from "./TreeNode.svelte";
</script>

<aside class="sidebar themed">
  <div class="brand">
    <img src="/mark-hulk.svg" alt="Mark-Hulk" class="logo" />
    <span class="title">Mark-Hulk</span>
  </div>

  <div class="section-head">
    <span>Explorer</span>
    <div class="actions">
      <button class="mini" title="New file" onclick={() => app.newFile()}>
        <FilePlus size={14} />
      </button>
      <button class="mini" title="Open folder" onclick={openFolder}>
        <FolderOpen size={14} />
      </button>
    </div>
  </div>

  <div class="tree">
    {#if app.tree.length === 0}
      <div class="empty">
        <FolderOpen size={28} strokeWidth={1.4} />
        <p>No folder opened</p>
        <button class="open-btn" onclick={openFolder}>Open folder…</button>
      </div>
    {:else}
      {#each app.tree as node (node.path)}
        <TreeNode {node} />
      {/each}
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: 250px;
    flex-shrink: 0;
    height: 100%;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.85rem 1rem;
  }
  .logo {
    width: 26px;
    height: 26px;
    border-radius: 7px;
  }
  .title {
    font-weight: 700;
    font-size: 0.95rem;
    letter-spacing: -0.01em;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 1rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
  }
  .actions {
    display: flex;
    gap: 0.15rem;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  .mini:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 0.25rem 0.5rem 1rem;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 2.5rem 1rem;
    color: var(--text-faint);
    text-align: center;
  }
  .empty p {
    margin: 0;
    font-size: 0.82rem;
  }
  .open-btn {
    margin-top: 0.3rem;
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-elevated);
    color: var(--text);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .open-btn:hover {
    border-color: var(--accent);
    color: var(--accent-soft);
  }
</style>
