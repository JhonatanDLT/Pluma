<script lang="ts">
  import type { SceneMeta } from "$lib/types";

  let {
    scenes,
    selectedSceneId,
    onselectscene,
    oncreatescene,
    ondeletescene,
    onmovescene,
    ontoggle,
  }: {
    scenes: SceneMeta[];
    selectedSceneId: string | null;
    onselectscene: (id: string) => void;
    oncreatescene: (title: string | null) => void;
    ondeletescene: (id: string) => void;
    onmovescene: (id: string, index: number) => void;
    ontoggle: () => void;
  } = $props();

  let sceneInput = $state("");
  let showInput = $state(false);
  let dragId = $state<string | null>(null);

  function submit() {
    const title = sceneInput.trim();
    oncreatescene(title ? title : null);
    sceneInput = "";
    showInput = false;
  }
</script>

<div class="scenes-panel">
  <div class="panel-head">
    <span class="label">Escenas</span>
    <div class="head-actions">
      <button class="btn-icon" title="Nueva escena" onclick={() => (showInput = !showInput)}>＋</button>
      <button class="btn-icon" title="Colapsar panel" onclick={ontoggle}>⟨</button>
    </div>
  </div>

  {#if showInput}
    <div class="create-row">
      <input
        class="input"
        bind:value={sceneInput}
        placeholder="Título de la escena…"
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
      <button class="btn btn-primary" onclick={submit}>Crear</button>
    </div>
  {/if}

  <ul class="scene-list">
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
        <button class="item-del" title="Eliminar escena" onclick={() => ondeletescene(scene.id)}>×</button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .scenes-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 6px;
  }

  .head-actions {
    display: flex;
    gap: 2px;
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
    padding: 6px 8px 8px;
  }

  .input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }

  .scene-list {
    list-style: none;
    margin: 0;
    padding: 0 8px 12px;
  }

  .scene-list li {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
  }

  .scene-list li.selected {
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

  .item-del {
    display: none;
    border: none;
    background: transparent;
    color: var(--text-faint);
    font-size: 15px;
    padding: 0 6px;
    cursor: pointer;
  }

  .scene-list li:hover .item-del {
    display: inline-flex;
  }

  .item-del:hover {
    color: var(--danger);
  }
</style>
