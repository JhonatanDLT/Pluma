//! Análisis narrativo (Capa 1): detección de personajes y relaciones por reglas.
//!
//! Heurísticas ligeras y sin dependencias de modelos: detección de nombres propios,
//! agrupación de alias por similitud de cadenas, descripción por contexto e
//! inferencia del tipo de relación (familia, romance, rivalidad, amistad).

use std::collections::{HashMap, HashSet};

use strsim::normalized_levenshtein;

use crate::engines::{Embedder, NerEngine};

/// Personaje detectado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedCharacter {
    pub name: String,
    pub aliases: Vec<String>,
    pub role: String,
    pub mention_count: usize,
    pub description: String,
}

/// Relación detectada por co-ocurrencia y marcadores lingüísticos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedRelationship {
    pub a: String,
    pub b: String,
    pub cooccurrences: usize,
    pub kind: Option<String>,
}

/// Entidad no humana detectada (lugar, facción/organización, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedEntity {
    pub name: String,
    /// `location` (ciudades/lugares) o `faction` (facciones/organizaciones).
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub mention_count: usize,
    pub description: String,
}

/// Resultado completo del análisis narrativo.
pub struct AnalysisResult {
    pub characters: Vec<DetectedCharacter>,
    pub relationships: Vec<DetectedRelationship>,
    pub entities: Vec<DetectedEntity>,
}

/// Menciones recolectadas por tipo de entidad.
#[derive(Debug, Default, Clone)]
pub struct Mentions {
    pub person: Vec<(usize, String)>,
    pub location: Vec<(usize, String)>,
    pub faction: Vec<(usize, String)>,
}

/// Analiza un conjunto de textos (escenas) usando únicamente reglas locales.
/// Es el camino por defecto: no requiere descargar ningún modelo.
/// (Se usa principalmente en tests; el comando usa la variante por chunks.)
#[allow(dead_code)]
pub fn analyze<E: Embedder + ?Sized>(texts: &[String], embedder: Option<&mut E>) -> AnalysisResult {
    let mentions = collect_mentions(texts, None, &mut |_, _| {});
    analyze_mentions(texts, &mentions, embedder)
}

/// Recolecta menciones escena por escena (NER + reglas), informando del progreso
/// `(escenas_procesadas, total)` tras cada escena. Permite procesar por chunks sin
/// bloquear y mostrando avance.
pub fn collect_mentions(
    texts: &[String],
    mut ner: Option<&mut dyn NerEngine>,
    on_scene: &mut dyn FnMut(usize, usize),
) -> Mentions {
    let mut mentions = Mentions::default();
    let total = texts.len();

    for (scene_idx, text) in texts.iter().enumerate() {
        if let Some(ner) = ner.as_deref_mut() {
            if let Ok(found) = ner.extract(text) {
                for mention in found {
                    match mention.label.as_str() {
                        "person" => mentions.person.push((scene_idx, mention.text)),
                        "location" => mentions.location.push((scene_idx, mention.text)),
                        "organization" => mentions.faction.push((scene_idx, mention.text)),
                        _ => {}
                    }
                }
            }
        }
        for name in extract_candidates(text) {
            mentions.person.push((scene_idx, name));
        }
        on_scene(scene_idx + 1, total);
    }

    mentions
}

/// Construye el informe (personajes, relaciones y entidades) a partir de las
/// menciones ya recolectadas.
pub fn analyze_mentions<E: Embedder + ?Sized>(
    texts: &[String],
    mentions: &Mentions,
    embedder: Option<&mut E>,
) -> AnalysisResult {
    let (characters, relationships) = build_report(texts, &mentions.person, embedder);

    let mut entities = Vec::new();
    entities.extend(build_entities(texts, &mentions.location, "location"));
    entities.extend(build_entities(texts, &mentions.faction, "faction"));

    AnalysisResult {
        characters,
        relationships,
        entities,
    }
}

/// Construye personajes y relaciones a partir de menciones `(índice de escena, nombre)`.
fn build_report<E: Embedder + ?Sized>(
    texts: &[String],
    mentions: &[(usize, String)],
    embedder: Option<&mut E>,
) -> (Vec<DetectedCharacter>, Vec<DetectedRelationship>) {
    // Frecuencia de cada candidato y escenas en las que aparece.
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut scenes_by_name: HashMap<String, HashSet<usize>> = HashMap::new();

    for (scene_idx, name) in mentions {
        *counts.entry(name.clone()).or_insert(0) += 1;
        scenes_by_name
            .entry(name.clone())
            .or_default()
            .insert(*scene_idx);
    }

    // Filtra por frecuencia mínima (evita ruido).
    let mut names: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(name, _)| name)
        .collect();
    names.sort();

    // Agrupa alias (similitud de cadenas).
    let mut clusters = cluster_aliases(&names);

    // Mejora opcional: fusiona alias por similitud semántica de sus descripciones.
    if let Some(embedder) = embedder {
        clusters = merge_by_semantics(clusters, texts, embedder);
    }

    // Construye personajes (unión de escenas y menciones de todos sus alias).
    let mut characters: Vec<DetectedCharacter> = clusters
        .into_iter()
        .map(|cluster| {
            let mut mention_count = 0usize;
            let mut all_scenes: HashSet<usize> = HashSet::new();
            for name in &cluster {
                mention_count += scenes_by_name.get(name).map(|s| s.len()).unwrap_or(0);
                if let Some(s) = scenes_by_name.get(name) {
                    all_scenes.extend(s.iter().copied());
                }
            }
            let description = extract_description(&cluster, texts);
            DetectedCharacter {
                name: cluster[0].clone(),
                aliases: cluster[1..].to_vec(),
                role: String::new(),
                mention_count,
                description,
            }
        })
        .collect();

    characters.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
    for (i, c) in characters.iter_mut().enumerate() {
        c.role = if i == 0 { "protagonista" } else { "secundario" }.to_string();
    }

    // Relaciones por co-ocurrencia + tipo inferido.
    let mut relationships: Vec<DetectedRelationship> = Vec::new();
    for i in 0..characters.len() {
        for j in (i + 1)..characters.len() {
            let a = &characters[i];
            let b = &characters[j];
            let scenes_a = scenes_by_name.get(&a.name).cloned().unwrap_or_default();
            let scenes_b = scenes_by_name.get(&b.name).cloned().unwrap_or_default();
            let shared: Vec<usize> = scenes_a.intersection(&scenes_b).copied().collect();
            if shared.is_empty() {
                continue;
            }

            let shared_texts: Vec<&str> = shared
                .iter()
                .filter_map(|&idx| texts.get(idx).map(|t| t.as_str()))
                .collect();
            let kind = infer_relationship_type(&shared_texts).map(str::to_string);

            relationships.push(DetectedRelationship {
                a: a.name.clone(),
                b: b.name.clone(),
                cooccurrences: shared.len(),
                kind,
            });
        }
    }
    relationships.sort_by(|a, b| b.cooccurrences.cmp(&a.cooccurrences));

    // Marca como antagonista al personaje con relación de rivalidad con el protagonista.
    if let Some(protagonist) = characters.first().map(|c| c.name.clone()) {
        for rel in &relationships {
            if rel.kind.as_deref() == Some("rivalidad") {
                let other = if rel.a == protagonist {
                    &rel.b
                } else if rel.b == protagonist {
                    &rel.a
                } else {
                    continue;
                };
                if let Some(c) = characters.iter_mut().find(|c| &c.name == other) {
                    if c.role == "secundario" {
                        c.role = "antagonista".to_string();
                    }
                }
            }
        }
    }

    (characters, relationships)
}

/// Construye entidades (lugares/facciones) a partir de menciones, reutilizando la
/// agrupación de alias y la extracción de descripción de los personajes.
fn build_entities(
    texts: &[String],
    mentions: &[(usize, String)],
    entity_type: &str,
) -> Vec<DetectedEntity> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut scenes_by_name: HashMap<String, HashSet<usize>> = HashMap::new();
    for (scene_idx, name) in mentions {
        *counts.entry(name.clone()).or_insert(0) += 1;
        scenes_by_name
            .entry(name.clone())
            .or_default()
            .insert(*scene_idx);
    }

    let mut names: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(name, _)| name)
        .collect();
    names.sort();

    let mut entities: Vec<DetectedEntity> = cluster_aliases(&names)
        .into_iter()
        .map(|cluster| {
            let mention_count = cluster
                .iter()
                .map(|name| scenes_by_name.get(name).map(|s| s.len()).unwrap_or(0))
                .sum();
            let description = extract_description(&cluster, texts);
            DetectedEntity {
                name: cluster[0].clone(),
                entity_type: entity_type.to_string(),
                aliases: cluster[1..].to_vec(),
                mention_count,
                description,
            }
        })
        .collect();

    entities.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
    entities
}

/// Extrae candidatos a nombre propio (palabra en mayúscula, no a inicio de frase).
fn extract_candidates(text: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "el",
        "la",
        "los",
        "las",
        "un",
        "una",
        "unos",
        "unas",
        "de",
        "del",
        "y",
        "e",
        "o",
        "u",
        "a",
        "al",
        "en",
        "por",
        "para",
        "con",
        "sin",
        "sobre",
        "entre",
        "hasta",
        "desde",
        "que",
        "se",
        "su",
        "sus",
        "mi",
        "mis",
        "tu",
        "tus",
        "le",
        "les",
        "lo",
        "es",
        "son",
        "fue",
        "era",
        "no",
        "si",
        "ya",
        "muy",
        "más",
        "mas",
        "pero",
        "como",
        "cuando",
        "donde",
        "todo",
        "todos",
        "este",
        "esta",
        "ese",
        "esa",
        "aquel",
        "usted",
        "señor",
        "señora",
        "don",
        "doña",
        "ni",
        "tan",
        "ha",
        "han",
        "he",
        "hemos",
        "había",
        "está",
        "estaba",
        "ser",
        "estar",
        "me",
        "te",
        "nos",
        "yo",
        "tú",
        "él",
        "ella",
        "ellos",
        "ellas",
        "señorita",
        "doctor",
        "doctora",
        "capítulo",
        "capitulo",
        "escena",
        "parte",
        "prólogo",
        "prologo",
        "epílogo",
        "epilogo",
    ];

    let mut result = Vec::new();
    let mut sentence_start = true;

    for raw in text.split_whitespace() {
        let word = raw.trim_matches(|c: char| !c.is_alphabetic());
        // Los guiones de diálogo (`—`, `–`) no marcan fin de frase: permiten
        // detectar al hablante ("—dijo Ana—").
        let ends_sentence = raw
            .chars()
            .last()
            .map(|c| matches!(c, '.' | '!' | '?' | ':' | ';'))
            .unwrap_or(false);

        if word.chars().count() >= 2 {
            let first = word.chars().next().unwrap();
            if first.is_uppercase() && !sentence_start {
                let lower = word.to_lowercase();
                if !STOPWORDS.contains(&lower.as_str()) {
                    result.push(word.to_string());
                }
            }
        }

        sentence_start = ends_sentence;
    }

    result
}

/// Agrupa nombres que se refieren a la misma persona.
fn cluster_aliases(names: &[String]) -> Vec<Vec<String>> {
    let mut clusters: Vec<Vec<String>> = Vec::new();

    for name in names {
        let mut matched = false;
        for cluster in clusters.iter_mut() {
            let similar = cluster
                .iter()
                .any(|existing| is_same_character(existing, name));
            if similar {
                cluster.push(name.clone());
                matched = true;
                break;
            }
        }
        if !matched {
            clusters.push(vec![name.clone()]);
        }
    }

    for cluster in clusters.iter_mut() {
        cluster.sort();
    }
    clusters.sort_by(|a, b| a[0].cmp(&b[0]));
    clusters
}

fn is_same_character(a: &str, b: &str) -> bool {
    let first_token = |s: &str| s.split_whitespace().next().unwrap_or(s).to_lowercase();
    let same_first = first_token(a) == first_token(b);
    let contains = a.to_lowercase().contains(&b.to_lowercase())
        || b.to_lowercase().contains(&a.to_lowercase());
    let similar = normalized_levenshtein(&a.to_lowercase(), &b.to_lowercase()) > 0.75;
    same_first && (contains || similar)
}

/// Fusiona grupos cuyas descripciones son semánticamente similares (embeddings).
fn merge_by_semantics<E: Embedder + ?Sized>(
    clusters: Vec<Vec<String>>,
    texts: &[String],
    embedder: &mut E,
) -> Vec<Vec<String>> {
    if clusters.len() <= 1 {
        return clusters;
    }

    let descriptions: Vec<String> = clusters
        .iter()
        .map(|c| extract_description(c, texts))
        .collect();

    let embeddings = match embedder.embed(&descriptions) {
        Ok(e) => e,
        Err(_) => return clusters,
    };

    let mut merged: Vec<Vec<String>> = Vec::new();
    let mut skip = vec![false; clusters.len()];
    for i in 0..clusters.len() {
        if skip[i] {
            continue;
        }
        let mut group = clusters[i].clone();
        for j in (i + 1)..clusters.len() {
            if skip[j] || descriptions[i].is_empty() || descriptions[j].is_empty() {
                continue;
            }
            if crate::engines::cosine(&embeddings[i], &embeddings[j]) > 0.9 {
                group.extend(clusters[j].clone());
                skip[j] = true;
            }
        }
        merged.push(group);
    }

    for group in merged.iter_mut() {
        group.sort();
    }
    merged
}

/// Devuelve la primera frase que menciona a cualquiera de los alias.
fn extract_description(names: &[String], texts: &[String]) -> String {
    for text in texts {
        for sentence in split_sentences(text) {
            let lower = sentence.to_lowercase();
            if names.iter().any(|n| lower.contains(&n.to_lowercase())) {
                let s = sentence.trim();
                if !s.is_empty() {
                    return s.chars().take(220).collect();
                }
            }
        }
    }
    String::new()
}

fn split_sentences(text: &str) -> Vec<&str> {
    text.split(|c| matches!(c, '.' | '!' | '?'))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Infiere el tipo de relación a partir de marcadores lingüísticos.
fn infer_relationship_type(shared_texts: &[&str]) -> Option<&'static str> {
    const FAMILY: &[&str] = &[
        "hermano",
        "hermana",
        "padre",
        "madre",
        "hijo",
        "hija",
        "tío",
        "tía",
        "primo",
        "prima",
        "abuelo",
        "abuela",
        "sobrino",
        "sobrina",
        "madrastra",
        "padrastro",
    ];
    const ROMANCE: &[&str] = &[
        "amaba",
        "amor",
        "enamorado",
        "enamorada",
        "beso",
        "besó",
        "pareja",
        "esposo",
        "esposa",
        "novio",
        "novia",
        "amantes",
    ];
    const RIVALRY: &[&str] = &[
        "odiaba",
        "odio",
        "enemigo",
        "enemiga",
        "rival",
        "enfrentó",
        "traicionó",
        "venganza",
        "desprecio",
    ];
    const FRIENDSHIP: &[&str] = &["amigo", "amiga", "amistad", "compañero", "compañera"];

    for text in shared_texts {
        let lower = text.to_lowercase();
        if FAMILY.iter().any(|&m| lower.contains(m)) {
            return Some("familia");
        }
        if ROMANCE.iter().any(|&m| lower.contains(m)) {
            return Some("romance");
        }
        if RIVALRY.iter().any(|&m| lower.contains(m)) {
            return Some("rivalidad");
        }
        if FRIENDSHIP.iter().any(|&m| lower.contains(m)) {
            return Some("amistad");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_capitalized_names() {
        let text = "Ana miró a Pedro, y Juan habló con María.";
        let candidates = extract_candidates(text);
        assert!(candidates.contains(&"Pedro".to_string()));
        assert!(candidates.contains(&"Juan".to_string()));
        assert!(candidates.contains(&"María".to_string()));
    }

    #[test]
    fn detects_dialogue_speakers() {
        let text = "—No lo sé —dijo Ana—, pero Pedro sí lo sabía.";
        let candidates = extract_candidates(text);
        assert!(candidates.contains(&"Ana".to_string()));
        assert!(candidates.contains(&"Pedro".to_string()));
    }

    #[test]
    fn clusters_aliases() {
        let names = vec![
            "Ana".to_string(),
            "Ana Pérez".to_string(),
            "Pedro".to_string(),
        ];
        let clusters = cluster_aliases(&names);
        assert_eq!(clusters.len(), 2);
        let ana = clusters.iter().find(|c| c[0] == "Ana").unwrap();
        assert_eq!(ana.len(), 2);
    }

    #[test]
    fn analysis_detects_relationships() {
        let texts = vec![
            "La casa de Ana estaba cerca de Pedro.".to_string(),
            "La casa de Pedro estaba cerca de Ana.".to_string(),
        ];
        let result = analyze::<crate::engines::FastEmbedder>(&texts, None);
        assert!(result.characters.iter().any(|c| c.name == "Ana"));
        assert!(result.characters.iter().any(|c| c.name == "Pedro"));
        assert!(
            result
                .relationships
                .iter()
                .any(|r| { (r.a == "Ana" && r.b == "Pedro") || (r.a == "Pedro" && r.b == "Ana") })
        );
    }

    #[test]
    fn infers_relationship_type() {
        let shared = vec!["Ana era la hermana de Pedro."];
        assert_eq!(infer_relationship_type(&shared), Some("familia"));

        let shared = vec!["Pedro amaba a Ana en secreto."];
        assert_eq!(infer_relationship_type(&shared), Some("romance"));
    }

    #[test]
    fn builds_entities_from_mentions() {
        let texts = vec![
            "Viajaron a Roma y se quedaron en Roma.".to_string(),
            "Roma era hermosa en primavera.".to_string(),
        ];
        let mentions = vec![
            (0, "Roma".to_string()),
            (0, "Roma".to_string()),
            (1, "Roma".to_string()),
        ];
        let entities = build_entities(&texts, &mentions, "location");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Roma");
        assert_eq!(entities[0].entity_type, "location");
    }
}
