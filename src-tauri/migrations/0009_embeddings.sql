-- Índice semántico de escenas para el chat con contexto (RAG).
-- Guarda el vector de embedding de cada escena como JSON.
CREATE TABLE scene_embeddings (
    project_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    embedding TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, scene_id)
);
