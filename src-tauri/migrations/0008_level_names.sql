-- 0008_level_names.sql — Nombres configurables de los niveles (capítulos/escenas, volúmenes/capítulos, …).

ALTER TABLE project ADD COLUMN level1_name TEXT NOT NULL DEFAULT 'Capítulos';
ALTER TABLE project ADD COLUMN level2_name TEXT NOT NULL DEFAULT 'Escenas';
