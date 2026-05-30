<script>
  import { onMount } from "svelte";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "@codemirror/view";
  import { EditorState } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
  import { languages } from "@codemirror/language-data";
  import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { tags as t } from "@lezer/highlight";
  import { app } from "$lib/stores/app.svelte.js";

  let el;
  let view;

  // Theme reads CSS custom properties, so it follows Sage/Emerald automatically.
  const theme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "var(--bg)",
      color: "var(--text)",
      fontSize: "0.9rem",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-mono)",
      lineHeight: "1.7",
      padding: "1.5rem 0",
    },
    ".cm-content": { caretColor: "var(--accent-soft)", padding: "0 1rem" },
    "&.cm-focused": { outline: "none" },
    ".cm-gutters": {
      backgroundColor: "var(--bg)",
      color: "var(--text-faint)",
      border: "none",
      paddingLeft: "0.5rem",
    },
    ".cm-activeLine": { backgroundColor: "color-mix(in srgb, var(--bg-elevated) 45%, transparent)" },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--text-muted)" },
    ".cm-cursor": { borderLeftColor: "var(--accent-soft)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "color-mix(in srgb, var(--accent) 35%, transparent)",
    },
  });

  const highlightStyle = HighlightStyle.define([
    { tag: t.heading, color: "var(--text)", fontWeight: "700" },
    { tag: t.heading1, color: "var(--text)", fontWeight: "700", fontSize: "1.3em" },
    { tag: t.heading2, color: "var(--beige)", fontWeight: "700" },
    { tag: t.strong, color: "var(--accent-soft)", fontWeight: "700" },
    { tag: t.emphasis, fontStyle: "italic", color: "var(--text)" },
    { tag: t.link, color: "var(--accent-soft)", textDecoration: "underline" },
    { tag: t.url, color: "var(--text-muted)" },
    { tag: [t.monospace, t.contentSeparator], color: "var(--beige)" },
    { tag: t.list, color: "var(--accent)" },
    { tag: t.quote, color: "var(--text-muted)", fontStyle: "italic" },
    { tag: t.processingInstruction, color: "var(--accent)" }, // markup punctuation (#, -, *, >)
    { tag: t.comment, color: "var(--text-faint)" },
    { tag: t.keyword, color: "var(--accent-soft)" },
    { tag: t.string, color: "var(--beige)" },
    { tag: t.number, color: "#e0b97d" },
  ]);

  onMount(() => {
    view = new EditorView({
      parent: el,
      state: EditorState.create({
        doc: app.content,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          markdown({ base: markdownLanguage, codeLanguages: languages }),
          syntaxHighlighting(highlightStyle),
          theme,
          EditorView.lineWrapping,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) {
              app.content = u.state.doc.toString();
            }
          }),
        ],
      }),
    });

    return () => view?.destroy();
  });

  // Sync external content changes (file open, tree click) into the editor.
  $effect(() => {
    const incoming = app.content;
    if (view && incoming !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: incoming },
      });
    }
  });
</script>

<div class="cm-host" bind:this={el}></div>

<style>
  .cm-host {
    height: 100%;
    overflow: hidden;
    background: var(--bg);
  }
  .cm-host :global(.cm-editor) {
    height: 100%;
  }
</style>
