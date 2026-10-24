# 03 — IA local y ligera

Estrategia para detectar **personajes**, **relaciones** y **estructura narrativa** de
forma **100 % local** y **ligera** (CPU, sin GPU, modelos pequeños). Se propone un
enfoque **en capas**: reglas + embeddings + NER siempre activos, y un LLM local opcional
para enriquecimiento profundo.

---

## 1. Principios

1. **Offline-first**: nada se envía a servicios externos.
2. **Progresivo**: lo básico funciona sin LLM; el LLM es un extra opcional.
3. **Ligero**: modelos ≤ ~200 MB, inferencia en CPU con latencia aceptable.
4. **Determinista + asistido**: la IA **sugiere**, el autor **confirma/edita**.

---

## 2. Arquitectura de IA en capas

```mermaid
flowchart TB
    subgraph L1["Capa 1 — Siempre activa (Rust, embebida)"]
        RULE["Reglas / heurísticas"]
        EMB["Embeddings (fastembed-rs)"]
        NER["NER (ort + modelo pequeño)"]
    end

    subgraph L2["Capa 2 — Bajo demanda (local, opcional)"]
        LLM["Ollama + modelo 1–3B cuantizado"]
    end

    DOC["Manuscrito Markdown"] --> RULE
    DOC --> NER
    NER --> EMB
    RULE --> RESULT["Candidatos + relaciones"]
    EMB --> RESULT
    RESULT --> LLM
    LLM --> ENRICH["Fichas enriquecidas / relaciones profundas"]
```

---

## 3. Tareas de NLP y solución recomendada

### 3.1 Estructura (capítulos / escenas / beats)

- **No requiere ML.** Se resuelve con un parser de encabezados Markdown (`pulldown-cmark`)
  y patrones configurables:
  - `# Título` → capítulo
  - `## Título` → escena
  - `### Título` → beat
- Regex para títulos numéricos/romanos: `Capítulo\s+\d+`, `Chapter\s+I+`, etc.
- **Recomendación**: implementación propia en Rust (`parser/`). Rápido y determinista.

### 3.2 Detección de personajes (NER)

| Opción | Estado | Librería | Modelo | Tamaño | Notas |
|--------|--------|----------|--------|--------|-------|
| **A (reglas)** | ✅ implementada | Heurísticas propias | — | 0 | Nombres en mayúscula, `— dijo Ana —`, apodos |
| **B (GLiNER2)** | ✅ implementada (opt-in) | `gliner2-rs` (span, ONNX) | `gliner2-multi-v1` | ~1 GB | NER zero-shot multilingüe, etiqueta `person` |

> Alternativa considerada: `rust-bert` (con `tch`/libtorch). Se **descarta** por ser
> pesado y de mantenimiento irregular. GLiNER2 sobre ONNX Runtime es más ligero y preciso.

**Recomendación**: usar **A** siempre (fallback) y activar **B** cuando se desee mayor
precisión con nombres ambiguos o no hispanos. Ambos orígenes se fusionan en el mismo
pipeline antes de agrupar alias.

### 3.3 Resolución de alias y agrupación

- **Embeddings** con `fastembed-rs` (crate `fastembed`), modelo
  `intfloat/multilingual-e5-small` (~118 MB) o `all-MiniLM-L6-v2` (~22 MB, solo inglés).
- Pipeline: embedding de cada mención contextual → similitud coseno + Levenshtein →
  agrupación → personaje canónico.
- **Recomendación**: `multilingual-e5-small` por su soporte de español.

### 3.4 Extracción de relaciones

**Sin LLM (Capa 1):**
- **Co-ocurrencia**: dos personajes en la misma escena → relación candidata.
- **Señales lingüísticas**: verbos de vínculo ("amaba", "odiaba", "era su hermano"),
  diálogo dirigido, proximidad.
- **Sentimiento** aproximado por léxico (polaridad en español) → positivo/negativo/neutro.

**Con LLM (Capa 2, opcional):** dado un fragmento con dos personajes, pedir
`{ tipo, intensidad, descripción }` en JSON.

### 3.5 Fichas de personaje automáticas

- **Sin LLM**: ensamblar ficha con atributos por patrones ("era alto", "ojos verdes"),
  rol por frecuencia de aparición y POV.
- **Con LLM**: resumir descripción, extraer rasgos/objetivos/arco de fragmentos.

---

## 4. Recomendación de LLM local (Capa 2)

Se recomienda **Ollama** como runtime y un modelo pequeño cuantizado:

| Modelo | Tamaño (Q4) | Calidad español | Notas |
|--------|-------------|-----------------|-------|
| **`qwen2.5:1.5b-instruct-q4_K_M`** ⭐ | ~1.0 GB | Muy buena | Mejor relación calidad/tamaño para español |
| `llama3.2:1b` | ~0.8 GB | Buena | Muy ligero |
| `phi3.5:mini` | ~2.2 GB | Buena | Mejor razonamiento, más pesado |
| `gemma2:2b` | ~1.6 GB | Buena | Alternativa sólida |

- **Por qué Ollama**: instalación trivial, API HTTP en `localhost:11434`, multi-OS. La app
  se comunica con `reqwest` desde Rust.
- **Alternativa embebida**: `mistral-rs` o `llama.cpp` para empaquetar el modelo dentro
  del binario. Más integración; recomendado solo en fase avanzada.

### Formato de salida (JSON) para el LLM

```json
{
  "character": { "name": "Ana", "role": "protagonista", "traits": ["obstinada"], "arc": "..." },
  "relationships": [
    { "with": "Pedro", "type": "mentor", "intensity": 0.8, "description": "..." }
  ]
}
```

> Se usa *JSON mode* de Ollama para garantizar salida válida.

---

## 5. Dependencias Rust (en uso)

```toml
# nlp — embeddings y NER
fastembed = "6.1"            # embeddings locales (ONNX), multilingual-e5-small
gliner2-rs = { version = "0.9", default-features = false, features = ["hub"] }  # NER GLiNER2

# runtime ONNX Runtime (bootstrap autocontenido)
ureq = "2"                   # descarga del runtime y del modelo
tar = "0.4"                  # extracción del runtime
flate2 = "1"                  # descompresión gzip (.tgz)
sha2 = "0.10"                # verificación de integridad

# utilidades
pulldown-cmark = "0.13"      # parseo Markdown
strsim = "0.11"              # similitud de cadenas (Levenshtein/Jaro)
```

> El runtime de ONNX Runtime se descarga una sola vez (con permiso) desde los
> **releases oficiales** (`src/runtime.rs`) y se fija vía `ORT_DYLIB_PATH`, porque
> `ort` se compila en modo `load-dynamic` al convivir `fastembed` y `gliner2-rs`
> en el mismo árbol. Se usa el release oficial (no el CDN de `ort-sys`) porque este
> último solo publica la librería estática para macOS.

---

## 6. Presupuesto de recursos

| Componente | RAM (aprox.) | Disco | GPU |
|-----------|--------------|-------|-----|
| Embeddings `e5-small` | ~300 MB | ~118 MB | No |
| NER cuantizado | ~300 MB | ~150 MB | No |
| LLM `qwen2.5:1.5b` | ~1.5–2 GB | ~1.0 GB | No (CPU ok) |

> Total con todo: < ~2.5 GB RAM en picos de análisis. Sin LLM: < ~700 MB.

---

## 7. Riesgos y mitigación

| Riesgo | Mitigación |
|--------|-----------|
| NER impreciso con nombres hispanos/ambiguos | Combinar reglas + NER; permitir confirmación manual |
| Alias mal agrupados | Umbral de similitud configurable + revisión del usuario |
| LLM alucina relaciones | Mostrar siempre como **sugerencia** editable; nunca sobrescribir |
| Carga de CPU al analizar en tiempo real | Análisis **bajo demanda** (botón) y en *background task* de Tauri |
