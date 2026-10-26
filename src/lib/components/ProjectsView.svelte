<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Project } from "$lib/types";

  let {
    projects,
    focusSignal = 0,
    onselectproject,
    oncreateproject,
    ondeleteproject,
    onimportproject,
  }: {
    projects: Project[];
    focusSignal?: number;
    onselectproject: (id: string) => void;
    oncreateproject: (
      title: string,
      author: string | null,
      goal: number | null,
      description: string | null,
      coverPath: string | null,
    ) => void;
    ondeleteproject: (id: string) => void;
    onimportproject: (
      path: string,
      author: string | null,
      goal: number | null,
      description: string | null,
      coverPath: string | null,
    ) => void;
  } = $props();

  let search = $state("");
  let searchEl = $state<HTMLInputElement | undefined>(undefined);

  let formOpen = $state(false);
  let formMode = $state<"create" | "import">("create");
  let importPath = $state<string | null>(null);
  let formName = $state("");
  let formAuthor = $state("");
  let formGoal = $state("3000");
  let formSummary = $state("");
  let formCover = $state<string | null>(null);

  const filtered = $derived(
    search.trim()
      ? projects.filter((p) => p.title.toLowerCase().includes(search.trim().toLowerCase()))
      : projects,
  );

  $effect(() => {
    if (focusSignal > 0) searchEl?.focus();
  });

  function openCreate() {
    formMode = "create";
    importPath = null;
    formName = "";
    formAuthor = "";
    formGoal = "3000";
    formSummary = "";
    formCover = null;
    formOpen = true;
  }

  async function openImport() {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Importar proyecto (PDF, EPUB, DOCX, TXT…)",
      filters: [{ name: "Documentos", extensions: ["pdf", "epub", "docx", "md", "markdown", "txt"] }],
    });
    if (typeof selected !== "string") return;
    const name = selected.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "Documento";
    formMode = "import";
    importPath = selected;
    formName = name;
    formAuthor = "";
    formGoal = "3000";
    formSummary = "";
    formCover = null;
    formOpen = true;
  }

  async function pickCover() {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Portada del proyecto",
      filters: [{ name: "Imágenes", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    if (typeof selected === "string") formCover = selected;
  }

  function submit() {
    const parsed = Number(formGoal);
    const goal = Number.isFinite(parsed) && parsed > 0 ? parsed : 3000;
    const summary = formSummary.trim() || null;
    const author = formAuthor.trim() || null;

    if (formMode === "import" && importPath) {
      onimportproject(importPath, author, goal, summary, formCover);
    } else {
      const name = formName.trim();
      if (!name) return;
      oncreateproject(name, author, goal, summary, formCover);
    }
    formOpen = false;
  }
</script>

<div class="projects-view">
  <div class="projects-inner">
    <div class="hero">
      <span class="hero-mark">✒</span>
      <h1>Pluma</h1>
      <p>Editor de narrativa literaria</p>
    </div>

    <div class="projects-head">
      <h2>Tus novelas</h2>
      <div class="head-actions">
        <button class="btn" onclick={openImport} title="Importar un PDF, EPUB, DOCX o TXT como proyecto">Importar</button>
        <button class="btn btn-primary" onclick={openCreate}>＋ Nuevo proyecto</button>
      </div>
    </div>

    <div class="search-row">
      <input
        class="input"
        bind:this={searchEl}
        bind:value={search}
        placeholder="Buscar proyecto… (Ctrl K)"
      />
    </div>

    {#if filtered.length === 0}
      <p class="empty">
        {search.trim() ? "No hay proyectos que coincidan." : "Crea o importa tu primer proyecto para empezar a escribir."}
      </p>
    {:else}
      <ul class="project-grid">
        {#each filtered as project (project.id)}
          <li>
            <button class="card" onclick={() => onselectproject(project.id)}>
              {#if project.cover_path}
                <img class="card-cover" src={convertFileSrc(project.cover_path)} alt={project.title} />
              {:else}
                <span class="card-cover card-cover-empty">✒</span>
              {/if}
              <span class="card-title">{project.title}</span>
              {#if project.description}
                <span class="card-desc">{project.description}</span>
              {/if}
            </button>
            <button class="card-del" title="Eliminar proyecto" onclick={() => ondeleteproject(project.id)}>×</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if formOpen}
    <div class="form-overlay">
      <button class="form-backdrop" aria-label="Cerrar" onclick={() => (formOpen = false)}></button>
      <div class="form" role="dialog" aria-modal="true">
        <h3>{formMode === "import" ? "Importar proyecto" : "Nuevo proyecto"}</h3>

        <label class="field">
          <span>Nombre</span>
          <input
            class="input"
            bind:value={formName}
            disabled={formMode === "import"}
            placeholder="Título de la novela"
          />
        </label>

        <label class="field">
          <span>Autor (opcional)</span>
          <input class="input" bind:value={formAuthor} placeholder="Nombre del autor" />
        </label>

        <label class="field">
          <span>Objetivo de palabras (por capítulo)</span>
          <input class="input" type="number" min="1" bind:value={formGoal} />
        </label>

        <label class="field">
          <span>Resumen</span>
          <textarea class="input" rows="3" bind:value={formSummary} placeholder="De qué trata…"></textarea>
        </label>

        <div class="field">
          <span>Portada (opcional)</span>
          <div class="cover-row">
            {#if formCover}
              <img class="cover-preview" src={convertFileSrc(formCover)} alt="Portada" />
            {:else}
              <span class="cover-preview cover-preview-empty">Sin portada</span>
            {/if}
            <button class="btn" onclick={pickCover}>Elegir imagen</button>
          </div>
        </div>

        <div class="form-actions">
          <button class="btn" onclick={() => (formOpen = false)}>Cancelar</button>
          <button class="btn btn-primary" onclick={submit} disabled={formMode === "create" && !formName.trim()}>
            {formMode === "import" ? "Importar" : "Crear"}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .projects-view {
    flex: 1;
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }

  .projects-inner {
    max-width: 720px;
    margin: 0 auto;
    padding: 64px 24px;
  }

  .hero {
    text-align: center;
    margin-bottom: 48px;
  }

  .hero-mark {
    display: block;
    font-size: 56px;
    line-height: 1;
    color: var(--accent);
  }

  .hero h1 {
    font-family: var(--font-serif);
    font-size: 40px;
    font-weight: 600;
    margin: 16px 0 4px;
  }

  .hero p {
    color: var(--text-muted);
    margin: 0;
  }

  .projects-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .projects-head h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .head-actions {
    display: flex;
    gap: 8px;
  }

  .search-row {
    margin-bottom: 16px;
  }

  .input {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    font-size: 14px;
    padding: 9px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .empty {
    color: var(--text-muted);
  }

  .project-grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 14px;
  }

  .project-grid li {
    position: relative;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 0 0 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    cursor: pointer;
    overflow: hidden;
    transition: border-color 0.15s, transform 0.15s;
  }

  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }

  .card-cover {
    width: 100%;
    height: 160px;
    object-fit: cover;
    display: block;
  }

  .card-cover-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-hover);
    color: var(--accent);
    font-size: 32px;
  }

  .card-title {
    font-family: var(--font-serif);
    font-size: 17px;
    font-weight: 600;
    color: var(--text);
    padding: 0 14px;
  }

  .card-desc {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 14px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .card-del {
    position: absolute;
    top: 8px;
    right: 8px;
    display: none;
    border: none;
    background: rgba(0, 0, 0, 0.35);
    color: #fff;
    font-size: 16px;
    line-height: 1;
    padding: 4px 7px;
    border-radius: 4px;
    cursor: pointer;
  }

  .project-grid li:hover .card-del {
    display: block;
  }

  .card-del:hover {
    background: var(--danger);
  }

  .form-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .form-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    border: none;
  }

  .form {
    position: relative;
    width: min(460px, 92vw);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.25);
  }

  .form h3 {
    margin: 0;
    font-size: 18px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-muted);
  }

  textarea.input {
    resize: vertical;
  }

  .cover-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cover-preview {
    width: 64px;
    height: 90px;
    object-fit: cover;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .cover-preview-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--text-faint);
    text-align: center;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
