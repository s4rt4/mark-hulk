import mdCss from "$lib/markdown/markdown.css?raw";
import katexCss from "katex/dist/katex.min.css?raw";
import { app } from "$lib/stores/app.svelte.js";
import { render } from "$lib/markdown/render.js";
import { isTauri } from "./fs.js";

// Theme tokens, mirrored from app.css so exported files are self-contained.
const THEMES = {
  sage: {
    "--bg": "#0b231a",
    "--bg-sidebar": "#0e2c20",
    "--bg-elevated": "#123528",
    "--bg-hover": "#16402f",
    "--border": "#1e4634",
    "--accent": "#348a63",
    "--accent-soft": "#95d1af",
    "--beige": "#cfc1a1",
    "--text": "#e8f0ea",
    "--text-muted": "#8fa896",
    "--text-faint": "#5f7568",
    "--code-bg": "#0a1f17",
    "--code-border": "#1a3a2b",
    "--scroll-thumb": "#1f4a37",
  },
  emerald: {
    "--bg": "#0a0c0a",
    "--bg-sidebar": "#131613",
    "--bg-elevated": "#1e241e",
    "--bg-hover": "#232b23",
    "--border": "#263026",
    "--accent": "#2ecc71",
    "--accent-soft": "#33d375",
    "--beige": "#cfc1a1",
    "--text": "#ffffff",
    "--text-muted": "#8a958a",
    "--text-faint": "#5a635a",
    "--code-bg": "#101410",
    "--code-border": "#262d26",
    "--scroll-thumb": "#2a332a",
  },
};

function themeVars(theme) {
  const t = THEMES[theme] || THEMES.sage;
  const lines = Object.entries(t).map(([k, v]) => `  ${k}: ${v};`);
  lines.push('  --font-sans: "Inter", "Segoe UI", system-ui, sans-serif;');
  lines.push('  --font-mono: "JetBrains Mono", "Cascadia Code", monospace;');
  return `:root {\n${lines.join("\n")}\n}`;
}

/** Prefer the already-rendered preview DOM (has KaTeX/Mermaid output baked in). */
function currentBodyHtml() {
  const live = document.querySelector(".md-body");
  if (live && live.innerHTML.trim()) return live.innerHTML;
  return render(app.content);
}

function buildHtmlDocument() {
  const title = app.fileName.replace(/\.(md|markdown|mdx)$/i, "");
  return `<!doctype html>
<html lang="en" data-theme="${app.theme}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${title}</title>
<style>
${themeVars(app.theme)}
html, body { margin: 0; background: var(--bg); }
${katexCss}
${mdCss}
</style>
</head>
<body>
<article class="md-body">
${currentBodyHtml()}
</article>
</body>
</html>`;
}

/** Export the active document as a standalone .html file. */
export async function exportHtml() {
  const html = buildHtmlDocument();
  const name = app.fileName.replace(/\.(md|markdown|mdx)$/i, "") + ".html";

  if (isTauri()) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { invoke } = await import("@tauri-apps/api/core");
    const path = await save({
      defaultPath: name,
      filters: [{ name: "HTML", extensions: ["html"] }],
    });
    if (!path) return;
    await invoke("write_file", { path, content: html });
  } else {
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    URL.revokeObjectURL(url);
  }
}

/** Open the print dialog (the OS "Save as PDF" target produces a PDF). */
export async function exportPdf() {
  // Printing relies on the rendered preview; make sure it is visible first.
  app.setView("preview");
  // Give the preview a moment to render math/diagrams before printing.
  await new Promise((r) => setTimeout(r, 350));
  window.print();
}
