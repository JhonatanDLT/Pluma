<script lang="ts">
  import MetadataPanel from "$lib/components/MetadataPanel.svelte";
  import CharactersPanel from "$lib/components/CharactersPanel.svelte";
  import VersionsPanel from "$lib/components/VersionsPanel.svelte";
  import type {
    Chapter,
    Character,
    Entity,
    IaStatus,
    Relationship,
    Scene,
    SnapshotMeta,
  } from "$lib/types";

  let {
    chapter,
    scene,
    onsavechapter,
    onsavescene,
    characters,
    relationships,
    entities,
    analyzing,
    enriching,
    iaStatus,
    onanalyze,
    onenrich,
    ondeletecharacter,
    ondeleteentity,
    onupdateembeddings,
    oninstallembeddings,
    onupdatenerengine,
    oninstallgliner,
    oninstallonnxruntime,
    onupdatekeeploaded,
    snapshots,
    snapshotBusy,
    oncreatesnapshot,
    onrestoresnapshot,
    ondeletesnapshot,
    onloadsnapshots,
  }: {
    chapter: Chapter | null;
    scene: Scene | null;
    onsavechapter: (status: string, summary: string | null) => void;
    onsavescene: (meta: {
      status: string;
      pov: string | null;
      location: string | null;
      timeLabel: string | null;
      summary: string | null;
    }) => void;
    characters: Character[];
    relationships: Relationship[];
    entities: Entity[];
    analyzing: boolean;
    enriching: boolean;
    iaStatus: IaStatus | null;
    onanalyze: () => void;
    onenrich: () => void;
    ondeletecharacter: (id: string) => void;
    ondeleteentity: (id: string) => void;
    onupdateembeddings: (enabled: boolean) => void;
    oninstallembeddings: () => void;
    onupdatenerengine: (engine: string) => void;
    oninstallgliner: () => void;
    oninstallonnxruntime: () => void;
    onupdatekeeploaded: (enabled: boolean) => void;
    snapshots: SnapshotMeta[];
    snapshotBusy: boolean;
    oncreatesnapshot: (message: string | null) => void;
    onrestoresnapshot: (id: string) => void;
    ondeletesnapshot: (id: string) => void;
    onloadsnapshots: () => void;
  } = $props();

  type Tab = "chars" | "meta" | "versions";
  let tab = $state<Tab>("chars");

  $effect(() => {
    if (tab === "versions") onloadsnapshots();
  });
</script>

<div class="options-panel">
  <div class="tabs">
    <button class="tab" class:active={tab === "chars"} onclick={() => (tab = "chars")}>Personajes</button>
    <button class="tab" class:active={tab === "meta"} onclick={() => (tab = "meta")}>Metadatos</button>
    <button class="tab" class:active={tab === "versions"} onclick={() => (tab = "versions")}>Versiones</button>
  </div>

  <div class="tab-body">
    {#if tab === "chars"}
      <CharactersPanel
        {characters}
        {relationships}
        {entities}
        {analyzing}
        {enriching}
        {iaStatus}
        onanalyze={onanalyze}
        onenrich={onenrich}
        ondeletecharacter={ondeletecharacter}
        ondeleteentity={ondeleteentity}
        onupdateembeddings={onupdateembeddings}
        oninstallembeddings={oninstallembeddings}
        onupdatenerengine={onupdatenerengine}
        oninstallgliner={oninstallgliner}
        oninstallonnxruntime={oninstallonnxruntime}
        onupdatekeeploaded={onupdatekeeploaded}
      />
    {:else if tab === "meta"}
      <MetadataPanel
        {chapter}
        {scene}
        onsavechapter={onsavechapter}
        onsavescene={onsavescene}
      />
    {:else}
      <VersionsPanel
        {snapshots}
        busy={snapshotBusy}
        oncreate={oncreatesnapshot}
        onrestore={onrestoresnapshot}
        ondelete={ondeletesnapshot}
      />
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
    padding: 7px 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
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
</style>
