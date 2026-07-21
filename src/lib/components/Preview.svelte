<script>
  import "../markdown/markdown.css";
  import "katex/dist/katex.min.css";
  import { onMount, tick } from "svelte";
  import mermaid from "mermaid";
  import { render } from "../markdown/render.js";
  import { app } from "$lib/stores/app.svelte.js";
  import { registerPane, syncFrom } from "$lib/services/scrollSync.js";

  let container;
  const html = $derived(render(app.content));

  onMount(() => {
    // Delegated handler for the injected Copy buttons (real <button> elements,
    // so keyboard access is preserved).
    container.addEventListener("click", onClick);
    const unregister = registerPane("preview", container);
    return () => {
      container.removeEventListener("click", onClick);
      unregister();
    };
  });

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

  // Wrap each code block so a Copy button can sit in its top-right corner.
  // {@html} recreates the DOM on every content change, so this re-runs too.
  function addCopyButtons() {
    if (!container) return;
    for (const pre of container.querySelectorAll("pre.hljs")) {
      if (pre.parentElement?.classList.contains("md-code")) continue;
      const wrap = document.createElement("div");
      wrap.className = "md-code";
      pre.parentNode.insertBefore(wrap, pre);
      wrap.appendChild(pre);
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "md-copy";
      btn.title = "Copy code";
      btn.textContent = "Copy";
      wrap.appendChild(btn);
    }
  }

  async function copyText(text) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (_) {
      // WebView fallback when the async clipboard API is unavailable.
      try {
        const ta = document.createElement("textarea");
        ta.value = text;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        ta.style.userSelect = "text";
        document.body.appendChild(ta);
        ta.select();
        const ok = document.execCommand("copy");
        ta.remove();
        return ok;
      } catch (_) {
        return false;
      }
    }
  }

  async function onClick(e) {
    const btn = e.target.closest?.(".md-copy");
    if (!btn) return;
    const pre = btn.parentElement.querySelector("pre");
    const ok = await copyText(pre ? pre.innerText : "");
    btn.textContent = ok ? "Copied!" : "Failed";
    btn.classList.add("done");
    setTimeout(() => {
      btn.textContent = "Copy";
      btn.classList.remove("done");
    }, 1400);
  }

  // Re-run whenever the rendered HTML or the theme changes.
  $effect(() => {
    html;
    app.theme;
    tick().then(() => {
      addCopyButtons();
      runMermaid();
    });
  });
</script>

<div
  class="preview-scroll"
  bind:this={container}
  onscroll={() => {
    if (app.view === "split") syncFrom("preview");
  }}
>
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
