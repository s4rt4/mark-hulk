<script>
  import { X, Plus } from "lucide-svelte";
  import { app } from "$lib/stores/app.svelte.js";

  function close(e, id) {
    e.stopPropagation();
    app.closeTab(id);
  }
</script>

<div class="tabbar themed">
  <div class="tabs">
    {#each app.tabs as tab (tab.id)}
      <button
        class="tab"
        class:active={tab.id === app.activeId}
        title={tab.path || tab.name}
        onclick={() => app.activate(tab.id)}
      >
        <span class="label">{tab.name}</span>
        {#if tab.dirty}
          <span class="dirty" aria-label="unsaved">●</span>
        {/if}
        <span
          class="close"
          role="button"
          tabindex="-1"
          aria-label="Close tab"
          onclick={(e) => close(e, tab.id)}
          onkeydown={() => {}}
        >
          <X size={13} />
        </span>
      </button>
    {/each}
  </div>
  <button class="new" title="New file (Ctrl+N)" onclick={() => app.newFile()}>
    <Plus size={16} />
  </button>
</div>

<style>
  .tabbar {
    display: flex;
    align-items: stretch;
    height: 38px;
    background: var(--bg-sidebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    height: 0;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.6rem 0 0.9rem;
    max-width: 200px;
    border: none;
    border-right: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font-size: 0.82rem;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s, color 0.12s;
  }
  .tab:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .tab.active {
    background: var(--bg);
    color: var(--text);
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dirty {
    color: var(--accent);
    font-size: 0.6rem;
  }
  .close {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    color: var(--text-faint);
    flex-shrink: 0;
  }
  .close:hover {
    background: color-mix(in srgb, var(--text-muted) 25%, transparent);
    color: var(--text);
  }
  .new {
    display: grid;
    place-items: center;
    width: 38px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .new:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
</style>
