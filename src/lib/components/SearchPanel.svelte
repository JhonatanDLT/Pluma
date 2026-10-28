<script lang="ts">
  import { onMount } from "svelte";
  import DOMPurify from "dompurify";
  import * as api from "$lib/api";
  import { trapFocus } from "$lib/focusTrap";
  import type { SearchHit } from "$lib/types";

  let {
    projectId,
    onselect,
    onclose,
  }: {
    projectId: string;
    onselect: (chapterId: string, sceneId: string) => void;
    onclose: () => void;
  } = $props();

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let index = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  let timer: number | undefined;

  $effect(() => {
    const q = query.trim();
    if (timer) window.clearTimeout(timer);
    if (q.length < 2) {
      hits = [];
      loading = false;
      error = null;
      return;
    }
    loading = true;
    timer = window.setTimeout(async () => {
      try {
        hits = await api.searchProject(projectId, q, 20);
        index = 0;
        error = null;
      } catch (e) {
        hits = [];
        error = String(e);
      } finally {
        loading = false;
      }
    }, 220);
    return () => {
      if (timer) window.clearTimeout(timer);
    };
  });

  $effect(() => {
    if (index >= hits.length) index = Math.max(hits.length - 1, 0);
    if (index < 0) index = 0;
  });

  function cleanSnippet(snippet: string): string {
    return DOMPurify.sanitize(snippet, {
      ALLOWED_TAGS: ["mark"],
      ALLOWED_ATTR: [],
    });
  }

  function select(hit: SearchHit) {
    onselect(hit.chapter_id, hit.scene_id);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      index = Math.min(index + 1, hits.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      index = Math.max(index - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const hit = hits[index];
      if (hit) select(hit);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    }
  }

  onMount(() => {
    queueMicrotask(() => inputEl?.focus());
  });
</script>

<div class="search-overlay">
  <button class="search-backdrop" aria-label="Cerrar" onclick={onclose}></button>
  <div
    class="search"
    role="dialog"
    aria-modal="true"
    aria-label="Buscar en la novela"
    use:trapFocus={{ onClose: onclose }}
  >
    <div class="search-input-row">
      <input
        class="search-input"
        bind:this={inputEl}
        bind:value={query}
        placeholder="Buscar en toda la novela…"
        onkeydown={onKeydown}
      />
      <kbd class="search-kbd">Ctrl Shift P</kbd>
    </div>

    <div class="search-results">
      {#if loading}
        <p class="search-empty">Buscando…</p>
      {:else if error}
        <p class="search-empty error">{error}</p>
      {:else if query.trim().length < 2}
        <p class="search-empty">Escribe al menos 2 caracteres.</p>
      {:else if hits.length === 0}
        <p class="search-empty">Sin resultados.</p>
      {:else}
        {#each hits as hit, i (hit.scene_id)}
          <button
            class="search-hit"
            class:active={i === index}
            onmouseenter={() => (index = i)}
            onclick={() => select(hit)}
          >
            <span class="search-hit-title">
              {hit.chapter_title}{#if hit.scene_title} › {hit.scene_title}{/if}
            </span>
            <span class="search-hit-snippet">{@html cleanSnippet(hit.snippet)}</span>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .search-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 1000;
  }

  .search-backdrop {
    position: absolute;
    inset: 0;
    background: var(--overlay, rgba(0, 0, 0, 0.42));
    border: none;
    cursor: default;
  }

  .search {
    position: relative;
    width: min(680px, 92vw);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg, 0 16px 48px rgba(0, 0, 0, 0.25));
    overflow: hidden;
  }

  .search-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
  }

  .search-input {
    flex: 1;
    box-sizing: border-box;
    font: inherit;
    font-size: 16px;
    padding: 14px 4px;
    border: none;
    background: transparent;
    color: var(--text);
  }

  .search-kbd {
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: 11px;
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
  }

  .search-results {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 6px;
    max-height: 55vh;
    overflow-y: auto;
  }

  .search-hit {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 9px 10px;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    font: inherit;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .search-hit.active {
    background: var(--accent-soft);
  }

  .search-hit-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .search-hit-snippet {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .search-hit-snippet :global(mark) {
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 2px;
    padding: 0 1px;
  }

  .search-empty {
    color: var(--text-muted);
    padding: 12px 10px;
    margin: 0;
  }

  .search-empty.error {
    color: var(--danger);
  }
</style>
