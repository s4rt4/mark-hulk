<script>
  import { ChevronRight, FileText, Folder, FolderOpen } from "lucide-svelte";
  import { app } from "$lib/stores/app.svelte.js";
  import { isTauri, openPath } from "$lib/services/fs.js";
  import Self from "./TreeNode.svelte";

  let { node, depth = 0 } = $props();
  let open = $state(depth < 1);

  const isActive = $derived(app.currentPath === node.path);

  async function click() {
    if (node.isDir) {
      open = !open;
    } else if (isTauri()) {
      await openPath(node.path);
    } else {
      // Browser preview: sample nodes carry inline demo content.
      app.openDoc({ path: node.path, content: node.content ?? "" });
    }
  }
</script>

<button
  class="row"
  class:active={isActive}
  style="padding-left: {0.5 + depth * 0.85}rem"
  onclick={click}
>
  {#if node.isDir}
    <span class="chev" class:open>
      <ChevronRight size={14} />
    </span>
    {#if open}<FolderOpen size={15} />{:else}<Folder size={15} />{/if}
  {:else}
    <span class="chev placeholder"></span>
    <FileText size={15} />
  {/if}
  <span class="name">{node.name}</span>
</button>

{#if node.isDir && open && node.children}
  {#each node.children as child (child.path)}
    <Self node={child} depth={depth + 1} />
  {/each}
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    padding: 0.32rem 0.6rem;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.85rem;
    text-align: left;
    cursor: pointer;
    border-radius: 6px;
    white-space: nowrap;
    overflow: hidden;
    transition: background 0.12s, color 0.12s;
  }
  .row:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .row.active {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chev {
    display: grid;
    place-items: center;
    width: 14px;
    transition: transform 0.15s;
    flex-shrink: 0;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .chev.placeholder {
    width: 14px;
  }
</style>
