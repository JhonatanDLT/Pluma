<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import Placeholder from "@tiptap/extension-placeholder";
  import Link from "@tiptap/extension-link";
  import Underline from "@tiptap/extension-underline";
  import { htmlToMarkdown, markdownToHtml } from "$lib/markdown";

  let {
    content,
    onchange,
    placeholder = "Escribe tu historia…",
  }: {
    content: string;
    onchange: (markdown: string) => void;
    placeholder?: string;
  } = $props();

  let editorEl: HTMLDivElement;
  let editor: Editor;

  let linkOpen = $state(false);
  let linkUrl = $state("");
  let linkInput = $state<HTMLInputElement | undefined>(undefined);

  let findOpen = $state(false);
  let findText = $state("");
  let replaceText = $state("");
  let findIndex = $state(-1);
  let findInput = $state<HTMLInputElement | undefined>(undefined);

  let active = $state({
    bold: false,
    italic: false,
    underline: false,
    strike: false,
    code: false,
    h1: false,
    h2: false,
    h3: false,
    bulletList: false,
    orderedList: false,
    blockquote: false,
    codeBlock: false,
    link: false,
  });

  function refreshActive() {
    active = {
      bold: editor.isActive("bold"),
      italic: editor.isActive("italic"),
      underline: editor.isActive("underline"),
      strike: editor.isActive("strike"),
      code: editor.isActive("code"),
      h1: editor.isActive("heading", { level: 1 }),
      h2: editor.isActive("heading", { level: 2 }),
      h3: editor.isActive("heading", { level: 3 }),
      bulletList: editor.isActive("bulletList"),
      orderedList: editor.isActive("orderedList"),
      blockquote: editor.isActive("blockquote"),
      codeBlock: editor.isActive("codeBlock"),
      link: editor.isActive("link"),
    };
  }

  onMount(() => {
    editor = new Editor({
      element: editorEl,
      extensions: [
        StarterKit,
        Underline,
        Link.configure({ openOnClick: false, autolink: true }),
        Placeholder.configure({ placeholder }),
      ],
      content: markdownToHtml(content),
      onUpdate: ({ editor }) => onchange(htmlToMarkdown(editor.getHTML())),
      onSelectionUpdate: refreshActive,
      onTransaction: refreshActive,
    });
    refreshActive();

    return () => editor.destroy();
  });

  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        findOpen = true;
        queueMicrotask(() => findInput?.focus());
      }
    }
    document.addEventListener("keydown", onKeydown);
    return () => document.removeEventListener("keydown", onKeydown);
  });

  function run(command: () => void) {
    command();
    refreshActive();
  }

  function toggleLink() {
    if (editor.isActive("link")) {
      run(() => editor.chain().focus().unsetLink().run());
      linkOpen = false;
      return;
    }
    linkOpen = true;
    linkUrl = "";
    queueMicrotask(() => linkInput?.focus());
  }

  function applyLink() {
    const url = linkUrl.trim();
    if (url) {
      run(() =>
        editor
          .chain()
          .focus()
          .extendMarkRange("link")
          .setLink({ href: url })
          .run(),
      );
    }
    linkOpen = false;
  }

  function findAll(): { from: number; to: number }[] {
    if (!findText) return [];
    const term = findText.toLowerCase();
    const results: { from: number; to: number }[] = [];
    editor.state.doc.descendants((node, pos) => {
      if (!node.isText) return;
      const text = node.text ?? "";
      const lower = text.toLowerCase();
      let idx = lower.indexOf(term);
      while (idx !== -1) {
        results.push({ from: pos + idx, to: pos + idx + findText.length });
        idx = lower.indexOf(term, idx + term.length);
      }
    });
    return results;
  }

  function selectMatch(m: { from: number; to: number }) {
    editor.chain().focus().setTextSelection({ from: m.from, to: m.to }).scrollIntoView().run();
  }

  function findNext() {
    const matches = findAll();
    if (!matches.length) return;
    findIndex = findIndex < 0 || findIndex >= matches.length - 1 ? 0 : findIndex + 1;
    selectMatch(matches[findIndex]);
  }

  function findPrev() {
    const matches = findAll();
    if (!matches.length) return;
    findIndex = findIndex <= 0 ? matches.length - 1 : findIndex - 1;
    selectMatch(matches[findIndex]);
  }

  function replace() {
    if (!findText) return;
    const { from, to } = editor.state.selection;
    const selected = editor.state.doc.textBetween(from, to);
    if (selected.toLowerCase() === findText.toLowerCase()) {
      editor.chain().focus().insertContentAt({ from, to }, replaceText).run();
    }
    findNext();
  }

  function replaceAll() {
    if (!findText) return;
    const matches = findAll();
    for (let i = matches.length - 1; i >= 0; i--) {
      const m = matches[i];
      editor.chain().insertContentAt({ from: m.from, to: m.to }, replaceText).run();
    }
  }
</script>

<div class="editor">
  <div class="toolbar">
    <button class="btn-icon" class:is-active={active.bold} title="Negrita" onclick={() => run(() => editor.chain().focus().toggleBold().run())}>
      <strong>B</strong>
    </button>
    <button class="btn-icon" class:is-active={active.italic} title="Cursiva" onclick={() => run(() => editor.chain().focus().toggleItalic().run())}>
      <em>I</em>
    </button>
    <button class="btn-icon" class:is-active={active.underline} title="Subrayado" onclick={() => run(() => editor.chain().focus().toggleUnderline().run())}>
      <u>U</u>
    </button>
    <button class="btn-icon" class:is-active={active.strike} title="Tachado" onclick={() => run(() => editor.chain().focus().toggleStrike().run())}>
      <s>S</s>
    </button>
    <button class="btn-icon" class:is-active={active.code} title="Código en línea" onclick={() => run(() => editor.chain().focus().toggleCode().run())}>
      {"</>"}
    </button>

    <span class="divider"></span>

    <button class="btn-icon" class:is-active={active.h1} title="Título 1" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 1 }).run())}>H1</button>
    <button class="btn-icon" class:is-active={active.h2} title="Título 2" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 2 }).run())}>H2</button>
    <button class="btn-icon" class:is-active={active.h3} title="Título 3" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 3 }).run())}>H3</button>

    <span class="divider"></span>

    <button class="btn-icon" class:is-active={active.bulletList} title="Lista con viñetas" onclick={() => run(() => editor.chain().focus().toggleBulletList().run())}>•</button>
    <button class="btn-icon" class:is-active={active.orderedList} title="Lista numerada" onclick={() => run(() => editor.chain().focus().toggleOrderedList().run())}>1.</button>
    <button class="btn-icon" class:is-active={active.blockquote} title="Cita" onclick={() => run(() => editor.chain().focus().toggleBlockquote().run())}>❝</button>
    <button class="btn-icon" class:is-active={active.codeBlock} title="Bloque de código" onclick={() => run(() => editor.chain().focus().toggleCodeBlock().run())}>{"{ }"}</button>

    <span class="divider"></span>

    <button class="btn-icon" title="Línea horizontal" onclick={() => run(() => editor.chain().focus().setHorizontalRule().run())}>—</button>
    <button class="btn-icon" class:is-active={active.link} title="Enlace" onclick={toggleLink}>🔗</button>

    {#if linkOpen}
      <div class="link-pop">
        <input
          class="input"
          bind:this={linkInput}
          bind:value={linkUrl}
          placeholder="https://…"
          onkeydown={(e) => {
            if (e.key === "Enter") applyLink();
            if (e.key === "Escape") (linkOpen = false);
          }}
        />
        <button class="btn btn-primary" onclick={applyLink}>Aplicar</button>
      </div>
    {/if}

    <span class="spacer"></span>

    <button
      class="btn-icon"
      class:is-active={findOpen}
      title="Buscar y reemplazar (Ctrl/Cmd F)"
      onclick={() => {
        findOpen = !findOpen;
        queueMicrotask(() => findInput?.focus());
      }}
    >🔍</button>
    <button class="btn-icon" title="Deshacer" onclick={() => run(() => editor.chain().focus().undo().run())}>↶</button>
    <button class="btn-icon" title="Rehacer" onclick={() => run(() => editor.chain().focus().redo().run())}>↷</button>
  </div>

  {#if findOpen}
    <div class="find-bar">
      <input
        class="find-input"
        bind:this={findInput}
        bind:value={findText}
        placeholder="Buscar…"
        onkeydown={(e) => {
          if (e.key === "Enter") findNext();
          if (e.key === "Escape") (findOpen = false);
        }}
      />
      <button class="btn-icon" title="Anterior" onclick={findPrev}>↑</button>
      <button class="btn-icon" title="Siguiente" onclick={findNext}>↓</button>
      <input class="find-input" bind:value={replaceText} placeholder="Reemplazar…" />
      <button class="btn" onclick={replace}>Reemplazar</button>
      <button class="btn" onclick={replaceAll}>Todo</button>
      <button class="btn-icon" title="Cerrar" onclick={() => (findOpen = false)}>×</button>
    </div>
  {/if}

  <div class="surface" bind:this={editorEl}></div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    flex-wrap: wrap;
  }

  .divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 6px;
  }

  .spacer {
    flex: 1;
  }

  .find-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .find-input {
    width: 160px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }

  .link-pop {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-left: 6px;
  }

  .link-pop .input {
    width: 180px;
  }

  .surface {
    flex: 1;
    overflow-y: auto;
    padding: 0 32px 96px;
  }

  :global(.surface .ProseMirror) {
    max-width: 720px;
    margin: 0 auto;
    padding: 40px 0;
    min-height: 100%;
    outline: none;
    font-family: var(--font-serif);
    font-size: 18px;
    line-height: 1.75;
    color: var(--text);
  }

  :global(.surface .ProseMirror p) {
    margin: 0 0 1em;
  }

  :global(.surface .ProseMirror h1),
  :global(.surface .ProseMirror h2),
  :global(.surface .ProseMirror h3) {
    font-family: var(--font-ui);
    font-weight: 600;
    color: var(--text);
    margin: 1.6em 0 0.6em;
    line-height: 1.25;
  }

  :global(.surface .ProseMirror h1) {
    font-size: 1.8em;
  }

  :global(.surface .ProseMirror h2) {
    font-size: 1.45em;
  }

  :global(.surface .ProseMirror h3) {
    font-size: 1.2em;
  }

  :global(.surface .ProseMirror blockquote) {
    margin: 1em 0;
    padding: 0.1em 1.2em;
    border-left: 3px solid var(--accent);
    color: var(--text-muted);
    font-style: italic;
  }

  :global(.surface .ProseMirror pre) {
    background: var(--bg-sidebar);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 14px 16px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.6;
  }

  :global(.surface .ProseMirror code) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--bg-sidebar);
    border-radius: 4px;
    padding: 2px 5px;
  }

  :global(.surface .ProseMirror pre code) {
    background: transparent;
    padding: 0;
  }

  :global(.surface .ProseMirror a) {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  :global(.surface .ProseMirror hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 2em auto;
    width: 40%;
  }

  :global(.surface .ProseMirror p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: var(--text-faint);
    float: left;
    height: 0;
    pointer-events: none;
  }
</style>
