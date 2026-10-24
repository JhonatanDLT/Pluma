<script lang="ts">
  import type { SnapshotMeta } from "$lib/types";

  let {
    snapshots,
    busy,
    oncreate,
    onrestore,
    ondelete,
  }: {
    snapshots: SnapshotMeta[];
    busy: boolean;
    oncreate: (message: string | null) => void;
    onrestore: (id: string) => void;
    ondelete: (id: string) => void;
  } = $props();

  let message = $state("");

  function save() {
    const value = message.trim();
    oncreate(value.length > 0 ? value : null);
    message = "";
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString();
  }
</script>

<div class="versions-panel">
  <div class="panel-head">
    <span class="label">Versiones</span>
  </div>

  <div class="save-row">
    <input
      class="input"
      placeholder="Mensaje de la versión (opcional)"
      bind:value={message}
      onkeydown={(e) => {
        if (e.key === "Enter") save();
      }}
    />
    <button class="btn btn-primary" onclick={save} disabled={busy}>
      {busy ? "Guardando…" : "Guardar"}
    </button>
  </div>

  {#if snapshots.length === 0}
    <p class="hint">
      Guarda una <strong>versión</strong> del manuscrito para poder volver a un
      punto anterior. Solo se guarda si hay cambios desde la última versión.
    </p>
  {:else}
    <ul class="snap-list">
      {#each snapshots as s (s.id)}
        <li>
          <div class="snap-main">
            <span class="snap-date">{formatDate(s.created_at)}</span>
            <span class="snap-meta">{s.scene_count} escenas</span>
            {#if s.message}
              <span class="snap-msg">{s.message}</span>
            {/if}
          </div>
          <div class="snap-actions">
            <button class="btn-icon" title="Restaurar esta versión" onclick={() => onrestore(s.id)}>↩</button>
            <button class="btn-icon" title="Eliminar versión" onclick={() => ondelete(s.id)}>×</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .versions-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .panel-head {
    padding: 14px 16px 8px;
  }

  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .save-row {
    display: flex;
    gap: 6px;
    padding: 4px 16px 10px;
    border-bottom: 1px solid var(--border);
  }

  .input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .hint {
    padding: 0 16px;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.6;
  }

  .snap-list {
    list-style: none;
    margin: 0;
    padding: 0 10px;
  }

  .snap-list li {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 8px 6px;
    border-radius: var(--radius-sm);
  }

  .snap-list li:hover {
    background: var(--bg-hover);
  }

  .snap-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .snap-date {
    font-size: 13px;
    font-weight: 600;
  }

  .snap-meta {
    font-size: 11px;
    color: var(--text-muted);
  }

  .snap-msg {
    font-size: 12px;
    color: var(--text-muted);
    font-style: italic;
  }

  .snap-actions {
    display: flex;
    gap: 2px;
  }
</style>
