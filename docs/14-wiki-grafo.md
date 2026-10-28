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
    character_id TEXT NOT NULL REFERENCES character(id) ON DELETE CASCADE,
    entity_id    TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    role         TEXT,                      -- member | leader | ally | rival
    source       TEXT NOT NULL DEFAULT 'manual',
    confidence   REAL,
    evidence     TEXT,
    PRIMARY KEY (character_id, entity_id, role)
);

CREATE TABLE entity_relation (
    a_entity_id TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    b_entity_id TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    kind        TEXT,                       -- territory | alliance | conflict
    source      TEXT NOT NULL DEFAULT 'manual',
    confidence  REAL,
    evidence    TEXT,
    PRIMARY KEY (a_entity_id, b_entity_id, kind)
);
```

- `character_affiliation` une personaje → facción/lugar (según `entity_type`).
- `entity_relation` une facción ↔ lugar (territorio) o facción ↔ facción.
- Las relaciones personaje-personaje siguen en `relationship`; se puede usar
  `source_evidence_json` (ya existe) para evidencia/confianza.

---

## 4. Grafo

### 4.1 Datos

Comando `wiki_graph(project_id)`:

```ts
type GraphNode = {
  id: string;
  kind: "character" | "faction" | "location";
  label: string;
  mentions: number;
  role?: string | null;
  factionId?: string | null;
  locationId?: string | null;
};

type GraphEdge = {
  id: string;
  source: string;
  target: string;
  kind: "relationship" | "affiliation" | "presence" | "territory";
  subtype?: string | null;   // mentor, rival, ally…
  weight: number;            // intensidad / coocurrencias
  evidence?: string | null;
};

type GraphData = { nodes: GraphNode[]; edges: GraphEdge[]; stats: {...} };
```

- **Personajes:** tamaño por menciones; color por facción; borde por rol.
- **Facciones/lugares:** nodos más grandes, color propio.
- **Aristas:**
  - `relationship`: personaje ↔ personaje (grosor por intensidad).
  - `affiliation`: personaje → facción (rol: líder, miembro, aliado, rival).
  - `presence`: personaje → lugar (si hay evidencia en escenas o IA).
  - `territory`: facción → lugar.

### 4.2 UI (`WorldGraph.svelte`)

- Simulación **force-directed** con `d3-force` + SVG (o canvas si hay > 250
  nodos).
- Zoom/pan, arrastrar nodos, etiquetas al hacer hover, búsqueda.
- Filtros:
  - por facción, lugar, tipo de relación y rol;
  - “solo el entorno de este personaje” (ego network) al doble clic;
  - “ocultar lugares” / “ocultar facciones” para reducir ruido.
- Clic en nodo → abre la ficha existente en el panel de detalle.
- Botón “Agrupar por facción”: layout con clusters alrededor del nodo facción.
- Estado vacío: “Analiza el manuscrito para construir el grafo”.
- Fallback accesible: lista de relaciones y tabla de nodos para lectores de
  pantalla.

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

### 5.3 Campos personalizados

- El autor crea “Color de ojos” con hint “solo si se menciona explícitamente”.
- El prompt incluye ese campo; el LLM lo rellena solo si encuentra una cita.
- Si el autor añade un campo después, puede re-enriquecer solo ese personaje.
- Los campos built-in se pueden ocultar o renombrar en la UI, pero su `key`
  interna no cambia.

### 5.4 Modelo

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
| `wiki_graph(project_id)` | Nodos y aristas |
| `list_affiliations(project_id)` / `set_affiliation(...)` | Afiliaciones |
| `set_entity_relation(...)` | Territorio/alianzas |

---

## 7. Fases

### Fase 1 — Grafo MVP (✅ implementado)

- `WorldGraph.svelte`: force layout (`d3-force`), búsqueda, etiquetas, zoom,
  pan, arrastre de nodos y clic para abrir la ficha.
- Usa los `characters` y `relationships` ya cargados en World Building; sin
  cambios de esquema ni consultas extra.
- Pestaña **Grafo** dentro de World Building.
- Nodos dimensionados por grado de relación; protagonistas en color de acento.
- **Pendiente para la Fase 3:** nodos de facciones/lugares, afiliaciones,
  presencia y clustering por facción (requiere `character_affiliation`).

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

### Fase 3 — Afiliaciones y grafo enriquecido (1 sprint)

- Migración `0012`; extracción de afiliaciones y territorio.
- Grafo agrupado por facción, ego network, presencia en lugares.
- **Aceptación:** clusters de facción y aristas personaje-facción visibles.

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

---

## 9. Decisiones abiertas

1. **Built-in vs EAV:** mantener built-in como columnas y custom en EAV
   (recomendado).
2. **Librería de grafo:** `d3-force` + SVG por control y tamaño; `cytoscape` si
   se necesita más interacción.
3. **Tipos de campo iniciales:** text, longtext, list, number.
4. **Afiliaciones:** IA + manual; la presencia en lugares puede salir de
   `scene.location` además del LLM.
