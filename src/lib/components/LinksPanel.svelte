<script lang="ts">
  import * as api from "$lib/api";
  import type {
    Affiliation,
    Character,
    Entity,
    EntityRelation,
  } from "$lib/types";

  let {
    projectId,
    characters,
    entities,
    refreshKey = 0,
    onchanged,
    onlog,
  }: {
    projectId: string;
    characters: Character[];
    entities: Entity[];
    refreshKey?: number;
    onchanged: () => void;
    onlog?: (message: string) => void;
  } = $props();

  const ROLE_LABEL: Record<string, string> = {
    leader: "Líder",
    member: "Miembro",
    ally: "Aliado",
    rival: "Rival",
    resident: "Residente",
    origin: "Origen",
    visitor: "Visitante",
  };
  const KIND_LABEL: Record<string, string> = {
    alliance: "Alianza",
    conflict: "Conflicto",
    territory: "Territorio",
  };
  const FACTION_ROLES = ["leader", "member", "ally", "rival"];
  const LOCATION_ROLES = ["resident", "origin", "visitor"];

  let affiliations = $state<Affiliation[]>([]);
  let relations = $state<EntityRelation[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let detecting = $state(false);
  let error = $state<string | null>(null);

  let characterId = $state("");
  let entityId = $state("");
  let role = $state("member");

  let aId = $state("");
  let bId = $state("");
  let kind = $state("alliance");

  let loadToken = 0;

  const linkableEntities = $derived(
    entities.filter((e) => e.entity_type === "faction" || e.entity_type === "location"),
  );
  const factions = $derived(entities.filter((e) => e.entity_type === "faction"));
  const locations = $derived(entities.filter((e) => e.entity_type === "location"));

  const selectedEntity = $derived(
    linkableEntities.find((e) => e.id === entityId) ?? null,
  );
  const roleOptions = $derived(
    selectedEntity?.entity_type === "location" ? LOCATION_ROLES : FACTION_ROLES,
  );

  const aOptions = $derived(kind === "territory" ? factions : factions);
  const bOptions = $derived(kind === "territory" ? locations : factions);

  const pendingAffiliations = $derived(
    affiliations.filter((a) => a.source === "ai").length,
  );
  const pendingRelations = $derived(
    relations.filter((r) => r.source === "ai").length,
  );

  async function load() {
    const token = ++loadToken;
    if (!projectId) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      const [affs, rels] = await Promise.all([
        api.listAffiliations(projectId),
        api.listEntityRelations(projectId),
      ]);
      if (token === loadToken) {
        affiliations = affs;
        relations = rels;
      }
    } catch (e) {
      if (token === loadToken) error = String(e);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void projectId;
    void refreshKey;
    void load();
  });

  $effect(() => {
    if (!roleOptions.includes(role)) {
      role = roleOptions[0] ?? "member";
    }
  });

  $effect(() => {
    if (aId && !aOptions.some((e) => e.id === aId)) aId = "";
    if (bId && !bOptions.some((e) => e.id === bId)) bId = "";
  });

  $effect(() => {
    if (!characterId && characters.length > 0) characterId = characters[0].id;
    if (!entityId && linkableEntities.length > 0) entityId = linkableEntities[0].id;
  });

  function entityTypeLabel(type: string): string {
    return type === "faction" ? "Facción" : type === "location" ? "Lugar" : type;
  }

  function roleLabel(value: string): string {
    return ROLE_LABEL[value] ?? value;
  }

  function kindLabel(value: string): string {
    return KIND_LABEL[value] ?? value;
  }

  async function addAffiliation() {
    if (!characterId || !entityId) return;
    busy = true;
    error = null;
    try {
      await api.setAffiliation(projectId, characterId, entityId, role);
      await load();
      onchanged();
      onlog?.("Afiliación añadida");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function addRelation() {
    if (!aId || !bId || aId === bId) return;
    busy = true;
    error = null;
    try {
      await api.setEntityRelation(projectId, aId, bId, kind);
      aId = "";
      bId = "";
      await load();
      onchanged();
      onlog?.(`Relación añadida: ${kindLabel(kind)}`);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function acceptAffiliationRow(row: Affiliation) {
    busy = true;
    try {
      await api.acceptAffiliation(row.id);
      await load();
      onchanged();
      onlog?.(`Afiliación aceptada: ${row.character_name} → ${row.entity_name}`);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function rejectAffiliationRow(row: Affiliation) {
    busy = true;
    try {
      await api.rejectAffiliation(row.id);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function deleteAffiliationRow(row: Affiliation) {
    if (!window.confirm(`¿Eliminar el vínculo ${row.character_name} → ${row.entity_name}?`)) {
      return;
    }
    busy = true;
    try {
      await api.deleteAffiliation(row.id);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function acceptRelationRow(row: EntityRelation) {
    busy = true;
    try {
      await api.acceptEntityRelation(row.id);
      await load();
      onchanged();
      onlog?.(`Relación aceptada: ${row.a_name} ↔ ${row.b_name}`);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function rejectRelationRow(row: EntityRelation) {
    busy = true;
    try {
      await api.rejectEntityRelation(row.id);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function deleteRelationRow(row: EntityRelation) {
    if (!window.confirm(`¿Eliminar la relación ${row.a_name} ↔ ${row.b_name}?`)) return;
    busy = true;
    try {
      await api.deleteEntityRelation(row.id);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function detect() {
    detecting = true;
    error = null;
    try {
      const report = await api.extractAffiliations(projectId);
      await load();
      onchanged();
      onlog?.(
        `Afiliaciones IA: ${report.proposed} propuestas, ${report.insufficient} sin evidencia`,
      );
    } catch (e) {
      error = String(e);
    } finally {
      detecting = false;
    }
  }

  function confidence(value: number | null): string {
    return value === null ? "" : `${Math.round(value * 100)} %`;
  }
</script>

<div class="links">
  <header class="links-head">
    <div>
      <h4>Vínculos del mundo</h4>
      <p class="links-muted">
        Personajes en facciones y lugares, y relaciones entre facciones o
        territorios. La IA propone afiliaciones con cita textual; las manuales
        nunca se sobrescriben.
      </p>
    </div>
    <button
      class="btn btn-primary"
      onclick={detect}
      disabled={detecting || characters.length === 0 || linkableEntities.length === 0}
    >
      {detecting ? "Detectando…" : "Detectar afiliaciones con IA"}
    </button>
  </header>

  {#if error}
    <p class="links-error" role="alert">{error}</p>
  {/if}

  {#if loading && affiliations.length === 0 && relations.length === 0}
    <p class="links-muted">Cargando vínculos…</p>
  {:else}
    <section class="links-section">
      <h5>
        Personajes en facciones y lugares
        {#if pendingAffiliations > 0}
          <span class="links-badge">{pendingAffiliations} por revisar</span>
        {/if}
      </h5>

      {#if characters.length === 0 || linkableEntities.length === 0}
        <p class="links-muted">
          Necesitas al menos un personaje y una facción o lugar. Usa
          <strong>Analizar</strong> o edita las fichas del mundo.
        </p>
      {:else}
        <div class="links-form">
          <label>
            <span>Personaje</span>
            <select class="input" bind:value={characterId}>
              {#each characters as c (c.id)}
                <option value={c.id}>{c.name}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Facción o lugar</span>
            <select class="input" bind:value={entityId}>
              {#each linkableEntities as e (e.id)}
                <option value={e.id}>{e.name} · {entityTypeLabel(e.entity_type)}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Rol</span>
            <select class="input" bind:value={role}>
              {#each roleOptions as option (option)}
                <option value={option}>{roleLabel(option)}</option>
              {/each}
            </select>
          </label>
          <button class="btn" onclick={addAffiliation} disabled={busy}>
            Añadir vínculo
          </button>
        </div>
      {/if}

      {#if affiliations.length === 0}
        <p class="links-empty">
          Aún no hay afiliaciones. Añádelas a mano o usa la detección con IA.
        </p>
      {:else}
        <ul class="links-list">
          {#each affiliations as row (row.id)}
            <li class="links-row" class:is-ai={row.source === "ai"}>
              <div class="links-main">
                <span class="links-names">
                  <strong>{row.character_name}</strong>
                  <span class="links-arrow">→</span>
                  <strong>{row.entity_name}</strong>
                </span>
                <span class="links-chip">{roleLabel(row.role)}</span>
                <span class="links-chip">{entityTypeLabel(row.entity_type)}</span>
                {#if row.source === "ai"}
                  <span class="links-chip links-chip-ai">IA · {confidence(row.confidence)}</span>
                {/if}
              </div>
              {#if row.evidence}
                <blockquote class="links-evidence">«{row.evidence}»</blockquote>
              {/if}
              <div class="links-actions">
                {#if row.source === "ai"}
                  <button class="btn btn-primary" onclick={() => acceptAffiliationRow(row)} disabled={busy}>
                    Aceptar
                  </button>
                  <button class="btn" onclick={() => rejectAffiliationRow(row)} disabled={busy}>
                    Descartar
                  </button>
                {:else}
                  <button class="btn-icon" title="Eliminar vínculo" onclick={() => deleteAffiliationRow(row)} disabled={busy}>
                    ×
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="links-section">
      <h5>
        Relaciones entre facciones y lugares
        {#if pendingRelations > 0}
          <span class="links-badge">{pendingRelations} por revisar</span>
        {/if}
      </h5>

      {#if factions.length === 0}
        <p class="links-muted">
          Necesitas al menos una facción para crear alianzas, conflictos o
          territorios.
        </p>
      {:else}
        <div class="links-form">
          <label>
            <span>{kind === "territory" ? "Facción" : "Entidad A"}</span>
            <select class="input" bind:value={aId}>
              <option value="">Elegir…</option>
              {#each aOptions as e (e.id)}
                <option value={e.id}>{e.name}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Tipo</span>
            <select class="input" bind:value={kind}>
              <option value="alliance">Alianza</option>
              <option value="conflict">Conflicto</option>
              <option value="territory">Territorio</option>
            </select>
          </label>
          <label>
            <span>{kind === "territory" ? "Lugar" : "Entidad B"}</span>
            <select class="input" bind:value={bId}>
              <option value="">Elegir…</option>
              {#each bOptions as e (e.id)}
                <option value={e.id}>{e.name}</option>
              {/each}
            </select>
          </label>
          <button class="btn" onclick={addRelation} disabled={busy || !aId || !bId || aId === bId}>
            Añadir relación
          </button>
        </div>
      {/if}

      {#if relations.length === 0}
        <p class="links-empty">
          Aún no hay alianzas, conflictos ni territorios definidos.
        </p>
      {:else}
        <ul class="links-list">
          {#each relations as row (row.id)}
            <li class="links-row" class:is-ai={row.source === "ai"}>
              <div class="links-main">
                <span class="links-names">
                  <strong>{row.a_name}</strong>
                  <span class="links-arrow">↔</span>
                  <strong>{row.b_name}</strong>
                </span>
                <span class="links-chip">{kindLabel(row.kind)}</span>
                {#if row.source === "ai"}
                  <span class="links-chip links-chip-ai">IA · {confidence(row.confidence)}</span>
                {/if}
              </div>
              {#if row.evidence}
                <blockquote class="links-evidence">«{row.evidence}»</blockquote>
              {/if}
              <div class="links-actions">
                {#if row.source === "ai"}
                  <button class="btn btn-primary" onclick={() => acceptRelationRow(row)} disabled={busy}>
                    Aceptar
                  </button>
                  <button class="btn" onclick={() => rejectRelationRow(row)} disabled={busy}>
                    Descartar
                  </button>
                {:else}
                  <button class="btn-icon" title="Eliminar relación" onclick={() => deleteRelationRow(row)} disabled={busy}>
                    ×
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  .links {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 900px;
  }

  .links-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  .links-head h4 {
    margin: 0 0 4px;
  }

  .links-section h5 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .links-badge {
    font-size: 10px;
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
    text-transform: none;
    letter-spacing: 0;
  }

  .links-muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .links-empty {
    margin: 8px 0 0;
    color: var(--text-muted);
    font-size: 12px;
    font-style: italic;
  }

  .links-error {
    margin: 0;
    color: var(--danger, #d64545);
    font-size: 12px;
  }

  .links-form {
    display: grid;
    grid-template-columns: 1fr 1fr 130px auto;
    gap: 8px;
    align-items: end;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .links-form label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .links-list {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .links-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .links-row.is-ai {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .links-main {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    flex: 1;
    min-width: 0;
  }

  .links-names {
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .links-arrow {
    color: var(--text-muted);
    margin: 0 4px;
  }

  .links-chip {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .links-chip-ai {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    color: var(--accent);
  }

  .links-evidence {
    flex-basis: 100%;
    margin: 2px 0 0;
    padding-left: 10px;
    border-left: 3px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    color: var(--text-muted);
    font-size: 11px;
    font-style: italic;
    overflow-wrap: anywhere;
  }

  .links-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  @media (max-width: 760px) {
    .links-form {
      grid-template-columns: 1fr;
    }
  }
</style>
