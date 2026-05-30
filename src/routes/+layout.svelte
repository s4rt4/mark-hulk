<script>
  import "../app.css";
  import { app } from "$lib/stores/app.svelte.js";

  let { children } = $props();

  // Keep <html data-theme> in sync with the store.
  $effect(() => {
    document.documentElement.setAttribute("data-theme", app.theme);
  });
</script>

{@render children()}
