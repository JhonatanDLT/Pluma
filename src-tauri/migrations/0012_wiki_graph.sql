-- 0012_wiki_graph.sql — Afiliaciones y relaciones del mundo.
--
-- Une personajes con facciones/lugares y entidades entre sí. La presencia en
-- lugares se deriva de las escenas y no se persiste aquí.
--
-- Invariantes:
--  * Un personaje tiene como máximo un rol por entidad.
--  * `source = manual` nunca se sobrescribe desde la IA.
--  * Las relaciones alliance/conflict son simétricas y se canonicalizan con
--    a_entity_id < b_entity_id; territory es direccional (facción → lugar).
--  * Los roles y tipos se validan en Rust porque dependen de entity_type.

CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_project_ident
    ON entity(project_id, id);

CREATE TABLE IF NOT EXISTS character_affiliation (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    character_id      TEXT NOT NULL,
    entity_id         TEXT NOT NULL,
    role              TEXT NOT NULL
                      CHECK (role IN ('leader','member','ally','rival',
                                      'resident','origin','visitor')),
    source            TEXT NOT NULL DEFAULT 'manual'
                      CHECK (source IN ('manual','ai')),
    confidence        REAL
                      CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
    evidence          TEXT,
    evidence_scene_id TEXT REFERENCES scene(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    UNIQUE (character_id, entity_id),
    FOREIGN KEY (project_id, character_id)
        REFERENCES character(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, entity_id)
        REFERENCES entity(project_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_affiliation_project_character
    ON character_affiliation(project_id, character_id);
CREATE INDEX IF NOT EXISTS idx_affiliation_project_entity
    ON character_affiliation(project_id, entity_id);

CREATE TABLE IF NOT EXISTS entity_relation (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    a_entity_id       TEXT NOT NULL,
    b_entity_id       TEXT NOT NULL,
    kind              TEXT NOT NULL
                      CHECK (kind IN ('alliance','conflict','territory')),
    source            TEXT NOT NULL DEFAULT 'manual'
                      CHECK (source IN ('manual','ai')),
    confidence        REAL
                      CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
    evidence          TEXT,
    evidence_scene_id TEXT REFERENCES scene(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    CHECK (a_entity_id <> b_entity_id),
    CHECK (kind = 'territory' OR a_entity_id < b_entity_id),
    UNIQUE (a_entity_id, b_entity_id, kind),
    FOREIGN KEY (project_id, a_entity_id)
        REFERENCES entity(project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, b_entity_id)
        REFERENCES entity(project_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_entity_relation_project_a
    ON entity_relation(project_id, a_entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_relation_project_b
    ON entity_relation(project_id, b_entity_id);

-- Una pareja no puede ser aliada y enemiga a la vez.
CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_relation_pair
    ON entity_relation(a_entity_id, b_entity_id)
    WHERE kind IN ('alliance','conflict');
