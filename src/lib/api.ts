import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisReport,
  Chapter,
  Character,
  Entity,
  IaSettings,
  IaStatus,
  LanguageInfo,
  NavEntry,
  Project,
  Relationship,
  Scene,
  SceneMeta,
  SnapshotMeta,
  SpellReport,
} from "./types";

// Proyectos
export const listProjects = () => invoke<Project[]>("list_projects");
export const createProject = (
  title: string,
  goal: number | null,
  description: string | null,
  coverPath: string | null,
) => invoke<Project>("create_project", { title, goal, description, coverPath });
export const renameProject = (id: string, title: string) =>
  invoke<Project>("rename_project", { id, title });
export const deleteProject = (id: string) =>
  invoke<void>("delete_project", { id });
export const updateProjectGoal = (id: string, goal: number) =>
  invoke<Project>("update_project_goal", { id, goal });
export const updateProjectLevels = (
  id: string,
  level1Name: string,
  level2Name: string,
) => invoke<Project>("update_project_levels", { id, level1Name, level2Name });

// Capítulos
export const listChapters = (projectId: string) =>
  invoke<Chapter[]>("list_chapters", { projectId });
export const createChapter = (projectId: string, title: string) =>
  invoke<Chapter>("create_chapter", { projectId, title });
export const renameChapter = (id: string, title: string) =>
  invoke<Chapter>("rename_chapter", { id, title });
export const deleteChapter = (id: string) =>
  invoke<void>("delete_chapter", { id });

// Escenas
export const listScenes = (chapterId: string) =>
  invoke<SceneMeta[]>("list_scenes", { chapterId });
export const listNavigation = (projectId: string) =>
  invoke<NavEntry[]>("list_navigation", { projectId });
export const createScene = (chapterId: string, title: string | null) =>
  invoke<Scene>("create_scene", { chapterId, title });
export const getScene = (id: string) => invoke<Scene>("get_scene", { id });
export const saveScene = (id: string, content: string) =>
  invoke<Scene>("save_scene", { id, content });
export const deleteScene = (id: string) => invoke<void>("delete_scene", { id });

// Importación de manuscritos
export const importMarkdown = (projectId: string, markdown: string) =>
  invoke<Chapter[]>("import_markdown", { projectId, markdown });
export const importFile = (projectId: string, path: string) =>
  invoke<Chapter[]>("import_file", { projectId, path });
export const importFileAsProject = (
  path: string,
  goal: number | null,
  description: string | null,
  coverPath: string | null,
) => invoke<Project>("import_file_as_project", { path, goal, description, coverPath });

// Metadatos
export const updateChapterMeta = (
  id: string,
  status: string,
  summary: string | null,
) => invoke<Chapter>("update_chapter_meta", { id, status, summary });

export const updateSceneMeta = (
  id: string,
  status: string,
  pov: string | null,
  location: string | null,
  timeLabel: string | null,
  summary: string | null,
) =>
  invoke<Scene>("update_scene_meta", {
    id,
    status,
    pov,
    location,
    timeLabel,
    summary,
  });

// Reordenamiento
export const moveChapter = (
  projectId: string,
  id: string,
  newPosition: number,
) => invoke<Chapter[]>("move_chapter", { projectId, id, newPosition });

export const moveScene = (
  chapterId: string,
  id: string,
  newPosition: number,
) => invoke<SceneMeta[]>("move_scene", { chapterId, id, newPosition });

// Exportación
export const exportProject = (
  projectId: string,
  format: string,
  path: string,
) => invoke<void>("export_project", { projectId, format, path });

// Personajes y relaciones (IA)
export const listCharacters = (projectId: string) =>
  invoke<Character[]>("list_characters", { projectId });
export const listRelationships = (projectId: string) =>
  invoke<Relationship[]>("list_relationships", { projectId });
export const listEntities = (projectId: string) =>
  invoke<Entity[]>("list_entities", { projectId });
export const deleteCharacter = (id: string) =>
  invoke<void>("delete_character", { id });
export const deleteEntity = (id: string) =>
  invoke<void>("delete_entity", { id });
export const analyzeProject = (projectId: string, chapterId: string | null) =>
  invoke<AnalysisReport>("analyze_project", { projectId, chapterId });
export const enrichProject = (projectId: string, model: string | null) =>
  invoke<Character[]>("enrich_project", { projectId, model });
export const correctText = (text: string, model: string | null) =>
  invoke<string>("correct_text", { text, model });
export const chat = (message: string, model: string | null) =>
  invoke<string>("chat", { message, model });

// Ortografía (Hunspell, sin LLM)
export const spellCheck = (text: string) =>
  invoke<SpellReport>("spell_check", { text });
export const spellInstall = (code: string) =>
  invoke<void>("spell_install", { code });
export const spellLanguages = () =>
  invoke<LanguageInfo[]>("spell_languages");

// Versiones (snapshots)
export const createSnapshot = (projectId: string, message: string | null) =>
  invoke<SnapshotMeta>("create_snapshot", { projectId, message });
export const listSnapshots = (projectId: string) =>
  invoke<SnapshotMeta[]>("list_snapshots", { projectId });
export const restoreSnapshot = (snapshotId: string) =>
  invoke<void>("restore_snapshot", { snapshotId });
export const deleteSnapshot = (snapshotId: string) =>
  invoke<void>("delete_snapshot", { snapshotId });

// Configuración de IA (opt-in)
export const iaStatus = () => invoke<IaStatus>("ia_status");
export const iaSetSettings = (settings: IaSettings) =>
  invoke<IaSettings>("ia_set_settings", { settings });
export const iaInstall = (engine: string) => invoke<void>("ia_install", { engine });

// Texto a voz
export const ttsSpeak = (text: string, voice: string | null, rate: number) =>
  invoke<void>("tts_speak", { text, voice, rate });
export const ttsStop = () => invoke<void>("tts_stop");
