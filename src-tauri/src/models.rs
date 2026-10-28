use serde::{Deserialize, Serialize};

/// Marca de tiempo ISO-8601 (RFC 3339) generada en Rust.
///
/// Se almacena como texto: es ordenable lexicográficamente y parseable por el
/// frontend con `new Date(...)`.
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub word_goal: i64,
    pub cover_path: Option<String>,
    pub level1_name: String,
    pub level2_name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn new(title: impl Into<String>) -> Self {
        let ts = now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            author: None,
            description: None,
            word_goal: 3000,
            cover_path: None,
            level1_name: "Capítulos".to_string(),
            level2_name: "Escenas".to_string(),
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub project_id: String,
    pub position: i64,
    pub title: String,
    pub status: String,
    pub summary: Option<String>,
    pub word_count: i64,
}

impl Chapter {
    pub fn new(project_id: impl Into<String>, title: impl Into<String>, position: i64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            position,
            title: title.into(),
            status: "draft".into(),
            summary: None,
            word_count: 0,
        }
    }
}

/// Vista ligera de una escena (sin `content_markdown`), usada en listados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMeta {
    pub id: String,
    pub chapter_id: String,
    pub position: i64,
    pub title: Option<String>,
    pub word_count: i64,
    pub updated_at: String,
}

/// Escena completa, incluyendo el contenido Markdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub chapter_id: String,
    pub position: i64,
    pub title: Option<String>,
    pub content_markdown: String,
    pub summary: Option<String>,
    pub status: String,
    pub pov: Option<String>,
    pub location: Option<String>,
    pub time_label: Option<String>,
    pub word_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Scene {
    pub fn new(chapter_id: impl Into<String>, position: i64, title: Option<String>) -> Self {
        let ts = now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            chapter_id: chapter_id.into(),
            position,
            title,
            content_markdown: String::new(),
            summary: None,
            status: "draft".into(),
            pov: None,
            location: None,
            time_label: None,
            word_count: 0,
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}

/// Personaje de un proyecto, generado/detectado por el análisis de IA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub biography: Option<String>,
    pub traits: Vec<String>,
    pub goals: Vec<String>,
    pub arc: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Character {
    pub fn new(project_id: impl Into<String>, name: impl Into<String>) -> Self {
        let ts = now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            name: name.into(),
            aliases: Vec::new(),
            role: None,
            description: None,
            biography: None,
            traits: Vec::new(),
            goals: Vec::new(),
            arc: None,
            color: None,
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}

/// Relación entre dos personajes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub project_id: String,
    pub character_a_id: String,
    pub character_b_id: String,
    pub kind: Option<String>,
    pub intensity: f64,
    pub description: Option<String>,
}

/// Entidad no humana detectada por el análisis (lugar, facción, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// `location` (ciudades/lugares) o `faction` (facciones/organizaciones).
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub mention_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Entity {
    pub fn new(project_id: impl Into<String>, name: impl Into<String>, entity_type: &str) -> Self {
        let ts = now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            name: name.into(),
            entity_type: entity_type.to_string(),
            aliases: Vec::new(),
            description: None,
            mention_count: 0,
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}

/// Resultado del análisis narrativo (personajes, relaciones y entidades).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub characters: Vec<Character>,
    pub relationships: Vec<Relationship>,
    pub entities: Vec<Entity>,
}

/// Metadatos de una versión (snapshot) del proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub id: String,
    pub project_id: String,
    pub message: Option<String>,
    pub scene_count: i64,
    pub created_at: String,
}

/// Entrada plana del índice de navegación (capítulo o escena), para el
/// conmutador rápido y la navegación por teclado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavEntry {
    pub chapter_id: String,
    pub chapter_title: String,
    /// `None` cuando la entrada representa un capítulo sin escena asociada.
    pub scene_id: Option<String>,
    pub scene_title: Option<String>,
}

/// Resultado de una búsqueda full-text en las escenas de un proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub scene_id: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub scene_title: Option<String>,
    /// Fragmento con las coincidencias marcadas con `<mark>…</mark>`.
    pub snippet: String,
}
