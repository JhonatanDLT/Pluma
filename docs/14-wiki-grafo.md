# 14 — Wiki, grafo y campos enriquecidos

Plan de evolución de la Biblia de la novela: pasar de una lista de fichas a un
**mapa visual del mundo** y a un sistema de **campos definidos por el autor** que
la IA pueda rellenar con evidencia, sin inventar.

---

## 1. Estado inicial y límites (antes de las fases 1–2)

La base ya tenía:

- `character`: nombre, alias, rol, descripción, biografía, rasgos, objetivos,
  arco y color.
- `entity`: lugares y facciones con alias, descripción y menciones.
- `relationship`: personaje A ↔ personaje B, tipo, intensidad y descripción.
- `enrich_project`: pedía al LLM una ficha y escribía directamente los campos.

Problemas detectados:

1. **El enriquecimiento no siempre llena todos los campos.** El modelo pequeño
   suele devolver solo `description`; biografía, rasgos, objetivos y arco quedan
   vacíos o genéricos.
2. **No hay evidencia ni confianza.** No se distingue lo que el texto afirma de
   lo que el modelo inventa.
3. **No hay campos personalizados.** El autor no puede añadir “color de ojos”,
   “arma”, “título” o “facción” y pedirle a la IA que los rellene.
4. **No hay grafo.** Las relaciones solo se ven como listas; no se entiende la
   estructura del mundo de un vistazo.
5. **No hay vínculos personaje-facción/lugar.** El grafo solo podría conectar
   personajes entre sí, no agruparlos por facción ni situarlos en lugares.
6. **Riesgo de sobrescritura.** Un reanálisis no debería pisar lo que el autor
   escribió a mano.

---

## 2. Objetivos

- **Grafo visible e interactivo** de personajes, facciones y lugares, con
  relaciones, pertenencia y presencia.
- **Campos definidos por el autor** (built-in + personalizados), editables.
- **IA con evidencia:** cada campo propuesto incluye cita, confianza y origen;
  si no hay texto suficiente, se dice explícitamente.
- **Manual gana:** lo que el autor edita no se sobrescribe sin aceptación.
- **Rendimiento:** cientos de personajes sin que el grafo o la UI se congelen.

---

## 3. Modelo de datos

### 3.1 Mantener los campos built-in

Los campos actuales de `character` se conservan como columnas de primera clase
para no romper datos ni consultas. La UI los presenta como campos “base”.

### 3.2 Campos personalizados (migración `0011_character_fields.sql`)

```sql
CREATE TABLE character_field_def (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,              -- slug estable, p. ej. "color_ojos"
    label       TEXT NOT NULL,              -- "Color de ojos"
    field_type  TEXT NOT NULL DEFAULT 'text', -- text | longtext | list | number
    options_json TEXT,
    prompt_hint TEXT,                       -- "solo si se menciona explícitamente"
    position    INTEGER NOT NULL DEFAULT 0,
    builtin     INTEGER NOT NULL DEFAULT 0, -- 1 = campo base de `character`
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE(project_id, key),
    UNIQUE(project_id, id)
);

CREATE TABLE character_field_value (
    project_id   TEXT NOT NULL,
    character_id TEXT NOT NULL,
    field_id     TEXT NOT NULL,
    value_json   TEXT NOT NULL,             -- string, lista, número o "null"
    source       TEXT NOT NULL DEFAULT 'manual', -- manual | ai
    confidence   REAL CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
    evidence     TEXT,                      -- cita literal del manuscrito
    reason       TEXT,                      -- "sin evidencia suficiente", etc.
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (character_id, field_id),
    FOREIGN KEY (project_id, character_id)
        REFERENCES character(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, field_id)
        REFERENCES character_field_def(project_id, id) ON DELETE CASCADE
);
```

Reglas:

- `source = manual` **nunca** se sobrescribe con una sugerencia `ai`.
- Las sugerencias de IA se guardan con `source = ai` y se pueden aceptar
  (pasan a `manual`) o descartar.
- Los campos base también tienen una definición (`builtin = 1`) para que la UI
  los trate igual y para guardar sugerencias pendientes. Al aceptar una
  sugerencia se escribe la columna de `character`.
- Si el campo base ya tiene valor, `set_field_suggestion` ignora la propuesta.
- `key` es estable y no cambia al renombrar `label`.
- `field_type` admite `text`, `longtext`, `list` y `number`.
- La migración siembra las definiciones base de los proyectos existentes; los
  proyectos nuevos las crean al darse de alta.
- Borrar un campo personalizado borra sus valores (cascada). Los campos base no
  se pueden renombrar ni eliminar.
- Duplicar o restaurar un proyecto copia campos personalizados y valores.

### 3.3 Afiliaciones y grafo (migración `0012_wiki_graph.sql`)

```sql
CREATE TABLE character_affiliation (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    character_id      TEXT NOT NULL,
    entity_id         TEXT NOT NULL,
    role              TEXT NOT NULL,   -- facción: leader|member|ally|rival
                                       -- lugar: resident|origin|visitor
    source            TEXT NOT NULL DEFAULT 'manual', -- manual | ai
    confidence        REAL,
    evidence          TEXT,
    evidence_scene_id TEXT REFERENCES scene(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    UNIQUE (character_id, entity_id),
    FOREIGN KEY (project_id, character_id) REFERENCES character(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, entity_id)    REFERENCES entity(project_id, id)    ON DELETE CASCADE
);

CREATE TABLE entity_relation (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    a_entity_id       TEXT NOT NULL,
    b_entity_id       TEXT NOT NULL,
    kind              TEXT NOT NULL,   -- alliance | conflict | territory
    source            TEXT NOT NULL DEFAULT 'manual',
    confidence        REAL,
    evidence          TEXT,
    evidence_scene_id TEXT REFERENCES scene(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    CHECK (a_entity_id <> b_entity_id),
    CHECK (kind = 'territory' OR a_entity_id < b_entity_id),
    UNIQUE (a_entity_id, b_entity_id, kind),
    FOREIGN KEY (project_id, a_entity_id) REFERENCES entity(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, b_entity_id) REFERENCES entity(project_id, id) ON DELETE CASCADE
);
```

- Un personaje tiene **un solo rol por entidad**; el rol depende del tipo:
  facción (`leader`, `member`, `ally`, `rival`) o lugar (`resident`, `origin`,
  `visitor`). Se valida en Rust.
- `alliance` y `conflict` son simétricos: se guardan con `a_entity_id <
  b_entity_id` y un índice parcial impide que la misma pareja sea aliada y
  enemiga.
- `territory` es direccional: facción → lugar.
- `manual` nunca se sobrescribe desde la IA; las propuestas quedan como `ai`
  hasta que el autor las acepta o descarta.
- Las relaciones personaje-personaje siguen en `relationship`.
- **Presencia** personaje → lugar es derivada: se calcula desde
  `scene.location` (y el texto como respaldo), no se persiste. Es una señal de
  coocurrencia, no una afirmación de residencia.
- Las IDs de nodo van con prefijo (`char:` / `entity:`) porque los espacios de
  IDs de personajes y entidades son independientes.

---

## 4. Grafo

### 4.1 Datos

Comando `wiki_graph(project_id)`:

```ts
type GraphNode = {
  id: string;                 // char:<id> | entity:<id>
  kind: "character" | "faction" | "location";
  label: string;
  mentions: number;
  role: string | null;
  faction_ids: string[];      // facciones a las que pertenece
};

type GraphEdge = {
  id: string;
  source: string;
  target: string;
  kind: "relationship" | "affiliation" | "presence"
      | "territory" | "alliance" | "conflict";
  subtype: string | null;     // tipo de relación o rol de afiliación
  weight: number;             // intensidad / coocurrencias
  origin: "manual" | "ai" | "derived";
  evidence: string | null;
  evidence_scene_id: string | null;
};

type GraphData = { nodes: GraphNode[]; edges: GraphEdge[]; stats: {...} };
```

- **Personajes:** tamaño por menciones; color por facción; borde por rol.
- **Facciones/lugares:** nodos más grandes (forma distinta), color propio.
- **Aristas:**
  - `relationship`: personaje ↔ personaje (grosor por intensidad).
  - `affiliation`: personaje → facción/lugar (rol: líder, miembro, aliado,
    rival, residente, origen, visitante).
  - `presence`: personaje → lugar **derivado** de las escenas (hasta 5 lugares
    por personaje).
  - `territory`: facción → lugar.
  - `alliance` / `conflict`: facción ↔ facción.
- Cada arista conserva `origin` y, cuando existe, la cita y la escena de
  evidencia.

### 4.2 UI (`WorldGraph.svelte`)

- Simulación **force-directed** con `d3-force` + SVG.
- Zoom/pan, arrastrar nodos, etiquetas y búsqueda que atenúa.
- Filtros por tipo de nodo (personajes, facciones, lugares) y por tipo de
  arista (relaciones, afiliaciones, presencia, alianzas/territorio).
- **Agrupar por facción:** una fuerza extra acerca a cada personaje hacia su
  facción principal.
- **Inspector:** al hacer clic en un nodo se muestra su tipo, rol, menciones,
  vínculos y facciones, con «Abrir ficha» y «Aislar entorno» (ego network de un
  salto). No se cambia de pestaña al explorar el grafo.
- Formas distintas para personaje (círculo), facción (hexágono) y lugar
  (rectángulo), además de color; las aristas usan estilo de línea redundante.
- Estado vacío: “Analiza el manuscrito para construir el grafo”.
- Panel **Vínculos** para crear/editar afiliaciones y relaciones entre
  entidades, revisar sugerencias IA con cita y aceptarlas o descartarlas.
- Fallback accesible: los nodos son enfocables por teclado y tienen
  `aria-label`; la pestaña Vínculos ofrece la lista equivalente.

### 4.3 Rendimiento

- La simulación corre en el cliente; el backend solo entrega nodos/aristas.
- Límite inicial: 250 nodos visibles; si hay más, agrupar por facción o pedir
  filtro.
- No cargar contenido de escenas en el grafo; solo metadatos.

---

## 5. IA: enriquecimiento con evidencia

### 5.1 Pipeline

1. **Contexto:** escenas donde aparece el personaje (por menciones/alias), las
   más relevantes y con suficiente texto; incluir título, POV, lugar y tiempo.
2. **Esquema:** campos built-in + definiciones personalizadas (`label`, `type`,
   `prompt_hint`).
3. **Prompt:** “Extrae solo lo que el texto sostiene. Devuelve para cada campo
   `{ value, confidence, evidence }`. Si no hay evidencia, `value = null` y
   `reason = 'sin evidencia'`. No inventes.”
4. **Validación:**
   - JSON estricto y tipos correctos;
   - `evidence` debe ser una cita real del contexto (normalizada);
   - confianza mínima configurable (p. ej. 0.5);
   - longitudes máximas y listas limpias.
5. **Persistencia:** sugerencias con `source = ai`, confianza y evidencia; no
   tocar `manual`.
6. **Revisión:** UI por campo con “Aceptar”, “Descartar” y “Editar”; mostrar la
   cita y el motivo si está vacío.
7. **Progreso:** eventos `enrich-progress` por personaje/campo.

### 5.2 Texto insuficiente

- Si el personaje tiene menos de N menciones o el contexto es menor de M
  caracteres, **no se llama al LLM** para campos descriptivos: se marca
  “sin evidencia suficiente”.
- Las reglas pueden rellenar alias/rol básico.
- La UI explica por qué un campo está vacío; no se inventa una biografía.

### 5.3 Afiliaciones con IA

- `extract_affiliations` recorre los personajes con contexto suficiente.
- El prompt recibe listas cerradas de facciones y lugares; el modelo debe
  elegir solo de esas listas y citar evidencia.
- El nombre propuesto se resuelve contra nombre/alias normalizados; si no
  existe, se descarta (nunca se crean entidades automáticamente).
- El rol se valida contra el tipo de entidad y la cita contra el contexto; la
  escena que contiene la cita se guarda como `evidence_scene_id`.
- Las propuestas se guardan con `source = ai`. Al re-detectar se borran solo las
  sugerencias IA anteriores del personaje; las manuales nunca se tocan.
- La revisión (Aceptar / Descartar) está en la pestaña **Vínculos**.

### 5.4 Campos personalizados

- El autor crea “Color de ojos” con hint “solo si se menciona explícitamente”.
- El prompt incluye ese campo; el LLM lo rellena solo si encuentra una cita.
- Si el autor añade un campo después, puede re-enriquecer solo ese personaje.
- Los campos built-in se pueden ocultar o renombrar en la UI, pero su `key`
  interna no cambia.

### 5.5 Modelo

- Local: Qwen 1.5B/4B en modo JSON.
- Nube: opcional, con consentimiento; mismo esquema.
- Validación y evidencia son obligatorias, independientemente del modelo.
- Cada personaje se procesa de forma aislada: un fallo de formato no aborta el
  lote; ese personaje cuenta como omitido y se registra en el log.

---

## 6. API y comandos

| Comando | Uso |
| --- | --- |
| `list_field_defs(project_id)` | Definiciones de campos |
| `create_field_def(...)` / `update_field_def` / `delete_field_def` | CRUD |
| `list_character_fields(character_id)` | Valores built-in + custom |
| `set_character_field(character_id, field_id, value)` | Edición manual |
| `accept_field_suggestion(character_id, field_id)` | Acepta una propuesta |
| `reject_field_suggestion(character_id, field_id)` | Descarta una propuesta |
| `enrich_character_fields(project_id, character_id, model)` | Sugerencias con evidencia |
| `enrich_project(project_id, model)` | Sugerencias para todos |
| `wiki_graph(project_id)` | Nodos y aristas (incluye presencia derivada) |
| `list_affiliations(project_id)` | Afiliaciones con nombre/tipo |
| `set_affiliation(...)` / `delete_affiliation(id)` | Afiliación manual |
| `accept_affiliation(id)` / `reject_affiliation(id)` | Revisar propuesta IA |
| `list_entity_relations(project_id)` | Alianzas, conflictos y territorios |
| `set_entity_relation(...)` / `delete_entity_relation(id)` | Relación manual |
| `accept_entity_relation(id)` / `reject_entity_relation(id)` | Revisar propuesta IA |
| `extract_affiliations(project_id, model)` | Detección IA con evidencia |

---

## 7. Fases

### Fase 1 — Grafo MVP (✅ implementado)

- `WorldGraph.svelte`: force layout (`d3-force`), búsqueda, etiquetas, zoom,
  pan, arrastre de nodos y clic para abrir la ficha.
- Usa los `characters` y `relationships` ya cargados en World Building; sin
  cambios de esquema ni consultas extra.
- Pestaña **Grafo** dentro de World Building.
- Nodos dimensionados por grado de relación; protagonistas en color de acento.
- Ampliado en la **Fase 3** con facciones, lugares, afiliaciones, presencia y
  clustering por facción.

### Fase 2 — Campos personalizados + IA con evidencia (✅ implementado)

- Migración `0011_character_fields.sql`: definiciones base + personalizadas,
  valores con `source`, `confidence`, `evidence` y `reason`.
- Repo: `list/create/update/delete_field_def`, `reorder_field_defs`,
  `list_character_fields`, `set_character_field`,
  `set_field_suggestion`, `accept/reject_field_suggestion`.
- IA: `llm::suggest_character_fields` pide un objeto JSON por campo
  (`{value, confidence, evidence, reason}`); cada cita se valida contra el
  contexto normalizado (sin tildes, minúsculas y espacios). Sin cita válida no
  hay valor: se guarda «sin evidencia suficiente».
- `enrich_project` ya no sobrescribe la ficha: guarda propuestas pendientes.
  `enrich_character_fields` permite re-enriquecer un solo personaje.
- UI: pestaña **Campos** (`FieldManager.svelte`) para crear, renombrar,
  reordenar y borrar campos; la ficha del personaje
  (`CharacterFields.svelte`) muestra valores base + personalizados, sugerencias
  con cita, confianza y botones Aceptar/Descartar.
- Duplicar y restaurar proyectos conserva los campos personalizados.
- **Aceptación:** el autor añade un campo, la IA lo rellena con cita y lo
  manual sobrevive a un reanálisis. ✔

### Fase 3 — Afiliaciones y grafo enriquecido (✅ implementado)

- Migración `0012_wiki_graph.sql`: `character_affiliation` y
  `entity_relation`, con validación de roles/tipos, simetría de
  alianzas/conflictos y evidencia opcional con escena.
- Detección IA de afiliaciones con listas cerradas, cita validada y revisión
  por el autor (`extract_affiliations`).
- Panel **Vínculos** para CRUD manual y revisión de sugerencias.
- Grafo con facciones y lugares, filtros por tipo, inspector, ego network,
  agrupación por facción y presencia derivada de `scene.location`/texto.
- Duplicar y restaurar proyectos conserva afiliaciones y relaciones.
- **Aceptación:** se ven aristas personaje→facción/lugar y clusters por
  facción; la presencia derivada se distingue de los vínculos manuales. ✔

### Fase 4 — Pulido

- Exportar grafo (SVG/PNG), accesibilidad, rendimiento, documentación.
- Timeline/cronología (opcional).

---

## 8. Riesgos y mitigaciones

| Riesgo | Mitigación |
| --- | --- |
| El LLM alucina | Evidencia obligatoria + confianza + manual gana + “sin evidencia” |
| Grafo ilegible (“hairball”) | Filtros, clusters por facción, ego network, etiquetas al hover |
| Muchos nodos | Límite 250, agrupar, canvas si hace falta |
| Modelo local flojo | JSON validado, campos vacíos permitidos, nube opcional |
| Migración de datos | Tablas aditivas; no se borran columnas existentes |
| Pérdida de ediciones manuales | `source = manual` intocable por la IA |
| Presencia derivada mal interpretada | Arista punteada, `origin = derived` y texto «presencia (derivada)»; se calcula por escena y se limita a 5 lugares por personaje |
| Falsos positivos de nombres («Ana» en «Anabel») | Coincidencia por límites de palabra y alias de al menos 3 caracteres |
| Duplicados simétricos de alianzas | Canonicalización `a < b` e índice parcial por pareja |

---

## 9. Decisiones abiertas

1. **Built-in vs EAV:** mantener built-in como columnas y custom en EAV
   (recomendado).
2. **Librería de grafo:** `d3-force` + SVG por control y tamaño; `cytoscape` si
   se necesita más interacción.
3. **Tipos de campo iniciales:** text, longtext, list, number.
4. **Afiliaciones:** IA + manual; la presencia en lugares puede salir de
   `scene.location` además del LLM.
