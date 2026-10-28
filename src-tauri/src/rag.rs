//! Retrieval-Augmented Generation: indexa las escenas de un proyecto por
//! similitud semántica y recupera las más relevantes para dar contexto al chat.

use crate::engines::Embedder;
use crate::error::Result;

/// Número de escenas de contexto que se inyectan en el chat.
pub const TOP_K: usize = 3;

/// Embebe las escenas **sin tocar la base de datos**. El llamador puede soltar
/// el lock de SQLite mientras se ejecuta el modelo (la parte lenta) y persistir
/// el resultado después con `repo::replace_embeddings`.
pub fn embed_scenes<E: Embedder>(
    scenes: &[(String, String)],
    embedder: &mut E,
) -> Result<Vec<(String, Vec<f32>)>> {
    let texts: Vec<String> = scenes.iter().map(|(_, c)| c.clone()).collect();
    let embeddings = embedder.embed(&texts)?;
    Ok(scenes
        .iter()
        .map(|(scene_id, _)| scene_id.clone())
        .zip(embeddings)
        .collect())
}

/// Devuelve los IDs de las `top_k` escenas más similares a `query_embedding`.
/// No toca la base de datos: permite embeber la consulta con el lock liberado.
pub fn top_scene_ids(
    stored: &[(String, Vec<f32>)],
    query_embedding: &[f32],
    top_k: usize,
) -> Vec<String> {
    rank(stored, query_embedding)
        .into_iter()
        .take(top_k)
        .map(|(_, scene_id)| scene_id)
        .collect()
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
        let embeddings = embed_scenes(&scenes, &mut embedder).unwrap();
        crate::repo::replace_embeddings(&conn, "p1", &embeddings).unwrap();

        let listed = crate::repo::list_embeddings(&conn, "p1").unwrap();
        assert_eq!(listed.len(), 2);
    }
}
