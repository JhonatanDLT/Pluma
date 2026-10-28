# 09 — Pluma V1.0 Prioritized Roadmap

> Working document for the V1.0 release decision.
> **Basis:** `README.md`, `docs/01`–`08`, `docs/Notas extras.md`, and a code audit of
> `chronotriggerv0` on 2026-10-28.
>
> **Verification run:** `pnpm check` → 0 errors / 0 warnings; `cargo test --lib` →
> 58 passed, 2 ignored. No packaged build, signed installer, or GUI E2E run was
> performed for this audit.

---

## 0. Executive summary

Pluma is **feature-broad and demo-ready**, but **not release-ready**.

The core loop exists: write → structure → analyze → export. The codebase compiles
cleanly and has meaningful Rust unit tests. The gap is not "more features." The gap is
**trust**:

1. **Normal AI operations can destroy the story bible.** Analyzing one chapter deletes
   the project-wide characters, relationships and entities and replaces them with only
   that chapter's results. Re-analysis also wipes enriched profiles.
2. **Several one-click actions can erase a novel without confirmation or undo.**
   Deleting a project, deleting a chapter/scene, and importing a file into an existing
   project are all unguarded. Import deletes every chapter before inserting.
3. **The save path can silently lose edits.** There is no `Ctrl/Cmd+S`, no close/quit
   flush, no retry after a save error, and export/analyze read the DB without flushing
   pending edits first.
4. **There is no external backup.** Snapshots are manual, scene-only, and live in the
   same SQLite file as the manuscript.
5. **The bible is read-only.** The author cannot correct a character, merge duplicates,
   or dismiss a bad suggestion, despite FR-22 requiring an editable ficha.

**Recommendation:** run **three release-focused sprints** (not feature phases), then a
short beta. With one full-time developer plus part-time QA/design, target a **release
candidate in 6–7 weeks** and **public 1.0 in 8–9 weeks**. A solo developer without QA
should plan **9–11 weeks**. Freeze new AI modalities until after 1.0.

**The V1.0 promise should be:**

> "Write your novel offline. Never lose a word. Keep a story bible you can correct.
> Export to Word, EPUB and PDF. AI is optional and never takes control."

Not: "the most AI features."

---

## 1. Where Pluma actually is

### 1.1 Verified strengths

| Area | Evidence |
|---|---|
| Stack | Svelte 5 + SvelteKit SPA, Tauri v2, Rust 2024, SQLite (`rusqlite` bundled). |
| Size | ~14.5k lines across Svelte/TS/Rust; 52 Tauri commands; 9 versioned migrations. |
| Editor | TipTap WYSIWYG + CodeMirror source, Markdown round-trip, tables, images, links. |
| Structure | Chapters/scenes/beats parser, drag-and-drop, metadata, tree/flat views, quick switcher. |
| Import/export | MD, TXT, HTML, DOCX, PDF, EPUB; cover images are already wired into PDF/EPUB/DOCX (docs still say "pending"). |
| World building | Characters, relationships, factions, locations, search, change log (in memory). |
| AI | Rule-based NER + optional GLiNER2, embeddings, Ollama/cloud LLM, RAG chat, enrichment, grammar, Hunspell. |
| Quality | `pnpm check` clean; 58 Rust tests pass; strong module boundaries; typed IPC client. |
| Documentation | Unusually strong product vision and constraints in `docs/08`. |

### 1.1b Progress scorecard (PM estimate, not a metric)

| Dimension | Progress | Rationale |
|---|---:|---|
| Core editor + structure | 85% | Dual editor, parser, navigation and metadata work; polish and save edge cases remain. |
| Persistence + recovery | 55% | SQLite/WAL/migrations work; no close flush, retry, crash journal or external backup. |
| Import/export | 75% | Broad format support; destructive import, untested EPUB round-trip, raw-HTML edge cases. |
| World bible | 50% | Rich read-only data; no editing/merge/dismiss; analysis overwrites it. |
| AI analysis | 55% | Broad pipeline and models; destructive, scope-incorrect, validated on one chapter. |
| TTS | 45% | System voice play/stop; no pause/voice/rate; reads Markdown syntax. |
| Release readiness | 10% | 0.1.0, unsigned, no CI, no license/privacy, CSP null. |
| **Overall** | **Demo 80% / release 40%** | The demo is convincing; the trust and distribution layer is missing. |

### 1.2 Feature claim vs release readiness

| Area | Claimed | Verified in code | V1 verdict |
|---|---|---|---|
| Editor + autosave | ✅ | Works, but save failure/close path is weak | **Needs hardening** |
| Structure | ✅ | Works | Ready |
| Import | ✅ | Works, but **replaces the whole manuscript** | **Blocked** |
| Export | ✅ | Broad; EPUB round-trip untested; raw HTML passthrough | **Needs fidelity tests** |
| World building | ✅ base | **Read-only**; analysis is destructive | **Blocked** |
| AI analysis | ✅ | Broad but scope-incorrect and non-idempotent | **Blocked** |
| TTS | ✅ | Play/stop only; reads raw Markdown; fixed rate | **Partial** |
| Versions | ✅ | Manual, scene-only, same DB | **Partial** |
| Search | ✅ project/world | No global manuscript search / FTS5 | Can wait |
| Release engineering | — | 0.1.0, unsigned, no CI, no LICENSE, CSP `null` | **Blocked** |
| Performance proof | "500k words" | No benchmark harness | **Unproven** |

### 1.3 Release-blocking findings (P0)

#### F1 — AI analysis destroys and mis-scopes the story bible

**Evidence**

- `analyze_project` with a `chapter_id` collects **only that chapter's text**
  (`src-tauri/src/commands.rs:396-430`).
- It then calls `replace_characters`, `replace_relationships`, `replace_entities`
  (`commands.rs:596-598`), which `DELETE` **all project rows** and insert the new
  results (`src-tauri/src/repo.rs:556-566`, `641`, `677`).
- Marking a chapter as "Revisado" or "Final" automatically calls `analyze()` with the
  selected chapter (`src/routes/+page.svelte:558-566`).
- `enrich_project` writes `description`, `biography`, `traits`, `goals`, `arc` onto the
  character row. The next analysis deletes that row.

**Impact**

- Marking Chapter 2 as "Revisado" can reduce a 40-character bible to the characters
  found in Chapter 2.
- Enriched profiles are lost on the next analysis.
- Character IDs change on every run, breaking selections, relationships and any future
  user edits.
- The core promise ("la biblia se alimenta sola y tú la corriges") is currently false.

**Fix direction**

- Replace `replace_*` with **reconcile/upsert** by a stable canonical key.
- Make analysis **scope-aware**: a chapter run merges its mentions into the project
  graph and never deletes entities outside the scope.
- Add provenance (`origin = rules|gliner|llm|manual`) and `locked` flags; never overwrite
  manual fields.
- Mark missing entities as `stale`/`not_seen` instead of deleting them; review queue.
- Add regression tests: chapter-2 analysis preserves chapter-1 characters; re-analysis
  preserves enrichment; manual edits survive.

**Estimate:** 4 dev-days (schema migration + merge logic + tests).

---

#### F2 — Destructive actions are unguarded

**Evidence**

- Project delete: `ProjectsView.svelte:157` calls `ondeleteproject(project.id)` directly
  from the card's `×`; `delete_project` cascades chapters, scenes, characters,
  relationships, entities and snapshots (`repo.rs:100-106`, `0001_init.sql`).
- Chapter/scene delete: no confirmation in `+page.svelte:444-478`.
- Import into an existing project: `repo::import_manuscript` executes
  `DELETE FROM chapter WHERE project_id = ?1` before inserting (`repo.rs:339-342`).
  The UI "Importar" calls it immediately after the file picker (`+page.svelte` import
  flow). There is no "replace vs append vs new project" choice.
- No trash, undo, or automatic pre-operation snapshot.

**Impact**

A single misclick can erase a 500k-word novel. A repeated import can silently replace a
manuscript. This is the kind of defect that produces one-star reviews and destroys
word-of-mouth.

**Fix direction**

- Typed confirmation for project deletion; confirmation for chapter/scene deletion.
- Soft delete (`deleted_at`) + "Papelera" with 30-day restore.
- Import dialog: **"Reemplazar manuscrito"** vs **"Añadir como nuevo proyecto"** vs
  **"Añadir capítulos al final"**, with preview and backup.
- Auto-snapshot before import, restore, bulk analysis and delete.

**Estimate:** 1.5 dev-days.

---

#### F3 — The save path can silently lose edits

**Evidence**

- `flushSave` only retries when `saveState === "saving"`. After a save error
  (`saveState === "error"`), it does nothing; `selectScene` then loads the new scene and
  overwrites the in-memory content (`+page.svelte:1069-1076`, `268-278`).
- No `Ctrl/Cmd+S` handler; the global keydown handler only covers `Cmd+K`, chapter and
  scene navigation (`+page.svelte:178-220`). FR-03 requires manual save.
- No window close/quit handler: the last 600 ms of typing can be lost on quit.
- `exportDocument` and `analyze` do not `flushSave` before reading the DB
  (`+page.svelte:656-672`, `677-690`), so exports/analysis can use stale text.
- `saveChapterMeta` triggers analysis immediately after the metadata update, without
  flushing the scene first.

**Impact**

Silent data loss on transient disk errors or quit; exports missing the last edits;
analysis of stale content. For a writing tool this is a trust-ending failure.

**Fix direction**

- Explicit save state machine (`idle|dirty|saving|saved|error`) with retry/backoff.
- Flush barriers before scene switch, project switch, export, analysis, status change,
  window close and quit.
- `Ctrl/Cmd+S` and a visible "unsaved" indicator.
- Crash-recovery draft journal (see F4/M3).
- Never replace `sceneContent` while an unsaved/error state exists; block navigation and
  show a persistent banner.

**Estimate:** 2–3 dev-days (excluding crash journal).

---

#### F4 — No external backup; snapshots are not a recovery plan

**Evidence**

- Snapshots are created manually; `create_snapshot` refuses when the fingerprint has not
  changed (`repo.rs`), and stores full scene copies in `snapshot_scene`
  (`0005_snapshots.sql`). They cover **scenes only**, not characters, relationships,
  entities or settings.
- Snapshots live in the same SQLite file; corruption or accidental project deletion
  removes them too.
- Full-project backup export (zip of manuscript + bible + notes) is explicitly listed as
  pending in `docs/08` §5.7.
- The DB lives in the platform app-data directory with no automatic copy.

**Impact**

A corrupted DB, failed migration, or deleted project is unrecoverable. Years of work can
be lost. This is the single largest business risk in the product.

**Fix direction**

- Daily automatic backup to a user-visible `backups/` folder, rotating 7–14 copies.
- Manual "Exportar copia de seguridad" → zip with `manuscript.md`, `bible.json`,
  `project.json`, `snapshots/`, `settings.json` (no API keys).
- "Restaurar copia" validates and imports as a **new project**, never silently
  overwriting.
- Extend snapshots to the world bible, or add a separate bible version table.
- Add a "Abrir carpeta de datos" action and a documented backup location.

**Estimate:** 2–3 dev-days.

---

#### F5 — The story bible is read-only

**Evidence**

- Commands expose `list_characters`, `list_relationships`, `list_entities`,
  `delete_character`, `delete_entity`; there is no update/merge/rename command.
- The only writer is `enrich_project` (`update_character_profile`), which overwrites
  description/biography/traits/goals/arc.
- `WorldBuilding.svelte` has a search input and delete actions; no editable fields.
- `docs/01` FR-22 and `docs/08` §5.3 claim an editable ficha. The code does not.

**Impact**

Users cannot fix a wrong name, merge "Klein" and "Klein Moretti", correct a role, or
dismiss a false positive. AI errors become permanent noise, and the bible loses
credibility. This is the product's central differentiator, so it is a V1 blocker.

**Fix direction**

- Editable character/entity fields: name, aliases, role, description, biography,
  traits, goals, arc, color, notes.
- Merge duplicates (choose canonical, merge aliases/relationships).
- Dismiss/ignore suggestions and keep them from reappearing.
- Review queue: "Nuevos", "Posibles duplicados", "Ya no aparecen".
- Provenance badges (`auto` / `manual`) and field locks.
- Relationship editing: type, intensity, description, delete.
- Persist the change log.

**Estimate:** 4 dev-days.

---

#### F6 — Release and trust infrastructure is missing

**Evidence**

- Version `0.1.0` in `package.json` and `tauri.conf.json`.
- No `LICENSE`, no `.github/`, no CI, no signed/notarized build, no Windows installer
  signing, no update check, no crash logging.
- `csp: null` in `tauri.conf.json`.
- UI strings are hardcoded Spanish; no i18n layer (acceptable if Spanish-first is an
  explicit launch decision).
- No privacy policy/EULA or third-party license notices (models, dictionaries, fonts).

**Impact**

Gatekeeper/SmartScreen friction, no reproducible builds, no support diagnostics, and no
legal/trust package. A writing app cannot launch without these.

**Fix direction**

See M10/M11 in §2 and the release checklist in §7.

**Estimate:** 3–4 dev-days + procurement lead time (Apple Developer ID, Windows cert).

---

#### F7 — Privacy positioning contradicts the cloud feature

**Evidence**

- `README.md` and `docs/08` promise "todo local, todo tuyo" and "el manuscrito nunca sale
  del equipo."
- `docs/03` §9 and the README also say cloud AI is **"activa por defecto si configuras
  una API key"**, with OpenAI/Anthropic/Gemini/etc.
- `docs/02` §7 still says "Zero cloud".
- `csp: null`; imported Markdown images can point to remote URLs and be fetched by the
  editor.

**Impact**

A privacy-first product that silently sends manuscript text to a cloud provider when a
key exists is a positioning and legal risk. Users will not read the settings carefully.

**Fix direction**

- Local-only by default, always.
- Cloud AI behind an explicit consent modal naming provider/model and the data sent;
  persistent "cloud" badge in the chat panel.
- Never auto-select cloud because a key exists.
- CSP + block remote images in imported content by default.
- Privacy policy and a "what leaves this device" page.

**Estimate:** 2 dev-days.

---

#### F8 — The 500k-word performance promise is unproven

**Evidence**

- No benchmark harness or fixture at target scale.
- `get_project_content` loads all scenes for export/analysis; relationship analysis and
  RAG indexing are potentially expensive; project search is client-side; world search is
  `LIKE`.
- `docs/08` §6 and §9 make explicit `<2 s` open and `<50 ms` typing claims.

**Impact**

The persona is a 500k-word saga on an 8 GB laptop. If the first review reports lag, the
product fails its core promise regardless of feature breadth.

**Fix direction**

- Synthetic 500k-word fixture + scripted benchmark (open, scene switch, typing latency,
  full analysis, export, search).
- Optimize the top bottleneck only; document measured numbers in the README.
- Keep analysis incremental and background-only.

**Estimate:** 3 dev-days.

---

### 1.4 Documentation drift (process risk)

The docs mix **as-built** and **design intent**. This is not cosmetic; it makes planning
unreliable.

| Doc claim | Code reality |
|---|---|
| "Autoguardado ... + crash recovery" (`docs/08` §5.1) | No crash recovery; no close handler. |
| "Ficha de personaje editable" (FR-22, `docs/08` §5.3) | Read-only; only delete/enrich. |
| "Guardado manual `Ctrl/Cmd+S`" (FR-03) | Not implemented. |
| TTS "reproducir, pausar, detener, velocidad y selección de voz" (FR-31, must) | Play/stop only; fixed rate; reads raw Markdown. |
| Project "duplicar" (FR-04, must) | No command, no UI. |
| "Portada en exportación pendiente" (`docs/06`, `docs/08`) | Already implemented in PDF/EPUB/DOCX. |
| `docs/05` schema with `mention`, `setting`, `tag` | Not in migrations; actual schema differs. |
| `docs/02` "Zero cloud" | Cloud AI implemented. |

**Recommendation:** after each sprint, update the README and docs from the code, and add
a "known limitations" section. Treat docs claims as test cases: if a claim is not tested,
it is not done.

---

## 2. V1.0 must-haves

### 2.1 Release gates

A V1.0 build is release-ready only when all five gates pass:

| Gate | Question | Pass condition |
|---|---|---|
| **G1 Data safety** | Can the user lose work? | No unguarded destructive action; save/close/recovery tests pass; external backup restore works on a clean profile. |
| **G2 Bible ownership** | Can the user trust and correct the bible? | Analysis is non-destructive; edit/merge/dismiss works; manual edits survive re-analysis and restart. |
| **G3 Core loop** | Can a new user write, import and export? | Create/import → write → structure → export in <10 minutes; EPUB round-trip passes. |
| **G4 Performance** | Does it hold at target scale? | 500k-word fixture meets documented open/typing/analysis targets. |
| **G5 Distribution & trust** | Can we ship it safely? | Signed installers, CI, license/privacy, crash logs, clean-machine install, migration test. |

### 2.2 Prioritized must-have list

| ID | Must-have | Priority | Why it is V1 | Est. |
|---|---|---|---|---|
| **M1** | Non-destructive, scope-aware analysis with stable IDs and provenance | P0 | Prevents bible destruction; enables trust | 4d |
| **M2** | Destructive-action safety: confirm, soft delete/trash, safe import modes, auto-snapshot | P0 | Prevents catastrophic loss | 1.5d |
| **M3** | Save integrity: `Cmd/Ctrl+S`, close/quit flush, retry, flush barriers, crash-recovery journal | P0 | "Never lose a word" | 4d |
| **M4** | External backup/restore + bible snapshots + daily auto-backup | P0 | Only real disaster recovery | 2.5d |
| **M5** | Editable/mergeable bible + review queue + dismiss/ignore | P0 | Product's core differentiator | 4d |
| **M6** | Project completeness: duplicate project, metadata edit, deletion guard | P1 | FR-04 must; cheap | 0.5d |
| **M7** | TTS: voice picker, rate, pause/resume, strip Markdown before speaking | P1 | FR-31 must; current UX is visibly broken | 2d |
| **M8** | Export/import fidelity suite: EPUB round-trip, DOCX/PDF golden tests, raw-HTML safety | P1 | Export is the monetization proof | 2.5d |
| **M9** | 500k-word benchmark + fix top bottleneck | P1 | Core persona promise | 3d |
| **M10** | Privacy/consent/security: local-only default, explicit cloud consent, CSP, remote-image block, sanitize | P0 | Brand/legal trust | 2d |
| **M11** | Release engineering: 1.0.0, CI matrix, macOS signing/notarization, Windows signing, clean install/migration, crash logs, update check | P0 | Cannot ship without it | 3.5d |
| **M12** | First-run onboarding: create/import/sample, local-AI explainer with sizes, backup prompt | P1 | Activation and support load | 1.5d |
| **M13** | Accessibility/error UX/docs: keyboard nav, focus, contrast, Spanish error messages, user guide | P2 | Launch quality | 1.5d |

**Total:** ~34 dev-days before buffer (sprint-level roll-up; some rows overlap slightly).

### 2.3 Must-have acceptance criteria

**M1 — Non-destructive analysis**

- Analyze Chapter 2 → characters/relationships/entities from Chapter 1 remain.
- Analyze the full project → existing manual edits and enrichment survive.
- Re-running analysis with no text changes is idempotent (no duplicate rows, no ID
  churn).
- A character absent from the latest scope is marked `stale`, not deleted.
- Tests cover all of the above.

**M2 — Destructive-action safety**

- Deleting a project requires typing its title; chapter/scene deletion requires a
  confirmation dialog.
- Deleted items go to a trash with restore for 30 days.
- Importing into an existing project offers replace/append/new-project; replace creates
  a backup first.
- An auto-snapshot exists before every destructive operation.

**M3 — Save integrity**

- `Cmd/Ctrl+S` saves immediately and shows confirmation.
- Closing the window with unsaved changes flushes or prompts.
- A simulated save failure blocks scene switching and offers retry; no silent loss.
- Export and analysis always include the latest edits.
- Crash recovery restores at most ~2 seconds of typing.

**M4 — Backup**

- Daily auto-backup runs without user action; last 7–14 retained.
- Manual backup zip contains manuscript, bible, project metadata and snapshots.
- Restore on a clean app profile produces a working project.
- "Open data folder" is discoverable in settings.

**M5 — Bible ownership**

- Edit every character field and entity field; changes persist and survive re-analysis.
- Merge two characters; aliases and relationships are preserved.
- Dismiss a false positive; it does not reappear as new.
- Review queue shows new/stale/duplicate suggestions.
- Manual edits are visually marked and never overwritten by AI.

**M6 — Project completeness**

- Duplicate project copies chapters, scenes, characters, relationships, entities and
  settings.
- Author/description/cover/goal editable after creation.

**M7 — TTS**

- Voice list from the OS, rate control, pause/resume (sentence-level is acceptable).
- Markdown syntax is stripped; only prose is read.
- Stop does not kill unrelated system speech processes.

**M8 — Export fidelity**

- EPUB export → import → chapter/scene count and titles match; automated test.
- DOCX/PDF golden files open in Word/Preview; headings, emphasis, tables, images and
  cover render.
- Raw HTML in Markdown cannot inject scripts into exported HTML/EPUB.
- Export of a 500k-word project completes without OOM.

**M9 — Performance**

- Fixture: 500k words, 40+ characters, ~1,500 scenes.
- Targets: open <2 s; scene switch <150 ms; typing <50 ms; full analysis background and
  cancellable; export <15 s; search response <300 ms (or documented limitation).
- Numbers recorded in `README` and a benchmark script checked in.

**M10 — Privacy**

- Fresh install is local-only; no network calls for core writing.
- Cloud AI requires explicit consent; a persistent badge shows when active.
- CSP is set; imported remote images are blocked or proxied with consent.
- API keys remain in the OS keychain; no plaintext fallback.

**M11 — Release engineering**

- Version 1.0.0; `LICENSE`, privacy policy, third-party notices.
- CI builds macOS + Windows on every PR; `pnpm check`, `cargo test`, `cargo clippy`.
- macOS app signed and notarized; Windows installer signed.
- Clean VM install → create/import → export → uninstall/reinstall → data intact.
- DB migration from a 0.1 database passes.
- Crash log file with rotation; "open logs" action; no manuscript content in logs.

**M12 — Onboarding**

- First run offers: create project, import manuscript, open sample.
- Explains local-only promise and optional AI downloads with sizes/time.
- Prompts to choose a backup folder.
- A new user can write 500 words in under 2 minutes.

**M13 — Accessibility/docs**

- Full keyboard navigation for the main workspace; visible focus states; contrast check.
- User-facing errors are actionable Spanish messages, not raw Rust errors.
- A short user guide covers import, backup, AI opt-in and export.

---

## 3. Can-waits (deliberately delayed)

Do not build these before 1.0. They are valuable, but they either add model/support risk
or do not protect the core promise.

| Feature | Why delay | Revisit trigger |
|---|---|---|
| **Piper neural TTS + per-character voices** | Model packaging, voice licensing, size/support; system TTS is enough for proof-listening | 1.1; >20% of beta users ask for better voices |
| **Laya relation classifier** | Checkpoint I/O unverified; adds Tier-1 complexity; current relation typing is acceptable if editable | 1.1, after eval shows relation typing is the weakest link |
| **Timeline/cronología + inconsistency detection** | New schema and UX; large scope | 1.2, after bible editing is stable |
| **Artifacts, research notes, full persisted change log** | Nice-to-have world-building depth | 1.1/1.2 |
| **Relationship graph visualization** | Presentation layer; data already exists | 1.1 |
| **Global FTS5 search** | Useful, but Ctrl+K + find/replace covers V1; FTS5 needs schema/index work | 1.1; first post-launch feature if beta search complaints |
| **Split view, focus mode, command palette, configurable shortcuts** | Productivity polish, not trust | 1.1 |
| **Daily goals, streaks, writing stats** | Motivation, not core | 1.1 |
| **Comments/margin notes** | Revision workflow; needs anchors and UI | 1.2 |
| **Selective export (chapter/selection), templates** | Whole-project export is the V1 need | 1.1 |
| **Additional cloud providers / model marketplace** | Maintenance and privacy surface; current providers already cover the need | Freeze until 1.1; keep advanced |
| **Silent auto-update** | Signing/rollback complexity; an update check is enough for 1.0 | 1.1 |
| **Git-light diff backup** | External zip backup solves disaster recovery for V1 | 1.2 |
| **OCR improvements, EPUB round-trip edge cases beyond tests** | Optional path | As needed |
| **Multi-language UI** | Spanish-first launch is a deliberate wedge | After Spanish PMF |
| **Mobile/web/collaboration/cloud sync** | Explicit non-goals in `docs/08` §10 | Not planned |
| **Plugin system** | Premature extensibility | Post-2.0 |

> **Rule:** if a feature does not improve G1–G5, it does not enter the 1.0 train.

---

## 4. Sequencing: AI features vs core writing

### 4.1 The principle

> **AI proposes; the author owns; the core never depends on AI.**

The current build inverts this: AI can overwrite the author's bible, and the author cannot
correct it. More AI would make the product worse until that is fixed.

### 4.2 Two tracks, one critical path

```
Core track (release-critical):
save integrity → destructive-action safety → backup → export fidelity → benchmark → signed release

AI track (enabling, not headline):
stop destructive writes → provenance → edit/merge/dismiss → evaluation → quality fixes
```

The AI track must never block the core track. The core loop must pass with
`ai_enabled = false`.

### 4.3 Phase order

**Phase A — Make AI safe (Sprint 1)**

- Freeze all new AI features and model integrations.
- Replace destructive `replace_*` with reconcile/upsert.
- Scope chapter analysis correctly.
- Add provenance + manual locks.
- Add regression tests.
- Deliverable: AI can no longer destroy the bible.

**Phase B — Give control to the author (Sprint 2)**

- Edit/merge/dismiss/review queue.
- AI suggestions are draft state with "apply/ignore".
- Manual edits survive re-analysis.
- Deliverable: every AI output is correctable and reversible.

**Phase C — Improve quality where evidence says it matters (Sprint 3)**

- Build a golden evaluation set: 8–10 Spanish literary chapters across genres, with
  gold entities/relations.
- Measure precision/recall, duplicate rate, relationship-type accuracy, latency.
- Fix only the top gaps: RAG relevance threshold, pair-level context for relationship
  typing, model download failure/cancel/resume.
- No new modalities (Piper/Laya/timeline) before 1.0.

**Phase D — After 1.0**

- Laya, Piper, timeline extraction, graph view, semantic search, etc.

### 4.4 Definition of "AI-ready"

- No model installed → full write/import/export/bible flow works; analysis falls back to
  rules.
- Model installed → measurable improvement on the eval set; no new destructive behavior.
- Model removed → no data loss; user edits remain.
- Every AI action is previewable, dismissible, and undoable.
- Cloud AI is never the default; local is the default.

### 4.5 Anti-patterns to avoid before 1.0

- Adding a new model while the author still cannot edit its output.
- Auto-running analysis on status change without a diff/review step.
- Regenerating entities with new IDs on every run.
- Making cloud AI the "recommended" path because it is easier to demo.
- Using AI to compensate for missing core features (search, backup, export fidelity).
- Shipping a model download that can fail without a clear fallback.

---

## 5. Three-sprint plan

### 5.1 Assumptions

- **Sprint length:** 2 weeks (10 working days) in the baseline.
- **Capacity:** 1 senior full-stack developer (Rust + Svelte), 0.5 QA/UX, part-time
  designer. Release engineering may need a contractor for 3–4 days.
- **Scope freeze:** no new features outside this plan; bug fixes only.
- **Procurement starts day 1:** Apple Developer ID, Windows code-signing certificate,
  domain/handles, trademark check.
- **Beta cohort recruited by end of Sprint 2:** 20–50 Spanish-speaking novelists.

### 5.2 Timeline at a glance

| Milestone | 2-week sprints (1 dev + 0.5 QA) | Solo, 3-week sprints |
|---|---|---|
| Sprint 1 complete — trust spine | End week 2 | End week 3 |
| Sprint 2 complete — bible ownership + writing polish | End week 4 | End week 6 |
| Sprint 3 complete — release candidate | End week 6–7 | End week 9 |
| Private beta | Weeks 7–8 | Weeks 10–11 |
| Public 1.0 | Week 8–9 | Week 11–12 |

> The task total is ~34 dev-days before buffer. It **does not fit** one developer in
> three 2-week sprints without either (a) 3-week sprints, (b) a second developer or
> release-engineering contractor, or (c) cutting M6/M7/M12. Do not cut M1–M5 or M10–M11.

### 5.3 Sprint 1 — "Stop the bleeding" (trust spine)

**Goal:** no normal operation can destroy the manuscript or the bible.

| ID | Task | Acceptance | Est. |
|---|---|---|---|
| 1.1 | Save integrity: `Cmd/Ctrl+S`, close/quit flush, retry/backoff, block navigation on failure, flush before export/analyze/status | Simulated failure loses nothing; export includes latest edits | 2d |
| 1.2 | Destructive-action safety: confirmations, trash/soft delete, import modes (replace/append/new), auto-snapshot | Misclick is recoverable; import never silently replaces | 1.5d |
| 1.3 | Non-destructive analysis: migration, stable keys, scope-aware merge, provenance/locks, preserve enrichment | Chapter-2 analysis keeps Chapter-1 bible; re-analysis preserves edits | 4d |
| 1.4 | External backup/restore: daily auto-backup, manual zip, restore as new project, bible snapshot | Restore on clean profile works | 2d |
| 1.5 | Regression tests + docs truth pass | Tests cover F1–F4; README no longer claims unbuilt features | 0.5d |

**Exit criteria**

- Close the window with unsaved changes → flush or prompt; no loss.
- Delete project/chapter/scene → confirmation + trash restore.
- Import into an existing project → explicit choice + backup.
- Analyze a single chapter → the rest of the bible remains.
- Restore a backup on a clean profile.
- `cargo test` and `pnpm check` green.

### 5.4 Sprint 2 — "Own the bible + writing polish"

**Goal:** the author can correct every AI output and the core writing loop feels complete.

| ID | Task | Acceptance | Est. |
|---|---|---|---|
| 2.1 | Editable/mergeable bible + review queue + dismiss/ignore + relationship editing | All fields editable; merge preserves aliases/relations; manual edits survive analysis/restart | 4d |
| 2.2 | Crash-recovery journal + automatic pre-operation snapshots + retention | Crash restores latest draft; snapshots rotate | 2d |
| 2.3 | TTS controls: voice list, rate, pause/resume, strip Markdown | Prose-only playback; controls work on macOS/Windows | 2d |
| 2.4 | Export/import fidelity: EPUB round-trip test, DOCX/PDF golden files, raw-HTML sanitization, cover regression | Automated round-trip passes; golden files open correctly | 2d |
| 2.5 | Project duplicate + first-run onboarding + backup prompt | Duplicate copies all data; new user writes 500 words in <2 min | 1.5d |

**Exit criteria**

- Edit a character, re-analyze the project, restart the app → edit persists.
- Merge two characters → one canonical row, no orphan relationships.
- Dismiss a false positive → it does not reappear.
- EPUB export → import → structure matches.
- TTS speaks prose without reading `#` or `**`.
- Force-quit during typing → reopen → recover at most ~2 s of work.
- A new user completes create/import → write → export without help.

### 5.5 Sprint 3 — "Release candidate"

**Goal:** prove the promise at scale and ship a signed, private, supportable build.

| ID | Task | Acceptance | Est. |
|---|---|---|---|
| 3.1 | 500k-word benchmark harness + fix top bottleneck | Targets from M9 met and documented | 3d |
| 3.2 | Privacy/consent/security: local-only default, explicit cloud consent, CSP, remote-image block, sanitize | No network in core loop; cloud badge; CSP set | 2d |
| 3.3 | Release engineering: 1.0.0, CI matrix, signing/notarization, installers, clean install, migration, crash logs, update check | Signed builds install on clean macOS/Windows; 0.1 DB migrates | 3.5d |
| 3.4 | AI evaluation harness + RAG relevance threshold + model failure/cancel/resume UX | Golden-set metrics recorded; failures fall back gracefully | 2d |
| 3.5 | Accessibility basics + error UX + user guide + third-party notices | Keyboard-only main flow; actionable errors; docs shipped | 1d |
| 3.6 | RC buffer / beta feedback triage | No open P0; beta cohort onboarded | 1d |

**Exit criteria**

- 500k-word benchmark report checked in.
- Signed macOS and Windows artifacts; clean-VM install passes.
- Fresh install makes zero network calls until the user opts in.
- Cloud AI requires explicit consent and is visibly marked.
- Beta cohort of 20–50 writers; crash-free sessions ≥99.5%; no P0 bugs for one week.
- README/docs match the shipped build.

### 5.6 Critical path and dependencies

```
M1/M2 (data integrity) ─┬─> M5 (bible editing) ─> beta
M3/M4 (save/backup) ────┘
M8 (export fidelity) ──────> beta
M9 (perf) ─────────────────> RC
M11 (signing/CI) ──────────> RC   [start procurement week 1]
M10 (privacy) ─────────────> RC
```

- **Schema migration 0010** (provenance, stable keys, stale flags, bible snapshots) must
  land in Sprint 1; later migrations should be additive only.
- **Signing lead time** is the most common schedule risk; start week 1.
- **Beta recruitment** starts week 1; do not wait for RC.

### 5.7 If the team is behind

Cut in this order, never the P0s:

1. M6 project duplicate → 1.1 (workaround: create new project and import).
2. M7 TTS → hide the buttons in 1.0, ship in 1.1 (do not ship broken TTS).
3. M12 onboarding sample → keep create/import, drop sample.
4. M13 accessibility → keep keyboard/focus basics, defer audit.
5. M9 benchmark scope → measure open/typing/export only, defer analysis/search.

**Never cut:** M1, M2, M3, M4, M5, M10, M11.

---

## 6. Business synthesis

> These are **hypotheses drawn from the product docs**, not validated market research.
> Validate with 10–15 target writers before pricing/GTM lock.

### 6.0 Top business findings

1. **Feature breadth is not the buying criterion.** The buyer is purchasing *trust*:
   "my manuscript is safe, my bible is correct, my export opens in Word." Pluma currently
   demos AI breadth but fails all three trust tests in edge cases.
2. **AI is a double-edged wedge.** It attracts attention, but every wrong character or
   lost enrichment is a reason to churn. The correction UX (M5) is the actual product,
   not the model.
3. **Spanish-first is a real differentiator.** Most competitors are English-first. Do not
   dilute the launch with multi-language UI before Spanish product-market fit.
4. **One-time pricing fits the audience; prominent cloud AI does not.** A subscription
   contradicts the "own your tools" promise, and cloud AI contradicts "never leaves your
   device." Keep cloud as an advanced, explicit opt-in.
5. **Local-model support cost must be designed down.** Rules-only default, clear download
   sizes, graceful failure and a "modo modesto" profile are business requirements, not
   just engineering details.

### 6.1 ICP and job-to-be-done

**Primary ICP:** Spanish-speaking novelists writing long works (100k–500k+ words),
especially fantasy/sci-fi sagas, on a mid-range laptop, privacy-conscious, skeptical of
subscriptions and cloud AI.

**JTBD:**

1. "Let me write without fighting the tool."
2. "Help me keep names, relationships and world details consistent across years."
3. "Let me correct the tool when it is wrong."
4. "Give me files I can send to my editor/publisher (DOCX/EPUB/PDF)."
5. "Never lose my work."

### 6.2 Positioning

**Current implicit positioning:** "an AI-powered writing app."

**Recommended positioning:** "the offline, Spanish-first writing studio for long novels —
your manuscript, your bible, your exports, and AI only when you ask."

**Moat:** not the model. The moat is the integrated, user-owned story bible plus export
fidelity and Spanish UX. Models are commodities; a trusted data model and workflow are
not.

**Proof points for launch:**

- "Never lose a word" — backup, recovery, destructive-action demo.
- "Edit every AI suggestion" — merge/dismiss/review.
- "Works offline" — network monitor demo.
- "500k words, measured" — benchmark numbers.
- "Exports that open in Word/Calibre" — fidelity samples.

### 6.3 Pricing hypothesis

| Decision | Recommendation | Rationale |
|---|---|---|
| Model | One-time license, free 1.x updates, paid 2.0 | Audience distrusts subscriptions; local AI means near-zero COGS; aligns with "own your tools." |
| Price | €49 standard, €29 launch/founder | Comparable to Scrivener; below AI subscription fatigue; regional pricing for LATAM. |
| Cloud AI | Bring-your-own key, no markup | Avoids inference COGS and privacy liability; advanced opt-in only. |
| Trial | 30 days full-featured | Writers need a real project to evaluate. |
| Refunds | 30-day no-questions | Reduces risk for a new brand. |

**Open question:** whether the Spanish market supports €49 at launch or whether €29
should be the standard price with a €59 "supporter" tier. Test with the beta cohort.

### 6.4 Go-to-market hypothesis

- **Direct download** (Paddle/Lemon Squeezy/Gumroad) rather than app stores: sandboxing
  conflicts with local model downloads and arbitrary manuscript folders.
- **Mac first**, Windows second; the persona is a laptop writer and the design is
  macOS-friendly, but Windows is a large Spanish-speaking segment.
- **Channels:** Spanish writing YouTube/podcasts, NaNoWriMo communities, self-publishing
  associations, writing subreddits/Discords, local workshops, affiliate deals with writing
  coaches.
- **Content:** "cómo mantener la biblia de tu novela", "Scrivener vs Pluma", "IA local
  para escritores sin GPU", "exportar a EPUB sin dolor".
- **Launch asset:** a 3-minute demo showing import → bible → correction → export, with
  the network disconnected.

### 6.5 Business risks

| Risk | Evidence / why | Mitigation |
|---|---|---|
| **Data-loss reputation** | Unguarded deletes/import/analysis | Fix G1 before any marketing; publish a backup guide. |
| **AI trust collapse** | Read-only, destructive bible | Fix M1/M5; show provenance; make correction the demo. |
| **Privacy contradiction** | Cloud "default if key" vs "never leaves device" | Local-only default; explicit consent; remove contradictory copy. |
| **Support burden** | Ollama/ONNX/GLiNER downloads, OS TTS differences | Default rules-only; clear sizes; one-click fallback; FAQ. |
| **Distribution friction** | Unsigned 0.1.0; Gatekeeper/SmartScreen | Budget Apple/Windows certs; start week 1. |
| **Scope creep** | 9-phase roadmap with 40+ open features | Freeze 1.0 to G1–G5; can-wait list in §3. |
| **Competition** | Scrivener (mature), Novelcrafter/Sudowrite (AI, cloud), Obsidian (DIY) | Compete on offline + Spanish + editable bible + export, not model count. |
| **Name/trademark** | "Pluma" is generic and likely used elsewhere | Clear trademark, domain and handles before launch. |
| **Pricing too low** | One-time €29 with high support cost | Model support cost per user; consider €49; paid major upgrades. |

### 6.6 Beta / 1.0 metrics

| Metric | Target |
|---|---|
| Activation (create/import + first 500 words) | ≥60% |
| D7 retention | ≥35% |
| Export rate in first week | ≥25% |
| Crash-free sessions | ≥99.5% |
| AI opt-in (any local model) | ≥30% (not a goal in itself) |
| Time to first analysis | <2 min after opt-in |
| Support tickets per 100 users/month | <10 |
| Refund rate | <5% |
| NPS after 2 weeks | ≥30 |

### 6.7 Decisions needed before Sprint 1

1. **Team capacity:** one dev + part-time QA, or two devs/contractor?
2. **Platforms at 1.0:** macOS only, or macOS + Windows?
3. **Cloud AI in 1.0:** hide it, or ship explicit opt-in? (Recommendation: explicit
   opt-in, advanced, never default.)
4. **TTS in 1.0:** polish or hide? (Recommendation: polish voice/rate; pause at
   sentence level.)
5. **Pricing:** €29 launch / €49 standard? Validate with beta.
6. **Beta size and recruitment:** 20–50 writers; who owns it?
7. **Name clearance:** trademark/domain/handles.

---

## 7. Release checklist (definition of done)

### Product and data
- [ ] M1–M5 accepted; no P0 data-loss path.
- [ ] Delete/import/restore demos pass on a clean profile.
- [ ] A 500k-word project meets the documented performance targets.
- [ ] EPUB round-trip and DOCX/PDF golden files pass.
- [ ] TTS reads prose only; voice/rate/pause work.
- [ ] Project duplicate works or is explicitly out of 1.0.

### Quality
- [ ] `pnpm check` 0 errors/warnings.
- [ ] `cargo test` green; new regression tests for analysis/save/backup.
- [ ] `cargo clippy -D warnings` green.
- [ ] At least a smoke E2E: create → write → restart → export → import.
- [ ] Clean-VM install and DB migration from 0.1 tested.
- [ ] No open P0 bugs for one week before public release.

### Distribution and trust
- [ ] Version 1.0.0; `LICENSE`, privacy policy, EULA, third-party notices.
- [ ] macOS signed + notarized; Windows installer signed.
- [ ] CI builds both platforms on every PR.
- [ ] Crash logs with rotation; "open logs"; no manuscript content.
- [ ] Update check (not silent auto-update).
- [ ] Support channel and user guide.
- [ ] README/docs match the shipped build.
- [ ] Rollback plan for a bad release.

### Privacy
- [ ] Fresh install is local-only; no network calls in the core loop.
- [ ] Cloud AI explicit consent + persistent badge.
- [ ] CSP set; remote images blocked by default.
- [ ] API keys only in the OS keychain.
- [ ] "What leaves this device" page.

### Beta
- [ ] 20–50 target writers onboarded.
- [ ] Feedback triaged; no recurring data-loss reports.
- [ ] Crash-free ≥99.5%.
- [ ] At least 10 users complete a real export.

---

## 8. Evidence appendix

### 8.1 Commands run

| Command | Result |
|---|---|
| `pnpm check` | 0 errors, 0 warnings |
| `cargo test --lib` | 58 passed, 0 failed, 2 ignored |

Not run: `pnpm tauri build`, signed installer verification, GUI E2E, 500k benchmark.

### 8.2 Key code references

| Finding | Reference |
|---|---|
| Chapter-scoped analysis + project-wide replace | `src-tauri/src/commands.rs:396-430`, `596-598` |
| Replace deletes all rows | `src-tauri/src/repo.rs:556-566`, `641`, `677` |
| Auto-analysis on status change | `src/routes/+page.svelte:558-566` |
| Import deletes all chapters | `src-tauri/src/repo.rs:332-342` |
| Project delete without confirmation | `src/lib/components/ProjectsView.svelte:157` |
| Save retry bug | `src/routes/+page.svelte:1069-1076` |
| No Ctrl/Cmd+S | `src/routes/+page.svelte:178-220` |
| Export/analyze without flush | `src/routes/+page.svelte:656-672`, `677-690` |
| Snapshots scene-only | `src-tauri/migrations/0005_snapshots.sql` |
| Character commands read-only | `src-tauri/src/lib.rs:51-104` |
| TTS raw Markdown, fixed rate | `src/routes/+page.svelte:916-920` |
| CSP null | `src-tauri/tauri.conf.json` |
| Cover export already implemented | `src-tauri/src/export.rs:450-540`, `753`, `951` |
| No LICENSE/CI | repository root |

### 8.3 Limitations of this audit

- No manual GUI session; behavior inferred from code and docs.
- No packaged build/signing test.
- No real 500k-word manuscript benchmark.
- Business hypotheses are not validated with users.
- Docs `05` is partly aspirational; the audit used actual migrations as source of truth.

---

## 9. One-page decision summary

| Question | Answer |
|---|---|
| **What must be in 1.0?** | Non-destructive editable bible; save/recovery/backup; destructive-action guards; export fidelity; 500k performance proof; signed installers; privacy consent; onboarding. |
| **What can wait?** | Piper, Laya, timeline, graph, FTS5, split view, focus mode, stats, comments, selective export, extra cloud providers, sync/collab. |
| **AI vs core?** | Core first: AI must stop destroying data and become correctable before any new AI. New models after 1.0. |
| **Timeline?** | 3 sprints to RC: 6–7 weeks with 1 dev + part-time QA; 9 weeks solo; public 1.0 8–9 weeks (or 11–12 solo). |
| **Biggest risk?** | Data loss, not missing features. Fix trust before marketing. |
| **Business wedge?** | Offline, Spanish-first, user-owned story bible, export fidelity—not "most AI." |
