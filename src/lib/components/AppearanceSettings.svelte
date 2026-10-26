<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { THEMES, getTheme, applyTheme, type ThemeId } from "$lib/theme";
  import {
    FONT_PRESETS,
    addCustomFont,
    removeCustomFont,
    getCustomFonts,
    getFontSettings,
    saveFontSettings,
    applyFontSettings,
    type CustomFont,
    type FontSlot,
  } from "$lib/fonts";

  let theme = $state<ThemeId>(getTheme());
  let settings = $state(getFontSettings());
  let customFonts = $state<CustomFont[]>(getCustomFonts());
  let loadingFont = $state(false);

  const slots: { slot: FontSlot; label: string }[] = [
    { slot: "serif", label: "Texto (serif)" },
    { slot: "ui", label: "Interfaz (títulos)" },
    { slot: "mono", label: "Código (mono)" },
  ];

  function selectTheme(t: ThemeId) {
    theme = t;
    applyTheme(t);
  }

  function apply() {
    saveFontSettings(settings);
    applyFontSettings(settings);
  }

  function pickPreset(slot: FontSlot, value: string) {
    if (!value) return;
    settings = { ...settings, [slot]: value };
    apply();
  }

  function setSlot(slot: FontSlot, value: string) {
    settings = { ...settings, [slot]: value };
    apply();
  }

  function customValue(font: CustomFont, slot: FontSlot): string {
    const fallback = slot === "mono" ? "monospace" : slot === "serif" ? "serif" : "sans-serif";
    return `"${font.name}", ${fallback}`;
  }

  async function loadFont() {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Cargar fuente personalizada",
      filters: [{ name: "Fuentes", extensions: ["ttf", "otf", "woff", "woff2"] }],
    });
    if (typeof selected !== "string") return;
    loadingFont = true;
    try {
      await addCustomFont(selected);
      customFonts = getCustomFonts();
    } finally {
      loadingFont = false;
    }
  }

  async function dropFont(name: string) {
    await removeCustomFont(name);
    customFonts = getCustomFonts();
  }
</script>

<div class="appearance">
  <section>
    <h4>Tema de color</h4>
    <div class="theme-grid">
      {#each THEMES as t (t.id)}
        <button
          class="theme"
          class:active={theme === t.id}
          title={t.label}
          onclick={() => selectTheme(t.id)}
        >
          <span class="swatch" style:background={t.swatch}></span>
          <span class="theme-label">{t.label}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h4>Tipografía</h4>
    <p class="hint">
      Elige una fuente distinta para el cuerpo del texto, los títulos y el código.
      Puedes cargar fuentes personalizadas (.ttf, .otf, .woff).
    </p>

    {#each slots as { slot, label } (slot)}
      <div class="field">
        <span>{label}</span>
        <select
          class="input"
          value=""
          onchange={(e) => pickPreset(slot, e.currentTarget.value)}
        >
          <option value="" disabled>Elegir preset…</option>
          {#each FONT_PRESETS[slot] as p (p.label)}
            <option value={p.value}>{p.label}</option>
          {/each}
          {#each customFonts as f (f.path)}
            <option value={customValue(f, slot)}>{f.name}</option>
          {/each}
        </select>
        <input
          class="input font-value"
          value={settings[slot]}
          onchange={(e) => setSlot(slot, e.currentTarget.value)}
        />
      </div>
    {/each}

    <div class="custom-fonts">
      <button class="btn" onclick={loadFont} disabled={loadingFont}>
        {loadingFont ? "Cargando…" : "＋ Cargar fuente personalizada"}
      </button>

      {#if customFonts.length > 0}
        <ul class="font-list">
          {#each customFonts as f (f.path)}
            <li>
              <span class="font-name" style:font-family={f.name}>{f.name}</span>
              <button class="btn-icon" title="Quitar fuente" onclick={() => dropFont(f.name)}>×</button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </section>
</div>

<style>
  .appearance {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
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

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .font-value {
    font-size: 12px;
    color: var(--text);
  }

  .custom-fonts {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .font-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .font-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .font-name {
    font-size: 14px;
  }
</style>
