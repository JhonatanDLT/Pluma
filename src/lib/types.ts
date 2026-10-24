export interface Project {
  id: string;
  title: string;
  author: string | null;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface Chapter {
  id: string;
  project_id: string;
  position: number;
  title: string;
  status: string;
  summary: string | null;
  word_count: number;
}

export interface SceneMeta {
  id: string;
  chapter_id: string;
  position: number;
  title: string | null;
  word_count: number;
  updated_at: string;
}

export interface Scene {
  id: string;
  chapter_id: string;
  position: number;
  title: string | null;
  content_markdown: string;
  summary: string | null;
  word_count: number;
  created_at: string;
  updated_at: string;
}

export type EditorMode = "wysiwyg" | "source";
export type SaveState = "idle" | "saving" | "saved" | "error";
