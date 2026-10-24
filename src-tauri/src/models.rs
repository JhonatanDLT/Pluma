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
            word_count: 0,
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}
