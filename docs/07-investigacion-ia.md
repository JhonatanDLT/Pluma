# 07 — Investigación de IA: modelos y algoritmos

Investigación previa a la implementación de la **Fase 2** (personajes y relaciones).
Objetivo: elegir modelos **locales, ligeros y ejecutables en Rust** (CPU / Apple Silicon),
especializados en las tareas de narrativa: detección de personajes (NER), resolución de
alias, embeddings y extracción de relaciones.

> Fuentes verificadas: repositorios de GLiNER, `fastembed-rs` y `mistral.rs` (2026).

---

## 1. Resumen ejecutivo

| Tarea | Solución recomendada | Runtime en Rust | Tamaño aprox. |
|-------|----------------------|-----------------|---------------|
| Detección de personajes (NER) | **GLiNER2** (`gliner2-multi-v1`) | `gliner2-rs` (ONNX) | ~1 GB |
| Embeddings / resolución de alias | **`multilingual-e5-small`** | `fastembed-rs` (`ort`) | ~118 MB |
| Reranking (opcional) | `jina-reranker-v2-base-multilingual` | `fastembed-rs` | ~300 MB |
| Extracción profunda / coreferencia | **Qwen3-1.7B / 4B** (cuantizado) | **`mistral.rs`** (embebido) o Ollama | 1–2.5 GB |

La novedad clave frente a `docs/03`: existe un **puerto Rust de GLiNER** (`gline-rs`) y
**`mistral.rs`** permite empaquetar el LLM **dentro del binario** con aceleración **Metal**
en Apple Silicon, eliminando la dependencia de Ollama si se desea.

---

## 2. Detección de personajes (NER)

### GLiNER ⭐

- **Qué es**: NER *zero-shot* — extrae cualquier tipo de entidad (ej. `persona`) sin
  entrenamiento específico, competitivo con LLMs mucho mayores.
- **Por qué encaja**: podemos pedir directamente la etiqueta `persona` sobre texto
  literario en español; modelos multilingües cubren 100+ idiomas.
- **Opciones de implementación en Rust**:
  1. **`gline-rs`** (implementación nativa en Rust) — sin dependencias de Python/ONNX.
  2. **GLiNER.cpp** (C++) vía FFI.
  3. **`ort`** con un checkpoint GLiNER exportado a ONNX.
- **Modelos relevantes**:
  - `urchade/gliner_multi` / `gliner_multi-v2.1` — multilingüe general (recomendado).
  - `gliner-community/gliner_small-v2.5` — más ligero, calidad algo menor.
  - `knowledgator/gliner-relex-large-v1.0` — **NER + relaciones en un solo pase** (RelEx).

> **Decisión final**: implementado con `gliner2-rs` 0.9.6 (arquitectura *span*) y el
> checkpoint `gliner2-multi-v1`, de forma **opt-in** y con fallback a reglas. El runtime
> de ONNX Runtime se descarga bajo demanda (`src/runtime.rs`) porque `ort` se compila en
> modo `load-dynamic` al convivir con `fastembed` en el mismo árbol de dependencias.

### Alternativas descartadas

| Opción | Motivo del descarte |
|--------|---------------------|
| `rust-bert` + `tch`/libtorch | Pesado (libtorch), mantenimiento irregular |
| spaCy (Python) | Requiere incrustar Python; rompe el enfoque "solo Rust" |
| BookNLP | Especializado en literatura pero Python/spaCy |

---

## 3. Embeddings y resolución de alias

### fastembed-rs (crate `fastembed` v7) ⭐

- Síncrono, **sin Tokio**, usa `ort` (ONNX) + `tokenizers`.
- Descarga el modelo en el primer uso y lo **cachea localmente** (offline después).
- Incluye helpers de **similitud coseno** y `top_k`.

### Modelos recomendados

| Modelo | Idiomas | Tamaño | Notas |
|--------|---------|--------|-------|
| **`intfloat/multilingual-e5-small`** ⭐ | 90+ | ~118 MB | Mejor relación tamaño/calidad para español |
| `BAAI/bge-m3` | 100+ | ~1.2 GB | Dense + sparse + ColBERT; más calidad, más peso |
| `sentence-transformers/paraphrase-multilingual-mpnet-base-v2` | 50+ | ~1 GB | Buena alternativa multilingüe |

**Pipeline de alias** (sin LLM):
1. Embedding de cada mención con su contexto ("la doctora", "Ana", "Ana Pérez").
2. Similitud coseno + Levenshtein (`strsim`).
3. Agrupación por umbral → personaje canónico.

> **Decisión**: `multilingual-e5-small` para el MVP; `bge-m3` si se requiere más
> precisión (y se acepta ~1.2 GB).

---

## 4. Extracción de relaciones

Tres niveles, de menor a mayor coste:

1. **Co-ocurrencia** (sin ML): dos personajes en la misma escena → relación candidata.
2. **GLiNER RelEx** (`gliner-relex`): NER + relaciones en un solo pase (para vínculos
   explícitos: "hermano de", "amaba a").
3. **LLM local** (Qwen3): extracción estructurada de `{ tipo, intensidad, descripción }`
   a partir de fragmentos, con salida JSON.

---

## 5. LLM local (extracción profunda / coreferencia)

### Opciones de runtime

| Runtime | Modelo | Empaquetado | Notas |
|---------|--------|-------------|-------|
| **`mistral.rs`** ⭐ | Qwen3-1.7B/4B | **Dentro del binario** | Metal (Apple Silicon), GGUF + ISQ, Rust puro |
| **Ollama** | qwen3:1.7b/4b | Externo (proceso aparte) | Más simple de configurar, API HTTP local |

### Modelos (generación Qwen3, 2025)

| Modelo | Tamaño (Q4) | Calidad español | Uso sugerido |
|--------|-------------|-----------------|--------------|
| **`Qwen3-1.7B`** ⭐ | ~1.2 GB | Buena | Extracción de relaciones y fichas ligeras |
| `Qwen3-4B` | ~2.5 GB | Muy buena | Coreferencia y resúmenes de mayor calidad |
| `Qwen3-8B` | ~5 GB | Excelente | Solo si se justifica (más RAM) |

### Recomendación

- **Fase 2 (Capa 2)**: `mistral.rs` con **Qwen3-1.7B** cuantizado, embebido. Ventajas:
  - Sin instalar nada externo (mejor UX), todo local.
  - Aceleración **Metal** en el Mac del usuario.
- **Fallback/alternativa**: Ollama si se prefiere separar el proceso y facilitar el
  cambio de modelo.

### Tareas para el LLM (prompts estructurados, salida JSON)

- Resolución de alias/coreferencia: "¿Estas menciones se refieren a la misma persona?"
- Ficha de personaje: extraer rasgos, objetivos, arco.
- Relaciones: `{ with, type, intensity, description }` entre dos personajes.

---

## 6. Dependencias Rust propuestas (actualizadas)

```toml
# Embeddings (Capa 1) — en uso
fastembed = "6.1"                    # multilingual-e5-small, cache local

# NER (Fase 2) — en uso
gliner2-rs = { version = "0.9", default-features = false, features = ["hub"] }

# LLM local (Capa 2, opcional) — Ollama (proceso externo) por ahora
# mistralrs = { version = "0.9", features = ["metal"] }  # pendiente de evaluar

# Utilidades
strsim = "0.11"                      # similitud de cadenas
```

---

## 7. Plan de adopción por fases

1. **Fase 2a (Capa 1, sin ML pesado)**: reglas + gazetteer + `fastembed` para alias. ✅
2. **Fase 2b**: añadir GLiNER2 (`gliner2-rs`) para NER preciso. ✅
3. **Fase 2c (opcional)**: `mistral.rs` + Qwen3-1.7B para coreferencia y relaciones profundas.
4. **Fase 5**: si el usuario lo desea, ampliar a Qwen3-4B/8B o modelos específicos.

---

## 8. Riesgos y mitigación

| Riesgo | Mitigación |
|--------|-----------|
| `gline-rs` inmaduro o desactualizado | Usar `ort` + GLiNER-ONNX como plan B probado |
| `mistral.rs` aumenta el binario (~decenas de MB + modelo) | Mantenerlo **opcional** (feature flag) y ofrecer Ollama |
| Modelos grandes → latencia en CPU | Análisis bajo demanda y en *background task* de Tauri |
| Alucinaciones del LLM en relaciones/fichas | Mostrar siempre como **sugerencia** editable; nunca sobrescribir |
