<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import SourceEditor from "$lib/components/SourceEditor.svelte";
  import * as api from "$lib/api";
  import type {
    Chapter,
    EditorMode,
    Project,
    SaveState,
    SceneMeta,
  } from "$lib/types";

  let projects = $state<Project[]>([]);
  let chapters = $state<Chapter[]>([]);
  let scenes = $state<SceneMeta[]>([]);

  let selectedProjectId = $state<string | null>(null);
  let selectedChapterId = $state<string | null>(null);
  let selectedSceneId = $state<string | null>(null);

  let sceneContent = $state("");
  let sceneTitle = $state<string | null>(null);

  let mode = $state<EditorMode>("wysiwyg");
  let saveState = $state<SaveState>("idle");
  let errorMsg = $state<string | null>(null);

  let saveTimer: number | undefined;

  const selectedProject = $derived(
    projects.find((p) => p.id === selectedProjectId) ?? null,
  );
  const selectedChapter = $derived(
    chapters.find((c) => c.id === selectedChapterId) ?? null,
  );
  const wordCount = $derived(countWords(sceneContent));

  onMount(async () => {
    await loadProjects();
  });

  async function loadProjects() {
    projects = await api.listProjects();
  }

  function countWords(markdown: string): number {
    const text = markdown.replace(/[#*_>`~\-\[\]()!|]/g, " ");
    return text.split(/\s+/).filter(Boolean).length;
  }

  function reportError(e: unknown) {
    console.error(e);
    errorMsg = typeof e === "string" ? e : String(e);
  }

  // -------------------------------------------------------------------------
  // Selección
  // -------------------------------------------------------------------------

  async function selectProject(id: string) {
    if (id === selectedProjectId) return;
    await flushSave();
    selectedProjectId = id;
    selectedChapterId = null;
    selectedSceneId = null;
    sceneContent = "";
    sceneTitle = null;
    chapters = await api.listChapters(id);
    scenes = [];
  }

  async function selectChapter(id: string) {
    if (id === selectedChapterId) return;
    await flushSave();
    selectedChapterId = id;
    selectedSceneId = null;
    sceneContent = "";
    sceneTitle = null;
    scenes = await api.listScenes(id);
  }

  async function selectScene(id: string) {
    if (id === selectedSceneId) return;
    await flushSave();
    const scene = await api.getScene(id);
    sceneContent = scene.content_markdown;
    sceneTitle = scene.title;
    selectedSceneId = id;
  }

  // -------------------------------------------------------------------------
  // Creación y borrado
  // -------------------------------------------------------------------------

  async function createProject(title: string) {
    try {
      const project = await api.createProject(title);
      projects = await api.listProjects();
      await selectProject(project.id);
    } catch (e) {
      reportError(e);
    }
  }

  async function createChapter(title: string) {
    if (!selectedProjectId) return;
    try {
      const chapter = await api.createChapter(selectedProjectId, title);
      chapters = await api.listChapters(selectedProjectId);
      await selectChapter(chapter.id);
    } catch (e) {
      reportError(e);
    }
  }

  async function createScene(title: string | null) {
    if (!selectedChapterId) return;
    try {
      const scene = await api.createScene(selectedChapterId, title);
      scenes = await api.listScenes(selectedChapterId);
      await selectScene(scene.id);
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteProject(id: string) {
    try {
      await api.deleteProject(id);
      if (selectedProjectId === id) {
        await flushSave();
        selectedProjectId = null;
        selectedChapterId = null;
        selectedSceneId = null;
        sceneContent = "";
        sceneTitle = null;
        chapters = [];
        scenes = [];
      }
      projects = await api.listProjects();
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteChapter(id: string) {
    try {
      await api.deleteChapter(id);
      if (selectedChapterId === id) {
        selectedChapterId = null;
        selectedSceneId = null;
        sceneContent = "";
        sceneTitle = null;
        scenes = [];
      }
      if (selectedProjectId) chapters = await api.listChapters(selectedProjectId);
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteScene(id: string) {
    try {
      await api.deleteScene(id);
      if (selectedSceneId === id) {
        selectedSceneId = null;
        sceneContent = "";
        sceneTitle = null;
      }
      if (selectedChapterId) scenes = await api.listScenes(selectedChapterId);
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // Autoguardado
  // -------------------------------------------------------------------------

  function onContentChange(markdown: string) {
    sceneContent = markdown;
    scheduleSave();
  }

  function scheduleSave() {
    if (saveTimer) window.clearTimeout(saveTimer);
    const id = selectedSceneId;
    if (!id) return;
    saveState = "saving";
    saveTimer = window.setTimeout(() => {
      saveTimer = undefined;
      void persistScene(id);
    }, 600);
  }

  async function persistScene(id: string) {
    try {
      await api.saveScene(id, sceneContent);
      saveState = "saved";
    } catch (e) {
      console.error(e);
      saveState = "error";
    }
  }

  async function flushSave() {
    if (saveTimer) {
      window.clearTimeout(saveTimer);
      saveTimer = undefined;
    }
    const id = selectedSceneId;
    if (id && saveState === "saving") {
      await persistScene(id);
    }
  }
</script>

<div class="app-shell">
  <Sidebar
    {projects}
    {chapters}
    {scenes}
    {selectedProjectId}
    {selectedChapterId}
    {selectedSceneId}
    onselectproject={selectProject}
    onselectchapter={selectChapter}
    onselectscene={selectScene}
    oncreateproject={createProject}
    oncreatechapter={createChapter}
    oncreatescene={createScene}
    ondeleteproject={deleteProject}
    ondeletechapter={deleteChapter}
    ondeletescene={deleteScene}
  />

  <main class="main">
    <header class="topbar">
      <div class="crumbs">
        {#if selectedProject}
          <span class="crumb">{selectedProject.title}</span>
          {#if selectedChapter}
            <span class="sep">›</span>
            <span class="crumb">{selectedChapter.title}</span>
          {/if}
          {#if selectedSceneId}
            <span class="sep">›</span>
            <span class="crumb muted">{sceneTitle ?? "Escena"}</span>
          {/if}
        {:else}
          <span class="crumb muted">Sin proyecto</span>
        {/if}
      </div>

      <div class="top-actions">
        {#if selectedSceneId}
          <span class="wordcount">{wordCount.toLocaleString()} palabras</span>
        {/if}

        <span class="save-status" class:error={saveState === "error"}>
          {#if saveState === "saving"}Guardando…{/if}
          {#if saveState === "saved"}Guardado{/if}
          {#if saveState === "error"}Error al guardar{/if}
        </span>

        <div class="segmented">
          <button
            class:active={mode === "wysiwyg"}
            onclick={() => (mode = "wysiwyg")}
          >Editor</button>
          <button
            class:active={mode === "source"}
            onclick={() => (mode = "source")}
          >Markdown</button>
        </div>
      </div>
    </header>

    {#if errorMsg}
      <div class="banner">
        <span>{errorMsg}</span>
        <button class="btn-icon" onclick={() => (errorMsg = null)}>×</button>
      </div>
    {/if}

    <div class="editor-area">
      {#if selectedSceneId}
        {#key selectedSceneId}
          {#if mode === "wysiwyg"}
            <Editor content={sceneContent} onchange={onContentChange} />
          {:else}
            <SourceEditor content={sceneContent} onchange={onContentChange} />
          {/if}
        {/key}
      {:else}
        <div class="empty">
          <div class="empty-mark">✒</div>
          <h2>Comienza tu historia</h2>
          <p>
            Crea un <strong>proyecto</strong>, añade un <strong>capítulo</strong> y
            luego una <strong>escena</strong> para empezar a escribir.
          </p>
        </div>
      {/if}
    </div>
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .crumbs {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    font-weight: 500;
  }

  .crumb {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .crumb.muted,
  .sep {
    color: var(--text-muted);
  }

  .sep {
    font-weight: 400;
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-shrink: 0;
  }

  .wordcount {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .save-status {
    font-size: 12px;
    color: var(--text-muted);
    min-width: 76px;
    text-align: right;
  }

  .save-status.error {
    color: var(--danger);
  }

  .segmented {
    display: flex;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: var(--bg-sidebar);
    border: 1px solid var(--border);
  }

  .segmented button {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
  }

  .segmented button.active {
    background: var(--bg-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    background: var(--accent-soft);
    color: var(--accent);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }

  .editor-area {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 8px;
    color: var(--text-muted);
    padding: 24px;
  }

  .empty-mark {
    font-size: 40px;
    color: var(--text-faint);
    margin-bottom: 8px;
  }

  .empty h2 {
    font-family: var(--font-serif);
    font-weight: 500;
    color: var(--text);
  }

  .empty p {
    max-width: 360px;
    line-height: 1.6;
  }
</style>
