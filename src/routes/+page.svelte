<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import SourceEditor from "$lib/components/SourceEditor.svelte";
  import MetadataPanel from "$lib/components/MetadataPanel.svelte";
  import CharactersPanel from "$lib/components/CharactersPanel.svelte";
  import * as api from "$lib/api";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import type {
    Chapter,
    Character,
    EditorMode,
    IaStatus,
    Project,
    Relationship,
    SaveState,
    Scene,
    SceneMeta,
  } from "$lib/types";

  let projects = $state<Project[]>([]);
  let chapters = $state<Chapter[]>([]);
  let scenes = $state<SceneMeta[]>([]);
  let characters = $state<Character[]>([]);
  let relationships = $state<Relationship[]>([]);

  let selectedProjectId = $state<string | null>(null);
  let selectedChapterId = $state<string | null>(null);
  let selectedSceneId = $state<string | null>(null);

  let sceneContent = $state("");
  let sceneTitle = $state<string | null>(null);
  let currentScene = $state<Scene | null>(null);

  let mode = $state<EditorMode>("wysiwyg");
  let saveState = $state<SaveState>("idle");
  let errorMsg = $state<string | null>(null);
  let importing = $state(false);
  let analyzing = $state(false);
  let enriching = $state(false);
  let iaStatus = $state<IaStatus | null>(null);
  let showMeta = $state(false);
  let showChars = $state(false);
  let exportOpen = $state(false);

  let saveTimer: number | undefined;

  const selectedProject = $derived(
    projects.find((p) => p.id === selectedProjectId) ?? null,
  );
  const selectedChapter = $derived(
    chapters.find((c) => c.id === selectedChapterId) ?? null,
  );
  const wordCount = $derived(countWords(sceneContent));

  const exportFormats = [
    { format: "md", label: "Markdown", ext: "md" },
    { format: "txt", label: "Texto plano", ext: "txt" },
    { format: "html", label: "HTML", ext: "html" },
    { format: "docx", label: "Word", ext: "docx" },
    { format: "pdf", label: "PDF", ext: "pdf" },
  ];

  onMount(async () => {
    await loadProjects();
    await loadIaStatus();
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
    currentScene = null;
    chapters = await api.listChapters(id);
    scenes = [];
    characters = await api.listCharacters(id);
    relationships = await api.listRelationships(id);
  }

  async function selectChapter(id: string) {
    if (id === selectedChapterId) return;
    await flushSave();
    selectedChapterId = id;
    selectedSceneId = null;
    sceneContent = "";
    sceneTitle = null;
    currentScene = null;
    scenes = await api.listScenes(id);
  }

  async function selectScene(id: string) {
    if (id === selectedSceneId) return;
    await flushSave();
    const scene = await api.getScene(id);
    sceneContent = scene.content_markdown;
    sceneTitle = scene.title;
    currentScene = scene;
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
        currentScene = null;
        chapters = [];
        scenes = [];
        characters = [];
        relationships = [];
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
        currentScene = null;
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
        currentScene = null;
      }
      if (selectedChapterId) scenes = await api.listScenes(selectedChapterId);
    } catch (e) {
      reportError(e);
    }
  }

  async function importManuscript() {
    if (!selectedProjectId) return;
    await flushSave();
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Importar documento",
        filters: [
          {
            name: "Todos los formatos",
            extensions: [
              "md", "markdown", "txt", "pdf", "docx",
              "png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp",
            ],
          },
          { name: "Markdown / Texto", extensions: ["md", "markdown", "txt"] },
          { name: "PDF", extensions: ["pdf"] },
          { name: "Word", extensions: ["docx"] },
          {
            name: "Imágenes (OCR)",
            extensions: ["png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp"],
          },
        ],
      });
      if (typeof selected !== "string") return;

      importing = true;
      await api.importFile(selectedProjectId, selected);

      selectedChapterId = null;
      selectedSceneId = null;
      sceneContent = "";
      sceneTitle = null;
      currentScene = null;
      scenes = [];

      chapters = await api.listChapters(selectedProjectId);
      if (chapters.length > 0) {
        await selectChapter(chapters[0].id);
      }
    } catch (e) {
      reportError(e);
    } finally {
      importing = false;
    }
  }

  // -------------------------------------------------------------------------
  // Metadatos
  // -------------------------------------------------------------------------

  async function saveChapterMeta(status: string, summary: string | null) {
    if (!selectedChapterId) return;
    try {
      await api.updateChapterMeta(selectedChapterId, status, summary);
      if (selectedProjectId) chapters = await api.listChapters(selectedProjectId);
    } catch (e) {
      reportError(e);
    }
  }

  async function saveSceneMeta(meta: {
    status: string;
    pov: string | null;
    location: string | null;
    timeLabel: string | null;
    summary: string | null;
  }) {
    if (!selectedSceneId) return;
    try {
      const scene = await api.updateSceneMeta(
        selectedSceneId,
        meta.status,
        meta.pov,
        meta.location,
        meta.timeLabel,
        meta.summary,
      );
      currentScene = scene;
      if (selectedChapterId) scenes = await api.listScenes(selectedChapterId);
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // Reordenamiento
  // -------------------------------------------------------------------------

  async function moveChapter(id: string, index: number) {
    if (!selectedProjectId) return;
    try {
      chapters = await api.moveChapter(selectedProjectId, id, index);
    } catch (e) {
      reportError(e);
    }
  }

  async function moveScene(id: string, index: number) {
    if (!selectedChapterId) return;
    try {
      scenes = await api.moveScene(selectedChapterId, id, index);
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // Exportación
  // -------------------------------------------------------------------------

  async function exportDocument(format: string, ext: string) {
    if (!selectedProjectId) return;
    exportOpen = false;
    try {
      const defaultPath = `${selectedProject?.title ?? "manuscrito"}.${ext}`;
      const path = await save({
        title: "Exportar manuscrito",
        defaultPath,
        filters: [{ name: format.toUpperCase(), extensions: [ext] }],
      });
      if (!path) return;
      await api.exportProject(selectedProjectId, format, path);
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // IA (personajes y relaciones)
  // -------------------------------------------------------------------------

  async function analyze() {
    if (!selectedProjectId) return;
    analyzing = true;
    try {
      const report = await api.analyzeProject(selectedProjectId);
      characters = report.characters;
      relationships = report.relationships;
    } catch (e) {
      reportError(e);
    } finally {
      analyzing = false;
    }
  }

  async function deleteCharacter(id: string) {
    try {
      await api.deleteCharacter(id);
      if (selectedProjectId) {
        characters = await api.listCharacters(selectedProjectId);
        relationships = await api.listRelationships(selectedProjectId);
      }
    } catch (e) {
      reportError(e);
    }
  }

  async function enrich() {
    if (!selectedProjectId) return;
    enriching = true;
    try {
      characters = await api.enrichProject(selectedProjectId, null);
    } catch (e) {
      reportError(e);
    } finally {
      enriching = false;
    }
  }

  async function loadIaStatus() {
    try {
      iaStatus = await api.iaStatus();
    } catch (e) {
      console.error(e);
    }
  }

  async function updateEmbeddings(enabled: boolean) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, embeddings: enabled };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function installEmbeddings() {
    try {
      await api.iaInstall("embeddings");
      iaStatus = await api.iaStatus();
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // Texto a voz
  // -------------------------------------------------------------------------

  async function playTts() {
    if (!selectedSceneId) return;
    try {
      await api.ttsSpeak(sceneContent, null, 1.0);
    } catch (e) {
      reportError(e);
    }
  }

  async function stopTts() {
    try {
      await api.ttsStop();
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
    onmovechapter={moveChapter}
    onmovescene={moveScene}
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
          <button class="btn-icon" title="Reproducir" onclick={playTts}>▶</button>
          <button class="btn-icon" title="Detener" onclick={stopTts}>■</button>
        {/if}

        <button
          class="btn btn-toggle"
          class:active={showMeta}
          onclick={() => (showMeta = !showMeta)}
        >Metadatos</button>

        <button
          class="btn btn-toggle"
          class:active={showChars}
          onclick={() => (showChars = !showChars)}
        >Personajes</button>

        <div class="dropdown">
          <button class="btn" onclick={() => (exportOpen = !exportOpen)}>Exportar</button>
          {#if exportOpen}
            <div class="dropdown-menu">
              {#each exportFormats as f (f.format)}
                <button onclick={() => exportDocument(f.format, f.ext)}>
                  {f.label} (.{f.ext})
                </button>
              {/each}
            </div>
          {/if}
        </div>

        {#if selectedProjectId}
          <button class="btn" onclick={importManuscript} disabled={importing}>
            {importing ? "Importando…" : "Importar"}
          </button>
        {/if}

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

    {#if showMeta}
      <MetadataPanel
        chapter={selectedChapter}
        scene={currentScene}
        onsavechapter={saveChapterMeta}
        onsavescene={saveSceneMeta}
      />
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

  {#if showChars}
    <aside class="right-panel">
      <CharactersPanel
        {characters}
        {relationships}
        {analyzing}
        {enriching}
        {iaStatus}
        onanalyze={analyze}
        onenrich={enrich}
        ondeletecharacter={deleteCharacter}
        onupdateembeddings={updateEmbeddings}
        oninstallembeddings={installEmbeddings}
      />
    </aside>
  {/if}
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
    gap: 8px;
    flex-shrink: 0;
  }

  .wordcount {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    margin-left: 6px;
  }

  .save-status {
    font-size: 12px;
    color: var(--text-muted);
    min-width: 76px;
    text-align: right;
    margin-left: 6px;
  }

  .save-status.error {
    color: var(--danger);
  }

  .btn-toggle.active {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent);
  }

  .dropdown {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-width: 180px;
    padding: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
  }

  .dropdown-menu button {
    border: none;
    background: transparent;
    color: var(--text);
    text-align: left;
    padding: 8px 10px;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
  }

  .dropdown-menu button:hover {
    background: var(--bg-hover);
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

  .right-panel {
    width: 280px;
    min-width: 280px;
    height: 100%;
    background: var(--bg-sidebar);
    border-left: 1px solid var(--border);
    overflow-y: auto;
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
