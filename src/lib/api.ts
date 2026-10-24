import { invoke } from "@tauri-apps/api/core";
import type { Chapter, Project, Scene, SceneMeta } from "./types";

// Proyectos
export const listProjects = () => invoke<Project[]>("list_projects");
export const createProject = (title: string) =>
  invoke<Project>("create_project", { title });
export const renameProject = (id: string, title: string) =>
  invoke<Project>("rename_project", { id, title });
export const deleteProject = (id: string) =>
  invoke<void>("delete_project", { id });

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
export const createScene = (chapterId: string, title: string | null) =>
  invoke<Scene>("create_scene", { chapterId, title });
export const getScene = (id: string) => invoke<Scene>("get_scene", { id });
export const saveScene = (id: string, content: string) =>
  invoke<Scene>("save_scene", { id, content });
export const deleteScene = (id: string) => invoke<void>("delete_scene", { id });
