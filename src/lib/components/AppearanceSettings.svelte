<script lang="ts">
  import { THEMES, getTheme, applyTheme, type ThemeId } from "$lib/theme";

  let theme = $state<ThemeId>(getTheme());

  function select(t: ThemeId) {
    theme = t;
    applyTheme(t);
  }
</script>

<div class="appearance">
  <div class="theme-grid">
    {#each THEMES as t (t.id)}
      <button
        class="theme"
        class:active={theme === t.id}
        title={t.label}
        onclick={() => select(t.id)}
      >
        <span class="swatch" style:background={t.swatch}></span>
        <span class="theme-label">{t.label}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .appearance {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 8px;
  }

  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }

  .theme.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }

  .swatch {
    width: 100%;
    height: 36px;
    border-radius: 4px;
    border: 1px solid var(--border);
    display: block;
  }

  .theme-label {
    color: var(--text-muted);
  }

  .theme.active .theme-label {
    color: var(--text);
    font-weight: 600;
  }
</style>
