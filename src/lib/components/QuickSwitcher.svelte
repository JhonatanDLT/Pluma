<script lang="ts">
  import { onMount } from "svelte";
  import type { NavEntry } from "$lib/types";

  let {
    entries,
    selectedChapterId,
    selectedSceneId,
    onselect,
    onclose,
  }: {
    entries: NavEntry[];
    selectedChapterId: string | null;
    selectedSceneId: string | null;
    onselect: (entry: NavEntry) => void;
    onclose: () => void;
  } = $props();

  let query = $state("");
  let index = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  type Item = {
    kind: "chapter" | "scene";
    entry: NavEntry;
    label: string;
    sub: string;
  };

  const items = $derived.by((): Item[] => {
    const q = query.trim().toLowerCase();
    const chapters = new Map<string, NavEntry>();
    const scenes: Item[] = [];

    for (const entry of entries) {
      if (!chapters.has(entry.chapter_id)) {
        chapters.set(entry.chapter_id, entry);
      }
      if (entry.scene_id) {
        scenes.push({
          kind: "scene",
          entry,
          label: `${entry.chapter_title} › ${entry.scene_title ?? "Escena"}`,
          sub: "Escena",
        });
      }
    }

    const all: Item[] = [
      ...Array.from(chapters.values()).map((entry) => ({
        kind: "chapter" as const,
        entry,
        label: entry.chapter_title,
        sub: "Capítulo",
      })),
      ...scenes,
    ];

    if (!q) return all;
    return all.filter((it) => it.label.toLowerCase().includes(q));
  });

  $effect(() => {
    if (index >= items.length) index = Math.max(items.length - 1, 0);
    if (index < 0) index = 0;
  });

  function select(item: Item) {
    onselect(item.entry);
  }

  function isCurrent(item: Item): boolean {
    if (item.kind === "chapter") {
      return item.entry.chapter_id === selectedChapterId;
    }
    return item.entry.scene_id === selectedSceneId;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      index = Math.min(index + 1, items.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      index = Math.max(index - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = items[index];
      if (item) select(item);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    }
  }

  onMount(() => {
    queueMicrotask(() => inputEl?.focus());
  });
</script>

<div class="switcher-overlay">
  <button class="switcher-backdrop" aria-label="Cerrar" onclick={onclose}></button>
  <div class="switcher" role="dialog" aria-modal="true">
    <div class="switcher-input-row">
      <input
        class="switcher-input"
        bind:this={inputEl}
        bind:value={query}
        placeholder="Ir a capítulo o escena…"
        onkeydown={onKeydown}
      />
      <kbd class="switcher-kbd">Ctrl K</kbd>
    </div>
    <div class="switcher-list">
      {#each items as item, i (`${item.kind}-${item.entry.chapter_id}-${item.entry.scene_id ?? "c"}`)}
        <button
          class="switcher-item"
          class:active={i === index}
          class:current={isCurrent(item)}
          onmouseenter={() => (index = i)}
          onclick={() => select(item)}
        >
          <span class="switcher-kind">{item.sub}</span>
          <span class="switcher-label">{item.label}</span>
          {#if isCurrent(item)}
            <span class="switcher-current">●</span>
          {/if}
        </button>
      {/each}
      {#if items.length === 0}
        <div class="switcher-empty">Sin resultados</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .switcher-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 1000;
  }

  .switcher-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    border: none;
    cursor: default;
  }

  .switcher {
    position: relative;
    width: min(560px, 90vw);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.25);
    overflow: hidden;
  }

  .switcher-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
  }

  .switcher-input {
    flex: 1;
    box-sizing: border-box;
    font: inherit;
    font-size: 16px;
    padding: 14px 4px;
    border: none;
    background: transparent;
    color: var(--text);
  }

  .switcher-kbd {
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: 11px;
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
  }

  .switcher-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 6px;
    max-height: 50vh;
    overflow-y: auto;
  }

  .switcher-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    font: inherit;
    font-size: 14px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .switcher-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .switcher-current {
    flex-shrink: 0;
    font-size: 8px;
    color: var(--accent);
  }

  .switcher-kind {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
    min-width: 52px;
  }

  .switcher-item.active .switcher-kind {
    color: var(--accent);
  }

  .switcher-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .switcher-empty {
    color: var(--text-muted);
    cursor: default;
    padding: 9px 10px;
  }
</style>
