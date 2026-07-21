<script>
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import Search from "$lib/components/Search.svelte";
  import Home from "$lib/components/Home.svelte";
  import { app, SAMPLE } from "$lib/stores/app.svelte.js";
  import {
    isTauri,
    initWatcher,
    loadLaunchFile,
    saveFile,
  } from "$lib/services/fs.js";
  import { onMount } from "svelte";

  onMount(() => {
    initWatcher();
    loadLaunchFile();
  });

  // Split-view ratio (left pane fraction), draggable and remembered.
  let split = $state(0.5);
  let splitEl = $state(null);
  try {
    const saved = parseFloat(localStorage.getItem("mh-split"));
    if (saved >= 0.2 && saved <= 0.8) split = saved;
  } catch (_) {
    /* localStorage unavailable */
  }

  function startDrag(e) {
    e.preventDefault();
    const rect = splitEl.getBoundingClientRect();
    const move = (ev) => {
      const r = (ev.clientX - rect.left) / rect.width;
      split = Math.min(0.8, Math.max(0.2, r));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      try {
        localStorage.setItem("mh-split", String(split));
      } catch (_) {
        /* localStorage unavailable */
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function onKeydown(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) return;
    if (e.key === "s") {
      e.preventDefault();
      saveFile();
    } else if (e.key === "n") {
      e.preventDefault();
      app.newFile();
    } else if (e.shiftKey && (e.key === "F" || e.key === "f")) {
      e.preventDefault();
      app.searchOpen = !app.searchOpen;
    } else if (e.key === "b") {
      e.preventDefault();
      app.sidebarOpen = !app.sidebarOpen;
    } else if (e.key === "1") {
      e.preventDefault();
      app.setView("preview");
    } else if (e.key === "2") {
      e.preventDefault();
      app.setView("split");
    } else if (e.key === "3") {
      e.preventDefault();
      app.setView("edit");
    }
  }

  // Demo workspace (browser preview only) so the explorer looks alive
  // without the native Rust backend. In Tauri the user opens a real folder.
  const demoTree = [
    {
      name: "Workspace",
      path: "/ws",
      isDir: true,
      children: [
        { name: "README.md", path: "/ws/README.md", isDir: false, content: "# README\n\nThis is a sample file in the demo workspace.\n" },
        { name: "Welcome to Mark-Hulk.md", path: "/ws/welcome.md", isDir: false, content: SAMPLE },
        {
          name: "Notes",
          path: "/ws/notes",
          isDir: true,
          children: [
            { name: "Ideas.md", path: "/ws/notes/ideas.md", isDir: false, content: "# Ideas\n\n- [ ] Ship Mark-Hulk\n- [ ] Add export to PDF\n" },
            { name: "Meeting.md", path: "/ws/notes/meeting.md", isDir: false, content: "# Meeting notes\n\n> Keep it lightweight.\n" },
          ],
        },
      ],
    },
  ];

  if (!isTauri()) {
    app.tree = demoTree;
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  {#if app.sidebarOpen}
    <Sidebar />
  {/if}

  <div class="main">
    <Toolbar />
    {#if app.tabs.length > 0}
      <TabBar />
    {/if}
    <div class="content">
      {#if app.tabs.length === 0}
        <Home />
      {:else if app.view === "preview"}
        <Preview />
      {:else if app.view === "edit"}
        <CodeEditor />
      {:else}
        <div class="split" bind:this={splitEl}>
          <div class="pane left" style="width: {split * 100}%"><CodeEditor /></div>
          <div
            class="divider"
            role="separator"
            aria-orientation="vertical"
            title="Drag to resize"
            onpointerdown={startDrag}
          ></div>
          <div class="pane"><Preview /></div>
        </div>
      {/if}
    </div>
  </div>

  {#if app.searchOpen}
    <Search />
  {/if}
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .content {
    flex: 1;
    min-height: 0;
  }
  .split {
    display: flex;
    height: 100%;
  }
  .pane {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
  .pane.left {
    flex: none;
  }
  .divider {
    width: 6px;
    flex-shrink: 0;
    cursor: col-resize;
    background: linear-gradient(
      to right,
      transparent calc(50% - 0.5px),
      var(--border) calc(50% - 0.5px),
      var(--border) calc(50% + 0.5px),
      transparent calc(50% + 0.5px)
    );
    transition: background 0.15s;
  }
  .divider:hover,
  .divider:active {
    background: var(--accent);
  }
</style>
