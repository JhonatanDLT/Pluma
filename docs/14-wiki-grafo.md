# 14 — Wiki, grafo y campos enriquecidos

Plan de evolución de la Biblia de la novela: pasar de una lista de fichas a un
**mapa visual del mundo** y a un sistema de **campos definidos por el autor** que
la IA pueda rellenar con evidencia, sin inventar.

---

## 1. Estado actual y límites

La base ya tiene:

- `character`: nombre, alias, rol, descripción, biografía, rasgos, objetivos,
  arco y color.
- `entity`: lugares y facciones con alias, descripción y menciones.
- `relationship`: personaje A ↔ personaje B, tipo, intensidad y descripción.
- `enrich_project`: pide al LLM una ficha y actualiza los campos del personaje.

Problemas:

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

### 3.2 Campos personalizados (migración `0011_wiki_fields.sql`)

```sql
CREATE TABLE character_field_def (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,              -- slug estable, p. ej. "color_ojos"
    label       TEXT NOT NULL,              -- "Color de ojos"
    field_type  TEXT NOT NULL DEFAULT 'text', -- text | longtext | list | number | date | enum
    options_json TEXT,                      -- para enum
    prompt_hint TEXT,                       -- "solo si se menciona explícitamente"
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE(project_id, key)
);

CREATE TABLE character_field_value (
    character_id TEXT NOT NULL REFERENCES character(id) ON DELETE CASCADE,
    field_id     TEXT NOT NULL REFERENCES character_field_def(id) ON DELETE CASCADE,
    value_json   TEXT NOT NULL,             -- string, lista o número serializado
    source       TEXT NOT NULL DEFAULT 'manual', -- manual | ai
    confidence   REAL,                      -- 0..1 cuando source = ai
    evidence     TEXT,                      -- cita del manuscrito
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (character_id, field_id)
);
```

Reglas:

- `source = manual` **nunca** se sobrescribe con una sugerencia `ai`.
- Las sugerencias de IA se guardan con `source = ai` y se pueden aceptar
  (pasan a `manual`) o descartar.
- `key` es estable y no cambia al renombrar `label`.
- `field_type` empieza con `text`, `longtext`, `list`, `number`; `enum`/`date`
  en una fase posterior.

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

- Local: Qwen 1.5B/4B en modo JSON, temperatura 0.
- Nube: opcional, con consentimiento; mismo esquema.
- Validación y evidencia son obligatorias, independientemente del modelo.

---

## 6. API y comandos

| Comando | Uso |
| --- | --- |
| `list_field_defs(project_id)` | Definiciones de campos |
| `create_field_def(...)` / `update_field_def` / `delete_field_def` | CRUD |
| `list_character_fields(character_id)` | Valores built-in + custom |
| `set_character_field(character_id, field_id, value)` | Edición manual |
| `enrich_character(project_id, character_id, model)` | Sugerencias con evidencia |
| `enrich_project(project_id, model)` | Sugerencias para todos |
| `wiki_graph(project_id)` | Nodos y aristas |
| `list_affiliations(project_id)` / `set_affiliation(...)` | Afiliaciones |
| `set_entity_relation(...)` | Territorio/alianzas |

---

## 7. Fases

### Fase 1 — Grafo MVP (1 sprint)

- `wiki_graph` con datos existentes: personajes, relaciones, facciones, lugares.
- `WorldGraph.svelte`: force layout, filtros, clic a ficha, zoom/pan.
- Sin cambios de esquema.
- **Aceptación:** el grafo se dibuja, filtra y abre fichas; 100+ nodos fluido.

### Fase 2 — Campos personalizados + IA con evidencia (1–2 sprints)

- Migración `0011`; definiciones y valores; editor de campos en la ficha.
- Prompt extendido (built-in + custom), validación, evidencia y confianza.
- Sugerencias aceptar/descartar; manual gana.
- **Aceptación:** el autor añade un campo, la IA lo rellena con cita, y lo
  manual sobrevive a un reanálisis.

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
