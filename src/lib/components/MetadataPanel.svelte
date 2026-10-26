<script lang="ts">
  import type { Chapter, Project, Scene } from "$lib/types";

  let {
    chapter,
    scene,
    project,
    onsavechapter,
    onsavescene,
    onupdateprojectmeta,
  }: {
    chapter: Chapter | null;
    scene: Scene | null;
    project: Project | null;
    onsavechapter: (status: string, summary: string | null) => void;
    onsavescene: (meta: {
      status: string;
      pov: string | null;
      location: string | null;
      timeLabel: string | null;
      summary: string | null;
    }) => void;
    onupdateprojectmeta: (author: string | null, description: string | null) => void;
  } = $props();

  let chapterStatus = $state("draft");
  let chapterSummary = $state("");

  let sceneStatus = $state("draft");
  let pov = $state("");
  let location = $state("");
  let timeLabel = $state("");
  let sceneSummary = $state("");

  let author = $state("");
  let description = $state("");

  $effect(() => {
    if (chapter) {
      chapterStatus = chapter.status;
      chapterSummary = chapter.summary ?? "";
    }
    if (scene) {
      sceneStatus = scene.status;
      sceneSummary = scene.summary ?? "";
      pov = scene.pov ?? "";
      location = scene.location ?? "";
      timeLabel = scene.time_label ?? "";
    }
    if (project) {
      author = project.author ?? "";
      description = project.description ?? "";
    }
  });

  function saveChapter() {
    if (chapter) onsavechapter(chapterStatus, chapterSummary.trim() || null);
  }

  function saveScene() {
    if (scene) {
      onsavescene({
        status: sceneStatus,
        pov: pov.trim() || null,
        location: location.trim() || null,
        timeLabel: timeLabel.trim() || null,
        summary: sceneSummary.trim() || null,
      });
    }
  }

  function saveProjectMeta() {
    if (project) {
      onupdateprojectmeta(author.trim() || null, description.trim() || null);
    }
  }
</script>

{#if project || chapter || scene}
  <div class="meta-panel">
    {#if project}
      <section class="meta-section">
        <h4>Proyecto</h4>
        <div class="meta-field">
          <label for="project-author">Autor</label>
          <input id="project-author" class="input" bind:value={author} placeholder="Nombre del autor" />
        </div>
        <div class="meta-field grow">
          <label for="project-desc">Resumen</label>
          <textarea id="project-desc" class="input" bind:value={description} rows="2" placeholder="De qué trata la novela…"></textarea>
        </div>
        <button class="btn btn-primary" onclick={saveProjectMeta}>Guardar proyecto</button>
      </section>
    {/if}

    {#if chapter}
      <section class="meta-section">
        <h4>Capítulo</h4>
        <div class="meta-row">
          <div class="meta-field">
            <label for="chapter-status">Estado</label>
            <select id="chapter-status" class="input" bind:value={chapterStatus}>
              <option value="draft">Borrador</option>
              <option value="revised">Revisado</option>
              <option value="final">Final</option>
            </select>
          </div>
          <div class="meta-field grow">
            <label for="chapter-summary">Resumen</label>
            <textarea id="chapter-summary" class="input" bind:value={chapterSummary} rows="2" placeholder="De qué trata este capítulo…"></textarea>
          </div>
        </div>
        <button class="btn btn-primary" onclick={saveChapter}>Guardar capítulo</button>
      </section>
    {/if}

    {#if scene}
      <section class="meta-section">
        <h4>Escena</h4>
        <div class="meta-row">
          <div class="meta-field">
            <label for="scene-status">Estado</label>
            <select id="scene-status" class="input" bind:value={sceneStatus}>
              <option value="draft">Borrador</option>
              <option value="revised">Revisado</option>
              <option value="final">Final</option>
            </select>
          </div>
          <div class="meta-field">
            <label for="scene-pov">Punto de vista</label>
            <input id="scene-pov" class="input" bind:value={pov} placeholder="Ana / Tercera persona…" />
          </div>
          <div class="meta-field">
            <label for="scene-loc">Lugar</label>
            <input id="scene-loc" class="input" bind:value={location} placeholder="Madrid, 1936…" />
          </div>
          <div class="meta-field">
            <label for="scene-time">Tiempo</label>
            <input id="scene-time" class="input" bind:value={timeLabel} placeholder="Noche / Al amanecer…" />
          </div>
        </div>
        <div class="meta-field grow">
          <label for="scene-summary">Resumen</label>
          <textarea id="scene-summary" class="input" bind:value={sceneSummary} rows="2" placeholder="De qué trata esta escena…"></textarea>
        </div>
        <button class="btn btn-primary" onclick={saveScene}>Guardar escena</button>
      </section>
    {/if}
  </div>
{/if}

<style>
  .meta-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    overflow-y: auto;
  }

  .meta-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-bottom: 14px;
    border-bottom: 1px dashed var(--border);
  }

  .meta-section:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .meta-section h4 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
  }

  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 140px;
  }

  .meta-field.grow {
    flex: 1;
    min-width: 200px;
  }

  label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
</style>
