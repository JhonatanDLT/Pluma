-- 0002_metadata.sql — Metadatos adicionales de escenas.

ALTER TABLE scene ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';
ALTER TABLE scene ADD COLUMN pov TEXT;
ALTER TABLE scene ADD COLUMN location TEXT;
ALTER TABLE scene ADD COLUMN time_label TEXT;
