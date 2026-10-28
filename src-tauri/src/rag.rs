//! Retrieval-Augmented Generation: indexa las escenas de un proyecto por
//! similitud semántica y recupera las más relevantes para dar contexto al chat.

use rusqlite::Connection;

use crate::engines::Embedder;
use crate::error::Result;

/// Número de escenas de contexto que se inyectan en el chat.
pub const TOP_K: usize = 3;

/// Reindexa las escenas de un proyecto: embebe cada una y guarda el vector.
/// Devuelve el número de escenas indexadas.
pub fn index_project<E: Embedder>(
    conn: &Connection,
    project_id: &str,
    scenes: &[(String, String)],
    embedder: &mut E,
) -> Result<usize> {
    crate::repo::clear_embeddings(conn, project_id)?;

    let texts: Vec<String> = scenes.iter().map(|(_, c)| c.clone()).collect();
    let embeddings = embedder.embed(&texts)?;

    for ((scene_id, _), embedding) in scenes.iter().zip(embeddings.iter()) {
        crate::repo::save_embedding(conn, project_id, scene_id, embedding)?;
    }

    Ok(scenes.len())
}

/// Recupera los textos de las top-k escenas más similares a `query`.
pub fn retrieve<E: Embedder>(
    conn: &Connection,
    project_id: &str,
    query: &str,
    top_k: usize,
    embedder: &mut E,
) -> Result<Vec<String>> {
    let stored = crate::repo::list_embeddings(conn, project_id)?;
    if stored.is_empty() {
        return Ok(Vec::new());
    }

    let query_embedding = embedder
        .embed(&[query.to_string()])?
        .into_iter()
        .next()
        .unwrap_or_default();

    let mut texts = Vec::new();
    for (_, scene_id) in rank(&stored, &query_embedding).into_iter().take(top_k) {
        if let Some(scene) = crate::repo::get_scene(conn, &scene_id)?
            && !scene.content_markdown.trim().is_empty()
        {
            texts.push(scene.content_markdown);
        }
    }
    Ok(texts)
}

/// Ordena `(scene_id, embedding)` por similitud coseno con `query`, de mayor a menor.
fn rank(stored: &[(String, Vec<f32>)], query: &[f32]) -> Vec<(f32, String)> {
    let mut scored: Vec<(f32, String)> = stored
        .iter()
        .map(|(scene_id, emb)| (crate::engines::cosine(query, emb), scene_id.clone()))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Embebedor de prueba: vector fijo por texto, para validar la recuperación
    /// sin depender de un modelo real.
    struct FakeEmbedder;

    impl Embedder for FakeEmbedder {
        fn embed(&mut self, texts: &[String]) -> crate::error::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .map(|t| {
                    if t == "alpha" || t == "alfa" {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect())
        }
    }

    #[test]
    fn ranks_by_cosine_similarity() {
        let stored = vec![
            ("a".to_string(), vec![1.0, 0.0]),
            ("b".to_string(), vec![0.0, 1.0]),
        ];
        let ranked = rank(&stored, &[1.0, 0.0]);
        assert_eq!(ranked[0].1, "a");
        assert_eq!(ranked[1].1, "b");
    }

    #[test]
    fn indexes_and_lists_embeddings() {
        let conn = crate::db::test_connection();
        let scenes = vec![
            ("a".to_string(), "alpha".to_string()),
            ("b".to_string(), "beta".to_string()),
        ];
        let mut embedder = FakeEmbedder;
        index_project(&conn, "p1", &scenes, &mut embedder).unwrap();

        let listed = crate::repo::list_embeddings(&conn, "p1").unwrap();
        assert_eq!(listed.len(), 2);
    }
}
