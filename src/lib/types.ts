export interface Project {
  id: string;
  title: string;
  author: string | null;
  description: string | null;
  word_goal: number;
  cover_path: string | null;
  level1_name: string;
  level2_name: string;
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
  status: string;
  pov: string | null;
  location: string | null;
  time_label: string | null;
  word_count: number;
  created_at: string;
  updated_at: string;
}

export interface Character {
  id: string;
  project_id: string;
  name: string;
  aliases: string[];
  role: string | null;
  description: string | null;
  biography: string | null;
  traits: string[];
  goals: string[];
  arc: string | null;
  color: string | null;
  created_at: string;
  updated_at: string;
}

export interface Relationship {
  id: string;
  project_id: string;
  character_a_id: string;
  character_b_id: string;
  kind: string | null;
  intensity: number;
  description: string | null;
}

export interface AnalysisReport {
  characters: Character[];
  relationships: Relationship[];
  entities: Entity[];
}

export interface SnapshotMeta {
  id: string;
  project_id: string;
  message: string | null;
  scene_count: number;
  created_at: string;
}

export interface NavEntry {
  chapter_id: string;
  chapter_title: string;
  scene_id: string | null;
  scene_title: string | null;
}

export interface Entity {
  id: string;
  project_id: string;
  name: string;
  entity_type: string;
  aliases: string[];
  description: string | null;
  mention_count: number;
  created_at: string;
  updated_at: string;
}

export interface SearchHit {
  scene_id: string;
  chapter_id: string;
  chapter_title: string;
  scene_title: string | null;
  snippet: string;
}

export interface IaSettings {
  ner_engine: string;
  embeddings: boolean;
  llm_model: string | null;
  relationship_model: string | null;
  cloud_provider: string | null;
  cloud_model: string | null;
  cloud_ai_enabled: boolean;
  keep_loaded: boolean;
  profile: string | null;
}

export interface IaStatus {
  settings: IaSettings;
  embeddings_cached: boolean;
  gliner_cached: boolean;
  onnxruntime_available: boolean;
  ollama_available: boolean;
  cloud_key_set: boolean;
}

export interface CloudProviderInfo {
  id: string;
  label: string;
  default_model: string;
}

export interface ProfileInfo {
  id: string;
  label: string;
  description: string;
  min_ram: string;
  min_cpu: string;
  min_disk: string;
  gpu: string;
}

export type EditorMode = "wysiwyg" | "source";
export type SaveState = "idle" | "saving" | "saved" | "error";

export interface ChangeEntry {
  time: string;
  summary: string;
}

export interface SpellIssue {
  word: string;
  suggestions: string[];
}

export interface SpellReport {
  language: string;
  language_name: string;
  installed: boolean;
  issues: SpellIssue[];
}

export interface LanguageInfo {
  code: string;
  name: string;
  installed: boolean;
}
