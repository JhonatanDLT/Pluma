<script lang="ts">
  import type { Character, IaStatus, Relationship } from "$lib/types";

  let {
    characters,
    relationships,
    analyzing,
    enriching,
    iaStatus,
    onanalyze,
    onenrich,
    ondeletecharacter,
    onupdateembeddings,
    oninstallembeddings,
  }: {
    characters: Character[];
    relationships: Relationship[];
    analyzing: boolean;
    enriching: boolean;
    iaStatus: IaStatus | null;
    onanalyze: () => void;
    onenrich: () => void;
    ondeletecharacter: (id: string) => void;
    onupdateembeddings: (enabled: boolean) => void;
    oninstallembeddings: () => void;
  } = $props();

  function nameFor(id: string): string {
    return characters.find((c) => c.id === id)?.name ?? "?";
  }

  function kindLabel(kind: string | null): string | null {
    if (!kind) return null;
    return kind.charAt(0).toUpperCase() + kind.slice(1);
  }
</script>

<div class="chars-panel">
  <div class="panel-head">
    <span class="label">Personajes</span>
    <div class="head-actions">
      <button class="btn" onclick={onanalyze} disabled={analyzing}>
        {analyzing ? "Analizando…" : "Analizar"}
      </button>
      <button
        class="btn"
        onclick={onenrich}
        disabled={enriching || characters.length === 0}
        title="Genera descripciones con un LLM local (Ollama)"
      >
        {enriching ? "Enriqueciendo…" : "Enriquecer"}
      </button>
    </div>
  </div>

  {#if iaStatus}
    <div class="ia-settings">
      <label class="toggle">
        <input
          type="checkbox"
          checked={iaStatus.settings.embeddings}
          onchange={(e) => onupdateembeddings(e.currentTarget.checked)}
        />
        <span>Embeddings (semántica)</span>
      </label>

      {#if iaStatus.settings.embeddings && !iaStatus.embeddings_cached}
        <button
          class="btn"
          onclick={oninstallembeddings}
          title="Descarga el modelo (~120 MB) una sola vez"
        >Descargar modelo</button>
      {/if}

      {#if !iaStatus.ollama_available}
        <p class="ia-note">Ollama no detectado: "Enriquecer" no estará disponible.</p>
      {/if}
    </div>
  {/if}

  {#if characters.length === 0}
    <p class="hint">
      Pulsa <strong>Analizar</strong> para detectar personajes y relaciones del
      manuscrito (IA local, sin conexión). Con <strong>Enriquecer</strong> puedes
      generar descripciones con un LLM local (Ollama).
    </p>
  {:else}
    <ul class="char-list">
      {#each characters as c (c.id)}
        <li>
          <div class="char-main">
            <span class="char-name">{c.name}</span>
            <span class="char-meta">
              {#if c.role}<span class="role">{c.role}</span>{/if}
              {#if c.aliases.length > 0}
                <span class="aliases">{c.aliases.join(", ")}</span>
              {/if}
            </span>
            {#if c.description}
              <p class="char-desc">{c.description}</p>
            {/if}
          </div>
          <button
            class="btn-icon"
            title="Eliminar personaje"
            onclick={() => ondeletecharacter(c.id)}
          >×</button>
        </li>
      {/each}
    </ul>

    {#if relationships.length > 0}
      <div class="panel-subhead">Relaciones</div>
      <ul class="rel-list">
        {#each relationships as r (r.id)}
          <li>
            <span class="rel-pair">
              {nameFor(r.character_a_id)} ↔ {nameFor(r.character_b_id)}
              {#if r.kind}<span class="rel-kind">{kindLabel(r.kind)}</span>{/if}
            </span>
            {#if r.description}<span class="rel-desc">{r.description}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .chars-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 14px 16px 8px;
  }

  .head-actions {
    display: flex;
    gap: 6px;
  }

  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .hint {
    padding: 0 16px;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.6;
  }

  .ia-settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 16px 10px;
    border-bottom: 1px solid var(--border);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }

  .ia-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-faint);
  }

  .char-list,
  .rel-list {
    list-style: none;
    margin: 0;
    padding: 0 10px;
  }

  .char-list li {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 8px 6px;
    border-radius: var(--radius-sm);
  }

  .char-list li:hover {
    background: var(--bg-hover);
  }

  .char-main {
    flex: 1;
    min-width: 0;
  }

  .char-name {
    font-weight: 600;
    font-size: 14px;
  }

  .char-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .role {
    color: var(--accent);
    font-weight: 500;
  }

  .aliases {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .char-desc {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .panel-subhead {
    padding: 16px 16px 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .rel-list li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 6px;
    border-radius: var(--radius-sm);
  }

  .rel-list li:hover {
    background: var(--bg-hover);
  }

  .rel-pair {
    font-size: 13px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .rel-kind {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: var(--accent-soft);
    border-radius: 4px;
    padding: 1px 6px;
  }

  .rel-desc {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
