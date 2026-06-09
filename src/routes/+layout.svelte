<script>
  import "../app.css";
  import { app } from "$lib/stores/app.svelte.js";

  let { children } = $props();

  // Keep <html data-theme> in sync with the store and remember the choice.
  $effect(() => {
    document.documentElement.setAttribute("data-theme", app.theme);
    try {
      localStorage.setItem("mh-theme", app.theme);
    } catch (_) {
      /* localStorage unavailable */
    }
  });
</script>

{@render children()}
