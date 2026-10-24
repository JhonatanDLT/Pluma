-- 0004_entities.sql — Entidades no humanas (lugares, facciones, …).

CREATE TABLE IF NOT EXISTS entity (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    entity_type   TEXT NOT NULL,
    aliases_json  TEXT,
    description   TEXT,
    mention_count INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entity_project ON entity(project_id);
