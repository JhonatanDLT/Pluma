-- 0001_init.sql — Esquema inicial de Pluma.
-- Las migraciones se aplican de forma versionada (PRAGMA user_version) y son idempotentes.

CREATE TABLE IF NOT EXISTS project (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    author      TEXT,
    description TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chapter (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    title      TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'draft',
    summary    TEXT,
    word_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS scene (
    id               TEXT PRIMARY KEY,
    chapter_id       TEXT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    position         INTEGER NOT NULL,
    title            TEXT,
    content_markdown TEXT NOT NULL DEFAULT '',
    summary          TEXT,
    word_count       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chapter_project ON chapter(project_id, position);
CREATE INDEX IF NOT EXISTS idx_scene_chapter   ON scene(chapter_id, position);
