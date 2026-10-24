<script lang="ts">
  import type { Chapter, Project, SceneMeta } from "$lib/types";

  let {
    projects,
    chapters,
    scenes,
    selectedProjectId,
    selectedChapterId,
    selectedSceneId,
    onselectproject,
    onselectchapter,
    onselectscene,
    oncreateproject,
    oncreatechapter,
    oncreatescene,
    ondeleteproject,
    ondeletechapter,
    ondeletescene,
    onmovechapter,
    onmovescene,
  }: {
    projects: Project[];
    chapters: Chapter[];
    scenes: SceneMeta[];
    selectedProjectId: string | null;
    selectedChapterId: string | null;
    selectedSceneId: string | null;
    onselectproject: (id: string) => void;
    onselectchapter: (id: string) => void;
    onselectscene: (id: string) => void;
    oncreateproject: (title: string) => void;
    oncreatechapter: (title: string) => void;
    oncreatescene: (title: string | null) => void;
    ondeleteproject: (id: string) => void;
    ondeletechapter: (id: string) => void;
    ondeletescene: (id: string) => void;
    onmovechapter: (id: string, index: number) => void;
    onmovescene: (id: string, index: number) => void;
  } = $props();

  let projectInput = $state("");
  let chapterInput = $state("");
  let sceneInput = $state("");
  let showProjectInput = $state(false);
  let showChapterInput = $state(false);
  let showSceneInput = $state(false);
  let dragId = $state<string | null>(null);

  function submitProject() {
    const title = projectInput.trim();
    if (!title) return;
    oncreateproject(title);
    projectInput = "";
    showProjectInput = false;
  }

  function submitChapter() {
    const title = chapterInput.trim();
    if (!title) return;
    oncreatechapter(title);
    chapterInput = "";
    showChapterInput = false;
  }

  function submitScene() {
    const title = sceneInput.trim();
    oncreatescene(title ? title : null);
    sceneInput = "";
    showSceneInput = false;
  }
</script>

<aside class="sidebar">
  <div class="brand">
    <span class="brand-mark">✒</span>
    <span class="brand-name">Pluma</span>
  </div>

  <section class="block">
    <div class="block-head">
      <span class="label">Proyectos</span>
      <button
        class="btn-icon"
        title="Nuevo proyecto"
        onclick={() => (showProjectInput = !showProjectInput)}
      >＋</button>
    </div>

    {#if showProjectInput}
      <div class="create-row">
        <input
          class="input"
          bind:value={projectInput}
          placeholder="Título de la novela…"
          onkeydown={(e) => e.key === "Enter" && submitProject()}
        />
        <button class="btn btn-primary" onclick={submitProject}>Crear</button>
      </div>
    {/if}

    <ul class="list">
      {#each projects as project (project.id)}
        <li class:selected={project.id === selectedProjectId}>
          <button class="item-main" onclick={() => onselectproject(project.id)}>
            <span class="item-title">{project.title}</span>
          </button>
          <button
            class="item-del"
            title="Eliminar proyecto"
            onclick={() => ondeleteproject(project.id)}
          >×</button>
        </li>
      {/each}
    </ul>
  </section>

  {#if selectedProjectId}
    <section class="block">
      <div class="block-head">
        <span class="label">Capítulos</span>
        <button
          class="btn-icon"
          title="Nuevo capítulo"
          onclick={() => (showChapterInput = !showChapterInput)}
        >＋</button>
      </div>

      {#if showChapterInput}
        <div class="create-row">
          <input
            class="input"
            bind:value={chapterInput}
            placeholder="Título del capítulo…"
            onkeydown={(e) => e.key === "Enter" && submitChapter()}
          />
          <button class="btn btn-primary" onclick={submitChapter}>Crear</button>
        </div>
      {/if}

      <ul class="list">
        {#each chapters as chapter, i (chapter.id)}
          <li
            class:selected={chapter.id === selectedChapterId}
            draggable="true"
            ondragstart={() => (dragId = chapter.id)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => {
              e.preventDefault();
              if (dragId) onmovechapter(dragId, i);
            }}
          >
            <button class="item-main" onclick={() => onselectchapter(chapter.id)}>
              <span class="item-title">{chapter.title}</span>
              <span class="item-count">{chapter.word_count.toLocaleString()}</span>
            </button>
            <button
              class="item-del"
              title="Eliminar capítulo"
              onclick={() => ondeletechapter(chapter.id)}
            >×</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if selectedChapterId}
    <section class="block">
      <div class="block-head">
        <span class="label">Escenas</span>
        <button
          class="btn-icon"
          title="Nueva escena"
          onclick={() => (showSceneInput = !showSceneInput)}
        >＋</button>
      </div>

      {#if showSceneInput}
        <div class="create-row">
          <input
            class="input"
            bind:value={sceneInput}
            placeholder="Título de la escena…"
            onkeydown={(e) => e.key === "Enter" && submitScene()}
          />
          <button class="btn btn-primary" onclick={submitScene}>Crear</button>
        </div>
      {/if}

      <ul class="list">
        {#each scenes as scene, i (scene.id)}
          <li
            class:selected={scene.id === selectedSceneId}
            draggable="true"
            ondragstart={() => (dragId = scene.id)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => {
              e.preventDefault();
              if (dragId) onmovescene(dragId, i);
            }}
          >
            <button class="item-main" onclick={() => onselectscene(scene.id)}>
              <span class="item-title">{scene.title ?? `Escena ${i + 1}`}</span>
            </button>
            <button
              class="item-del"
              title="Eliminar escena"
              onclick={() => ondeletescene(scene.id)}
            >×</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</aside>

<style>
  .sidebar {
    width: 260px;
    min-width: 260px;
    height: 100%;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 18px 16px 12px;
    font-family: var(--font-serif);
    font-size: 20px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .brand-mark {
    color: var(--accent);
  }

  .block {
    padding: 8px 10px 14px;
  }

  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 6px;
  }

  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .create-row {
    display: flex;
    gap: 6px;
    padding: 6px 6px 8px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
    transition: background 0.12s;
  }

  .list li.selected {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .item-main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    border-radius: 5px;
    min-width: 0;
    font-size: 13px;
  }

  .item-main:hover {
    background: var(--bg-hover);
  }

  .item-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-count {
    font-size: 11px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }

  .item-del {
    display: none;
    border: none;
    background: transparent;
    color: var(--text-faint);
    font-size: 15px;
    line-height: 1;
    padding: 0 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .list li:hover .item-del {
    display: inline-flex;
  }

  .item-del:hover {
    color: var(--danger);
  }
</style>
