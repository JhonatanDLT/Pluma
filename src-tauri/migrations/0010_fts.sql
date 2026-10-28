-- 0010_fts.sql — Búsqueda full-text (FTS5) sobre el contenido de las escenas.

CREATE VIRTUAL TABLE IF NOT EXISTS scene_fts USING fts5(
    scene_id UNINDEXED,
    chapter_id UNINDEXED,
    project_id UNINDEXED,
    title,
    content
);

-- Siembra el índice con las escenas existentes (en una base nueva no inserta nada).
INSERT INTO scene_fts (scene_id, chapter_id, project_id, title, content)
SELECT s.id, s.chapter_id, c.project_id, COALESCE(s.title, ''), s.content_markdown
FROM scene s
JOIN chapter c ON c.id = s.chapter_id;

-- Mantiene el índice sincronizado con la tabla `scene`.
CREATE TRIGGER IF NOT EXISTS scene_fts_ai AFTER INSERT ON scene BEGIN
    INSERT INTO scene_fts (scene_id, chapter_id, project_id, title, content)
    SELECT new.id, new.chapter_id, c.project_id, COALESCE(new.title, ''), new.content_markdown
    FROM chapter c WHERE c.id = new.chapter_id;
END;

CREATE TRIGGER IF NOT EXISTS scene_fts_ad AFTER DELETE ON scene BEGIN
    DELETE FROM scene_fts WHERE scene_id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS scene_fts_au AFTER UPDATE ON scene BEGIN
    DELETE FROM scene_fts WHERE scene_id = old.id;
    INSERT INTO scene_fts (scene_id, chapter_id, project_id, title, content)
    SELECT new.id, new.chapter_id, c.project_id, COALESCE(new.title, ''), new.content_markdown
    FROM chapter c WHERE c.id = new.chapter_id;
END;
