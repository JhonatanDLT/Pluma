# Pluma V1.0 — Development Task List

> Companion to [`docs/09-roadmap-v1.md`](../../../docs/09-roadmap-v1.md).
> Source of truth for priorities: `docs/09`. Source of truth for requirements:
> `docs/01-requisitos.md`; product vision: `docs/08-plan-maestro.md`.
>
> Last audit: 2026-10-28. `pnpm check` clean; `cargo test --lib` 58 passed, 2 ignored.

## Specification summary

**Product:** desktop editor for long-form Spanish-language fiction (novels/sagas),
offline-first, Svelte 5 + Tauri v2 + Rust + SQLite, hybrid TipTap/CodeMirror editor,
automatic chapter/scene structure, optional local AI for characters/relations, system TTS,
and export to Markdown/TXT/HTML/DOCX/PDF/EPUB.

**V1.0 promise:** write offline, never lose a word, keep a story bible the author can
correct, export to publishing formats, AI optional and correctable.

**Non-negotiable constraints (from `docs/08`):**

- Offline-first: 100% of the writing flow works without internet.
- No mandatory AI: the app is fully useful without any model.
- Privacy: the manuscript never leaves the device by default.
- Lightweight: CPU-only, 4–8 GB RAM target.
- Markdown is the canonical interchange format.
- No collaboration/sync/proprietary format in V1.

**Current state:** feature-broad, demo-ready, not release-ready. 52 commands, 9
migrations, ~14.5k LOC. Critical gaps: destructive analysis, unguarded deletes/import,
save-path data loss, no external backup, read-only bible, no release infrastructure.

---

## Sprint 1 — Stop the bleeding (trust spine)

**Goal:** no normal operation can destroy the manuscript or the bible.
**Duration:** 2 weeks (or 3 weeks solo).
**Exit:** data-loss scenarios pass; `cargo test` + `pnpm check` green.

### [ ] 1.1 Save integrity and flush barriers

**Description:** Make saving deterministic and visible.

- Add `Ctrl/Cmd+S` manual save.
- Replace `saveState` string with a state machine: `idle | dirty | saving | saved | error`.
- Retry failed saves with backoff; keep content in memory; block scene/project navigation
  while `error` with a persistent banner + "Reintentar" / "Copiar texto" actions.
- Flush before: scene switch, project switch, export, analysis, status change, window
  close/quit.
- Show "Sin guardar" when dirty; do not show "Guardado" for another scene.

**Acceptance criteria:**

- Typing then immediately exporting includes the latest text.
- Simulated save failure → switching scene is blocked; retry works; no content lost.
- Closing the window with unsaved changes flushes or prompts.
- `Cmd/Ctrl+S` saves and confirms.

**Files:** `src/routes/+page.svelte`, `src/lib/api.ts`, possibly a `save` store.
**Estimate:** 2d.

---

### [ ] 1.2 Destructive-action safety

**Description:** Guard every operation that can erase work.

- Project delete: modal requiring the project title; soft delete (`deleted_at`) + trash.
- Chapter/scene delete: confirmation + trash/restore.
- Import into existing project: explicit choice — **Reemplazar manuscrito** /
  **Añadir capítulos al final** / **Crear proyecto nuevo**; preview chapter count; auto-backup
  before replace.
- Character/entity delete: confirmation + undo.
- Add "Papelera" with restore for 30 days.

**Acceptance criteria:**

- A misclick on project delete cannot destroy the project without typing its title.
- Deleted project/chapter/scene can be restored from trash.
- Import replace always creates a backup first; append does not delete existing chapters.
- No `DELETE` path is reachable from a single unconfirmed click.

**Files:** `src/lib/components/ProjectsView.svelte`, `src/routes/+page.svelte`,
`src-tauri/src/commands.rs`, `src-tauri/src/repo.rs`, new migration
(`deleted_at` columns or trash table).
**Estimate:** 1.5d.

---

### [ ] 1.3 Non-destructive, scope-aware analysis

**Description:** Replace the current `replace_*` behavior with reconciliation.

- Add stable canonical keys for characters/entities (normalized name + aliases).
- Add provenance fields: `origin` (`rules|gliner|llm|manual`), `locked`, `last_seen_at`,
  `stale`.
- Chapter-scoped analysis: merge mentions into the project graph; never delete entities
  outside the scope.
- Full-project analysis: upsert; mark missing as `stale`; never delete automatically.
- Preserve `description`, `biography`, `traits`, `goals`, `arc`, `color` on re-analysis
  unless the field is empty and `origin != manual`.
- Preserve relationship type/intensity when re-analyzing.
- Keep IDs stable across runs.
- Add tests: chapter-2 analysis keeps chapter-1 characters; re-analysis preserves
  enrichment; manual edit survives; no duplicate rows.

**Acceptance criteria:**

- Analyzing one chapter does not reduce the project bible to that chapter.
- Re-running analysis does not lose enriched profiles.
- Character/relationship IDs do not churn on every run.
- `cargo test` covers the scenarios above.

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/repo.rs`, `src-tauri/src/models.rs`,
`src-tauri/src/nlp.rs`, new migration `0010_analysis_reconcile.sql`.
**Estimate:** 4d.

---

### [ ] 1.4 External backup, restore and bible snapshots

**Description:** Make disaster recovery real.

- Daily auto-backup to a user-visible `backups/` folder; keep 7–14 copies.
- Manual "Exportar copia de seguridad" → zip with `manuscript.md`, `bible.json`,
  `project.json`, `snapshots/`, `settings.json` (no API keys).
- "Restaurar copia" validates and imports as a **new project**, never silently overwrites.
- Extend snapshots (or add a separate version table) to include characters,
  relationships, entities and project settings.
- Add "Abrir carpeta de datos" in settings.

**Acceptance criteria:**

- Daily backup appears without user action.
- Restore on a clean app profile produces a working project with bible and snapshots.
- A corrupted/deleted DB is recoverable from the backup folder.
- API keys are never present in backup files.

**Files:** `src-tauri/src/db.rs`, `src-tauri/src/repo.rs`, `src-tauri/src/commands.rs`,
`src-tauri/src/lib.rs`, `src/lib/components/OptionsPanel.svelte` (settings UI).
**Estimate:** 2d.

---

### [ ] 1.5 Regression tests + docs truth pass

**Description:** Lock the fixes and correct the docs.

- Add regression tests for 1.1–1.4.
- Update README and `docs/06`/`docs/08` to remove claims that are not implemented
  (crash recovery, editable ficha, Ctrl+S, TTS pause, project duplicate).
- Add a "Known limitations" section.
- Mark design-intent docs (`docs/02`, `docs/05`) as aspirational where they differ from
  migrations.

**Acceptance criteria:**

- Tests fail if the destructive behavior returns.
- A new contributor can tell what is shipped vs planned.
- README claims match the code.

**Estimate:** 0.5d.

---

## Sprint 2 — Own the bible + writing polish

**Goal:** every AI output is correctable; the writing loop feels complete.
**Duration:** 2 weeks (or 3 weeks solo).

### [ ] 2.1 Editable and mergeable story bible

**Description:** Turn the read-only world view into the author's tool.

- Edit character fields: name, aliases, role, description, biography, traits, goals, arc,
  color, notes.
- Edit entity fields: name, aliases, type, description.
- Merge duplicates: choose canonical, merge aliases, relationships, mentions.
- Dismiss/ignore suggestions; keep them from reappearing.
- Review queue: "Nuevos", "Posibles duplicados", "Ya no aparecen".
- Relationship editing: type, intensity, description, delete.
- Provenance badges (`auto`/`manual`) and field locks.
- Persist the change log (currently in memory).

**Acceptance criteria:**

- All fields persist across restart.
- Manual edits survive re-analysis.
- Merge preserves relationships and aliases; no orphan rows.
- Dismissed character does not reappear as new.
- Review queue is usable with 40+ characters.

**Files:** `src/lib/components/WorldBuilding.svelte`,
`src/lib/components/MetadataPanel.svelte` (or a new character editor),
`src/routes/+page.svelte`, `src-tauri/src/commands.rs`, `src-tauri/src/repo.rs`,
`src-tauri/src/models.rs`, migration for provenance/log.
**Estimate:** 4d.

---

### [ ] 2.2 Crash-recovery journal + automatic snapshots

**Description:** Recover work after a crash or forced quit.

- Write a lightweight draft journal per scene (e.g., `drafts` table or app-data file)
  every ~2 s while dirty.
- On open, compare scene vs draft; offer "Recuperar" / "Descartar".
- Clear the draft after a successful save.
- Auto-snapshot before import, restore, bulk analysis, delete and project switch with
  unsaved changes.
- Retention: last 20 + one per day for 14 days.

**Acceptance criteria:**

- Force-quit during typing → reopen → at most ~2 s lost; recovery prompt appears.
- Draft is cleared after save.
- Auto-snapshots rotate and do not grow without bound.

**Files:** `src-tauri/src/db.rs`, `src-tauri/src/repo.rs`,
`src-tauri/src/commands.rs`, `src/routes/+page.svelte`, migration.
**Estimate:** 2d.

---

### [ ] 2.3 TTS controls and clean text

**Description:** Make proof-listening usable.

- Voice picker from the OS (macOS `say -v ?`, Windows installed voices, Linux
  `espeak-ng --voices`).
- Rate slider mapped to the platform.
- Pause/resume (sentence-level chunking is acceptable).
- Strip Markdown syntax before speaking; read prose only.
- Do not `killall say` globally; stop only the app's process.

**Acceptance criteria:**

- Voice and rate changes take effect.
- Pause/resume works without restarting the chapter.
- `#`, `**`, links and image syntax are not read aloud.
- Stop does not affect other apps' speech.

**Files:** `src-tauri/src/tts.rs`, `src-tauri/src/commands.rs`,
`src/routes/+page.svelte`, `src/lib/api.ts`.
**Estimate:** 2d.

---

### [ ] 2.4 Export/import fidelity suite

**Description:** Prove that writers can ship their manuscript.

- EPUB export → import → verify chapters, scenes, titles, order.
- DOCX/PDF golden-file tests with headings, emphasis, lists, tables, images, cover.
- Sanitize raw HTML in Markdown before export (pulldown-cmark passes raw HTML through).
- Verify cover in DOCX/PDF/EPUB (already implemented; add regression).
- Test large-project export memory/time.

**Acceptance criteria:**

- Automated EPUB round-trip passes.
- Golden files open correctly in Word/Preview/Calibre.
- No script/raw HTML injection in exported HTML/EPUB.
- Cover renders in all three formats.

**Files:** `src-tauri/src/export.rs`, `src-tauri/src/extract.rs`, tests/fixtures.
**Estimate:** 2d.

---

### [ ] 2.5 Project duplicate + first-run onboarding

**Description:** Complete FR-04 and reduce activation friction.

- Duplicate project: copy chapters, scenes, characters, relationships, entities, settings.
- First-run flow: create / import / open sample.
- Local-AI explainer with download sizes and "works without AI" message.
- Backup folder prompt.
- Empty states and a 3-step "write your first scene" guide.

**Acceptance criteria:**

- Duplicate produces an independent project (edits do not leak).
- New user can create/import and write 500 words in under 2 minutes without docs.
- AI is clearly optional; no surprise downloads.

**Files:** `src/lib/components/ProjectsView.svelte`, `src/routes/+page.svelte`,
`src-tauri/src/commands.rs`, `src-tauri/src/repo.rs`, new onboarding component.
**Estimate:** 1.5d.

---

## Sprint 3 — Release candidate

**Goal:** prove the promise at scale and ship a signed, private, supportable build.
**Duration:** 2 weeks (or 3 weeks solo; signing lead time starts week 1).

### [ ] 3.1 500k-word performance benchmark

**Description:** Build a synthetic saga fixture and measure the claims.

- Generate 500k words, 40+ characters, ~1,500 scenes.
- Measure: project open, scene switch, typing latency, full analysis (background),
  export, search.
- Fix the top bottleneck only; keep analysis incremental/background.
- Check in the benchmark script and document results in README.

**Acceptance criteria:**

- Open <2 s; scene switch <150 ms; typing <50 ms; export <15 s; search <300 ms (or a
  documented limitation).
- Benchmark reproducible on a mid-range laptop.
- No UI freeze during analysis.

**Files:** `scripts/benchmark` (new), `src-tauri/src/repo.rs`, `src-tauri/src/nlp.rs`,
`src-tauri/src/rag.rs`, README.
**Estimate:** 3d.

---

### [ ] 3.2 Privacy, consent and security baseline

**Description:** Align the product with its privacy promise.

- Local-only default; no network calls in the core loop.
- Cloud AI: explicit consent modal (provider, model, data sent); persistent badge;
  never auto-select because a key exists.
- Set a CSP in `tauri.conf.json` (replace `null`).
- Block remote images in imported Markdown by default; allowlist with consent.
- Sanitize raw HTML before rendering/exporting.
- Verify API keys only in the OS keychain; no plaintext fallback.
- Add a "What leaves this device" page.

**Acceptance criteria:**

- Fresh install works fully offline.
- Cloud AI requires explicit consent; badge visible while active.
- CSP set; remote images blocked.
- No plaintext key on disk.

**Files:** `src-tauri/tauri.conf.json`, `src-tauri/src/llm.rs`,
`src-tauri/src/secrets.rs`, `src-tauri/src/settings.rs`,
`src/lib/components/AiSettings.svelte`, `src/lib/markdown.ts`.
**Estimate:** 2d.

---

### [ ] 3.3 Release engineering

**Description:** Make the build shippable.

- Version 1.0.0 in `package.json` and `tauri.conf.json`.
- `LICENSE`, privacy policy, EULA, third-party notices (models, dictionaries, fonts,
  crates).
- CI: GitHub Actions for macOS + Windows; `pnpm check`, `cargo test`,
  `cargo clippy -D warnings`, build bundles.
- macOS Developer ID signing + notarization; Windows installer signing.
- Clean-VM install → create/import → export → uninstall/reinstall.
- DB migration from a 0.1 database.
- Crash logging with rotation; "open logs"; no manuscript content.
- Update check (not silent auto-update).
- Support channel and user guide.

**Acceptance criteria:**

- Signed artifacts install without Gatekeeper/SmartScreen warnings.
- CI is green on both platforms.
- 0.1 DB migrates without data loss.
- Logs exist and contain no manuscript text.

**Files:** `.github/workflows/`, `src-tauri/tauri.conf.json`, `package.json`,
`LICENSE`, `PRIVACY.md`, `CHANGELOG.md`, `src-tauri/src/lib.rs`.
**Estimate:** 3.5d + procurement lead time.

---

### [ ] 3.4 AI evaluation and failure UX

**Description:** Make AI quality measurable and failures graceful.

- Golden set: 8–10 Spanish literary chapters across genres, with gold entities/relations.
- Measure precision/recall, duplicate rate, relationship-type accuracy, latency.
- Fix only the top gaps: RAG relevance threshold, pair-level context for relationship
  typing, model download failure/cancel/resume.
- Ensure every model failure falls back to rules with a clear message.

**Acceptance criteria:**

- Metrics recorded in the repo.
- No regression vs the previous run.
- Cancelled/failed download leaves the app usable.
- RAG context excludes low-similarity scenes.

**Files:** `src-tauri/src/rag.rs`, `src-tauri/src/llm.rs`,
`src-tauri/src/nlp.rs`, `src-tauri/src/runtime.rs`, `scripts/eval` (new).
**Estimate:** 2d.

---

### [ ] 3.5 Accessibility, error UX and docs

**Description:** Polish the launch experience.

- Keyboard navigation for the main workspace; visible focus states; contrast check.
- Actionable Spanish error messages (no raw Rust errors).
- User guide: import, backup, AI opt-in, export, recovery.
- Update README screenshots/claims.

**Acceptance criteria:**

- Main flow is keyboard-only.
- Errors tell the user what to do.
- Docs match the shipped build.

**Estimate:** 1d.

---

### [ ] 3.6 RC buffer / beta triage

**Description:** Reserve time for beta feedback and release blockers.

- Onboard 20–50 writers.
- Triage by data-loss/security first, then core loop, then polish.
- No new features.
- Public release only after one week with no P0.

**Estimate:** 1d + 1–2 weeks beta.

---

## Quality requirements (apply to every task)

- [ ] No background processes in commands; no `&`.
- [ ] No server startup commands; assume the dev server is running.
- [ ] `pnpm check` 0 errors/warnings.
- [ ] `cargo test` green; add tests for every data-integrity fix.
- [ ] `cargo clippy -D warnings` green.
- [ ] All destructive operations have confirmation + recovery.
- [ ] No AI action can overwrite manual data.
- [ ] All UI strings in Spanish; error messages actionable.
- [ ] No network calls in the core loop.
- [ ] Update `docs/` when behavior changes.
- [ ] Performance: no UI thread blocking; analysis background/cancellable.
- [ ] Accessibility: keyboard reachable, focus visible.
- [ ] Before release: a smoke E2E against the built app (Tauri WebDriver or equivalent)
  covering create → write → restart → export → import.

## Definition of done for V1.0

- [ ] G1 Data safety: no unguarded destructive action; recovery/backup tested.
- [ ] G2 Bible ownership: editable/mergeable; manual edits survive analysis/restart.
- [ ] G3 Core loop: create/import → write → export in <10 min; EPUB round-trip.
- [ ] G4 Performance: 500k benchmark meets targets.
- [ ] G5 Distribution: signed installers, CI, license/privacy, crash logs, migration.
- [ ] Beta: 20–50 writers, crash-free ≥99.5%, no P0 for one week.
- [ ] Docs/README accurate.
- [ ] Support channel and rollback plan ready.

## Explicitly out of scope for V1.0

- Piper neural TTS / per-character voices.
- Laya relation classifier.
- Timeline/cronología, artifacts, research notes.
- Relationship graph visualization.
- Global FTS5 search (first post-launch candidate).
- Split view, focus mode, command palette, configurable shortcuts.
- Daily goals/streaks/stats.
- Comments/margin notes.
- Selective export/templates.
- Additional cloud providers/model marketplace.
- Silent auto-update.
- Git-light diff backup.
- Multi-language UI.
- Mobile/web/collaboration/cloud sync.
- Plugin system.
