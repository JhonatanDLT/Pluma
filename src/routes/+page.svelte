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
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import * as api from "$lib/api";
  import { applyTheme, getTheme } from "$lib/theme";
  import {
    applyFontSettings,
    getFontSettings,
    registerCustomFonts,
  } from "$lib/fonts";
  import { applyEditorSettings, getEditorSettings } from "$lib/editor";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import type {
    BackupInfo,
    ChangeEntry,
    Chapter,
    Character,
    CloudProviderInfo,
    EditorMode,
    Entity,
    IaStatus,
    NavEntry,
    ProfileInfo,
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
  let focusMode = $state(false);
  let zenMode = $state(false);
  let searchOpen = $state(false);
  const hideChrome = $derived(focusMode || zenMode);
  let saveState = $state<SaveState>("idle");

  const RECENT_KEY = "pluma.recentProjects";
  let recentIds = $state<string[]>(loadRecentIds());

  function loadRecentIds(): string[] {
    try {
      const raw = localStorage.getItem(RECENT_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed.slice(0, 5) : [];
    } catch {
      return [];
    }
  }

  function rememberProject(id: string) {
    recentIds = [id, ...recentIds.filter((x) => x !== id)].slice(0, 5);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(recentIds));
    } catch {
      // Sin localStorage no se persiste, pero la sesión actual funciona.
    }
  }
  let journalRecovery = $state<{
    sceneId: string;
    content: string;
    at: number;
  } | null>(null);
  let errorMsg = $state<string | null>(null);
  let importing = $state(false);
  let analyzing = $state(false);
  let analyzeProgress = $state<{ done: number; total: number; phase: string } | null>(null);
  let enriching = $state(false);
  let correcting = $state(false);
  let editorRevision = $state(0);
  let iaStatus = $state<IaStatus | null>(null);
  let cloudProviders = $state<CloudProviderInfo[]>([]);
  let profiles = $state<ProfileInfo[]>([]);
  let snapshots = $state<SnapshotMeta[]>([]);
  let snapshotBusy = $state(false);
  let backups = $state<BackupInfo[]>([]);
  let backupBusy = $state(false);
  let navigation = $state<NavEntry[]>([]);
  let switcherOpen = $state(false);
  let exportOpen = $state(false);
  let worldOpen = $state(false);
  let changeLog = $state<ChangeEntry[]>([]);
  let spellOpen = $state(false);
  let spellReport = $state<SpellReport | null>(null);
  const SPELL_IGNORED_KEY = "pluma.spell.ignored";
  let ignoredWords = $state<string[]>(loadIgnoredWords());
  let sessionIgnored = $state<string[]>([]);

  function loadIgnoredWords(): string[] {
    try {
      const raw = localStorage.getItem(SPELL_IGNORED_KEY);
      return raw ? (JSON.parse(raw) as string[]) : [];
    } catch {
      return [];
    }
  }

  function isIgnored(word: string): boolean {
    const key = word.toLowerCase();
    return (
      ignoredWords.some((w) => w.toLowerCase() === key) ||
      sessionIgnored.some((w) => w.toLowerCase() === key)
    );
  }

  function filterSpellReport(report: SpellReport): SpellReport {
    return {
      ...report,
      issues: report.issues.filter((issue) => !isIgnored(issue.word)),
    };
  }

  function ignoreWord(word: string, permanent: boolean) {
    if (permanent) {
      if (!ignoredWords.some((w) => w.toLowerCase() === word.toLowerCase())) {
        ignoredWords = [...ignoredWords, word];
        try {
          localStorage.setItem(SPELL_IGNORED_KEY, JSON.stringify(ignoredWords));
        } catch {
          // Sin localStorage el ignore permanente dura la sesión.
        }
      }
    } else if (
      !sessionIgnored.some((w) => w.toLowerCase() === word.toLowerCase())
    ) {
      sessionIgnored = [...sessionIgnored, word];
    }
    if (spellReport) {
      spellReport = {
        ...spellReport,
        issues: spellReport.issues.filter((issue) => issue.word !== word),
      };
    }
  }
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
    await loadCloudProviders();
    await loadProfiles();
    await loadTtsVoices();
    await loadBackups();
  });

  onMount(() => {
    applyTheme(getTheme());
    applyFontSettings(getFontSettings());
    applyEditorSettings(getEditorSettings());
    registerCustomFonts();
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

  // Asociaciones de archivos: macOS emite `open-file`; Windows/Linux pasan la
  // ruta por línea de comandos y se recoge con `take_opened_files`.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    listen<string[]>("open-file", (e) => {
      void openExternalFiles(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    void api
      .takeOpenedFiles()
      .then((paths) => {
        if (paths.length > 0) void openExternalFiles(paths);
      })
      .catch(() => {});
    return () => unlisten?.();
  });

  // Drag-and-drop de archivos sobre la ventana.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type === "drop" && event.payload.paths.length > 0) {
            void openExternalFiles(event.payload.paths);
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch(() => {});
    } catch {
      // Fuera de Tauri (p. ej. `pnpm dev` en navegador) no hay drag-drop nativo.
    }
    return () => unlisten?.();
  });

  // Flush de seguridad al cerrar la ventana: evita perder los últimos 600 ms
  // de escritura del autoguardado.
  onMount(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    let closing = false;
    win.onCloseRequested(async (event) => {
      if (closing) return;
      closing = true;
      event.preventDefault();
      try {
        await flushSave();
      } finally {
        await win.destroy();
      }
    }).then((fn) => {
      unlisten = fn;
    });
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      if (saveState === "saving") {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      unlisten?.();
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  });

  onMount(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        if (selectedProjectId) searchOpen = !searchOpen;
        return;
      }
      if (mod && e.shiftKey && e.key.toLowerCase() === "m") {
        e.preventDefault();
        zenMode = !zenMode;
        return;
      }
      if (mod && e.shiftKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        focusMode = !focusMode;
        return;
      }
      if (
        e.key === "Escape" &&
        zenMode &&
        !worldOpen &&
        !switcherOpen &&
        !spellOpen &&
        !searchOpen
      ) {
        zenMode = false;
        return;
      }
      if (
        e.key === "Escape" &&
        focusMode &&
        !worldOpen &&
        !switcherOpen &&
        !spellOpen &&
        !searchOpen
      ) {
        focusMode = false;
        return;
      }
      if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void flushSave().then(() => {
          if (saveState !== "error") saveState = "saved";
        });
        return;
      }
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

  /// Importa un archivo abierto desde el sistema (doble clic o drag-and-drop)
  /// como un proyecto nuevo y lo selecciona.
  async function openExternalFiles(paths: string[]) {
    if (paths.length === 0) return;
    try {
      const project = await api.importFileAsProject(paths[0], null, null, null, null);
      projects = await api.listProjects();
      await selectProject(project.id);
    } catch (e) {
      reportError(e);
    }
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
    rememberProject(id);
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
    journalRecovery = null;
    const scene = await api.getScene(id);
    sceneContent = scene.content_markdown;
    sceneTitle = scene.title;
    currentScene = scene;
    selectedSceneId = id;

    // Diario de recuperación: si hay contenido local más reciente que la
    // versión guardada (cierre inesperado), se ofrece recuperarlo.
    try {
      const raw = localStorage.getItem(journalKey(id));
      if (raw) {
        const entry = JSON.parse(raw) as { content: string; at: number };
        const savedAt = new Date(scene.updated_at).getTime();
        if (entry.content !== scene.content_markdown && entry.at > savedAt) {
          journalRecovery = { sceneId: id, content: entry.content, at: entry.at };
        } else {
          clearJournal(id);
        }
      }
    } catch {
      // Diario corrupto o localStorage no disponible: se ignora.
    }
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
    author: string | null,
    goal: number | null,
    description: string | null,
    coverPath: string | null,
  ) {
    try {
      const project = await api.createProject(title, author, goal, description, coverPath);
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

  async function duplicateProject(id: string) {
    try {
      const project = await api.duplicateProject(id);
      projects = await api.listProjects();
      await selectProject(project.id);
    } catch (e) {
      reportError(e);
    }
  }

  async function createSampleProject() {
    try {
      const project = await api.createSampleProject();
      projects = await api.listProjects();
      await selectProject(project.id);
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteProject(id: string) {
    const project = projects.find((p) => p.id === id);
    const title = project?.title ?? "este proyecto";
    const answer = window.prompt(
      `Para eliminar «${title}», escribe el título del proyecto:`,
      "",
    );
    if (answer?.trim() !== title) return;
    try {
      await api.deleteProject(id);
      recentIds = recentIds.filter((x) => x !== id);
      try {
        localStorage.setItem(RECENT_KEY, JSON.stringify(recentIds));
      } catch {
        // Sin localStorage no hay nada que limpiar.
      }
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
    const chapter = chapters.find((c) => c.id === id);
    if (
      !window.confirm(
        `¿Eliminar el capítulo «${chapter?.title ?? "sin título"}» y todas sus escenas?`,
      )
    ) {
      return;
    }
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
    const scene = scenes.find((s) => s.id === id);
    if (!window.confirm(`¿Eliminar la escena «${scene?.title ?? "sin título"}»?`)) {
      return;
    }
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
      importing = true;
      const imported = await api.importFileDialog(selectedProjectId);
      if (!imported || imported.length === 0) return;

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
    author: string | null,
    goal: number | null,
    description: string | null,
    coverPath: string | null,
  ) {
    try {
      importing = true;
      const project = await api.importFileAsProject(path, author, goal, description, coverPath);
      projects = await api.listProjects();
      await selectProject(project.id);
    } catch (e) {
      reportError(e);
    } finally {
      importing = false;
    }
  }

  async function updateProjectMeta(author: string | null, description: string | null) {
    if (!selectedProjectId) return;
    try {
      const updated = await api.updateProjectMeta(selectedProjectId, author, description);
      projects = projects.map((p) => (p.id === updated.id ? updated : p));
    } catch (e) {
      reportError(e);
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

  async function exportDocument(format: string, _ext: string) {
    if (!selectedProjectId) return;
    exportOpen = false;
    try {
      const path = await api.exportProjectDialog(selectedProjectId, format);
      if (path) logChange(`Exportado: ${path}`);
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

  async function updateCharacter(character: Character) {
    try {
      await api.updateCharacter(
        character.id,
        character.name,
        character.aliases,
        character.role,
        character.description,
        character.biography,
        character.traits,
        character.goals,
        character.arc,
        character.color,
      );
      if (selectedProjectId) {
        characters = await api.listCharacters(selectedProjectId);
      }
      logChange(`Personaje actualizado: ${character.name}`);
    } catch (e) {
      reportError(e);
    }
  }

  async function updateEntity(entity: Entity) {
    try {
      await api.updateEntity(
        entity.id,
        entity.name,
        entity.entity_type,
        entity.aliases,
        entity.description,
      );
      if (selectedProjectId) {
        entities = await api.listEntities(selectedProjectId);
      }
      logChange(`Entidad actualizada: ${entity.name}`);
    } catch (e) {
      reportError(e);
    }
  }

  async function deleteCharacter(id: string) {
    if (!window.confirm("¿Eliminar este personaje de la biblia?")) return;
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
    if (!window.confirm("¿Eliminar esta entidad de la biblia?")) return;
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

  async function loadCloudProviders() {
    try {
      cloudProviders = await api.iaCloudProviders();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadProfiles() {
    try {
      profiles = await api.iaProfiles();
    } catch (e) {
      console.error(e);
    }
  }

  async function applyProfile(profileId: string | null) {
    if (!iaStatus) return;
    try {
      if (!profileId) {
        // "Personalizado": solo marca como custom sin tocar los modelos.
        const settings = { ...iaStatus.settings, profile: null };
        const saved = await api.iaSetSettings(settings);
        iaStatus = { ...iaStatus, settings: saved };
        return;
      }
      const saved = await api.iaApplyProfile(profileId);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
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
      const settings = { ...iaStatus.settings, embeddings: enabled, profile: null };
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
      const settings = { ...iaStatus.settings, ner_engine: engine, profile: null };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateKeepLoaded(enabled: boolean) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, keep_loaded: enabled, profile: null };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateRelationshipModel(model: string | null) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, relationship_model: model, profile: null };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateCloudProvider(provider: string | null) {
    if (!iaStatus) return;
    try {
      const prev = iaStatus.settings.cloud_provider;
      const def =
        cloudProviders.find((p) => p.id === provider)?.default_model ?? null;
      // Al cambiar de proveedor se reinicia el modelo al predeterminado del nuevo.
      const settings = {
        ...iaStatus.settings,
        cloud_provider: provider,
        cloud_model: prev !== provider ? def : iaStatus.settings.cloud_model,
      };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  async function updateCloudApiKey(key: string | null) {
    if (!iaStatus) return;
    try {
      const cloudKeySet = await api.iaSetCloudKey(key);
      iaStatus = { ...iaStatus, cloud_key_set: cloudKeySet };
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

  async function updateCloudEnabled(enabled: boolean) {
    if (!iaStatus) return;
    try {
      const settings = { ...iaStatus.settings, cloud_ai_enabled: enabled };
      const saved = await api.iaSetSettings(settings);
      iaStatus = { ...iaStatus, settings: saved };
    } catch (e) {
      reportError(e);
    }
  }

  // -------------------------------------------------------------------------
  // Texto a voz
  // -------------------------------------------------------------------------

  let ttsVoices = $state<{ name: string; locale: string }[]>([]);
  let ttsVoice = $state<string>("");
  let ttsPaused = $state(false);
  let ttsRate = $state("1.0");

  async function loadTtsVoices() {
    try {
      ttsVoices = await api.ttsListVoices();
      const spanish = ttsVoices.find((v) => v.locale.startsWith("es"));
      if (spanish) ttsVoice = spanish.name;
    } catch {
      // TTS es opcional: si el sistema no expone voces, se usa la default.
    }
  }

  async function playTts() {
    if (!selectedSceneId) return;
    try {
      ttsPaused = false;
      await api.ttsSpeak(sceneContent, ttsVoice || null, Number(ttsRate));
    } catch (e) {
      reportError(e);
    }
  }

  async function pauseTts() {
    try {
      if (ttsPaused) {
        await api.ttsResume();
        ttsPaused = false;
      } else {
        await api.ttsPause();
        ttsPaused = true;
      }
    } catch (e) {
      reportError(e);
    }
  }

  async function stopTts() {
    try {
      ttsPaused = false;
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
      spellReport = filterSpellReport(await api.spellCheck(sceneContent));
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
      spellReport = filterSpellReport(await api.spellCheck(sceneContent));
    } catch (e) {
      reportError(e);
    } finally {
      spellBusy = false;
    }
  }

  function replaceWord(word: string, suggestion: string) {
    const escaped = word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    // Reemplaza solo la primera ocurrencia: el panel de ortografía muestra una
    // incidencia concreta y no debe cambiar el resto de la escena.
    sceneContent = sceneContent.replace(
      new RegExp(`\\b${escaped}\\b`, "i"),
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

  async function loadBackups() {
    try {
      backups = await api.listBackups();
    } catch (e) {
      console.error(e);
    }
  }

  async function createBackup() {
    backupBusy = true;
    try {
      await api.backupDatabase();
      await loadBackups();
      logChange("Copia de seguridad creada");
    } catch (e) {
      reportError(e);
    } finally {
      backupBusy = false;
    }
  }

  async function restoreBackupFile(file: string) {
    backupBusy = true;
    try {
      const project = await api.restoreBackup(file);
      projects = await api.listProjects();
      await selectProject(project.id);
      logChange(`Copia restaurada como proyecto: ${project.title}`);
    } catch (e) {
      reportError(e);
    } finally {
      backupBusy = false;
    }
  }

  async function deleteBackupFile(file: string) {
    if (!window.confirm(`¿Eliminar la copia ${file}?`)) return;
    try {
      await api.deleteBackup(file);
      await loadBackups();
    } catch (e) {
      reportError(e);
    }
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
    if (!window.confirm("¿Eliminar esta versión guardada?")) return;
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

  const JOURNAL_PREFIX = "pluma.journal.";

  function journalKey(id: string): string {
    return `${JOURNAL_PREFIX}${id}`;
  }

  function writeJournal(id: string, content: string) {
    try {
      localStorage.setItem(
        journalKey(id),
        JSON.stringify({ content, at: Date.now() }),
      );
    } catch {
      // localStorage lleno o no disponible: el autoguardado sigue funcionando.
    }
  }

  function clearJournal(id: string) {
    try {
      localStorage.removeItem(journalKey(id));
    } catch {
      // Sin localStorage no hay diario que limpiar.
    }
  }

  function onContentChange(markdown: string) {
    sceneContent = markdown;
    if (selectedSceneId) writeJournal(selectedSceneId, markdown);
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

  async function persistScene(id: string, attempt = 0): Promise<boolean> {
    try {
      await api.saveScene(id, sceneContent);
      if (selectedSceneId === id) saveState = "saved";

      // Refresca contadores en memoria (el panel no debe quedarse obsoleto
      // mientras se escribe) sin volver a consultar la base de datos.
      const words = countWords(sceneContent);
      if (currentScene && currentScene.id === id) {
        const delta = words - currentScene.word_count;
        if (delta !== 0 && selectedChapterId) {
          chapters = chapters.map((c) =>
            c.id === selectedChapterId
              ? { ...c, word_count: Math.max(0, c.word_count + delta) }
              : c,
          );
          scenes = scenes.map((s) =>
            s.id === id ? { ...s, word_count: words } : s,
          );
        }
        currentScene = {
          ...currentScene,
          word_count: words,
          content_markdown: sceneContent,
        };
      }

      clearJournal(id);
      return true;
    } catch (e) {
      console.error(e);
      // Un reintento ante fallos transitorios (fichero bloqueado, etc.).
      if (attempt < 1) {
        await new Promise((resolve) => setTimeout(resolve, 400));
        return persistScene(id, attempt + 1);
      }
      if (selectedSceneId === id) saveState = "error";
      return false;
    }
  }

  async function flushSave() {
    if (saveTimer) {
      window.clearTimeout(saveTimer);
      saveTimer = undefined;
    }
    const id = selectedSceneId;
    if (id && (saveState === "saving" || saveState === "error")) {
      await persistScene(id);
    }
  }
</script>

<div class="app-shell">
  {#if !selectedProjectId}
    <ProjectsView
      {projects}
      {recentIds}
      focusSignal={projectsFocusSignal}
      onselectproject={selectProject}
      oncreateproject={createProject}
      ondeleteproject={deleteProject}
      onduplicateproject={duplicateProject}
      onsampleproject={createSampleProject}
      onimportproject={importProject}
    />
  {:else}
    <div class="workspace" class:focus={hideChrome}>
      {#if !hideChrome && !chaptersCollapsed}
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

      {#if !hideChrome && viewMode === "split" && projectUsesScenes}
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
    {#if !zenMode}
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
          {#if ttsVoices.length > 0}
            <select class="tts-voice" bind:value={ttsVoice} title="Voz del sistema">
              <option value="">Voz por defecto</option>
              {#each ttsVoices.filter((v) => v.locale.startsWith("es")) as voice}
                <option value={voice.name}>{voice.name}</option>
              {/each}
            </select>
          {/if}
          <button class="btn-icon" title="Reproducir" onclick={playTts}>▶</button>
          <button
            class="btn-icon"
            title={ttsPaused ? "Reanudar" : "Pausar"}
            onclick={pauseTts}
          >{ttsPaused ? "⏵" : "⏸"}</button>
          <button class="btn-icon" title="Detener" onclick={stopTts}>■</button>
          <select class="tts-voice tts-rate" bind:value={ttsRate} title="Velocidad de lectura">
            <option value="0.75">0.75×</option>
            <option value="1.0">1×</option>
            <option value="1.25">1.25×</option>
            <option value="1.5">1.5×</option>
          </select>
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

        <span
          class="save-status"
          class:error={saveState === "error"}
          role="status"
          aria-live="polite"
        >
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

        <button
          class="btn-icon"
          title="Buscar en la novela (Ctrl/Cmd+Shift+P)"
          aria-label="Buscar en la novela"
          onclick={() => (searchOpen = true)}
        >⌕</button>

        <button
          class="btn-icon"
          title="Modo zen: ocultar todos los menús (Ctrl/Cmd+Shift+M)"
          aria-label="Activar modo zen"
          onclick={() => (zenMode = true)}
        >▢</button>

        <button
          class="btn-icon focus-exit"
          title={focusMode
            ? "Salir del modo foco (Esc)"
            : "Modo foco (Ctrl/Cmd+Shift+F)"}
          aria-label={focusMode ? "Salir del modo foco" : "Activar modo foco"}
          onclick={() => (focusMode = !focusMode)}
        >{focusMode ? "⤢" : "⛶"}</button>
      </div>
    </header>
    {:else}
      <div class="zen-pill" role="status">
        {#if selectedSceneId}
          <span class="wordcount">{wordCount.toLocaleString()} palabras</span>
        {/if}
        <span
          class="save-status"
          class:error={saveState === "error"}
          aria-live="polite"
        >
          {#if saveState === "saving"}Guardando…{/if}
          {#if saveState === "saved"}Guardado{/if}
          {#if saveState === "error"}Error al guardar{/if}
        </span>
        <button
          class="btn-icon"
          title="Salir del modo zen (Esc)"
          aria-label="Salir del modo zen"
          onclick={() => (zenMode = false)}
        >⤢</button>
      </div>
    {/if}

    {#if errorMsg}
      <div class="banner">
        <span>{errorMsg}</span>
        <button class="btn-icon" onclick={() => (errorMsg = null)}>×</button>
      </div>
    {/if}

    <div class="editor-area">
      {#if journalRecovery}
        <div class="journal-banner" role="status">
          <span>
            Hay cambios sin guardar de esta escena
            ({new Date(journalRecovery.at).toLocaleTimeString()}).
          </span>
          <button
            class="btn"
            onclick={() => {
              if (!journalRecovery) return;
              sceneContent = journalRecovery.content;
              journalRecovery = null;
              onContentChange(sceneContent);
            }}
          >Recuperar</button>
          <button
            class="btn"
            onclick={() => {
              if (!journalRecovery) return;
              clearJournal(journalRecovery.sceneId);
              journalRecovery = null;
            }}
          >Descartar</button>
        </div>
      {/if}
      {#if selectedSceneId}
        {#key selectedSceneId}
          {#if mode === "wysiwyg"}
            <Editor
              content={sceneContent}
              onchange={onContentChange}
              focus={hideChrome}
              revision={editorRevision}
            />
          {:else}
            <SourceEditor
              content={sceneContent}
              onchange={onContentChange}
              revision={editorRevision}
            />
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

      {#if !hideChrome && optionsOpen}
        <button class="divider" type="button" aria-label="Ajustar ancho" onmousedown={() => beginDrag("options")}></button>
        <aside class="panel panel-right" style:width="{optionsWidth}px">
          <OptionsPanel
            chapter={selectedChapter}
            scene={currentScene}
            project={selectedProject}
            onupdateprojectmeta={updateProjectMeta}
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
            {backups}
            {backupBusy}
            oncreatebackup={createBackup}
            onrestorebackup={restoreBackupFile}
            ondeletebackup={deleteBackupFile}
            onrefreshbackups={loadBackups}
            {iaStatus}
            {cloudProviders}
            {profiles}
            onapplyprofile={applyProfile}
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
            onupdatecloudenabled={updateCloudEnabled}
            onindexproject={indexProject}
          />
        </aside>
      {/if}
    </div>
  {/if}

  {#if searchOpen && selectedProjectId}
    <SearchPanel
      projectId={selectedProjectId}
      onselect={(chapterId, sceneId) => {
        searchOpen = false;
        void goToSceneInChapter(chapterId, sceneId);
      }}
      onclose={() => (searchOpen = false)}
    />
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
      onupdatecharacter={updateCharacter}
      onupdateentity={updateEntity}
      onclose={() => (worldOpen = false)}
    />
  {/if}

  {#if spellOpen}
    <SpellPanel
      report={spellReport}
      installing={spellBusy}
      oninstall={installSpell}
      onreplace={replaceWord}
      onignore={ignoreWord}
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
    position: relative;
  }

  /* Modo zen: todos los menús ocultos, solo un chip flotante discreto. */
  .zen-pill {
    position: absolute;
    top: 10px;
    right: 14px;
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px;
    border-radius: 999px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    opacity: 0.35;
    transition: opacity 0.18s;
  }

  .zen-pill:hover,
  .zen-pill:focus-within {
    opacity: 1;
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

  .workspace.focus .top-actions > * {
    display: none;
  }

  .workspace.focus .top-actions > .wordcount,
  .workspace.focus .top-actions > .save-status {
    display: inline;
  }

  .workspace.focus .top-actions > .focus-exit {
    display: inline-flex;
  }

  .workspace.focus .crumbs .btn-icon {
    display: none;
  }

  .workspace.focus .topbar {
    background: var(--bg);
    border-bottom-color: transparent;
  }

  .tts-rate {
    max-width: 72px;
  }

  .tts-voice {
    max-width: 120px;
    padding: 4px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text);
    font: inherit;
    font-size: 12px;
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

  .journal-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    background: var(--accent-soft);
    color: var(--text);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }

  .journal-banner span {
    flex: 1;
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
