<script>
  import { Search as SearchIcon, X } from "lucide-svelte";
  import { app } from "$lib/stores/app.svelte.js";
  import { searchWorkspace, openPath, isTauri } from "$lib/services/fs.js";

  let query = $state("");
  let results = $state([]);
  let busy = $state(false);
  let input;
  let timer;

  $effect(() => {
    if (input) input.focus();
  });

  function onInput() {
    clearTimeout(timer);
    const q = query;
    timer = setTimeout(async () => {
      busy = true;
      results = await searchWorkspace(q);
      busy = false;
    }, 180);
  }

  function findNode(nodes, path) {
    for (const n of nodes || []) {
      if (n.path === path) return n;
      if (n.isDir) {
        const f = findNode(n.children, path);
        if (f) return f;
      }
    }
    return null;
  }

  async function openHit(hit) {
    if (isTauri()) {
      await openPath(hit.path);
    } else {
      const node = findNode(app.tree, hit.path);
      app.openDoc({ path: hit.path, content: node?.content ?? "" });
    }
    app.searchOpen = false;
  }

  function close() {
    app.searchOpen = false;
  }

  function onKeydown(e) {
    if (e.key === "Escape") close();
  }

  // Highlight the matched substring in a snippet.
  function parts(text, q) {
    if (!q) return [{ t: text, hit: false }];
    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx === -1) return [{ t: text, hit: false }];
    return [
      { t: text.slice(0, idx), hit: false },
      { t: text.slice(idx, idx + q.length), hit: true },
      { t: text.slice(idx + q.length), hit: false },
    ];
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="search-overlay" role="dialog" aria-label="Search">
  <button class="backdrop" aria-label="Close search" onclick={close}></button>
  <div class="panel themed">
    <div class="field">
      <SearchIcon size={18} />
      <input
        bind:this={input}
        bind:value={query}
        oninput={onInput}
        placeholder="Search across the workspace…"
        spellcheck="false"
      />
      <button class="x" title="Close (Esc)" onclick={close}><X size={16} /></button>
    </div>

    <div class="results">
      {#if !app.rootPath && isTauri()}
        <p class="hint">Open a folder first to search across files.</p>
      {:else if query && !busy && results.length === 0}
        <p class="hint">No matches for “{query}”.</p>
      {:else if results.length}
        <div class="count">{results.length} match{results.length === 1 ? "" : "es"}</div>
        {#each results as hit (hit.path + ":" + hit.line)}
          <button class="hit" onclick={() => openHit(hit)}>
            <div class="hit-top">
              <span class="hit-name">{hit.name}</span>
              <span class="hit-line">:{hit.line}</span>
            </div>
            <div class="hit-text">
              {#each parts(hit.text, query) as p}<span class:mark={p.hit}>{p.t}</span>{/each}
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .search-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(640px, 92vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.9rem 1rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
  }
  .field input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 1rem;
    font-family: var(--font-sans);
  }
  .x {
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
  .x:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .results {
    overflow-y: auto;
    padding: 0.4rem;
  }
  .hint {
    padding: 1.2rem;
    text-align: center;
    color: var(--text-faint);
    font-size: 0.88rem;
  }
  .count {
    padding: 0.4rem 0.7rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
  }
  .hit {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.55rem 0.7rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
  }
  .hit:hover {
    background: var(--bg-hover);
  }
  .hit-top {
    display: flex;
    align-items: baseline;
    gap: 0.2rem;
    margin-bottom: 0.15rem;
  }
  .hit-name {
    color: var(--accent-soft);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .hit-line {
    color: var(--text-faint);
    font-size: 0.78rem;
  }
  .hit-text {
    color: var(--text-muted);
    font-size: 0.82rem;
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hit-text .mark {
    background: color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--text);
    border-radius: 3px;
  }
</style>
