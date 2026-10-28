<script lang="ts">
  import type { ChangeEntry, Character, Entity, Relationship } from "$lib/types";

  let {
    characters,
    relationships,
    entities,
    analyzing,
    enriching,
    progress,
    changeLog,
    onanalyze,
    onenrich,
    ondeletecharacter,
    ondeleteentity,
    onupdatecharacter,
    onupdateentity,
    onclose,
  }: {
    characters: Character[];
    relationships: Relationship[];
    entities: Entity[];
    analyzing: boolean;
    enriching: boolean;
    progress: { done: number; total: number; phase: string } | null;
    changeLog: ChangeEntry[];
    onanalyze: () => void;
    onenrich: () => void;
    ondeletecharacter: (id: string) => void;
    ondeleteentity: (id: string) => void;
    onupdatecharacter: (character: Character) => void | Promise<void>;
    onupdateentity: (entity: Entity) => void | Promise<void>;
    onclose: () => void;
  } = $props();

  type Tab = "characters" | "relationships" | "factions" | "locations" | "log";
  let tab = $state<Tab>("characters");
  let selectedCharacterId = $state<string | null>(null);
  let search = $state("");

  let editingCharacter = $state<Character | null>(null);
  let editingEntity = $state<Entity | null>(null);
  let form = $state({
    name: "",
    role: "",
    aliases: "",
    description: "",
    biography: "",
    traits: "",
    goals: "",
    arc: "",
    color: "",
    entityType: "location",
  });

  function splitList(value: string): string[] {
    return value
      .split(",")
      .map((v) => v.trim())
      .filter(Boolean);
  }

  function startEditCharacter(c: Character) {
    editingCharacter = c;
    form = {
      name: c.name,
      role: c.role ?? "",
      aliases: c.aliases.join(", "),
      description: c.description ?? "",
      biography: c.biography ?? "",
      traits: c.traits.join(", "),
      goals: c.goals.join(", "),
      arc: c.arc ?? "",
      color: c.color ?? "",
      entityType: "location",
    };
  }

  function startEditEntity(e: Entity) {
    editingEntity = e;
    form = {
      ...form,
      name: e.name,
      aliases: e.aliases.join(", "),
      description: e.description ?? "",
      entityType: e.entity_type,
    };
  }

  async function saveCharacter() {
    if (!editingCharacter) return;
    await onupdatecharacter({
      ...editingCharacter,
      name: form.name.trim(),
      role: form.role.trim() || null,
      aliases: splitList(form.aliases),
      description: form.description.trim() || null,
      biography: form.biography.trim() || null,
      traits: splitList(form.traits),
      goals: splitList(form.goals),
      arc: form.arc.trim() || null,
      color: form.color.trim() || null,
    });
    editingCharacter = null;
  }

  async function saveEntity() {
    if (!editingEntity) return;
    await onupdateentity({
      ...editingEntity,
      name: form.name.trim(),
      entity_type: form.entityType,
      aliases: splitList(form.aliases),
      description: form.description.trim() || null,
    });
    editingEntity = null;
  }

  const locations = $derived(entities.filter((e) => e.entity_type === "location"));
  const factions = $derived(entities.filter((e) => e.entity_type === "faction"));
  const selectedCharacter = $derived(
    characters.find((c) => c.id === selectedCharacterId) ?? null,
  );
  const entityList = $derived(
    tab === "factions" ? factions : tab === "locations" ? locations : [],
  );

  const filteredCharacters = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return characters;
    return characters.filter((c) =>
      [
        c.name,
        c.aliases.join(" "),
        c.description ?? "",
        c.biography ?? "",
        c.traits.join(" "),
        c.goals.join(" "),
        c.arc ?? "",
      ]
        .join(" ")
        .toLowerCase()
        .includes(q),
    );
  });

  const filteredEntities = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return entityList;
    return entityList.filter((e) =>
      [e.name, e.aliases.join(" "), e.description ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(q),
    );
  });

  const filteredRelationships = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return relationships;
    return relationships.filter((r) =>
      [nameFor(r.character_a_id), nameFor(r.character_b_id), r.kind ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(q),
    );
  });

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

  function relationshipsFor(id: string): Relationship[] {
    return relationships.filter((r) => r.character_a_id === id || r.character_b_id === id);
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
        <button class="btn-icon" title="Cerrar" onclick={onclose}>×</button>
      </div>
    </header>

    <div class="wb-search">
      <input
        class="input"
        bind:value={search}
        placeholder="Buscar en el mundo (personajes, lugares, facciones)…"
      />
    </div>

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
              {#each filteredCharacters as c (c.id)}
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
                <div class="wb-detail-head">
                  <h3>{selectedCharacter.name}</h3>
                  <button
                    class="btn"
                    onclick={() => startEditCharacter(selectedCharacter)}
                  >
                    Editar ficha
                  </button>
                </div>
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
                <div class="wb-profile-rel">
                  <h4>Relaciones</h4>
                  {#if relationshipsFor(selectedCharacter.id).length === 0}
                    <p class="wb-empty">Sin relaciones detectadas.</p>
                  {:else}
                    <ul class="wb-rel-links">
                      {#each relationshipsFor(selectedCharacter.id) as r (r.id)}
                        <li>
                          <button
                            class="wb-link"
                            onclick={() =>
                              (selectedCharacterId =
                                r.character_a_id === selectedCharacter.id
                                  ? r.character_b_id
                                  : r.character_a_id)}
                          >
                            {nameFor(
                              r.character_a_id === selectedCharacter.id
                                ? r.character_b_id
                                : r.character_a_id,
                            )}
                          </button>
                          {#if r.kind}<span class="wb-kind">{kindLabel(r.kind)}</span>{/if}
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </div>
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
            {#each filteredRelationships as r (r.id)}
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
            {#each filteredEntities as e (e.id)}
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
                  title="Editar"
                  onclick={() => startEditEntity(e)}
                >✎</button>
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

    {#if editingCharacter}
      <div
        class="wb-edit-overlay"
        role="presentation"
        tabindex="-1"
        onclick={() => (editingCharacter = null)}
        onkeydown={(e) => {
          if (e.key === "Escape") editingCharacter = null;
        }}
      >
        <div
          class="wb-edit"
          role="dialog"
          aria-modal="true"
          aria-label="Editar personaje"
          tabindex="-1"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
        >
          <h3>Editar personaje</h3>
          <label>Nombre<input class="input" bind:value={form.name} /></label>
          <label>
            Rol
            <input class="input" bind:value={form.role} placeholder="protagonista, antagonista…" />
          </label>
          <label>Alias (separados por comas)<input class="input" bind:value={form.aliases} /></label>
          <label>Descripción<textarea class="input" bind:value={form.description} rows="2"></textarea></label>
          <label>Biografía<textarea class="input" bind:value={form.biography} rows="3"></textarea></label>
          <label>Rasgos (separados por comas)<input class="input" bind:value={form.traits} /></label>
          <label>Objetivos (separados por comas)<input class="input" bind:value={form.goals} /></label>
          <label>Arco<textarea class="input" bind:value={form.arc} rows="2"></textarea></label>
          <div class="wb-edit-actions">
            <button class="btn" onclick={() => (editingCharacter = null)}>Cancelar</button>
            <button class="btn btn-primary" onclick={saveCharacter}>Guardar</button>
          </div>
        </div>
      </div>
    {/if}

    {#if editingEntity}
      <div
        class="wb-edit-overlay"
        role="presentation"
        tabindex="-1"
        onclick={() => (editingEntity = null)}
        onkeydown={(e) => {
          if (e.key === "Escape") editingEntity = null;
        }}
      >
        <div
          class="wb-edit"
          role="dialog"
          aria-modal="true"
          aria-label="Editar entidad"
          tabindex="-1"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
        >
          <h3>Editar entidad</h3>
          <label>Nombre<input class="input" bind:value={form.name} /></label>
          <label>
            Tipo
            <select class="input" bind:value={form.entityType}>
              <option value="location">Lugar</option>
              <option value="faction">Facción</option>
            </select>
          </label>
          <label>Alias (separados por comas)<input class="input" bind:value={form.aliases} /></label>
          <label>Descripción<textarea class="input" bind:value={form.description} rows="3"></textarea></label>
          <div class="wb-edit-actions">
            <button class="btn" onclick={() => (editingEntity = null)}>Cancelar</button>
            <button class="btn btn-primary" onclick={saveEntity}>Guardar</button>
          </div>
        </div>
      </div>
    {/if}
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

  .wb-search {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }

  .wb-profile-rel {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .wb-profile-rel h4 {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .wb-rel-links {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .wb-rel-links li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .wb-link {
    border: none;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0;
    text-align: left;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .wb-link:hover {
    color: var(--accent-hover);
  }

  .wb-detail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .wb-edit-overlay {
    position: absolute;
    inset: 0;
    background: var(--overlay, rgb(0 0 0 / 0.42));
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    padding: 24px;
  }

  .wb-edit {
    width: min(520px, 100%);
    max-height: 85vh;
    overflow-y: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow-lg, 0 16px 48px rgb(0 0 0 / 0.25));
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .wb-edit h3 {
    margin: 0 0 4px;
  }

  .wb-edit label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--text-secondary, var(--text-muted));
  }

  .wb-edit textarea.input {
    resize: vertical;
    font-family: inherit;
  }

  .wb-edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
</style>
