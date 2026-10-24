<script lang="ts">
  import type { Character, Entity, IaStatus, Relationship } from "$lib/types";

  let {
    characters,
    relationships,
    entities,
    analyzing,
    enriching,
    progress,
    iaStatus,
    onanalyze,
    onenrich,
    ondeletecharacter,
    ondeleteentity,
    onupdateembeddings,
    oninstallembeddings,
    onupdatenerengine,
    oninstallgliner,
    oninstallonnxruntime,
    onupdatekeeploaded,
  }: {
    characters: Character[];
    relationships: Relationship[];
    entities: Entity[];
    analyzing: boolean;
    enriching: boolean;
    progress: { done: number; total: number; phase: string } | null;
    iaStatus: IaStatus | null;
    onanalyze: () => void;
    onenrich: () => void;
    ondeletecharacter: (id: string) => void;
    ondeleteentity: (id: string) => void;
    onupdateembeddings: (enabled: boolean) => void;
    oninstallembeddings: () => void;
    onupdatenerengine: (engine: string) => void;
    oninstallgliner: () => void;
    oninstallonnxruntime: () => void;
    onupdatekeeploaded: (enabled: boolean) => void;
  } = $props();

  const progressPct = $derived(
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.done / progress.total) * 100))
      : 0,
  );

  type Tab = "characters" | "locations" | "factions";
  let tab = $state<Tab>("characters");
  let selectedCharacterId = $state<string | null>(null);

  const locations = $derived(entities.filter((e) => e.entity_type === "location"));
  const factions = $derived(entities.filter((e) => e.entity_type === "faction"));
  const selectedCharacter = $derived(
    characters.find((c) => c.id === selectedCharacterId) ?? null,
  );

  function nameFor(id: string): string {
    return characters.find((c) => c.id === id)?.name ?? "?";
  }

  function kindLabel(kind: string | null): string | null {
    if (!kind) return null;
    return kind.charAt(0).toUpperCase() + kind.slice(1);
  }

  function selectTab(next: Tab) {
    tab = next;
    selectedCharacterId = null;
  }
</script>

<div class="chars-panel">
  <div class="panel-head">
    <span class="label">Análisis</span>
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

  {#if analyzing}
    <div class="progress-wrap">
      <div class="progress-bar">
        <div class="progress-fill" style:width="{progressPct}%"></div>
      </div>
      <span class="progress-label">
        {progress ? (progress.phase === "informe" ? "Generando informe…" : `Escenas ${progress.done}/${progress.total}`) : "Analizando…"}
      </span>
    </div>
  {/if}

  {#if iaStatus}
    <div class="ia-settings">
      {#if !iaStatus.onnxruntime_available}
        <div class="runtime-row">
          <p class="ia-note">
            Los motores de IA locales (GLiNER2 y embeddings) requieren el runtime
            de ONNX Runtime. Se instala una sola vez y con tu permiso.
          </p>
          <button
            class="btn"
            onclick={oninstallonnxruntime}
            title="Descarga e instala ONNX Runtime (~40 MB)"
          >Instalar runtime ONNX</button>
        </div>
      {/if}

      <label class="field">
        <span>Motor NER</span>
        <select
          value={iaStatus.settings.ner_engine}
          onchange={(e) => onupdatenerengine(e.currentTarget.value)}
        >
          <option value="rules">Reglas (ligero)</option>
          <option value="gliner">GLiNER2 (local)</option>
        </select>
      </label>

      {#if iaStatus.settings.ner_engine === "gliner" && !iaStatus.gliner_cached}
        <button
          class="btn"
          onclick={oninstallgliner}
          title="Descarga el modelo GLiNER2 (~1 GB) una sola vez"
        >Descargar GLiNER2</button>
      {/if}

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

      <label class="toggle">
        <input
          type="checkbox"
          checked={iaStatus.settings.keep_loaded}
          onchange={(e) => onupdatekeeploaded(e.currentTarget.checked)}
        />
        <span>Mantener modelos en memoria (alto rendimiento)</span>
      </label>

      {#if !iaStatus.ollama_available}
        <p class="ia-note">Ollama no detectado: "Enriquecer" no estará disponible.</p>
      {/if}
    </div>
  {/if}

  <div class="tabs">
    <button class="tab" class:active={tab === "characters"} onclick={() => selectTab("characters")}>
      Personajes
    </button>
    <button class="tab" class:active={tab === "locations"} onclick={() => selectTab("locations")}>
      Lugares
    </button>
    <button class="tab" class:active={tab === "factions"} onclick={() => selectTab("factions")}>
      Facciones
    </button>
  </div>

  {#if selectedCharacter}
    <div class="sheet">
      <div class="sheet-head">
        <span class="sheet-name">{selectedCharacter.name}</span>
        <button class="btn-icon" title="Cerrar ficha" onclick={() => (selectedCharacterId = null)}>×</button>
      </div>
      {#if selectedCharacter.role}
        <span class="role">{selectedCharacter.role}</span>
      {/if}
      {#if selectedCharacter.aliases.length > 0}
        <div class="sheet-row"><span class="sheet-label">Alias</span>{selectedCharacter.aliases.join(", ")}</div>
      {/if}
      {#if selectedCharacter.description}
        <div class="sheet-row"><span class="sheet-label">Descripción</span>{selectedCharacter.description}</div>
      {/if}
      {#if selectedCharacter.biography}
        <div class="sheet-row"><span class="sheet-label">Biografía</span>{selectedCharacter.biography}</div>
      {/if}
      {#if selectedCharacter.traits.length > 0}
        <div class="sheet-row"><span class="sheet-label">Rasgos</span>{selectedCharacter.traits.join(", ")}</div>
      {/if}
      {#if selectedCharacter.goals.length > 0}
        <div class="sheet-row"><span class="sheet-label">Objetivos</span>{selectedCharacter.goals.join(", ")}</div>
      {/if}
      {#if selectedCharacter.arc}
        <div class="sheet-row"><span class="sheet-label">Arco</span>{selectedCharacter.arc}</div>
      {/if}
    </div>
  {:else if tab === "characters"}
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
            <button class="char-main" onclick={() => (selectedCharacterId = c.id)}>
              <span class="char-name">{c.name}</span>
              <span class="char-meta">
                {#if c.role}<span class="role">{c.role}</span>{/if}
                {#if c.aliases.length > 0}
                  <span class="aliases">{c.aliases.join(", ")}</span>
                {/if}
              </span>
              {#if c.description}
                <span class="char-desc">{c.description}</span>
              {/if}
            </button>
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
  {:else}
    {#if (tab === "locations" ? locations : factions).length === 0}
      <p class="hint">
        Activa <strong>GLiNER2</strong> y pulsa <strong>Analizar</strong> para
        detectar {tab === "locations" ? "lugares (ciudades, regiones…)" : "facciones y organizaciones"}.
      </p>
    {:else}
      <ul class="char-list">
        {#each (tab === "locations" ? locations : factions) as e (e.id)}
          <li>
            <div class="char-main">
              <span class="char-name">{e.name}</span>
              <span class="char-meta">
                <span class="aliases">{e.mention_count} menciones</span>
                {#if e.aliases.length > 0}
                  <span class="aliases">{e.aliases.join(", ")}</span>
                {/if}
              </span>
              {#if e.description}
                <span class="char-desc">{e.description}</span>
              {/if}
            </div>
            <button
              class="btn-icon"
              title="Eliminar"
              onclick={() => ondeleteentity(e.id)}
            >×</button>
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

  .progress-wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
  }

  .progress-bar {
    height: 6px;
    background: var(--bg-hover);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.2s;
  }

  .progress-label {
    font-size: 11px;
    color: var(--text-muted);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .field select {
    font: inherit;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .runtime-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ia-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-faint);
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    flex: 1;
    font: inherit;
    font-size: 12px;
    padding: 6px 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .tab.active {
    background: var(--bg-hover);
    color: var(--text);
    font-weight: 600;
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
    text-align: left;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: inherit;
  }

  .char-name {
    font-weight: 600;
    font-size: 14px;
    display: block;
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
    display: block;
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

  .sheet {
    margin: 8px 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .sheet-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sheet-name {
    font-size: 16px;
    font-weight: 600;
  }

  .sheet-row {
    font-size: 13px;
    line-height: 1.55;
    color: var(--text);
  }

  .sheet-label {
    display: block;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 1px;
  }
</style>
