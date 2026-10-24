<script lang="ts">
  import type { Chapter, Scene } from "$lib/types";

  let {
    chapter,
    scene,
    onsavechapter,
    onsavescene,
  }: {
    chapter: Chapter | null;
    scene: Scene | null;
    onsavechapter: (status: string, summary: string | null) => void;
    onsavescene: (meta: {
      status: string;
      pov: string | null;
      location: string | null;
      timeLabel: string | null;
      summary: string | null;
    }) => void;
  } = $props();

  let status = $state("draft");
  let summary = $state("");
  let pov = $state("");
  let location = $state("");
  let timeLabel = $state("");

  const isScene = $derived(scene !== null);

  $effect(() => {
    if (scene) {
      status = scene.status;
      summary = scene.summary ?? "";
      pov = scene.pov ?? "";
      location = scene.location ?? "";
      timeLabel = scene.time_label ?? "";
    } else if (chapter) {
      status = chapter.status;
      summary = chapter.summary ?? "";
      pov = "";
      location = "";
      timeLabel = "";
    }
  });

  function save() {
    if (scene) {
      onsavescene({
        status,
        pov: pov.trim() || null,
        location: location.trim() || null,
        timeLabel: timeLabel.trim() || null,
        summary: summary.trim() || null,
      });
    } else if (chapter) {
      onsavechapter(status, summary.trim() || null);
    }
  }
</script>

{#if chapter || scene}
  <div class="meta-panel">
    <div class="meta-field">
      <label for="meta-status">Estado</label>
      <select id="meta-status" class="input" bind:value={status}>
        <option value="draft">Borrador</option>
        <option value="revised">Revisado</option>
        <option value="final">Final</option>
      </select>
    </div>

    {#if isScene}
      <div class="meta-field">
        <label for="meta-pov">Punto de vista</label>
        <input id="meta-pov" class="input" bind:value={pov} placeholder="Ana / Tercera persona…" />
      </div>
      <div class="meta-field">
        <label for="meta-loc">Lugar</label>
        <input id="meta-loc" class="input" bind:value={location} placeholder="Madrid, 1936…" />
      </div>
      <div class="meta-field">
        <label for="meta-time">Tiempo</label>
        <input id="meta-time" class="input" bind:value={timeLabel} placeholder="Noche / Al amanecer…" />
      </div>
    {/if}

    <div class="meta-field grow">
      <label for="meta-summary">Resumen</label>
      <textarea id="meta-summary" class="input" bind:value={summary} rows="2" placeholder="De qué trata…"></textarea>
    </div>

    <button class="btn btn-primary" onclick={save}>Guardar</button>
  </div>
{/if}

<style>
  .meta-panel {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
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
