Para el análisis literario (donde abundan apodos, títulos nobiliarios, relaciones familiares, alianzas dinámicas y cambios de estatus), los modelos estándar de noticias no suelen bastar. La estrategia más eficiente y ligera combina modelos discriminativos pequeños (tipo BERT/GLiNER) para fijar las entidades y SLMs generativos compactos (1B–3B) para extraer y razonar las relaciones complejas.
1. Extracción de Entidades (NER): Ligeros y Deterministas
 * GLiNER (Generalist and Lightweight Model for NER)
   * Pesos: urchade/gliner_small-v2.1 (~150M parámetros) o gliner_medium-v2.1.
   * Por qué es ideal: A diferencia de los BERT convencionales limitados a PER, LOC, ORG, GLiNER permite definir clases arbitrarias en inferencia: "personaje", "apodo", "linaje", "artefacto", "facción".
   * Rendimiento: Corre en CPU a cientos de palabras por segundo con memoria RAM insignificante (<500 MB).
 * BookNLP / Literary RoBERTa
   * Pesos: Paquetes integrados basados en RoBERTa-base (~125M) o modelos fine-tuneados con LitBank.
   * Por qué es ideal: Entrenado exclusivamente con novelas y ficción. Además de NER literario, incluye resolución de correferencia (enlaza "el conde", "él", "Lord Sterling" a la misma entidad), un requisito indispensable antes de mapear cualquier relación.
 * spaCy (Pipeline Transformer es_dep_news_trf o en_core_web_trf)
   * Pesos: Basados en DeBERTa/RoBERTa (~400 MB).
   * Por qué es ideal: Eficiente para crear pipelines estructurados donde se extrae la sintaxis (árboles de dependencia sujeto-verbo-objeto) antes de tipificar la relación.
2. Extracción de Relaciones (RE) y Construcción de Grafos
 * REBEL (Relation Extraction By End-to-end Language generation)
   * Pesos: Babelscape/rebel-large (BART-large, ~400M) o versiones destiladas como rebel-base (~140M).
   * Por qué es ideal: Traduce oraciones o párrafos directamente en tripletas estructuradas [Sujeto | Relación | Objeto] sin necesidad de un LLM generativo pesado.
   * Limitación: Si el universo de la novela tiene relaciones fantásticas o de ficción muy específicas ("discípulo de la secuencia X", "lealtad secreta"), su vocabulario predeterminado de relaciones puede quedarse corto frente a un prompt guiado.
 * Small Language Models (SLMs de 1B a 3B) con Salida Estructurada
   * Modelos destacados:
     * Qwen2.5-1.5B-Instruct o Qwen2.5-3B-Instruct
     * Llama-3.2-1B-Instruct o Llama-3.2-3B-Instruct
   * Por qué son ideales: Al cuantizarlos (GGUF Q4_K_M o Q8), consumen entre 1.2 GB y 2.5 GB de VRAM/RAM. A través de gramáticas BNF o esquemas JSON rígidos (usando motores locales como llama.cpp o vLLM), extraen relaciones complejas con matiz emocional o cronológico:
     {
  "source": "Melissa",
  "target": "Klein",
  "relation_type": "hermana menor",
  "sentiment": "protectora/preocupada",
  "evidence": "párrafo 4"
}

Comparativa de Arquitectura y Recursos
| Modelo | Tamaño / VRAM | Tarea Principal | Fortaleza Literaria |
|---|---|---|---|
| GLiNER Small/Medium | ~300–600 MB (CPU/GPU) | NER de vocabulario abierto | Detecta roles específicos (ej. "dios", "mago", "mentor") sin reentrenar. |
| BookNLP (Pipeline) | ~500 MB (CPU) | NER + Correferencia | Conecta apodos, pronombres y títulos al nombre canónico. |
| REBEL Base | ~500 MB | Relaciones Directas | Emite tripletas (sujeto, relación, objeto) muy rápido. |
| Qwen2.5 1.5B / 3B | 1.5–2.5 GB (Q4) | Relaciones matizadas | Captura sutilezas literarias, cambios de lealtad y relaciones no explícitas. |
Flujo de Trabajo Recomendado (Pipeline Modular)
 * Preprocesamiento y Correferencia: Pasa el capítulo por un módulo ligero de correferencia (como BookNLP o fastcoref) para reemplazar pronombres ambiguos por el nombre de la entidad canónica.
 * Detección de Co-ocurrencias: Extrae las entidades con GLiNER e identifica qué personajes aparecen en la misma ventana de texto (1 a 3 párrafos).
 * Calificación de Relación: Pasa únicamente esos fragmentos con las entidades marcadas a un SLM (Qwen2.5-1.5B/3B o REBEL) pidiéndole el tipo de vínculo, polaridad y cita en formato JSON.

 Sí, se puede implementar completamente en Rust. De hecho, para procesamiento de novelas largas o procesamiento en lote de capítulos, la combinación nativa en Rust es significativamente más rápida, tiene un consumo de memoria plano y permite compilar todo en un único binario o integrarlo directamente como backend (por ejemplo, en Tauri o una CLI).
Stack de Crates en Rust
 * Inferencia de Modelos Encoders (GLiNER / Embeddings):
   * ort (ONNX Runtime para Rust): Es el estándar de oro. Conviertes el modelo GLiNER a .onnx y lo ejecutas con aceleración CPU (SIMD/AVX) o GPU (CUDA, DirectML/CoreML) con latencias mínimas.
   * Alternativa: candle (Hugging Face nativo en Rust), aunque exportar GLiNER a ONNX con ort suele ser más directo para arquitecturas con spans dinámicos.
 * Inferencia del SLM Generativo (Relaciones en JSON):
   * llama-cpp-2 o kalosm (un framework LLM de alto nivel escrito en Rust sobre Candle/llama.cpp).
   * Permite cargar archivos .gguf (ej. Qwen 2.5 1.5B/3B) y aplicar Grammars (GBNF) para garantizar que la salida sea un JSON estrictamente válido sin desperdiciar tokens de reintentos.
 * Estructuras de Datos y Grafos:
   * petgraph: Para construir y mantener el grafo de relaciones en memoria (Node = Personaje, Edge = Relación/Sentimiento).
   * serde / serde_json: Para deserializar las tripletas extraídas directamente en estructuras de datos de Rust.
Arquitectura del Pipeline en Rust
[ Texto del Capítulo ]
         │
         ▼
[ GLiNER vía `ort` ] ──> Vec<EntitySpan> (ej. "Klein", "Melissa", "Antigonus")
         │
         ▼
[ Filtro de Co-ocurrencias / Ventana de Párrafos ]
         │
         ▼
[ Qwen2.5 / Llama via `llama-cpp-2` + GBNF Grammar ]
         │
         ▼
[ `serde_json` -> `petgraph::Graph` ] ──> Grafo persistible o linter narrativo

Ejemplo de Implementación: Extracción de Relaciones con llama-cpp-2 y GBNF
A continuación se muestra cómo estructurar la inferencia del SLM en Rust garantizando salida estructurada en JSON:
1. Dependencias (Cargo.toml)
[dependencies]
llama-cpp-2 = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
petgraph = "0.6"

2. Código de Inferencia y Construcción del Grafo (main.rs)
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use serde::Deserialize;
use petgraph::graph::DiGraph;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CharacterRelation {
    source: String,
    target: String,
    relation_type: String,
    sentiment: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct ExtractionPayload {
    relations: Vec<CharacterRelation>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar backend de llama.cpp
    let backend = LlamaBackend::init()?;

    // 2. Cargar modelo cuantizado (ej. Qwen2.5-1.5B-Instruct-Q4_K_M.gguf)
    let model_params = LlamaModelParams::default();
    let model_path = Path::new("./models/qwen2.5-1.5b-instruct-q4_k_m.gguf");
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;

    let ctx_params = LlamaContextParams::default().with_n_ctx(Some(std::num::NonZeroU32::new(2048).unwrap()));
    let mut ctx = model.new_context(&backend, ctx_params)?;

    // 3. Gramática GBNF para forzar JSON estricto
    let grammar_str = r#"
        root ::= "{" ws "\"relations\":" ws "[" ws (relation ("," ws relation)*)? ws "]" ws "}"
        relation ::= "{" ws "\"source\":" ws string "," ws "\"target\":" ws string "," ws "\"relation_type\":" ws string "," ws "\"sentiment\":" ws string "," ws "\"description\":" ws string ws "}"
        string ::= "\"" [^\"]* "\""
        ws ::= [ \t\n\r]*
    "#;

    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::grammar(&model, grammar_str, "root")?,
    ]);

    // 4. Prompt enfocado con contexto literario y entidades pre-detectadas
    let passage = "Klein miró a Melissa con preocupación mientras ella ajustaba el engranaje del reloj de bolsillo. Aunque ella intentaba ocultar el cansancio para no preocupar a sus hermanos, Klein sabía que el dinero para la matrícula escolar se estaba agotando.";

    let prompt = format!(
        "<|im_start|>system\nEres un analizador literario. Extrae relaciones entre entidades en JSON.<|im_end|>\n\
         <|im_start|>user\nAnaliza el siguiente texto y extrae las relaciones entre 'Klein' y 'Melissa':\n\n{}\n<|im_end|>\n\
         <|im_start|>assistant\n",
        passage
    );

    // 5. Inferencia (generación token por token restringida por la gramática)
    let tokens = model.str_to_token(&prompt, llama_cpp_2::model::AddBos::Always)?;
    let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(512, 1);

    for (i, token) in tokens.iter().enumerate() {
        batch.add(*token, i as i32, &[0], i == tokens.len() - 1)?;
    }
    ctx.decode(&mut batch)?;

    let mut output_json = String::new();
    let mut n_cur = tokens.len() as i32;

    while n_cur < 2048 {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            break;
        }

        let piece = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)?;
        output_json.push_str(&piece);

        batch.clear();
        batch.add(token, n_cur, &[0], true)?;
        ctx.decode(&mut batch)?;
        n_cur += 1;
    }

    println!("JSON Generado:\n{}", output_json);

    // 6. Mapear al grafo en petgraph
    let payload: ExtractionPayload = serde_json::from_str(&output_json)?;
    let mut story_graph = DiGraph::<String, String>::new();

    for rel in payload.relations {
        let src_idx = story_graph.add_node(rel.source.clone());
        let tgt_idx = story_graph.add_node(rel.target.clone());
        let edge_label = format!("{}: {} ({})", rel.relation_type, rel.sentiment, rel.description);

        story_graph.add_edge(src_idx, tgt_idx, edge_label);
    }

    println!("Nodos en grafo: {}", story_graph.node_count());
    println!("Relaciones activas: {}", story_graph.edge_count());

    Ok(())
}

Integración de GLiNER en Rust (ort)
Para evitar tener que codificar tokens especiales de GLiNER a mano, el patrón estándar es:
 * Exportar GLiNER de PyTorch a ONNX una sola vez (torch.onnx.export o con la utilidad de Hugging Face Optimum).
 * Tokenizar el texto con el crate tokenizers de Hugging Face (carga directa de tokenizer.json).
 * Pasar los tensores al session runner de ort:
use ort::session::{Session, builder::GraphOptimizationLevel};

let session = Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?
    .commit_from_file("gliner_small.onnx")?;

// Entrada: input_ids, attention_mask, words_mask
// Salida: spans con coordenadas y probabilidades por etiqueta

Ventajas de hacerlo en Rust frente a Python
 * Embebible: No requiere entorno virtual, dependencias de Python ni intérprete instalado en el sistema del usuario final.
 * Consumo de memoria predecible: Con llama.cpp enlazado estáticamente vía C/C++ FFI y ort, el pipeline completo consume entre 1.5 GB y 3 GB fijos sin riesgo de memory leaks por garbage collector.
 * Concurrencia con rayon: Puedes procesar múltiples escenas o capítulos en paralelo dividiendo el texto por ventanas sin el cuello de botella del GIL de Python.
