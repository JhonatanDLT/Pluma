<script lang="ts">
  import AssistantPanel from "$lib/components/AssistantPanel.svelte";
  import MetadataPanel from "$lib/components/MetadataPanel.svelte";
  import VersionsPanel from "$lib/components/VersionsPanel.svelte";
  import type {
    Chapter,
    Project,
    Scene,
    SnapshotMeta,
  } from "$lib/types";

  let {
    chapter,
    scene,
    project,
    onsavechapter,
    onsavescene,
    onupdatelevels,
    ollamaAvailable,
    onchat,
    snapshots,
    snapshotBusy,
    oncreatesnapshot,
    onrestoresnapshot,
    ondeletesnapshot,
    onloadsnapshots,
  }: {
    chapter: Chapter | null;
    scene: Scene | null;
    project: Project | null;
    onsavechapter: (status: string, summary: string | null) => void;
    onsavescene: (meta: {
      status: string;
      pov: string | null;
      location: string | null;
      timeLabel: string | null;
      summary: string | null;
    }) => void;
    onupdatelevels: (level1Name: string, level2Name: string) => void;
    ollamaAvailable: boolean;
    onchat: (message: string) => Promise<string>;
    snapshots: SnapshotMeta[];
    snapshotBusy: boolean;
    oncreatesnapshot: (message: string | null) => void;
    onrestoresnapshot: (id: string) => void;
    ondeletesnapshot: (id: string) => void;
    onloadsnapshots: () => void;
  } = $props();

  type Tab = "assistant" | "meta" | "versions" | "settings";
  let tab = $state<Tab>("assistant");

  let level1Name = $state("");
  let level2Name = $state("");

  $effect(() => {
    if (project) {
      level1Name = project.level1_name;
      level2Name = project.level2_name;
    }
  });

  function saveLevels() {
    const l1 = level1Name.trim();
    const l2 = level2Name.trim();
    if (!l1 || !l2) return;
    onupdatelevels(l1, l2);
  }

  $effect(() => {
    if (tab === "versions") onloadsnapshots();
  });
</script>

<div class="options-panel">
  <div class="tabs">
    <button class="tab" class:active={tab === "assistant"} onclick={() => (tab = "assistant")}>Asistente</button>
    <button class="tab" class:active={tab === "meta"} onclick={() => (tab = "meta")}>Metadatos</button>
    <button class="tab" class:active={tab === "versions"} onclick={() => (tab = "versions")}>Versiones</button>
    <button class="tab" class:active={tab === "settings"} onclick={() => (tab = "settings")}>Ajustes</button>
  </div>

  <div class="tab-body">
    {#if tab === "assistant"}
      <AssistantPanel {ollamaAvailable} {onchat} />
    {:else if tab === "meta"}
      <MetadataPanel
        {chapter}
        {scene}
        onsavechapter={onsavechapter}
        onsavescene={onsavescene}
      />
    {:else if tab === "versions"}
      <VersionsPanel
        {snapshots}
        busy={snapshotBusy}
        oncreate={oncreatesnapshot}
        onrestore={onrestoresnapshot}
        ondelete={ondeletesnapshot}
      />
    {:else}
      <div class="settings-panel">
        <h4>Niveles</h4>
        <p class="hint">
          Renombra las etiquetas de la jerarquía de tu manuscrito. Por ejemplo,
          «Volúmenes» y «Capítulos», o «Partes» y «Escenas».
        </p>
        <label class="field">
          <span>Primer nivel</span>
          <input class="input" bind:value={level1Name} placeholder="Capítulos" />
        </label>
        <label class="field">
          <span>Segundo nivel</span>
          <input class="input" bind:value={level2Name} placeholder="Escenas" />
        </label>
        <button
          class="btn btn-primary"
          onclick={saveLevels}
          disabled={!level1Name.trim() || !level2Name.trim()}
        >Guardar niveles</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .options-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    flex: 1;
    font: inherit;
    font-size: 12px;
    padding: 7px 2px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    white-space: nowrap;
  }

  .tab.active {
    color: var(--text);
    font-weight: 600;
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .tab-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .settings-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
  }

  .settings-panel h4 {
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

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .input {
    font: inherit;
    font-size: 13px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
  }
</style>
