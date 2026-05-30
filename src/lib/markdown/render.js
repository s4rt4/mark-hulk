import MarkdownIt from "markdown-it";
import anchor from "markdown-it-anchor";
import taskLists from "markdown-it-task-lists";
import footnote from "markdown-it-footnote";
import katex from "@vscode/markdown-it-katex";
import hljs from "highlight.js/lib/common";

const md = new MarkdownIt({
  html: false, // safe: don't pass raw HTML through
  linkify: true,
  typographer: true,
  breaks: false,
  highlight(code, lang) {
    // Mermaid fences become a placeholder the Preview turns into an SVG.
    if (lang === "mermaid") {
      return `<div class="mermaid">${md.utils.escapeHtml(code)}</div>`;
    }
    if (lang && hljs.getLanguage(lang)) {
      try {
        const out = hljs.highlight(code, { language: lang, ignoreIllegals: true }).value;
        return `<pre class="hljs"><code class="language-${lang}">${out}</code></pre>`;
      } catch (_) {
        /* fall through */
      }
    }
    const escaped = md.utils.escapeHtml(code);
    return `<pre class="hljs"><code>${escaped}</code></pre>`;
  },
});

md.use(anchor, {
  permalink: anchor.permalink.linkInsideHeader({
    symbol: "#",
    placement: "after",
    ariaHidden: true,
  }),
});
md.use(taskLists, { enabled: true, label: true });
md.use(footnote);
md.use(katex.default ?? katex);

// Render YAML frontmatter (--- ... ---) as a styled block instead of a broken <hr>.
function extractFrontmatter(src) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(src);
  if (!match) return { frontmatter: null, body: src };
  return { frontmatter: match[1], body: src.slice(match[0].length) };
}

export function render(source) {
  const { frontmatter, body } = extractFrontmatter(source || "");
  let html = "";
  if (frontmatter) {
    html += `<div class="md-frontmatter"><div class="md-frontmatter-label">frontmatter</div><pre>${md.utils.escapeHtml(
      frontmatter
    )}</pre></div>`;
  }
  html += md.render(body);
  return html;
}
