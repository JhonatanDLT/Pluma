<script lang="ts">
  import * as api from "$lib/api";
  import type { Character, CharacterFieldView, FieldType } from "$lib/types";

  let {
    character,
    projectId,
    refreshKey = 0,
    onreload,
    onlog,
  }: {
    character: Character;
    projectId: string;
    refreshKey?: number;
    onreload: () => void | Promise<void>;
    onlog?: (message: string) => void;
  } = $props();

  const HIDDEN_BUILTINS = new Set(["name", "color"]);
  const TYPE_LABEL: Record<FieldType, string> = {
    text: "Texto corto",
    longtext: "Texto largo",
    list: "Lista",
    number: "Número",
  };

  let fields = $state<CharacterFieldView[]>([]);
  let loading = $state(false);
  let enriching = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let showAll = $state(false);
  let editingId = $state<string | null>(null);
  let draft = $state("");
  let loadToken = 0;

  const visibleFields = $derived.by(() => {
    return fields.filter((f) => {
      if (HIDDEN_BUILTINS.has(f.key)) return false;
      if (showAll) return true;
      return f.value !== null || f.source === "ai";
    });
  });

  const pending = $derived(fields.filter((f) => f.source === "ai").length);

  function isEmptyField(f: CharacterFieldView): boolean {
    return f.value === null || f.value === "" || (Array.isArray(f.value) && f.value.length === 0);
  }

  function displayValue(f: CharacterFieldView): string {
    if (f.value === null || f.value === undefined) return "";
    if (Array.isArray(f.value)) return f.value.join(", ");
    return String(f.value);
  }

  function draftFor(f: CharacterFieldView): string {
    return displayValue(f);
  }

  function parseDraft(f: CharacterFieldView): string | string[] | number | null {
    const raw = draft.trim();
    if (!raw) return null;
    if (f.field_type === "list") {
      return raw
        .split(",")
        .map((v) => v.trim())
        .filter(Boolean);
    }
    if (f.field_type === "number") {
      const n = Number(raw.replace(",", "."));
      if (!Number.isFinite(n)) throw new Error("Introduce un número válido.");
      return n;
    }
    return raw;
  }

  async function load() {
    const token = ++loadToken;
    loading = true;
    error = null;
    try {
      const data = await api.listCharacterFields(character.id);
      if (token === loadToken) fields = data;
    } catch (e) {
      if (token === loadToken) error = String(e);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void character.id;
    void refreshKey;
    void load();
  });

  $effect(() => {
    // Al cambiar de personaje se cierra cualquier edición en curso.
    void character.id;
    editingId = null;
    draft = "";
  });

  async function saveField(f: CharacterFieldView) {
    saving = true;
    error = null;
    try {
      const value = parseDraft(f);
      await api.setCharacterField(character.id, f.field_id, value);
      editingId = null;
      await load();
      await onreload();
      onlog?.(`Campo actualizado: ${f.label}`);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function accept(f: CharacterFieldView) {
    saving = true;
    error = null;
    try {
      await api.acceptFieldSuggestion(character.id, f.field_id);
      await load();
      await onreload();
      onlog?.(`Sugerencia aceptada: ${f.label}`);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function reject(f: CharacterFieldView) {
    saving = true;
    error = null;
    try {
      await api.rejectFieldSuggestion(character.id, f.field_id);
      await load();
      onlog?.(`Sugerencia descartada: ${f.label}`);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function enrich() {
    enriching = true;
    error = null;
    try {
      const report = await api.enrichCharacterFields(projectId, character.id);
      await load();
      onlog?.(
        `Sugerencias para ${character.name}: ${report.suggested} propuestas, ${report.insufficient} sin evidencia`,
      );
    } catch (e) {
      error = String(e);
    } finally {
      enriching = false;
    }
  }

  function startEdit(f: CharacterFieldView) {
    editingId = f.field_id;
    draft = draftFor(f);
  }

  function confidencePct(value: number | null): string {
    if (value === null) return "—";
    return `${Math.round(value * 100)} %`;
  }
</script>

<div class="cf">
  <div class="cf-toolbar">
    <div class="cf-summary">
      {#if pending > 0}
        <span class="cf-badge">{pending} sugerencia{pending === 1 ? "" : "s"} pendiente{pending === 1 ? "" : "s"}</span>
      {/if}
      <label class="cf-toggle">
        <input type="checkbox" bind:checked={showAll} />
        Mostrar campos vacíos
      </label>
    </div>
    <button
      class="btn cf-enrich"
      onclick={enrich}
      disabled={enriching || saving}
      title="Pide a la IA propuestas con cita textual; nunca reemplaza lo que ya escribiste"
    >
      {enriching ? "Buscando…" : "Sugerir con IA"}
    </button>
  </div>

  {#if error}
    <p class="cf-error" role="alert">{error}</p>
  {/if}

  {#if loading && fields.length === 0}
    <p class="cf-muted">Cargando ficha…</p>
  {:else if visibleFields.length === 0}
    <p class="cf-muted">
      No hay campos con contenido. Activa «Mostrar campos vacíos» para rellenarlos.
    </p>
  {:else}
    <dl class="cf-list">
      {#each visibleFields as field (field.field_id)}
        <div class="cf-row" class:cf-ai={field.source === "ai"}>
          <dt>
            <span class="cf-label">{field.label}</span>
            {#if field.builtin}<span class="cf-chip">base</span>{/if}
            {#if field.source === "ai"}
              <span class="cf-chip cf-chip-ai">IA · {confidencePct(field.confidence)}</span>
            {/if}
          </dt>
          <dd>
            {#if editingId === field.field_id}
              <div class="cf-editor">
                {#if field.field_type === "longtext"}
                  <textarea class="input" rows="3" bind:value={draft}></textarea>
                {:else if field.field_type === "list"}
                  <input class="input" bind:value={draft} placeholder="Separa los valores con comas" />
                {:else if field.field_type === "number"}
                  <input class="input" bind:value={draft} inputmode="decimal" />
                {:else}
                  <input class="input" bind:value={draft} />
                {/if}
                <div class="cf-actions">
                  <button class="btn btn-primary" onclick={() => saveField(field)} disabled={saving}>
                    Guardar
                  </button>
                  <button class="btn" onclick={() => (editingId = null)} disabled={saving}>
                    Cancelar
                  </button>
                </div>
              </div>
            {:else if field.source === "ai" && isEmptyField(field)}
              <div class="cf-suggestion">
                <p class="cf-insufficient">
                  Sin evidencia suficiente{field.reason ? `: ${field.reason}` : "."}
                </p>
                <div class="cf-actions">
                  <button class="btn" onclick={() => reject(field)} disabled={saving}>
                    Descartar
                  </button>
                </div>
              </div>
            {:else if field.source === "ai"}
              <div class="cf-suggestion">
                <p class="cf-value">{displayValue(field)}</p>
                {#if field.evidence}
                  <blockquote class="cf-evidence">«{field.evidence}»</blockquote>
                {/if}
                {#if field.reason}<p class="cf-muted cf-small">{field.reason}</p>{/if}
                <div class="cf-actions">
                  <button class="btn btn-primary" onclick={() => accept(field)} disabled={saving}>
                    Aceptar
                  </button>
                  <button class="btn" onclick={() => reject(field)} disabled={saving}>
                    Descartar
                  </button>
                </div>
              </div>
            {:else if isEmptyField(field)}
              <div class="cf-empty">
                <span class="cf-muted">Vacío</span>
                <button class="btn" onclick={() => startEdit(field)}>Añadir</button>
              </div>
            {:else}
              <div class="cf-value-wrap">
                <span class="cf-value">{displayValue(field)}</span>
                <button class="btn cf-edit" onclick={() => startEdit(field)}>Editar</button>
              </div>
            {/if}
            <span class="cf-type">{TYPE_LABEL[field.field_type] ?? field.field_type}</span>
          </dd>
        </div>
      {/each}
    </dl>
  {/if}
</div>

<style>
  .cf {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .cf-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .cf-summary {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .cf-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
    font-weight: 600;
  }

  .cf-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .cf-toggle input {
    accent-color: var(--accent);
  }

  .cf-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .cf-row {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .cf-row.cf-ai {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  dt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
  }

  .cf-label {
    font-weight: 600;
    font-size: 13px;
  }

  .cf-chip {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .cf-chip-ai {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    color: var(--accent);
  }

  dd {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .cf-value {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .cf-value-wrap {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .cf-edit {
    flex: none;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .cf-row:hover .cf-edit,
  .cf-row:focus-within .cf-edit {
    opacity: 1;
  }

  .cf-empty {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .cf-suggestion {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 7%, transparent);
  }

  .cf-evidence {
    margin: 0;
    padding-left: 10px;
    border-left: 3px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    color: var(--text-muted);
    font-style: italic;
    font-size: 12px;
  }

  .cf-insufficient {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .cf-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .cf-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .cf-type {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .cf-muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .cf-small {
    font-size: 11px;
  }

  .cf-error {
    margin: 0;
    color: var(--danger, #d64545);
    font-size: 12px;
  }

  @media (max-width: 640px) {
    .cf-row {
      grid-template-columns: 1fr;
    }
  }
</style>
