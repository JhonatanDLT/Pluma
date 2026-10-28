-- 0011_character_fields.sql — Campos de personaje definidos por el autor y
-- valores con procedencia (manual | IA), confianza y evidencia.
--
-- Los campos base (name, aliases, role, description, biography, traits, goals,
-- arc, color) siguen viviendo en columnas de `character`; sus definiciones
-- builtin solo sirven para adjuntar propuestas/evidencia y para que la UI los
-- trate igual que los personalizados.

CREATE UNIQUE INDEX IF NOT EXISTS idx_character_project_ident
    ON character(project_id, id);

CREATE TABLE IF NOT EXISTS character_field_def (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    key          TEXT NOT NULL,
    label        TEXT NOT NULL,
    field_type   TEXT NOT NULL DEFAULT 'text'
                 CHECK (field_type IN ('text','longtext','list','number')),
    options_json TEXT,
    prompt_hint  TEXT,
    position     INTEGER NOT NULL DEFAULT 0,
    builtin      INTEGER NOT NULL DEFAULT 0 CHECK (builtin IN (0,1)),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    UNIQUE (project_id, key),
    UNIQUE (project_id, id)
);

CREATE INDEX IF NOT EXISTS idx_cfd_project_position
    ON character_field_def(project_id, position);

CREATE TABLE IF NOT EXISTS character_field_value (
    project_id   TEXT NOT NULL,
    character_id TEXT NOT NULL,
    field_id     TEXT NOT NULL,
    value_json   TEXT NOT NULL,
    source       TEXT NOT NULL DEFAULT 'manual'
                 CHECK (source IN ('manual','ai')),
    confidence   REAL CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
    evidence     TEXT,
    reason       TEXT,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (character_id, field_id),
    FOREIGN KEY (project_id, character_id)
        REFERENCES character(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, field_id)
        REFERENCES character_field_def(project_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_cfv_field   ON character_field_value(field_id);
CREATE INDEX IF NOT EXISTS idx_cfv_project ON character_field_value(project_id);

-- Siembra los campos base en proyectos existentes.
INSERT OR IGNORE INTO character_field_def
    (id, project_id, key, label, field_type, position, builtin, created_at, updated_at)
SELECT 'builtin:' || p.id || ':' || d.key,
       p.id, d.key, d.label, d.field_type, d.position, 1,
       strftime('%Y-%m-%dT%H:%M:%fZ','now'),
       strftime('%Y-%m-%dT%H:%M:%fZ','now')
FROM project p
CROSS JOIN (
    SELECT 'name'        AS key, 'Nombre'      AS label, 'text'     AS field_type, 0 AS position
    UNION ALL SELECT 'aliases',     'Alias',       'list',     1
    UNION ALL SELECT 'role',        'Rol',         'text',     2
    UNION ALL SELECT 'description', 'Descripción', 'longtext', 3
    UNION ALL SELECT 'biography',   'Biografía',   'longtext', 4
    UNION ALL SELECT 'traits',      'Rasgos',      'list',     5
    UNION ALL SELECT 'goals',       'Objetivos',   'list',     6
    UNION ALL SELECT 'arc',         'Arco',        'longtext', 7
    UNION ALL SELECT 'color',       'Color',       'text',     8
) AS d;
