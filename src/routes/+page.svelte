<script lang="ts">
  import { onMount } from "svelte";
  import ProjectsView from "$lib/components/ProjectsView.svelte";
  import ChaptersPanel from "$lib/components/ChaptersPanel.svelte";
  import ScenesPanel from "$lib/components/ScenesPanel.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import SourceEditor from "$lib/components/SourceEditor.svelte";
  import OptionsPanel from "$lib/components/OptionsPanel.svelte";
  import WorldBuilding from "$lib/components/WorldBuilding.svelte";
  import SpellPanel from "$lib/components/SpellPanel.svelte";
  import QuickSwitcher from "$lib/components/QuickSwitcher.svelte";
  import * as api from "$lib/api";
  import { applyTheme, getTheme } from "$lib/theme";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import type {
    ChangeEntry,
    Chapter,
    Character,
    EditorMode,
    Entity,
    IaStatus,
    NavEntry,
    Project,
    Relationship,
    SaveState,
    Scene,
    SceneMeta,
    SnapshotMeta,
    SpellReport,
  } from "$lib/types";

  let projects = $state<Project[]>([]);
  let chapters = $state<Chapter[]>([]);
  let scenes = $state<SceneMeta[]>([]);
  let characters = $state<Character[]>([]);
  let relationships = $state<Relationship[]>([]);
  let entities = $state<Entity[]>([]);

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
  let analyzeProgress = $state<{ done: number; total: number; phase: string } | null>(null);
  let enriching = $state(false);
  let correcting = $state(false);
  let editorRevision = $state(0);
  let iaStatus = $state<IaStatus | null>(null);
  let snapshots = $state<SnapshotMeta[]>([]);
  let snapshotBusy = $state(false);
  let navigation = $state<NavEntry[]>([]);
  let switcherOpen = $state(false);
  let exportOpen = $state(false);
  let worldOpen = $state(false);
  let changeLog = $state<ChangeEntry[]>([]);
  let spellOpen = $state(false);
  let spellReport = $state<SpellReport | null>(null);
  let spellBusy = $state(false);

  // Paneles redimensionables / colapsables (persistidos entre sesiones).
  function loadNum(key: string, fallback: number): number {
    const v = localStorage.getItem(key);
    if (v === null) return fallback;
    const n = Number(v);
    return Number.isFinite(n) ? n : fallback;
  }
  function loadBool(key: string, fallback: boolean): boolean {
    const v = localStorage.getItem(key);
    if (v === null) return fallback;
    return v === "true";
  }

  let chaptersWidth = $state(loadNum("pluma.chaptersWidth", 240));
  let scenesWidth = $state(loadNum("pluma.scenesWidth", 220));
  let optionsWidth = $state(loadNum("pluma.optionsWidth", 300));
  let chaptersCollapsed = $state(loadBool("pluma.chaptersCollapsed", false));
  let scenesCollapsed = $state(loadBool("pluma.scenesCollapsed", false));
  let optionsOpen = $state(loadBool("pluma.optionsOpen", true));
  let viewMode = $state<"split" | "tree">(
    localStorage.getItem("pluma.viewMode") === "tree" ? "tree" : "split",
  );
  let projectsFocusSignal = $state(0);

  $effect(() => localStorage.setItem("pluma.chaptersWidth", String(chaptersWidth)));
  $effect(() => localStorage.setItem("pluma.scenesWidth", String(scenesWidth)));
  $effect(() => localStorage.setItem("pluma.optionsWidth", String(optionsWidth)));
  $effect(() => localStorage.setItem("pluma.chaptersCollapsed", String(chaptersCollapsed)));
  $effect(() => localStorage.setItem("pluma.scenesCollapsed", String(scenesCollapsed)));
  $effect(() => localStorage.setItem("pluma.optionsOpen", String(optionsOpen)));
  $effect(() => localStorage.setItem("pluma.viewMode", viewMode));

  let saveTimer: number | undefined;

  const selectedProject = $derived(
    projects.find((p) => p.id === selectedProjectId) ?? null,
  );
  const selectedChapter = $derived(
    chapters.find((c) => c.id === selectedChapterId) ?? null,
  );
  const wordCount = $derived(countWords(sceneContent));
  const ollamaAvailable = $derived(iaStatus?.ollama_available ?? false);

  // El segundo nivel (escenas) solo se considera "en uso" cuando al menos un
  // capítulo tiene más de una escena. Así, novelas de un único nivel (p. ej.
  // volúmenes/capítulos sin subdivisión) no muestran una jerarquía vacía.
  const projectUsesScenes = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const entry of navigation) {
      if (entry.scene_id !== null) {
        counts.set(entry.chapter_id, (counts.get(entry.chapter_id) ?? 0) + 1);
      }
    }
    for (const n of counts.values()) {
      if (n > 1) return true;
    }
    return false;
  });

  const exportFormats = [
    { format: "md", label: "Markdown", ext: "md" },
    { format: "txt", label: "Texto plano", ext: "txt" },
    { format: "html", label: "HTML", ext: "html" },
    { format: "docx", label: "Word", ext: "docx" },
    { format: "pdf", label: "PDF", ext: "pdf" },
    { format: "epub", label: "EPUB", ext: "epub" },
  ];

  onMount(async () => {
    await loadProjects();
    await loadIaStatus();
  });

  onMount(() => {
    applyTheme(getTheme());
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (getTheme() === "system") applyTheme("system");
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  });

  onMount(() => {
    const unlisten = listen("analyze-progress", (e) => {
      analyzeProgress = e.payload as { done: number; total: number; phase: string };
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  onMount(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (selectedProjectId) {
          switcherOpen = !switcherOpen;
        } else {
          projectsFocusSignal++;
        }
        return;
      }
      if (mod && e.altKey) {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          nextChapter();
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          prevChapter();
          return;
        }
      }
      if (mod && e.shiftKey && !e.altKey) {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          nextScene();
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          prevScene();
          return;
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
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
    entities = await api.listEntities(id);
    snapshots = await api.listSnapshots(id);
    navigation = await api.listNavigation(id);
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
    // Abre la primera escena automáticamente: en proyectos de un solo nivel esto
    // elimina el flujo «clic capítulo → clic escena», y en multi-escena ofrece un
    // punto de partida consistente con el conmutador rápido.
    if (scenes.length > 0) {
      await selectScene(scenes[0].id);
    }
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
  // Navegación (teclado y conmutador rápido)
  // -------------------------------------------------------------------------

  async function goToChapter(id: string) {
    await selectChapter(id);
  }

  async function goToSceneInChapter(chapterId: string, sceneId: string) {
    if (chapterId !== selectedChapterId) {
      await selectChapter(chapterId);
    }
    await selectScene(sceneId);
  }

  async function handleQuickSelect(entry: NavEntry) {
    switcherOpen = false;
    if (entry.scene_id) {
      await goToSceneInChapter(entry.chapter_id, entry.scene_id);
    } else {
      await goToChapter(entry.chapter_id);
    }
  }

  async function nextChapter() {
    if (!selectedChapterId || chapters.length === 0) return;
    const i = chapters.findIndex((c) => c.id === selectedChapterId);
    const next = chapters[Math.min(i + 1, chapters.length - 1)];
    if (next && next.id !== selectedChapterId) await goToChapter(next.id);
  }

  async function prevChapter() {
    if (!selectedChapterId || chapters.length === 0) return;
    const i = chapters.findIndex((c) => c.id === selectedChapterId);
    const prev = chapters[Math.max(i - 1, 0)];
    if (prev && prev.id !== selectedChapterId) await goToChapter(prev.id);
  }

  function sceneEntries(): NavEntry[] {
    return navigation.filter((e) => e.scene_id !== null);
  }

  async function nextScene() {
    if (!selectedSceneId) return;
    const entries = sceneEntries();
    if (entries.length === 0) return;
    const i = entries.findIndex((e) => e.scene_id === selectedSceneId);
    if (i === -1) return;
    const next = entries[Math.min(i + 1, entries.length - 1)];
    if (next && next.scene_id !== selectedSceneId && next.scene_id) {
      await goToSceneInChapter(next.chapter_id, next.scene_id);
    }
  }

  async function prevScene() {
    if (!selectedSceneId) return;
    const entries = sceneEntries();
    if (entries.length === 0) return;
    const i = entries.findIndex((e) => e.scene_id === selectedSceneId);
    if (i === -1) return;
    const prev = entries[Math.max(i - 1, 0)];
    if (prev && prev.scene_id !== selectedSceneId && prev.scene_id) {
      await goToSceneInChapter(prev.chapter_id, prev.scene_id);
    }
  }

  async function backToProjects() {
    await flushSave();
    selectedProjectId = null;
    selectedChapterId = null;
    selectedSceneId = null;
    sceneContent = "";
    sceneTitle = null;
    currentScene = null;
    chapters = [];
    scenes = [];
    navigation = [];
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function beginDrag(which: "chapters" | "scenes" | "options") {
    const move = (e: MouseEvent) => {
      if (which === "chapters") chaptersWidth = clamp(e.clientX, 140, 520);
      else if (which === "scenes") scenesWidth = clamp(e.clientX, 140, 520);
      else if (which === "options") optionsWidth = clamp(window.innerWidth - e.clientX, 240, 640);
    };
    const up = () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  }

  // -------------------------------------------------------------------------
  // Creación y borrado
  // -------------------------------------------------------------------------

  async function createProject(
    title: string,
    goal: number | null,
    description: string | null,
    coverPath: string | null,
  ) {
    try {
      const project = await api.createProject(title, goal, description, coverPath);
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
        entities = [];
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
              "md", "markdown", "txt", "pdf", "docx", "epub",
              "png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp",
            ],
          },
          { name: "Markdown / Texto", extensions: ["md", "markdown", "txt"] },
          { name: "PDF", extensions: ["pdf"] },
          { name: "Word", extensions: ["docx"] },
          { name: "EPUB", extensions: ["epub"] },
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

  async function importProject(
    path: string,
    goal: number | null,
    description: string | null,
    coverPath: string | null,
  ) {
    try {
      importing = true;
      const project = await api.importFileAsProject(path, goal, description, coverPath);
      projects = await api.listProjects();
      await selectProject(project.id);
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
      // Análisis automático en segundo plano al marcar como revisado/final.
      if (status === "revised" || status === "final") {
        void analyze();
      }
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

  async function moveSceneInChapter(chapterId: string, id: string, index: number) {
    try {
      const result = await api.moveScene(chapterId, id, index);
      if (chapterId === selectedChapterId) {
        scenes = result;
      }
      if (selectedProjectId) {
        navigation = await api.listNavigation(selectedProjectId);
      }
    } catch (e) {
      reportError(e);
    }
  }

  async function updateProjectGoal(goal: number) {
    if (!selectedProjectId) return;
    try {
      await api.updateProjectGoal(selectedProjectId, goal);
      projects = await api.listProjects();
    } catch (e) {
      reportError(e);
    }
  }

  async function updateProjectLevels(level1Name: string, level2Name: string) {
    if (!selectedProjectId) return;
    try {
      const updated = await api.updateProjectLevels(selectedProjectId, level1Name, level2Name);
      projects = projects.map((p) => (p.id === updated.id ? updated : p));
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
    analyzeProgress = { done: 0, total: 0, phase: "escenas" };
    const before = new Set(characters.map((c) => c.name));
    try {
      const report = await api.analyzeProject(selectedProjectId, selectedChapterId);
      characters = report.characters;
      relationships = report.relationships;
      entities = report.entities;

      const added = report.characters.filter((c) => !before.has(c.name)).length;
      const removed = [...before].filter(
        (n) => !report.characters.some((c) => c.name === n),
      ).length;
      const parts: string[] = [];
      if (added) parts.push(`+${added} personajes`);
      if (removed) parts.push(`-${removed} personajes`);
      parts.push(`${report.relationships.length} relaciones`);
      parts.push(`${report.entities.length} entidades`);
      logChange(`Análisis: ${parts.join(", ")}`);
    } catch (e) {
      reportError(e);
    } finally {
      analyzing = false;
      analyzeProgress = null;
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

  async function deleteEntity(id: string) {
    try {
      await api.deleteEntity(id);
      if (selectedProjectId) {
        entities = await api.listEntities(selectedProjectId);
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
      logChange(`Enriquecimiento: ${characters.length} descripciones generadas`);
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

  async function chat(message: string): Promise<string> {
    return await api.chat(message, iaStatus?.settings.llm_model ?? null, selectedProjectId);
  }

  async function indexProject() {
    if (!selectedProjectId) return;
    try {
      await api.indexProject(selectedProjectId);
    } catch (e) {
      reportError(e);
    }
  }

  function logChange(summary: string) {
    changeLog = [{ time: new Date().toLocaleTimeString(), summary }, ...changeLog];
    if (changeLog.length > 200) changeLog = changeLog.slice(0, 200);
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

  async function installGliner() {
    try {
      await api.iaInstall("gliner");
      iaStatus = await api.iaStatus();
    } catch (e) {
      reportError(e);
    }
  }

  async function installOnnxRuntime() {
    try {
      await api.iaInstall("onnxruntime");
      iaStatus = await api.iaStatus();
    } catch (e) {
      reportError(e);
    }
  }

  async function updateNerEngine(engine: string) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, ner_engine: engine };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateKeepLoaded(enabled: boolean) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, keep_loaded: enabled };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateRelationshipModel(model: string | null) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, relationship_model: model };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateCloudProvider(provider: string | null) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, cloud_provider: provider };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateCloudApiKey(key: string | null) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, cloud_api_key: key };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateCloudModel(model: string | null) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, cloud_model: model };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
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

  async function correctGrammar() {
    if (!selectedSceneId) return;
    correcting = true;
    try {
      const corrected = await api.correctText(sceneContent, null);
      sceneContent = corrected;
      editorRevision++;
      scheduleSave();
    } catch (e) {
      reportError(e);
    } finally {
      correcting = false;
    }
  }

  async function checkSpelling() {
    if (!selectedSceneId) return;
    spellBusy = true;
    try {
      spellReport = await api.spellCheck(sceneContent);
      spellOpen = true;
    } catch (e) {
      reportError(e);
    } finally {
      spellBusy = false;
    }
  }

  async function installSpell(code: string) {
    spellBusy = true;
    try {
      await api.spellInstall(code);
      spellReport = await api.spellCheck(sceneContent);
    } catch (e) {
      reportError(e);
    } finally {
      spellBusy = false;
    }
  }

  function replaceWord(word: string, suggestion: string) {
    const escaped = word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    sceneContent = sceneContent.replace(
      new RegExp(`\\b${escaped}\\b`, "gi"),
      suggestion,
    );
    editorRevision++;
    scheduleSave();
  }

  // -------------------------------------------------------------------------
  // Versiones (snapshots)
  // -------------------------------------------------------------------------

  async function loadSnapshots() {
    if (!selectedProjectId) return;
    snapshots = await api.listSnapshots(selectedProjectId);
  }

  async function createSnapshot(message: string | null) {
    if (!selectedProjectId) return;
    snapshotBusy = true;
    try {
      await api.createSnapshot(selectedProjectId, message);
      await loadSnapshots();
    } catch (e) {
      reportError(e);
    } finally {
      snapshotBusy = false;
    }
  }

  async function restoreSnapshot(id: string) {
    try {
      await api.restoreSnapshot(id);
      if (selectedProjectId) {
        chapters = await api.listChapters(selectedProjectId);
        scenes = [];
        if (selectedChapterId && selectedSceneId) {
          const scene = await api.getScene(selectedSceneId);
          sceneContent = scene.content_markdown;
          sceneTitle = scene.title;
          currentScene = scene;
          editorRevision++;
        } else {
          selectedChapterId = null;
          sceneContent = "";
          sceneTitle = null;
          currentScene = null;
        }
        await loadSnapshots();
      }
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteSnapshot(id: string) {
    try {
      await api.deleteSnapshot(id);
      await loadSnapshots();
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
  {#if !selectedProjectId}
    <ProjectsView
      {projects}
      focusSignal={projectsFocusSignal}
      onselectproject={selectProject}
      oncreateproject={createProject}
      ondeleteproject={deleteProject}
      onimportproject={importProject}
    />
  {:else}
    <div class="workspace">
      {#if !chaptersCollapsed}
        <aside class="panel" style:width="{chaptersWidth}px">
          <ChaptersPanel
            {chapters}
            {navigation}
            {selectedChapterId}
            {selectedSceneId}
            wordGoal={selectedProject?.word_goal ?? 3000}
            mode={viewMode === "tree" ? "tree" : "flat"}
            level1Name={selectedProject?.level1_name ?? "Capítulos"}
            level2Name={selectedProject?.level2_name ?? "Escenas"}
            showScenes={projectUsesScenes}
            onselectchapter={selectChapter}
            onselectscene={selectScene}
            oncreatechapter={createChapter}
            ondeletechapter={deleteChapter}
            onmovechapter={moveChapter}
            onmovescene={moveSceneInChapter}
            onupdategoal={updateProjectGoal}
            ontoggle={() => (chaptersCollapsed = true)}
          />
        </aside>
        <button class="divider" type="button" aria-label="Ajustar ancho" onmousedown={() => beginDrag("chapters")}></button>
      {:else}
        <button class="collapsed-tab" onclick={() => (chaptersCollapsed = false)}>{selectedProject?.level1_name ?? "Capítulos"}</button>
      {/if}

      {#if viewMode === "split"}
        {#if !scenesCollapsed}
          <aside class="panel" style:width="{scenesWidth}px">
            <ScenesPanel
              {scenes}
              {selectedSceneId}
              level2Name={selectedProject?.level2_name ?? "Escenas"}
              onselectscene={selectScene}
              oncreatescene={createScene}
              ondeletescene={deleteScene}
              onmovescene={moveScene}
              ontoggle={() => (scenesCollapsed = true)}
            />
          </aside>
          <button class="divider" type="button" aria-label="Ajustar ancho" onmousedown={() => beginDrag("scenes")}></button>
        {:else}
          <button class="collapsed-tab" onclick={() => (scenesCollapsed = false)}>{selectedProject?.level2_name ?? "Escenas"}</button>
        {/if}
      {/if}

      <main class="main">
    <header class="topbar">
      <div class="crumbs">
        <button class="btn-icon" title="Volver a proyectos" onclick={backToProjects}>‹</button>
        <span class="crumb">{selectedProject?.title}</span>
        {#if selectedChapter}
          <span class="sep">›</span>
          <span class="crumb">{selectedChapter.title}</span>
        {/if}
        {#if selectedSceneId}
          <span class="sep">›</span>
          <span class="crumb muted">{sceneTitle ?? "Escena"}</span>
        {/if}
      </div>

      <div class="top-actions">
        {#if selectedSceneId}
          <button class="btn-icon" title="Reproducir" onclick={playTts}>▶</button>
          <button class="btn-icon" title="Detener" onclick={stopTts}>■</button>
          <button
            class="btn"
            title="Corrige gramática y ortografía con un LLM local (Ollama)"
            onclick={correctGrammar}
            disabled={correcting}
          >{correcting ? "Corrigiendo…" : "Corregir"}</button>
          <button
            class="btn"
            title="Revisar ortografía (Hunspell, sin LLM)"
            onclick={checkSpelling}
            disabled={spellBusy}
          >{spellBusy ? "Revisando…" : "Ortografía"}</button>
        {/if}

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

        <button class="btn" onclick={importManuscript} disabled={importing}>
          {importing ? "Importando…" : "Importar"}
        </button>

        <button
          class="btn btn-toggle"
          class:active={worldOpen}
          onclick={() => (worldOpen = !worldOpen)}
          title="World Building: personajes, relaciones y mundo"
        >Mundo</button>

        <button
          class="btn btn-toggle"
          class:active={optionsOpen}
          onclick={() => (optionsOpen = !optionsOpen)}
          title="Asistente, metadatos y versiones"
        >Panel</button>

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

        <div class="segmented" title="Modo de visualización de capítulos y escenas">
          <button
            class:active={viewMode === "split"}
            onclick={() => (viewMode = "split")}
          >Separado</button>
          <button
            class:active={viewMode === "tree"}
            onclick={() => (viewMode = "tree")}
          >Árbol</button>
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
        {#key `${selectedSceneId}-${editorRevision}`}
          {#if mode === "wysiwyg"}
            <Editor content={sceneContent} onchange={onContentChange} />
          {:else}
            <SourceEditor content={sceneContent} onchange={onContentChange} />
          {/if}
        {/key}
      {:else}
        <div class="empty">
          <div class="empty-mark">✒</div>
          {#if selectedChapterId}
            <h2>Sin escenas todavía</h2>
            <p>Este capítulo no tiene contenido. Crea su primera escena para empezar a escribir.</p>
            <button class="btn btn-primary" onclick={() => createScene(null)}>＋ Nueva escena</button>
          {:else}
            <h2>Comienza tu historia</h2>
            <p>
              Crea un <strong>proyecto</strong>, añade un <strong>capítulo</strong> y
              luego una <strong>escena</strong> para empezar a escribir.
            </p>
          {/if}
        </div>
      {/if}
    </div>
  </main>

      {#if optionsOpen}
        <button class="divider" type="button" aria-label="Ajustar ancho" onmousedown={() => beginDrag("options")}></button>
        <aside class="panel panel-right" style:width="{optionsWidth}px">
          <OptionsPanel
            chapter={selectedChapter}
            scene={currentScene}
            project={selectedProject}
            onsavechapter={saveChapterMeta}
            onsavescene={saveSceneMeta}
            onupdatelevels={updateProjectLevels}
            ollamaAvailable={ollamaAvailable}
            onchat={chat}
            {snapshots}
            snapshotBusy={snapshotBusy}
            oncreatesnapshot={createSnapshot}
            onrestoresnapshot={restoreSnapshot}
            ondeletesnapshot={deleteSnapshot}
            onloadsnapshots={loadSnapshots}
            {iaStatus}
            oninstallonnxruntime={installOnnxRuntime}
            onupdatenerengine={updateNerEngine}
            oninstallgliner={installGliner}
            onupdateembeddings={updateEmbeddings}
            oninstallembeddings={installEmbeddings}
            onupdaterelationshipmodel={updateRelationshipModel}
            onupdatekeeploaded={updateKeepLoaded}
            onupdatecloudprovider={updateCloudProvider}
            onupdatecloudapikey={updateCloudApiKey}
            onupdatecloudmodel={updateCloudModel}
            onindexproject={indexProject}
          />
        </aside>
      {/if}
    </div>
  {/if}

  {#if switcherOpen}
    <QuickSwitcher
      entries={navigation}
      {selectedChapterId}
      {selectedSceneId}
      onselect={handleQuickSelect}
      onclose={() => (switcherOpen = false)}
    />
  {/if}

  {#if worldOpen}
    <WorldBuilding
      {characters}
      {relationships}
      {entities}
      {analyzing}
      {enriching}
      progress={analyzeProgress}
      {changeLog}
      onanalyze={analyze}
      onenrich={enrich}
      ondeletecharacter={deleteCharacter}
      ondeleteentity={deleteEntity}
      onclose={() => (worldOpen = false)}
    />
  {/if}

  {#if spellOpen}
    <SpellPanel
      report={spellReport}
      installing={spellBusy}
      oninstall={installSpell}
      onreplace={replaceWord}
      onclose={() => (spellOpen = false)}
    />
  {/if}
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .workspace {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .panel {
    flex-shrink: 0;
    height: 100%;
    overflow: hidden;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
  }

  .panel-right {
    border-right: none;
    border-left: 1px solid var(--border);
  }

  .divider {
    flex-shrink: 0;
    width: 4px;
    cursor: col-resize;
    background: transparent;
    border: none;
    padding: 0;
    transition: background 0.12s;
  }

  .divider:hover {
    background: var(--accent);
  }

  .collapsed-tab {
    flex-shrink: 0;
    writing-mode: vertical-rl;
    font: inherit;
    font-size: 12px;
    padding: 10px 4px;
    border: none;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
    color: var(--text-muted);
    cursor: pointer;
  }

  .collapsed-tab:hover {
    color: var(--accent);
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
