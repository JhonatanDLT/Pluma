-- 0006_word_goal.sql — Objetivo de palabras por proyecto.

ALTER TABLE project ADD COLUMN word_goal INTEGER NOT NULL DEFAULT 3000;
