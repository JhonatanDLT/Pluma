<script lang="ts">
  import type { Chapter, NavEntry } from "$lib/types";

  let {
    chapters,
    navigation,
    selectedChapterId,
    selectedSceneId,
    wordGoal,
    onselectchapter,
    onselectscene,
    oncreatechapter,
    ondeletechapter,
    onmovechapter,
    onmovescene,
    onupdategoal,
    ontoggle,
  }: {
    chapters: Chapter[];
    navigation: NavEntry[];
    selectedChapterId: string | null;
    selectedSceneId: string | null;
    wordGoal: number;
    onselectchapter: (id: string) => void;
    onselectscene: (id: string) => void;
    oncreatechapter: (title: string) => void;
    ondeletechapter: (id: string) => void;
    onmovechapter: (id: string, index: number) => void;
    onmovescene: (chapterId: string, sceneId: string, index: number) => void;
    onupdategoal: (goal: number) => void;
    ontoggle: () => void;
  } = $props();

  let expandedIds = $state<string[]>([]);
  let chapterInput = $state("");
  let showInput = $state(false);
  let dragId = $state<string | null>(null);
  let dragSceneId = $state<string | null>(null);
  let editingGoal = $state(false);
  let goalInput = $state("");

  function progress(chapter: Chapter): number {
    if (wordGoal <= 0) return 0;
    return Math.min(100, Math.round((chapter.word_count / wordGoal) * 100));
  }

  function startGoalEdit() {
    goalInput = String(wordGoal);
    editingGoal = true;
  }

  function commitGoal() {
    const value = Number(goalInput.trim());
    if (Number.isFinite(value) && value > 0) onupdategoal(value);
    editingGoal = false;
  }

  function isExpanded(id: string): boolean {
    return expandedIds.includes(id);
  }

  function toggleExpand(id: string) {
    expandedIds = isExpanded(id)
      ? expandedIds.filter((x) => x !== id)
      : [...expandedIds, id];
  }

  function scenesFor(chapterId: string): NavEntry[] {
    return navigation.filter((e) => e.chapter_id === chapterId && e.scene_id !== null);
  }

  function submit() {
    const title = chapterInput.trim();
    if (!title) return;
    oncreatechapter(title);
    chapterInput = "";
    showInput = false;
  }

  function selectChapter(id: string) {
    onselectchapter(id);
    if (!isExpanded(id)) expandedIds = [...expandedIds, id];
  }
</script>

<div class="chapters-panel">
  <div class="panel-head">
    <span class="label">Capítulos</span>
    <div class="goal">
      {#if editingGoal}
        <input
          class="goal-input"
          bind:value={goalInput}
          onkeydown={(e) => {
            if (e.key === "Enter") commitGoal();
            if (e.key === "Escape") (editingGoal = false);
          }}
          onblur={commitGoal}
        />
      {:else}
        <button class="goal-btn" title="Objetivo de palabras por capítulo (clic para editar)" onclick={startGoalEdit}>
          {wordGoal.toLocaleString()} pal.
        </button>
      {/if}
    </div>
    <div class="head-actions">
      <button class="btn-icon" title="Nuevo capítulo" onclick={() => (showInput = !showInput)}>＋</button>
      <button class="btn-icon" title="Colapsar panel" onclick={ontoggle}>⟨</button>
    </div>
  </div>

  {#if showInput}
    <div class="create-row">
      <input
        class="input"
        bind:value={chapterInput}
        placeholder="Título del capítulo…"
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
      <button class="btn btn-primary" onclick={submit}>Crear</button>
    </div>
  {/if}

  <ul class="chapter-list">
    {#each chapters as chapter, i (chapter.id)}
      <li
        draggable="true"
        ondragstart={() => (dragId = chapter.id)}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => {
          e.preventDefault();
          if (dragId) onmovechapter(dragId, i);
        }}
      >
        <div
          class="chapter-row"
          class:selected={chapter.id === selectedChapterId}
        >
          <button
            class="chevron"
            class:open={isExpanded(chapter.id)}
            title={isExpanded(chapter.id) ? "Contraer" : "Expandir"}
            onclick={() => toggleExpand(chapter.id)}
          >▸</button>
          <button class="item-main" onclick={() => selectChapter(chapter.id)}>
            <span class="item-top">
              <span class="item-title">{chapter.title}</span>
              <span class="item-count">{chapter.word_count.toLocaleString()}</span>
            </span>
            <span class="item-progress">
              <span class="item-progress-fill" style:width="{progress(chapter)}%"></span>
            </span>
          </button>
          <button class="item-del" title="Eliminar capítulo" onclick={() => ondeletechapter(chapter.id)}>×</button>
        </div>

        {#if isExpanded(chapter.id)}
          <ul class="scene-list">
            {#each scenesFor(chapter.id) as entry, si (entry.scene_id)}
              <li
                draggable="true"
                ondragstart={() => (dragSceneId = entry.scene_id)}
                ondragover={(e) => e.preventDefault()}
                ondrop={(e) => {
                  e.preventDefault();
                  if (dragSceneId && entry.scene_id) onmovescene(chapter.id, dragSceneId, si);
                }}
              >
                <button
                  class="scene-item"
                  class:selected={entry.scene_id === selectedSceneId}
                  onclick={() => entry.scene_id && onselectscene(entry.scene_id)}
                >
                  <span class="scene-title">{entry.scene_title ?? "Escena"}</span>
                </button>
              </li>
            {:else}
              <li class="scene-empty">Sin escenas</li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .chapters-panel {
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

  .goal {
    margin-left: 8px;
  }

  .goal-btn {
    font: inherit;
    font-size: 11px;
    color: var(--text-muted);
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    cursor: pointer;
  }

  .goal-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .goal-input {
    font: inherit;
    font-size: 11px;
    width: 70px;
    padding: 2px 6px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
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

  .chapter-list {
    list-style: none;
    margin: 0;
    padding: 0 8px 12px;
  }

  .chapter-row {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
  }

  .chapter-row.selected {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .chevron {
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 2px 4px;
    font-size: 11px;
    transition: transform 0.12s;
    line-height: 1;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .item-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
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

  .item-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .item-progress {
    display: block;
    height: 3px;
    background: var(--bg-hover);
    border-radius: 2px;
    overflow: hidden;
  }

  .item-progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
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
    padding: 0 6px;
    cursor: pointer;
  }

  .chapter-row:hover .item-del {
    display: inline-flex;
  }

  .item-del:hover {
    color: var(--danger);
  }

  .scene-list {
    list-style: none;
    margin: 0 0 4px;
    padding: 0 0 0 20px;
    border-left: 1px solid var(--border);
    margin-left: 10px;
  }

  .scene-item {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 12.5px;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .scene-item:hover {
    background: var(--bg-hover);
  }

  .scene-item.selected {
    color: var(--accent);
    font-weight: 600;
  }

  .scene-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scene-empty {
    color: var(--text-faint);
    font-size: 12px;
    padding: 4px 8px;
  }
</style>
