<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import Placeholder from "@tiptap/extension-placeholder";
  import Link from "@tiptap/extension-link";
  import Underline from "@tiptap/extension-underline";
  import Image from "@tiptap/extension-image";
  import Table from "@tiptap/extension-table";
  import TableRow from "@tiptap/extension-table-row";
  import TableHeader from "@tiptap/extension-table-header";
  import TableCell from "@tiptap/extension-table-cell";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { htmlToMarkdown, markdownToHtml } from "$lib/markdown";

  const ImageWithPath = Image.extend({
    addAttributes() {
      return {
        ...this.parent?.(),
        "data-path": {
          default: null,
          parseHTML: (element) => element.getAttribute("data-path"),
        },
        width: {
          default: null,
          parseHTML: (element) => element.getAttribute("width"),
        },
        height: {
          default: null,
          parseHTML: (element) => element.getAttribute("height"),
        },
      };
    },
  });

  let {
    content,
    onchange,
    placeholder = "Escribe tu historia…",
    focus = false,
    revision = 0,
  }: {
    content: string;
    onchange: (markdown: string) => void;
    placeholder?: string;
    focus?: boolean;
    revision?: number;
  } = $props();

  let editorEl: HTMLDivElement;
  let editor: Editor;
  let lastRevision = -1;

  // Actualiza el contenido desde fuera (corrección, ortografía, snapshot) sin
  // remontar el editor: conserva el scroll y recoloca el cursor.
  $effect(() => {
    if (!editor) return;
    if (lastRevision === -1) {
      lastRevision = revision;
      return;
    }
    if (revision === lastRevision) return;
    lastRevision = revision;
    const { from, to } = editor.state.selection;
    editor.commands.setContent(markdownToHtml(content), false);
    const size = editor.state.doc.content.size;
    editor.commands.setTextSelection({
      from: Math.min(from, size),
      to: Math.min(to, size),
    });
  });

  let linkOpen = $state(false);
  let linkUrl = $state("");
  let linkInput = $state<HTMLInputElement | undefined>(undefined);

  let findOpen = $state(false);
  let findText = $state("");
  let replaceText = $state("");
  let findIndex = $state(-1);
  let findInput = $state<HTMLInputElement | undefined>(undefined);
  let imageWidth = $state("");
  let tab = $state<"inicio" | "insertar">("inicio");

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
    image: false,
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
      image: editor.isActive("image"),
    };
    const attrs = editor.getAttributes("image");
    imageWidth = attrs.width ? String(attrs.width) : "";
  }

  onMount(() => {
    editor = new Editor({
      element: editorEl,
      extensions: [
        StarterKit,
        Underline,
        Link.configure({ openOnClick: false, autolink: true }),
        ImageWithPath.configure({ inline: false, allowBase64: false }),
        Table.configure({ resizable: true }),
        TableRow,
        TableHeader,
        TableCell,
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
      if (!mod || e.key.toLowerCase() !== "f") return;
      // Solo intercepta «buscar» cuando el foco está dentro del editor; así no
      // secuestra Ctrl/Cmd+F en el asistente, metadatos u otros campos.
      const target = e.target as HTMLElement | null;
      if (!target || !target.closest(".editor")) return;
      e.preventDefault();
      findOpen = true;
      queueMicrotask(() => findInput?.focus());
    }
    document.addEventListener("keydown", onKeydown);
    return () => document.removeEventListener("keydown", onKeydown);
  });

  function run(command: () => void) {
    command();
    refreshActive();
  }

  function insertTable() {
    run(() =>
      editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
    );
  }

  function applyImageWidth() {
    const w = Number(imageWidth);
    if (!Number.isFinite(w) || w <= 0) return;
    run(() => editor.chain().focus().updateAttributes("image", { width: w }).run());
  }

  async function insertImage() {
    try {
      const path = await invoke<string | null>("persist_image_dialog");
      if (!path) return;
      editor
        .chain()
        .focus()
        .insertContent({
          type: "image",
          attrs: { src: convertFileSrc(path), alt: "", "data-path": path },
        })
        .run();
    } catch (e) {
      console.error(e);
    }
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
  {#if !focus}
  <div class="ribbon">
  <div class="ribbon-tabs">
    <button class="ribbon-tab" class:active={tab === "inicio"} onclick={() => (tab = "inicio")}>Inicio</button>
    <button class="ribbon-tab" class:active={tab === "insertar"} onclick={() => (tab = "insertar")}>Insertar</button>
  </div>

  <div class="ribbon-panel">
    {#if tab === "inicio"}
      <div class="ribbon-group">
        <span class="group-label">Fuente</span>
        <div class="group-buttons">
          <button class="ribbon-btn" class:is-active={active.bold} title="Negrita (Cmd/Ctrl+B)" onclick={() => run(() => editor.chain().focus().toggleBold().run())}>
            <span class="ribbon-icon"><strong>B</strong></span><span class="ribbon-label">Negrita</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.italic} title="Cursiva (Cmd/Ctrl+I)" onclick={() => run(() => editor.chain().focus().toggleItalic().run())}>
            <span class="ribbon-icon"><em>I</em></span><span class="ribbon-label">Cursiva</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.underline} title="Subrayado" onclick={() => run(() => editor.chain().focus().toggleUnderline().run())}>
            <span class="ribbon-icon"><u>U</u></span><span class="ribbon-label">Subrayado</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.strike} title="Tachado" onclick={() => run(() => editor.chain().focus().toggleStrike().run())}>
            <span class="ribbon-icon"><s>S</s></span><span class="ribbon-label">Tachado</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.code} title="Código en línea" onclick={() => run(() => editor.chain().focus().toggleCode().run())}>
            <span class="ribbon-icon">{"</>"}</span><span class="ribbon-label">Código</span>
          </button>
        </div>
      </div>

      <div class="ribbon-group">
        <span class="group-label">Párrafo</span>
        <div class="group-buttons">
          <button class="ribbon-btn" class:is-active={active.h1} title="Título 1" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 1 }).run())}>
            <span class="ribbon-icon">H1</span><span class="ribbon-label">Título 1</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.h2} title="Título 2" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 2 }).run())}>
            <span class="ribbon-icon">H2</span><span class="ribbon-label">Título 2</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.h3} title="Título 3" onclick={() => run(() => editor.chain().focus().toggleHeading({ level: 3 }).run())}>
            <span class="ribbon-icon">H3</span><span class="ribbon-label">Título 3</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.bulletList} title="Lista con viñetas" onclick={() => run(() => editor.chain().focus().toggleBulletList().run())}>
            <span class="ribbon-icon">•</span><span class="ribbon-label">Viñetas</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.orderedList} title="Lista numerada" onclick={() => run(() => editor.chain().focus().toggleOrderedList().run())}>
            <span class="ribbon-icon">1.</span><span class="ribbon-label">Numerada</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.blockquote} title="Cita" onclick={() => run(() => editor.chain().focus().toggleBlockquote().run())}>
            <span class="ribbon-icon">❝</span><span class="ribbon-label">Cita</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.codeBlock} title="Bloque de código" onclick={() => run(() => editor.chain().focus().toggleCodeBlock().run())}>
            <span class="ribbon-icon">{"{ }"}</span><span class="ribbon-label">Código bloque</span>
          </button>
        </div>
      </div>

      <div class="ribbon-group">
        <span class="group-label">Edición</span>
        <div class="group-buttons">
          <button class="ribbon-btn" class:is-active={findOpen} title="Buscar y reemplazar (Cmd/Ctrl+F)" onclick={() => { findOpen = !findOpen; queueMicrotask(() => findInput?.focus()); }}>
            <span class="ribbon-icon">🔍</span><span class="ribbon-label">Buscar</span>
          </button>
          <button class="ribbon-btn" title="Deshacer (Cmd/Ctrl+Z)" onclick={() => run(() => editor.chain().focus().undo().run())}>
            <span class="ribbon-icon">↶</span><span class="ribbon-label">Deshacer</span>
          </button>
          <button class="ribbon-btn" title="Rehacer" onclick={() => run(() => editor.chain().focus().redo().run())}>
            <span class="ribbon-icon">↷</span><span class="ribbon-label">Rehacer</span>
          </button>
        </div>
      </div>
    {:else}
      <div class="ribbon-group">
        <span class="group-label">Contenido</span>
        <div class="group-buttons">
          <button class="ribbon-btn" title="Insertar tabla" onclick={insertTable}>
            <span class="ribbon-icon">▦</span><span class="ribbon-label">Tabla</span>
          </button>
          <button class="ribbon-btn" title="Insertar imagen" onclick={insertImage}>
            <span class="ribbon-icon">🖼</span><span class="ribbon-label">Imagen</span>
          </button>
          <button class="ribbon-btn" class:is-active={active.link} title="Enlace" onclick={toggleLink}>
            <span class="ribbon-icon">🔗</span><span class="ribbon-label">Enlace</span>
          </button>
          <button class="ribbon-btn" title="Línea horizontal" onclick={() => run(() => editor.chain().focus().setHorizontalRule().run())}>
            <span class="ribbon-icon">—</span><span class="ribbon-label">Línea</span>
          </button>
        </div>
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
      </div>

      {#if active.image}
        <div class="ribbon-group">
          <span class="group-label">Imagen</span>
          <div class="group-buttons">
            <label class="ribbon-field">
              <span class="group-label">Ancho (px)</span>
              <input
                class="find-input"
                type="number"
                min="10"
                bind:value={imageWidth}
                placeholder="Auto"
                onchange={applyImageWidth}
              />
            </label>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
  {/if}

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

  .ribbon {
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .ribbon-tabs {
    display: flex;
    gap: 2px;
    padding: 4px 10px 0;
  }

  .ribbon-tab {
    font: inherit;
    font-size: 13px;
    padding: 6px 14px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  }

  .ribbon-tab.active {
    color: var(--text);
    font-weight: 600;
    background: var(--bg);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .ribbon-panel {
    display: flex;
    align-items: flex-start;
    gap: 2px;
    padding: 6px 10px 8px;
    flex-wrap: wrap;
  }

  .ribbon-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 2px 10px;
    border-right: 1px solid var(--border);
  }

  .ribbon-group:last-child {
    border-right: none;
  }

  .group-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-faint);
  }

  .group-buttons {
    display: flex;
    gap: 2px;
    align-items: center;
  }

  .ribbon-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 46px;
    padding: 4px 6px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font: inherit;
  }

  .ribbon-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border);
  }

  .ribbon-btn.is-active {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .ribbon-icon {
    font-size: 15px;
    line-height: 1;
  }

  .ribbon-label {
    font-size: 10px;
    color: var(--text-muted);
  }

  .ribbon-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
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
    margin-top: 6px;
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
    max-width: var(--editor-measure, 720px);
    margin: 0 auto;
    padding: 40px 0;
    min-height: 100%;
    font-family: var(--font-serif);
    font-size: var(--editor-font-size, 18px);
    line-height: var(--editor-line-height, 1.75);
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

  :global(.surface .ProseMirror table) {
    border-collapse: collapse;
    margin: 1em 0;
    width: 100%;
    table-layout: fixed;
    overflow: hidden;
  }

  :global(.surface .ProseMirror th),
  :global(.surface .ProseMirror td) {
    border: 1px solid var(--border);
    padding: 0.45em 0.7em;
    text-align: left;
    vertical-align: top;
    position: relative;
  }

  :global(.surface .ProseMirror th) {
    background: var(--bg-sidebar);
    font-weight: 600;
  }

  :global(.surface .ProseMirror .selectedCell) {
    background: var(--accent-soft);
  }

  :global(.surface .ProseMirror img) {
    max-width: 100%;
    height: auto;
    border-radius: 6px;
    margin: 0.5em 0;
  }

  :global(.surface .ProseMirror img.ProseMirror-selectednode) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  :global(.surface .ProseMirror .column-resize-handle) {
    position: absolute;
    right: -2px;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--accent);
    pointer-events: none;
  }

  :global(.surface .ProseMirror p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: var(--text-faint);
    float: left;
    height: 0;
    pointer-events: none;
  }
</style>
