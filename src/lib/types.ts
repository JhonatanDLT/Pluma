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

export type FieldType = "text" | "longtext" | "list" | "number";

export interface CharacterFieldDef {
  id: string;
  project_id: string;
  key: string;
  label: string;
  field_type: FieldType;
  options: string[];
  prompt_hint: string | null;
  position: number;
  builtin: boolean;
  created_at: string;
  updated_at: string;
}

export interface CharacterFieldView {
  field_id: string;
  key: string;
  label: string;
  field_type: FieldType;
  options: string[];
  prompt_hint: string | null;
  builtin: boolean;
  value: string | string[] | number | null;
  source: "manual" | "ai" | "none";
  confidence: number | null;
  evidence: string | null;
  reason: string | null;
}

export interface EnrichReport {
  characters: number;
  suggested: number;
  insufficient: number;
  skipped: number;
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

export interface Affiliation {
  id: string;
  project_id: string;
  character_id: string;
  character_name: string;
  entity_id: string;
  entity_name: string;
  entity_type: string;
  role: string;
  source: "manual" | "ai";
  confidence: number | null;
  evidence: string | null;
  evidence_scene_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface EntityRelation {
  id: string;
  project_id: string;
  a_entity_id: string;
  a_name: string;
  a_type: string;
  b_entity_id: string;
  b_name: string;
  b_type: string;
  kind: "alliance" | "conflict" | "territory";
  source: "manual" | "ai";
  confidence: number | null;
  evidence: string | null;
  evidence_scene_id: string | null;
  created_at: string;
  updated_at: string;
}

export type GraphNodeKind = "character" | "faction" | "location";
export type GraphEdgeKind =
  | "relationship"
  | "affiliation"
  | "presence"
  | "territory"
  | "alliance"
  | "conflict";

export interface GraphNode {
  id: string;
  kind: GraphNodeKind;
  label: string;
  mentions: number;
  role: string | null;
  faction_ids: string[];
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  kind: GraphEdgeKind;
  subtype: string | null;
  weight: number;
  origin: "manual" | "ai" | "derived";
  evidence: string | null;
  evidence_scene_id: string | null;
}

export interface GraphStats {
  characters: number;
  factions: number;
  locations: number;
  relationships: number;
  affiliations: number;
  presence: number;
  entity_relations: number;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
  stats: GraphStats;
}

export interface WorldLinkReport {
  characters: number;
  proposed: number;
  skipped: number;
  insufficient: number;
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

export interface BackupInfo {
  file: string;
  size: number;
  created_at: string;
  kind: "auto" | "manual";
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
