//! Análisis narrativo (Capa 1): detección de personajes y relaciones por reglas.
//!
//! Heurísticas ligeras y sin dependencias de modelos: detección de nombres propios,
//! agrupación de alias por similitud de cadenas, descripción por contexto e
//! inferencia del tipo de relación (familia, romance, rivalidad, amistad).

use std::collections::{HashMap, HashSet};

use strsim::normalized_levenshtein;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

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
        match ner.as_deref_mut() {
            // Con NER activo (GLiNER2) no se ejecutan las reglas locales: evita
            // menciones duplicadas que fragmentan a un mismo personaje en varios
            // ("Ana María" vs "Ana" + "María") y reduce el trabajo a la mitad.
            Some(ner) => {
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
            // Sin NER: las reglas locales extraen candidatos a nombre propio.
            None => {
                for name in extract_candidates(text) {
                    mentions.person.push((scene_idx, name));
                }
            }
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
    // Frecuencia de menciones y escenas por cada forma superficial del nombre.
    let mut mention_counts: HashMap<String, usize> = HashMap::new();
    let mut scenes_by_name: HashMap<String, HashSet<usize>> = HashMap::new();

    for (scene_idx, name) in mentions {
        *mention_counts.entry(name.clone()).or_insert(0) += 1;
        scenes_by_name
            .entry(name.clone())
            .or_default()
            .insert(*scene_idx);
    }

    // Filtra por frecuencia mínima (evita ruido).
    let mut names: Vec<String> = mention_counts
        .iter()
        .filter(|(_, count)| **count >= 2)
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();

    // Agrupa alias (similitud de cadenas) y fusiona por semántica si hay embedder.
    let mut clusters = cluster_aliases(&names);
    if let Some(embedder) = embedder {
        clusters = merge_by_semantics(clusters, texts, embedder);
    }

    // Construye personajes. El nombre canónico es el alias más frecuente (no el
    // primero alfabéticamente), y sus escenas son la unión de las de todos sus alias.
    let mut characters: Vec<DetectedCharacter> = Vec::new();
    let mut scenes_by_character: HashMap<String, HashSet<usize>> = HashMap::new();
    for cluster in clusters {
        // Nombre canónico: el más completo (más tokens significativos) y, a
        // igualdad, el más frecuente. Así «Klein Moretti» gana a «Klein».
        let mut sorted = cluster.clone();
        sorted.sort_by(|a, b| {
            let ta = significant_tokens(a).len();
            let tb = significant_tokens(b).len();
            tb.cmp(&ta).then_with(|| {
                mention_counts
                    .get(b)
                    .unwrap_or(&0)
                    .cmp(mention_counts.get(a).unwrap_or(&0))
            })
        });
        let canonical = sorted[0].clone();
        let aliases: Vec<String> = sorted[1..].to_vec();

        let mut mention_count = 0usize;
        let mut all_scenes: HashSet<usize> = HashSet::new();
        for name in &cluster {
            mention_count += mention_counts.get(name).copied().unwrap_or(0);
            if let Some(s) = scenes_by_name.get(name) {
                all_scenes.extend(s.iter().copied());
            }
        }
        let description = extract_description(&cluster, texts);

        scenes_by_character.insert(canonical.clone(), all_scenes);
        characters.push(DetectedCharacter {
            name: canonical,
            aliases,
            role: String::new(),
            mention_count,
            description,
        });
    }

    characters.sort_by_key(|a| std::cmp::Reverse(a.mention_count));
    for (i, c) in characters.iter_mut().enumerate() {
        c.role = if i == 0 { "protagonista" } else { "secundario" }.to_string();
    }

    // Relaciones por co-ocurrencia + tipo inferido a nivel de oración.
    let mut relationships: Vec<DetectedRelationship> = Vec::new();
    for i in 0..characters.len() {
        for j in (i + 1)..characters.len() {
            let a = &characters[i];
            let b = &characters[j];
            let scenes_a = scenes_by_character
                .get(&a.name)
                .cloned()
                .unwrap_or_default();
            let scenes_b = scenes_by_character
                .get(&b.name)
                .cloned()
                .unwrap_or_default();
            let shared: Vec<usize> = scenes_a.intersection(&scenes_b).copied().collect();
            if shared.is_empty() {
                continue;
            }

            // El tipo de relación (familia, romance, …) es lingüístico y requiere
            // un modelo multilingüe. Las reglas agnósticas solo detectan
            // co-ocurrencia: se emite la relación si comparten 2+ escenas.
            if shared.len() < 2 {
                continue;
            }

            relationships.push(DetectedRelationship {
                a: a.name.clone(),
                b: b.name.clone(),
                cooccurrences: shared.len(),
                kind: None,
            });
        }
    }
    relationships.sort_by_key(|a| std::cmp::Reverse(a.cooccurrences));

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

    entities.sort_by_key(|a| std::cmp::Reverse(a.mention_count));
    entities
}

/// Extrae candidatos a nombre propio (palabra en mayúscula, no a inicio de frase).
/// Extrae candidatos a nombre propio de forma **agnóstica al idioma**:
/// secuencias de palabras con inicial mayúscula (title-case) en mitad de frase.
/// Sin diccionarios: distinguir persona de lugar u organización corresponde a
/// los modelos multilingües (GLiNER2).
fn extract_candidates(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut sentence_start = true;

    for raw in text.split_whitespace() {
        let word = raw.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let ends_sentence = raw
            .chars()
            .last()
            .map(|c| matches!(c, '.' | '!' | '?' | ':' | ';'))
            .unwrap_or(false);
        // Puntuación que cierra un nombre dentro de una frase. La coma separa
        // «Ana, Pedro» pero no rompe «Zhou Mingrui, que…» porque el token
        // completo es «Mingrui,».
        let closes = raw
            .chars()
            .last()
            .map(|c| matches!(c, ',' | '.' | '!' | '?' | ':' | ';'))
            .unwrap_or(false);

        // Token de nombre: inicial mayúscula, no todo en mayúsculas (descarta
        // acrónimos) y no al inicio de frase (evita verbos y conectores).
        let is_name_token = !sentence_start
            && word.chars().count() >= 2
            && word.chars().next().is_some_and(|c| c.is_uppercase())
            && !word.chars().all(|c| c.is_uppercase());

        if is_name_token {
            current.push(word);
        }
        if (!is_name_token || closes) && !current.is_empty() {
            result.push(current.join(" "));
            current.clear();
        }

        sentence_start = ends_sentence;
    }

    if !current.is_empty() {
        result.push(current.join(" "));
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

/// Normaliza una cadena para compararla: descompone en NFD, elimina diacríticos
/// (tildes) y convierte a minúsculas. Así «Lucía» == «Lucia» y «Pérez» == «Perez».
fn fold(s: &str) -> String {
    s.nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Tokens significativos de un nombre: normalizados y con longitud mínima.
/// Agnóstico al idioma: la dedup por subsecuencia y los modelos se encargan de
/// los títulos («Lord Sterling» vs «Sterling»).
fn significant_tokens(name: &str) -> Vec<String> {
    name.split_whitespace()
        .map(|t| fold(t.trim_matches(|c: char| !c.is_alphabetic())))
        .filter(|t| t.chars().count() >= 2)
        .collect()
}

/// `true` si `needle` aparece como subsecuencia contigua dentro de `haystack`.
fn is_contiguous_subsequence(needle: &[String], haystack: &[String]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Decide si dos formas superficiales se refieren al mismo personaje.
fn is_same_character(a: &str, b: &str) -> bool {
    let fa = fold(a);
    let fb = fold(b);
    if fa == fb {
        return true;
    }

    let ta = significant_tokens(a);
    let tb = significant_tokens(b);
    if ta.is_empty() || tb.is_empty() {
        return false;
    }
    // Un nombre es una forma corta del otro («Sterling» ⊂ «Lord Sterling»).
    if is_contiguous_subsequence(&ta, &tb) || is_contiguous_subsequence(&tb, &ta) {
        return true;
    }
    // Comparten apellido (último token) y uno de los dos es monopalabra.
    if ta.last() == tb.last() && (ta.len() == 1 || tb.len() == 1) {
        return true;
    }
    // Tolerancia a erratas solo para nombres largos (≥ 6 caracteres): evita
    // fusionar pares cortos distintos («Ana»/«Ada») o variantes de género
    // («Marina»/«Marino»).
    if fa.chars().count() >= 6 && fb.chars().count() >= 6 && normalized_levenshtein(&fa, &fb) > 0.85
    {
        return true;
    }

    false
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
            let folded = fold(sentence);
            if names.iter().any(|n| folded.contains(&fold(n))) {
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
    text.split(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

// El tipado de relaciones (familia, romance, …) es lingüístico y se delega a
// un modelo multilingüe (SLM); no hay heurística agnóstica fiable por diccionario.

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
    fn clusters_titles_and_apodos() {
        let names = vec![
            "Lord Sterling".to_string(),
            "Sterling".to_string(),
            "Ana".to_string(),
        ];
        let clusters = cluster_aliases(&names);
        assert_eq!(clusters.len(), 2);
        let sterling = clusters
            .iter()
            .find(|c| c.contains(&"Lord Sterling".to_string()))
            .unwrap();
        assert!(sterling.contains(&"Sterling".to_string()));
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

    #[test]
    fn merges_accent_and_case_variants() {
        assert!(is_same_character("Lucía", "Lucia"));
        assert!(is_same_character("Pérez", "Perez"));
        assert!(is_same_character("Ana María", "Ana Maria"));
    }

    #[test]
    fn does_not_merge_distinct_or_gendered_names() {
        assert!(!is_same_character("Ana", "Ada"));
        assert!(!is_same_character("Marina", "Marino"));
    }

    #[test]
    fn clusters_accent_variants() {
        let names = vec![
            "Lucía".to_string(),
            "Lucia".to_string(),
            "Pedro".to_string(),
        ];
        let clusters = cluster_aliases(&names);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn skips_noisy_single_scene_relationship() {
        let texts = vec![
            "La casa de Ana y Pedro era antigua.".to_string(),
            "El amigo de Ana llegó tarde.".to_string(),
            "Su hermano llamó a Pedro por la mañana.".to_string(),
        ];
        let result = analyze::<crate::engines::FastEmbedder>(&texts, None);
        assert!(result.characters.iter().any(|c| c.name == "Ana"));
        assert!(result.characters.iter().any(|c| c.name == "Pedro"));
        assert!(
            !result
                .relationships
                .iter()
                .any(|r| (r.a == "Ana" && r.b == "Pedro") || (r.a == "Pedro" && r.b == "Ana"))
        );
    }

    #[test]
    fn chunks_multword_names_agnostically() {
        // Verifica el chunking agnóstico: «Zhou Mingrui» se detecta como una
        // sola entidad (no fragmentado en «Zhou» + «Mingrui»). La limpieza fina
        // (distinguir persona de lugar, «Klein Moretti» completo) la aporta
        // GLiNER2, no las reglas.
        let text = include_str!("../../fixtures/sample_narrative.txt");
        let texts = vec![text.to_string()];
        let result = analyze::<crate::engines::FastEmbedder>(&texts, None);

        let names: Vec<&str> = result.characters.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"Zhou Mingrui"),
            "faltó Zhou Mingrui: {names:?}"
        );
        assert!(!names.contains(&"Zhou"), "nombre fragmentado: {names:?}");
        assert!(!names.contains(&"Mingrui"), "nombre fragmentado: {names:?}");
    }

    /// Golden set mínimo para detectar regresiones en el análisis en español.
    /// No sustituye a una evaluación con GLiNER2, pero cubre personajes
    /// principales y una relación con múltiples co-ocurrencias.
    #[test]
    fn golden_set_spanish_narrative() {
        let texts = vec![
            "Al amanecer, Elena llegó a la taberna. El capitán Rivas la esperaba junto al fuego."
                .to_string(),
            "—No deberías estar aquí —dijo Rivas—. Tu hermana Elena me pidió que te protegiera."
                .to_string(),
            "Rivas miró a Elena. Sabía que el capitán nunca mentía.".to_string(),
        ];
        let result = analyze::<crate::engines::FastEmbedder>(&texts, None);
        let names: Vec<&str> = result.characters.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Elena"), "faltó Elena: {names:?}");
        assert!(names.contains(&"Rivas"), "faltó Rivas: {names:?}");
        assert!(
            result.relationships.iter().any(|r| {
                (r.a == "Elena" && r.b == "Rivas") || (r.a == "Rivas" && r.b == "Elena")
            }),
            "no se detectó la relación Elena↔Rivas: {:?}",
            result.relationships
        );
    }
}
