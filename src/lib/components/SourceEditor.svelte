<script lang="ts">
  import { onMount } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { markdown } from "@codemirror/lang-markdown";

  let {
    content,
    onchange,
    placeholder = "",
    revision = 0,
  }: {
    content: string;
    onchange: (markdown: string) => void;
    placeholder?: string;
    revision?: number;
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView;
  let lastRevision = -1;

  // Aplica cambios externos sin recrear el editor.
  $effect(() => {
    if (!view) return;
    if (lastRevision === -1) {
      lastRevision = revision;
      return;
    }
    if (revision === lastRevision) return;
    lastRevision = revision;
    const current = view.state.doc.toString();
    if (current === content) return;
    const pos = Math.min(view.state.selection.main.head, content.length);
    view.dispatch({
      changes: { from: 0, to: current.length, insert: content },
      selection: { anchor: pos },
    });
  });

  const theme = EditorView.theme({
    "&": {
      height: "100%",
      fontSize: "calc(var(--editor-font-size, 18px) - 3px)",
      backgroundColor: "transparent",
      color: "var(--text)",
    },
    ".cm-content": {
      fontFamily: "var(--font-mono)",
      lineHeight: "var(--editor-line-height, 1.7)",
      padding: "40px 0 96px",
      maxWidth: "var(--editor-measure, 760px)",
      margin: "0 auto",
      caretColor: "var(--accent)",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--text)",
    },
    "&.cm-focused": {
      outline: "none",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
      backgroundColor: "var(--accent-soft)",
    },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--text-faint)",
      border: "none",
    },
    ".cm-activeLine": {
      backgroundColor: "transparent",
    },
  });

  onMount(() => {
    view = new EditorView({
      parent: container,
      doc: content,
      extensions: [
        basicSetup,
        markdown(),
        theme,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onchange(update.state.doc.toString());
          }
        }),
      ],
    });

    return () => view.destroy();
  });
</script>

<div class="source" bind:this={container}></div>

<style>
  .source {
    height: 100%;
    overflow: hidden;
    padding: 0 32px;
  }

  :global(.source .cm-editor) {
    height: 100%;
  }

  :global(.source .cm-scroller) {
    overflow: auto;
  }
</style>
