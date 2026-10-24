-- 0005_snapshots.sql — Versionado ligero (snapshots) y huella de análisis.

ALTER TABLE project ADD COLUMN analysis_fingerprint TEXT;

CREATE TABLE IF NOT EXISTS snapshot (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    message     TEXT,
    fingerprint TEXT NOT NULL,
    scene_count INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS snapshot_scene (
    snapshot_id      TEXT NOT NULL REFERENCES snapshot(id) ON DELETE CASCADE,
    scene_id         TEXT NOT NULL,
    chapter_id       TEXT NOT NULL,
    position         INTEGER NOT NULL,
    title            TEXT,
    content_markdown TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, scene_id)
);

CREATE INDEX IF NOT EXISTS idx_snapshot_project ON snapshot(project_id);
