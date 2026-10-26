<script lang="ts">
  import type { CloudProviderInfo, IaStatus, ProfileInfo } from "$lib/types";

  let {
    iaStatus,
    cloudProviders,
    profiles,
    onapplyprofile,
    oninstallonnxruntime,
    onupdatenerengine,
    oninstallgliner,
    onupdateembeddings,
    oninstallembeddings,
    onupdaterelationshipmodel,
    onupdatekeeploaded,
    onupdatecloudprovider,
    onupdatecloudapikey,
    onupdatecloudmodel,
    onindexproject,
  }: {
    iaStatus: IaStatus | null;
    cloudProviders: CloudProviderInfo[];
    profiles: ProfileInfo[];
    onapplyprofile: (profileId: string | null) => void;
    oninstallonnxruntime: () => void;
    onupdatenerengine: (engine: string) => void;
    oninstallgliner: () => void;
    onupdateembeddings: (enabled: boolean) => void;
    oninstallembeddings: () => void;
    onupdaterelationshipmodel: (model: string | null) => void;
    onupdatekeeploaded: (enabled: boolean) => void;
    onupdatecloudprovider: (provider: string | null) => void;
    onupdatecloudapikey: (key: string | null) => void;
    onupdatecloudmodel: (model: string | null) => void;
    onindexproject: () => void;
  } = $props();

  const selectedProvider = $derived(
    cloudProviders.find((p) => p.id === iaStatus?.settings.cloud_provider) ??
      null,
  );
  const selectedProfile = $derived(
    profiles.find((p) => p.id === iaStatus?.settings.profile) ?? null,
  );
</script>

<div class="ai-settings">
  {#if !iaStatus}
    <p class="ia-note">Cargando configuración…</p>
  {:else}
    {#if !iaStatus.onnxruntime_available}
      <div class="runtime-row">
        <p class="ia-note">
          Los motores locales (GLiNER2 y embeddings) requieren ONNX Runtime. Se
          instala una sola vez y con tu permiso.
        </p>
        <button class="btn" onclick={oninstallonnxruntime}>Instalar runtime ONNX</button>
      </div>
    {/if}

    <label class="field">
      <span>Perfil de capacidad</span>
      <select
        value={iaStatus.settings.profile ?? ""}
        onchange={(e) => onapplyprofile(e.currentTarget.value || null)}
      >
        <option value="">Personalizado</option>
        {#each profiles as profile (profile.id)}
          <option value={profile.id}>{profile.label}</option>
        {/each}
      </select>
    </label>

    {#if selectedProfile}
      <p class="ia-note">{selectedProfile.description}</p>
      <p class="ia-note">
        Requisitos mínimos: {selectedProfile.min_ram} de RAM · {selectedProfile.min_cpu} ·
        {selectedProfile.min_disk} de disco · GPU: {selectedProfile.gpu}
      </p>
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
      <button class="btn" onclick={oninstallgliner}>Descargar GLiNER2 (~1 GB)</button>
    {/if}

    <label class="toggle">
      <input
        type="checkbox"
        checked={iaStatus.settings.embeddings}
        onchange={(e) => onupdateembeddings(e.currentTarget.checked)}
      />
      <span>Embeddings (resolución de alias)</span>
    </label>

    {#if iaStatus.settings.embeddings && !iaStatus.embeddings_cached}
      <button class="btn" onclick={oninstallembeddings}>Descargar modelo (~120 MB)</button>
    {/if}

    {#if iaStatus.settings.embeddings && iaStatus.embeddings_cached}
      <button class="btn" onclick={onindexproject} title="Indexa las escenas para el chat con contexto">
        Indexar para chat
      </button>
    {/if}

    <label class="toggle">
      <input
        type="checkbox"
        checked={iaStatus.settings.relationship_model !== null}
        onchange={(e) =>
          onupdaterelationshipmodel(e.currentTarget.checked ? "qwen2.5:1.5b" : null)}
      />
      <span>Tipar relaciones con IA</span>
    </label>

    <label class="toggle">
      <input
        type="checkbox"
        checked={iaStatus.settings.keep_loaded}
        onchange={(e) => onupdatekeeploaded(e.currentTarget.checked)}
      />
      <span>Mantener modelos en memoria (alto rendimiento)</span>
    </label>

    <div class="cloud-block">
      <label class="field">
        <span>IA en la nube (opcional)</span>
        <select
          value={iaStatus.settings.cloud_provider ?? ""}
          onchange={(e) => onupdatecloudprovider(e.currentTarget.value || null)}
        >
          <option value="">Desactivada</option>
          {#each cloudProviders as provider (provider.id)}
            <option value={provider.id}>{provider.label}</option>
          {/each}
        </select>
      </label>

      {#if iaStatus.settings.cloud_provider}
        <label class="field">
          <span>API key</span>
          <input
            type="password"
            class="input"
            placeholder={iaStatus.cloud_key_set ? "•••••• (guardada en el llavero)" : "sk-…"}
            onchange={(e) => onupdatecloudapikey(e.currentTarget.value || null)}
          />
        </label>
        {#if iaStatus.cloud_key_set}
          <button class="btn" onclick={() => onupdatecloudapikey(null)}>
            Borrar clave del llavero
          </button>
        {/if}
        <label class="field">
          <span>Modelo</span>
          <input
            class="input"
            value={iaStatus.settings.cloud_model ?? ""}
            placeholder={selectedProvider?.default_model ?? "modelo"}
            onchange={(e) => onupdatecloudmodel(e.currentTarget.value || null)}
          />
        </label>
        <p class="ia-note">
          La clave se guarda cifrada en el llavero del sistema operativo (no en
          texto plano). Al configurarla, la nube queda activa por defecto.
        </p>
      {/if}
    </div>

    {#if !iaStatus.ollama_available}
      <p class="ia-note">
        Ollama no detectado: «Enriquecer» y el tipado de relaciones no estarán
        disponibles a menos que configures una IA en la nube.
      </p>
    {/if}
  {/if}
</div>

<style>
  .ai-settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .runtime-row {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .ia-note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--text-muted);
  }

  select,
  .input {
    font: inherit;
    font-size: 13px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text);
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  .cloud-block {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
