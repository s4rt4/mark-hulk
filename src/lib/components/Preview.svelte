<script>
  import "../markdown/markdown.css";
  import "katex/dist/katex.min.css";
  import { tick } from "svelte";
  import mermaid from "mermaid";
  import { render } from "../markdown/render.js";
  import { app } from "$lib/stores/app.svelte.js";

  let container;
  const html = $derived(render(app.content));

  const MERMAID_PALETTES = {
    sage: {
      primaryColor: "#123528",
      primaryTextColor: "#e8f0ea",
      primaryBorderColor: "#348a63",
      lineColor: "#8fa896",
      secondaryColor: "#0e2c20",
      tertiaryColor: "#0b231a",
    },
    emerald: {
      primaryColor: "#1e241e",
      primaryTextColor: "#ffffff",
      primaryBorderColor: "#2ecc71",
      lineColor: "#8a958a",
      secondaryColor: "#131613",
      tertiaryColor: "#0a0c0a",
    },
    light: {
      primaryColor: "#eef3f0",
      primaryTextColor: "#15241b",
      primaryBorderColor: "#2f8a5b",
      lineColor: "#56655c",
      secondaryColor: "#e6ede8",
      tertiaryColor: "#f4f8f5",
    },
  };

  function mermaidConfig(theme) {
    const palette = MERMAID_PALETTES[theme] || MERMAID_PALETTES.sage;
    return {
      startOnLoad: false,
      securityLevel: "loose",
      theme: "base",
      themeVariables: {
        background: "transparent",
        fontFamily: "Inter, system-ui, sans-serif",
        ...palette,
      },
    };
  }

  async function runMermaid() {
    if (!container) return;
    const nodes = [...container.querySelectorAll(".mermaid")];
    if (!nodes.length) return;
    // Restore each diagram's source so it can be re-rendered (e.g. theme change).
    for (const n of nodes) {
      if (n.dataset.src) n.textContent = n.dataset.src;
      else n.dataset.src = n.textContent;
      n.removeAttribute("data-processed");
    }
    mermaid.initialize(mermaidConfig(app.theme));
    try {
      await mermaid.run({ nodes });
    } catch (_) {
      /* invalid diagram — leave the source visible */
    }
  }

  // Re-run whenever the rendered HTML or the theme changes.
  $effect(() => {
    html;
    app.theme;
    tick().then(runMermaid);
  });
</script>

<div class="preview-scroll" bind:this={container}>
  <article class="md-body">
    {@html html}
  </article>
</div>

<style>
  .preview-scroll {
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }
</style>
