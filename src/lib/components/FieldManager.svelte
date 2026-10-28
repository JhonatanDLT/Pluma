<script lang="ts">
  import * as api from "$lib/api";
  import type { CharacterFieldDef, FieldType } from "$lib/types";

  let {
    projectId,
    onchanged,
  }: {
    projectId: string;
    onchanged: () => void;
  } = $props();

  const TYPE_LABEL: Record<FieldType, string> = {
    text: "Texto corto",
    longtext: "Texto largo",
    list: "Lista",
    number: "Número",
  };

  let defs = $state<CharacterFieldDef[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);

  let adding = $state(false);
  let newLabel = $state("");
  let newType = $state<FieldType>("text");
  let newHint = $state("");

  let editingId = $state<string | null>(null);
  let editLabel = $state("");
  let editType = $state<FieldType>("text");
  let editHint = $state("");

  const builtins = $derived(defs.filter((d) => d.builtin));
  const custom = $derived(defs.filter((d) => !d.builtin));
  let loadToken = 0;

  async function load() {
    const token = ++loadToken;
    if (!projectId) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      const data = await api.listFieldDefs(projectId);
      if (token === loadToken) defs = data;
    } catch (e) {
      if (token === loadToken) error = String(e);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void projectId;
    void load();
  });

  async function create() {
    if (!newLabel.trim()) return;
    busy = true;
    error = null;
    try {
      await api.createFieldDef(
        projectId,
        newLabel.trim(),
        newType,
        newHint.trim() || null,
      );
      newLabel = "";
      newHint = "";
      newType = "text";
      adding = false;
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function startEdit(def: CharacterFieldDef) {
    editingId = def.id;
    editLabel = def.label;
    editType = def.field_type;
    editHint = def.prompt_hint ?? "";
  }

  async function saveEdit(def: CharacterFieldDef) {
    busy = true;
    error = null;
    try {
      await api.updateFieldDef(
        def.id,
        editLabel.trim(),
        editType,
        editHint.trim() || null,
      );
      editingId = null;
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove(def: CharacterFieldDef) {
    if (
      !window.confirm(
        `¿Eliminar el campo «${def.label}»? Se borrarán sus valores en todos los personajes.`,
      )
    ) {
      return;
    }
    busy = true;
    error = null;
    try {
      await api.deleteFieldDef(def.id);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function move(def: CharacterFieldDef, direction: -1 | 1) {
    const order = custom.map((d) => d.id);
    const index = order.indexOf(def.id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= order.length) return;
    [order[index], order[target]] = [order[target], order[index]];
    busy = true;
    error = null;
    try {
      await api.reorderFieldDefs(projectId, order);
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="fm">
  <header class="fm-head">
    <div>
      <h4>Campos de la ficha</h4>
      <p class="fm-muted">
        Los campos personalizados se rellenan a mano o con sugerencias de la IA.
        Las sugerencias siempre citan el texto y nunca pisan lo que ya escribiste.
      </p>
    </div>
    <button class="btn btn-primary" onclick={() => (adding = !adding)} disabled={busy}>
      {adding ? "Cancelar" : "Nuevo campo"}
    </button>
  </header>

  {#if error}
    <p class="fm-error" role="alert">{error}</p>
  {/if}

  {#if adding}
    <div class="fm-add">
      <label>
        <span>Etiqueta</span>
        <input class="input" bind:value={newLabel} placeholder="Color de ojos" />
      </label>
      <label>
        <span>Tipo</span>
        <select class="input" bind:value={newType}>
          {#each Object.entries(TYPE_LABEL) as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      </label>
      <label class="fm-wide">
        <span>Indicio para la IA (opcional)</span>
        <input
          class="input"
          bind:value={newHint}
          placeholder="Solo si el texto describe el color de sus ojos"
        />
      </label>
      <div class="fm-add-actions">
        <button class="btn btn-primary" onclick={create} disabled={busy || !newLabel.trim()}>
          Crear campo
        </button>
      </div>
    </div>
  {/if}

  {#if loading && defs.length === 0}
    <p class="fm-muted">Cargando campos…</p>
  {:else}
    <section class="fm-section">
      <h5>Campos base</h5>
      <ul class="fm-list">
        {#each builtins as def (def.id)}
          <li class="fm-row">
            <span class="fm-lock" title="Campo base: no se puede renombrar ni eliminar">🔒</span>
            <span class="fm-name">{def.label}</span>
            <span class="fm-type">{TYPE_LABEL[def.field_type]}</span>
          </li>
        {/each}
      </ul>
      <p class="fm-muted fm-small">
        Nombre, alias, rol, descripción, biografía, rasgos, objetivos, arco y color
        viven en la ficha del personaje. La IA solo propone descripción, biografía,
        rasgos, objetivos y arco.
      </p>
    </section>

    <section class="fm-section">
      <h5>Campos personalizados</h5>
      {#if custom.length === 0}
        <p class="fm-muted">
          Aún no hay campos propios. Añade, por ejemplo, «Color de ojos», «Herida»
          o «Objeto simbólico».
        </p>
      {:else}
        <ul class="fm-list">
          {#each custom as def, index (def.id)}
            <li class="fm-row fm-custom">
              {#if editingId === def.id}
                <div class="fm-edit">
                  <input class="input" bind:value={editLabel} aria-label="Etiqueta" />
                  <select class="input" bind:value={editType} aria-label="Tipo">
                    {#each Object.entries(TYPE_LABEL) as [value, label] (value)}
                      <option {value}>{label}</option>
                    {/each}
                  </select>
                  <input
                    class="input fm-hint"
                    bind:value={editHint}
                    placeholder="Indicio para la IA (opcional)"
                    aria-label="Indicio para la IA"
                  />
                  <div class="fm-actions">
                    <button class="btn btn-primary" onclick={() => saveEdit(def)} disabled={busy}>
                      Guardar
                    </button>
                    <button class="btn" onclick={() => (editingId = null)} disabled={busy}>
                      Cancelar
                    </button>
                  </div>
                </div>
              {:else}
                <span class="fm-name">{def.label}</span>
                <span class="fm-type">{TYPE_LABEL[def.field_type]}</span>
                {#if def.prompt_hint}
                  <span class="fm-hint-text" title={def.prompt_hint}>{def.prompt_hint}</span>
                {/if}
                <div class="fm-actions">
                  <button
                    class="btn-icon"
                    title="Subir"
                    onclick={() => move(def, -1)}
                    disabled={busy || index === 0}
                  >↑</button>
                  <button
                    class="btn-icon"
                    title="Bajar"
                    onclick={() => move(def, 1)}
                    disabled={busy || index === custom.length - 1}
                  >↓</button>
                  <button class="btn" onclick={() => startEdit(def)} disabled={busy}>
                    Editar
                  </button>
                  <button class="btn-icon" title="Eliminar" onclick={() => remove(def)} disabled={busy}>
                    ×
                  </button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        <p class="fm-muted fm-small">
          Cambiar el tipo de un campo borra sus valores previos para evitar datos
          incoherentes.
        </p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .fm {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 860px;
  }

  .fm-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  .fm-head h4 {
    margin: 0 0 4px;
  }

  .fm-section h5 {
    margin: 0 0 8px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .fm-muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .fm-small {
    font-size: 11px;
  }

  .fm-error {
    margin: 0;
    color: var(--danger, #d64545);
    font-size: 12px;
  }

  .fm-add {
    display: grid;
    grid-template-columns: 1fr 160px;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .fm-add label,
  .fm-edit {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .fm-wide,
  .fm-add-actions {
    grid-column: 1 / -1;
  }

  .fm-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .fm-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .fm-custom {
    flex-wrap: wrap;
  }

  .fm-name {
    font-weight: 600;
    font-size: 13px;
  }

  .fm-type {
    font-size: 11px;
    color: var(--text-muted);
  }

  .fm-lock {
    font-size: 10px;
    opacity: 0.6;
  }

  .fm-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }

  .fm-edit {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 150px;
    gap: 8px;
  }

  .fm-edit .fm-hint,
  .fm-edit .fm-actions {
    grid-column: 1 / -1;
  }

  .fm-hint-text {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 640px) {
    .fm-add,
    .fm-edit {
      grid-template-columns: 1fr;
    }
  }
</style>
