<script lang="ts">
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
    oncreateproject: (title: string) => void;
    ondeleteproject: (id: string) => void;
    onimportproject: () => void;
  } = $props();

  let title = $state("");
  let showInput = $state(false);
  let search = $state("");
  let searchEl = $state<HTMLInputElement | undefined>(undefined);

  const filtered = $derived(
    search.trim()
      ? projects.filter((p) => p.title.toLowerCase().includes(search.trim().toLowerCase()))
      : projects,
  );

  $effect(() => {
    if (focusSignal > 0) searchEl?.focus();
  });

  function submit() {
    const value = title.trim();
    if (!value) return;
    oncreateproject(value);
    title = "";
    showInput = false;
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
        <button class="btn" onclick={onimportproject} title="Importar un PDF, EPUB, DOCX o TXT como proyecto">Importar</button>
        <button class="btn btn-primary" onclick={() => (showInput = !showInput)}>＋ Nuevo proyecto</button>
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

    {#if showInput}
      <div class="create-row">
        <input
          class="input"
          bind:value={title}
          placeholder="Título de la novela…"
          onkeydown={(e) => e.key === "Enter" && submit()}
        />
        <button class="btn btn-primary" onclick={submit}>Crear</button>
      </div>
    {/if}

    {#if filtered.length === 0}
      <p class="empty">
        {search.trim() ? "No hay proyectos que coincidan." : "Crea tu primer proyecto para empezar a escribir."}
      </p>
    {:else}
      <ul class="project-grid">
        {#each filtered as project (project.id)}
          <li>
            <button class="card" onclick={() => onselectproject(project.id)}>
              <span class="card-title">{project.title}</span>
              <span class="card-meta">Abrir →</span>
            </button>
            <button
              class="card-del"
              title="Eliminar proyecto"
              onclick={() => ondeleteproject(project.id)}
            >×</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
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

  .create-row {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }

  .search-row {
    margin-bottom: 16px;
  }

  .input {
    flex: 1;
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
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
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
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s;
  }

  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }

  .card-title {
    font-family: var(--font-serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }

  .card-meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .card-del {
    position: absolute;
    top: 8px;
    right: 8px;
    display: none;
    border: none;
    background: transparent;
    color: var(--text-faint);
    font-size: 16px;
    line-height: 1;
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
  }

  .project-grid li:hover .card-del {
    display: block;
  }

  .card-del:hover {
    color: var(--danger);
  }
</style>
