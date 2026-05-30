<script>
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import Search from "$lib/components/Search.svelte";
  import { app } from "$lib/stores/app.svelte.js";
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
        { name: "Welcome to Mark-Hulk.md", path: "/ws/welcome.md", isDir: false, content: app.content },
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
    <TabBar />
    <div class="content">
      {#if app.view === "preview"}
        <Preview />
      {:else if app.view === "edit"}
        <CodeEditor />
      {:else}
        <div class="split">
          <div class="pane"><CodeEditor /></div>
          <div class="divider"></div>
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
  .divider {
    width: 1px;
    background: var(--border);
    flex-shrink: 0;
  }
</style>
