<script lang="ts">
  import type { ChangeEntry, Character, Entity, IaStatus, Relationship } from "$lib/types";

  let {
    characters,
    relationships,
    entities,
    analyzing,
    enriching,
    progress,
    iaStatus,
    changeLog,
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
    onclose,
  }: {
    characters: Character[];
    relationships: Relationship[];
    entities: Entity[];
    analyzing: boolean;
    enriching: boolean;
    progress: { done: number; total: number; phase: string } | null;
    iaStatus: IaStatus | null;
    changeLog: ChangeEntry[];
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
    onclose: () => void;
  } = $props();

  type Tab = "characters" | "relationships" | "factions" | "locations" | "log";
  let tab = $state<Tab>("characters");
  let selectedCharacterId = $state<string | null>(null);
  let showSettings = $state(false);

  const locations = $derived(entities.filter((e) => e.entity_type === "location"));
  const factions = $derived(entities.filter((e) => e.entity_type === "faction"));
  const selectedCharacter = $derived(
    characters.find((c) => c.id === selectedCharacterId) ?? null,
  );
  const entityList = $derived(
    tab === "factions" ? factions : tab === "locations" ? locations : [],
  );

  const progressPct = $derived(
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.done / progress.total) * 100))
      : 0,
  );

  function nameFor(id: string): string {
    return characters.find((c) => c.id === id)?.name ?? "?";
  }

  function kindLabel(kind: string | null): string | null {
    if (!kind) return null;
    return kind.charAt(0).toUpperCase() + kind.slice(1);
  }
</script>

<div class="wb-overlay">
  <button class="wb-backdrop" aria-label="Cerrar" onclick={onclose}></button>

  <div class="wb-dialog" role="dialog" aria-modal="true">
    <header class="wb-head">
      <div class="wb-title">
        <h2>World Building</h2>
        <span class="wb-sub">Personajes, relaciones y mundo de tu novela</span>
      </div>
      <div class="wb-actions">
        <button class="btn btn-primary" onclick={onanalyze} disabled={analyzing}>
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
        <button
          class="btn-icon"
          class:is-active={showSettings}
          title="Configuración de IA"
          onclick={() => (showSettings = !showSettings)}
        >⚙</button>
        <button class="btn-icon" title="Cerrar" onclick={onclose}>×</button>
      </div>
    </header>

    {#if analyzing}
      <div class="wb-progress">
        <div class="progress-bar">
          <div class="progress-fill" style:width="{progressPct}%"></div>
        </div>
        <span class="progress-label">
          {progress
            ? progress.phase === "informe"
              ? "Generando informe…"
              : `Escenas ${progress.done}/${progress.total}`
            : "Analizando…"}
        </span>
      </div>
    {/if}

    {#if showSettings}
      <div class="wb-settings">
        {#if iaStatus && !iaStatus.onnxruntime_available}
          <div class="runtime-row">
            <p class="ia-note">
              Los motores de IA locales (GLiNER2 y embeddings) requieren el runtime
              de ONNX Runtime. Se instala una sola vez y con tu permiso.
            </p>
            <button class="btn" onclick={oninstallonnxruntime}>
              Instalar runtime ONNX
            </button>
          </div>
        {/if}

        {#if iaStatus}
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
            <button class="btn" onclick={oninstallgliner}>Descargar GLiNER2</button>
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
            <button class="btn" onclick={oninstallembeddings}>Descargar modelo</button>
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
        {/if}
      </div>
    {/if}

    <nav class="wb-tabs">
      <button class="tab" class:active={tab === "characters"} onclick={() => (tab = "characters")}>
        Personajes
      </button>
      <button class="tab" class:active={tab === "relationships"} onclick={() => (tab = "relationships")}>
        Relaciones
      </button>
      <button class="tab" class:active={tab === "factions"} onclick={() => (tab = "factions")}>
        Facciones
      </button>
      <button class="tab" class:active={tab === "locations"} onclick={() => (tab = "locations")}>
        Lugares
      </button>
      <button class="tab" class:active={tab === "log"} onclick={() => (tab = "log")}>
        Registro
      </button>
    </nav>

    <div class="wb-body">
      {#if tab === "characters"}
        {#if characters.length === 0}
          <p class="wb-empty">
            Pulsa <strong>Analizar</strong> para detectar personajes y relaciones del
            manuscrito (IA local, sin conexión).
          </p>
        {:else}
          <div class="wb-split">
            <ul class="wb-list">
              {#each characters as c (c.id)}
                <li>
                  <button
                    class="wb-item"
                    class:active={c.id === selectedCharacterId}
                    onclick={() => (selectedCharacterId = c.id)}
                  >
                    <span class="wb-item-name">{c.name}</span>
                    {#if c.role}
                      <span class="wb-item-meta">{c.role}</span>
                    {/if}
                    {#if c.aliases.length > 0}
                      <span class="wb-item-aliases">{c.aliases.join(", ")}</span>
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

            <div class="wb-detail">
              {#if selectedCharacter}
                <h3>{selectedCharacter.name}</h3>
                {#if selectedCharacter.role}
                  <span class="wb-role">{selectedCharacter.role}</span>
                {/if}
                <dl>
                  {#if selectedCharacter.aliases.length > 0}
                    <div><dt>Alias</dt><dd>{selectedCharacter.aliases.join(", ")}</dd></div>
                  {/if}
                  {#if selectedCharacter.description}
                    <div><dt>Descripción</dt><dd>{selectedCharacter.description}</dd></div>
                  {/if}
                  {#if selectedCharacter.biography}
                    <div><dt>Biografía</dt><dd>{selectedCharacter.biography}</dd></div>
                  {/if}
                  {#if selectedCharacter.traits.length > 0}
                    <div><dt>Rasgos</dt><dd>{selectedCharacter.traits.join(", ")}</dd></div>
                  {/if}
                  {#if selectedCharacter.goals.length > 0}
                    <div><dt>Objetivos</dt><dd>{selectedCharacter.goals.join(", ")}</dd></div>
                  {/if}
                  {#if selectedCharacter.arc}
                    <div><dt>Arco</dt><dd>{selectedCharacter.arc}</dd></div>
                  {/if}
                </dl>
              {:else}
                <p class="wb-empty">Selecciona un personaje para ver su ficha técnica.</p>
              {/if}
            </div>
          </div>
        {/if}
      {:else if tab === "relationships"}
        {#if relationships.length === 0}
          <p class="wb-empty">Aún no hay relaciones detectadas.</p>
        {:else}
          <ul class="wb-rel">
            {#each relationships as r (r.id)}
              <li>
                <span class="wb-rel-pair">
                  {nameFor(r.character_a_id)} ↔ {nameFor(r.character_b_id)}
                  {#if r.kind}<span class="wb-kind">{kindLabel(r.kind)}</span>{/if}
                </span>
                {#if r.description}<span class="wb-rel-desc">{r.description}</span>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else if tab === "factions" || tab === "locations"}
        {#if entityList.length === 0}
          <p class="wb-empty">
            Activa <strong>GLiNER2</strong> y pulsa <strong>Analizar</strong> para
            detectar {tab === "factions" ? "facciones y organizaciones" : "lugares"}.
          </p>
        {:else}
          <ul class="wb-list">
            {#each entityList as e (e.id)}
              <li>
                <div class="wb-item">
                  <span class="wb-item-name">{e.name}</span>
                  <span class="wb-item-meta">{e.mention_count} menciones</span>
                  {#if e.aliases.length > 0}
                    <span class="wb-item-aliases">{e.aliases.join(", ")}</span>
                  {/if}
                  {#if e.description}
                    <span class="wb-item-desc">{e.description}</span>
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
      {:else}
        {#if changeLog.length === 0}
          <p class="wb-empty">Los cambios del análisis aparecerán aquí.</p>
        {:else}
          <ul class="wb-log">
            {#each changeLog as entry, i (i)}
              <li>
                <span class="wb-log-time">{entry.time}</span>
                <span class="wb-log-summary">{entry.summary}</span>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .wb-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .wb-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    border: none;
  }

  .wb-dialog {
    position: relative;
    width: min(1080px, 94vw);
    height: min(760px, 90vh);
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-md);
    overflow: hidden;
  }

  .wb-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .wb-title h2 {
    font-family: var(--font-serif);
    font-size: 20px;
    margin: 0;
  }

  .wb-sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  .wb-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .wb-progress {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 20px;
    border-bottom: 1px solid var(--border);
  }

  .progress-bar {
    flex: 1;
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
    white-space: nowrap;
  }

  .wb-settings {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .runtime-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-basis: 100%;
  }

  .ia-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-faint);
  }

  .field {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .field select {
    font: inherit;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    cursor: pointer;
  }

  .wb-tabs {
    display: flex;
    gap: 2px;
    padding: 8px 14px 0;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    font: inherit;
    font-size: 13px;
    padding: 7px 12px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  }

  .tab.active {
    color: var(--text);
    font-weight: 600;
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .wb-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px 20px;
  }

  .wb-empty {
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.6;
    margin: 0;
    padding: 12px 0;
  }

  .wb-split {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 20px;
    align-items: start;
  }

  .wb-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .wb-list li {
    display: flex;
    align-items: flex-start;
    gap: 4px;
  }

  .wb-item {
    flex: 1;
    min-width: 0;
    text-align: left;
    border: none;
    background: transparent;
    color: inherit;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .wb-item:hover {
    background: var(--bg-hover);
  }

  .wb-item.active {
    background: var(--accent-soft);
  }

  .wb-item-name {
    font-weight: 600;
    font-size: 14px;
  }

  .wb-item-meta {
    font-size: 11px;
    color: var(--accent);
    font-weight: 500;
  }

  .wb-item-aliases {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wb-item-desc {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .wb-detail {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .wb-detail h3 {
    font-family: var(--font-serif);
    font-size: 20px;
  }

  .wb-role {
    color: var(--accent);
    font-weight: 600;
    font-size: 13px;
  }

  .wb-detail dl {
    margin: 8px 0 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .wb-detail dt {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 2px;
  }

  .wb-detail dd {
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
  }

  .wb-rel {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .wb-rel li {
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .wb-rel li:hover {
    background: var(--bg-hover);
  }

  .wb-rel-pair {
    font-size: 14px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .wb-kind {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: var(--accent-soft);
    border-radius: 4px;
    padding: 1px 6px;
  }

  .wb-rel-desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .wb-log {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .wb-log li {
    display: flex;
    gap: 12px;
    align-items: baseline;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
  }

  .wb-log li:hover {
    background: var(--bg-hover);
  }

  .wb-log-time {
    font-size: 11px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .wb-log-summary {
    font-size: 13px;
    color: var(--text);
  }
</style>
