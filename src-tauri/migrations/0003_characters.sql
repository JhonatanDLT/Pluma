-- 0003_characters.sql — Personajes y relaciones (asistidos por IA local).

CREATE TABLE IF NOT EXISTS character (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    aliases_json TEXT,
    role        TEXT,
    description TEXT,
    biography   TEXT,
    traits_json TEXT,
    goals_json  TEXT,
    arc         TEXT,
    color       TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS relationship (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    character_a_id      TEXT NOT NULL REFERENCES character(id) ON DELETE CASCADE,
    character_b_id      TEXT NOT NULL REFERENCES character(id) ON DELETE CASCADE,
    kind                TEXT,
    intensity           REAL NOT NULL DEFAULT 0,
    description         TEXT,
    source_evidence_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_character_project    ON character(project_id);
CREATE INDEX IF NOT EXISTS idx_relationship_project ON relationship(project_id);
