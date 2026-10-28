<script lang="ts">
  import { check, type Update } from "@tauri-apps/plugin-updater";

  let status = $state<"idle" | "checking" | "available" | "none" | "error">(
    "idle",
  );
  let update = $state<Update | null>(null);
  let error = $state<string | null>(null);
  let installing = $state(false);

  async function checkNow() {
    status = "checking";
    error = null;
    try {
      update = await check();
      status = update ? "available" : "none";
    } catch (e) {
      update = null;
      error = String(e);
      status = "error";
    }
  }

  async function install() {
    if (!update) return;
    installing = true;
    try {
      await update.downloadAndInstall();
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
    }
  }
</script>

<section class="updates">
  <h4>Actualizaciones</h4>
  <p class="hint">
    Comprueba si hay una versión firmada más reciente. El endpoint se configura
    en <code>tauri.conf.json</code>.
  </p>

  <button class="btn" onclick={checkNow} disabled={status === "checking"}>
    {status === "checking" ? "Comprobando…" : "Buscar actualizaciones"}
  </button>

  {#if status === "available" && update}
    <p class="update-info">
      Nueva versión <strong>{update.version}</strong>
      {#if update.date} · {new Date(update.date).toLocaleDateString()}{/if}
    </p>
    {#if update.body}
      <p class="hint">{update.body}</p>
    {/if}
    <button class="btn btn-primary" onclick={install} disabled={installing}>
      {installing ? "Descargando…" : "Descargar e instalar"}
    </button>
  {:else if status === "none"}
    <p class="hint">Estás al día.</p>
  {:else if status === "error"}
    <p class="error">
      No se pudo comprobar: {error}
    </p>
  {/if}
</section>

<style>
  .updates {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }

  h4 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  code {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .update-info {
    margin: 0;
    font-size: 13px;
    color: var(--text);
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
    line-height: 1.5;
    word-break: break-word;
  }
</style>
