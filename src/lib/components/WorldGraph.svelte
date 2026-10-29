<script lang="ts">
  import {
    forceCenter,
    forceCollide,
    forceLink,
    forceManyBody,
    forceSimulation,
    type Simulation,
    type SimulationLinkDatum,
    type SimulationNodeDatum,
  } from "d3-force";
  import * as api from "$lib/api";
  import type { GraphData, GraphEdgeKind, GraphNodeKind } from "$lib/types";

  let {
    projectId,
    refreshKey = 0,
    onselectcharacter,
  }: {
    projectId: string;
    refreshKey?: number;
    onselectcharacter: (id: string) => void;
  } = $props();

  const width = 900;
  const height = 560;
  const FACTION_COLORS = ["#A4552C", "#5F6B3A", "#3F5F7A", "#6E4A6B", "#9A6B1F", "#2F6F6A"];

  interface GNode extends SimulationNodeDatum {
    id: string;
    label: string;
    kind: GraphNodeKind;
    role: string | null;
    mentions: number;
    degree: number;
    radius: number;
    protagonist: boolean;
    factionId: string | null;
    factionLabels: string[];
  }

  interface GLink extends SimulationLinkDatum<GNode> {
    id: string;
    kind: GraphEdgeKind;
    subtype: string | null;
    weight: number;
    origin: string;
  }

  interface RNode {
    id: string;
    label: string;
    kind: GraphNodeKind;
    role: string | null;
    mentions: number;
    degree: number;
    radius: number;
    protagonist: boolean;
    factionId: string | null;
    factionLabels: string[];
    x: number;
    y: number;
  }

  interface RLink {
    id: string;
    sourceId: string;
    targetId: string;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    kind: GraphEdgeKind;
    subtype: string | null;
    weight: number;
    origin: string;
  }

  let data = $state<GraphData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let loadToken = 0;

  let rawNodes: GNode[] = [];
  let rawLinks: GLink[] = [];
  let nodes = $state<RNode[]>([]);
  let links = $state<RLink[]>([]);
  let sim: Simulation<GNode, GLink> | undefined;

  let query = $state("");
  let showLabels = $state(true);
  let cluster = $state(false);
  let showKinds = $state({ character: true, faction: true, location: true });
  let showEdges = $state({
    relationships: true,
    affiliations: true,
    presence: true,
    entities: true,
  });
  let selected = $state<RNode | null>(null);
  let focusId = $state<string | null>(null);

  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let svgEl = $state<SVGSVGElement | undefined>(undefined);

  let factionColorMap = new Map<string, string>();
  let dragNode: GNode | null = null;
  let nodeDragged = false;
  let panning = false;
  let panStart = { x: 0, y: 0, panX: 0, panY: 0 };
  let lastTick = 0;

  const egoSet = $derived.by(() => {
    if (!focusId || !data) return null;
    const set = new Set<string>([focusId]);
    for (const edge of data.edges) {
      if (edge.source === focusId) set.add(edge.target);
      if (edge.target === focusId) set.add(edge.source);
    }
    return set;
  });

  const visibleNodes = $derived(
    nodes.filter(
      (n) =>
        showKinds[n.kind] &&
        (!egoSet || egoSet.has(n.id)),
    ),
  );

  const visibleNodeIds = $derived(new Set(visibleNodes.map((n) => n.id)));

  const visibleLinks = $derived(
    links.filter(
      (l) =>
        showEdges[edgeGroup(l.kind)] &&
        visibleNodeIds.has(l.sourceId) &&
        visibleNodeIds.has(l.targetId),
    ),
  );

  const matchedIds = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    return new Set(
      visibleNodes.filter((n) => n.label.toLowerCase().includes(q)).map((n) => n.id),
    );
  });

  const stats = $derived(data?.stats ?? null);

  function edgeGroup(kind: GraphEdgeKind): keyof typeof showEdges {
    if (kind === "relationship") return "relationships";
    if (kind === "affiliation") return "affiliations";
    if (kind === "presence") return "presence";
    return "entities";
  }

  function factionColor(id: string | null): string {
    if (!id) return "var(--text-muted)";
    return factionColorMap.get(id) ?? "var(--text-muted)";
  }

  function updateRender() {
    nodes = rawNodes.map((n) => ({
      id: n.id,
      label: n.label,
      kind: n.kind,
      role: n.role,
      mentions: n.mentions,
      degree: n.degree,
      radius: n.radius,
      protagonist: n.protagonist,
      factionId: n.factionId,
      factionLabels: n.factionLabels,
      x: n.x ?? width / 2,
      y: n.y ?? height / 2,
    }));

    links = rawLinks
      .filter(
        (l) => typeof l.source !== "string" && typeof l.target !== "string",
      )
      .map((l) => {
        const s = l.source as GNode;
        const t = l.target as GNode;
        return {
          id: l.id,
          sourceId: s.id,
          targetId: t.id,
          x1: s.x ?? 0,
          y1: s.y ?? 0,
          x2: t.x ?? 0,
          y2: t.y ?? 0,
          kind: l.kind,
          subtype: l.subtype,
          weight: l.weight,
          origin: l.origin,
        };
      });
  }

  function factionClusterForce(alpha: number) {
    const byId = new Map(rawNodes.map((n) => [n.id, n]));
    for (const node of rawNodes) {
      if (node.kind !== "character" || !node.factionId) continue;
      const faction = byId.get(node.factionId);
      if (!faction) continue;
      node.vx = (node.vx ?? 0) + ((faction.x ?? width / 2) - (node.x ?? 0)) * 0.08 * alpha;
      node.vy = (node.vy ?? 0) + ((faction.y ?? height / 2) - (node.y ?? 0)) * 0.08 * alpha;
    }
  }

  function build() {
    if (!data) return;
    const byId = new Map(data.nodes.map((n) => [n.id, n]));
    const degree = new Map<string, number>();
    for (const edge of data.edges) {
      if (!byId.has(edge.source) || !byId.has(edge.target)) continue;
      degree.set(edge.source, (degree.get(edge.source) ?? 0) + 1);
      degree.set(edge.target, (degree.get(edge.target) ?? 0) + 1);
    }

    rawNodes = data.nodes.map((node) => {
      const d = degree.get(node.id) ?? 0;
      const isCharacter = node.kind === "character";
      const radius = isCharacter
        ? 8 + Math.min(14, Math.sqrt(node.mentions + 1) * 3 + Math.sqrt(d) * 1.5)
        : node.kind === "faction"
          ? 17 + Math.min(9, Math.sqrt(node.mentions + 1) * 2)
          : 11;
      const factionId = node.faction_ids.length
        ? `entity:${node.faction_ids[0]}`
        : null;
      const factionLabels = node.faction_ids
        .map((id) => byId.get(`entity:${id}`)?.label)
        .filter((label): label is string => Boolean(label));
      return {
        id: node.id,
        label: node.label,
        kind: node.kind,
        role: node.role,
        mentions: node.mentions,
        degree: d,
        radius,
        protagonist: (node.role ?? "").toLowerCase().includes("protagon"),
        factionId,
        factionLabels,
      };
    });

    factionColorMap = new Map(
      rawNodes
        .filter((n) => n.kind === "faction")
        .map((n, index) => [n.id, FACTION_COLORS[index % FACTION_COLORS.length]]),
    );

    const nodeIds = new Set(rawNodes.map((n) => n.id));
    rawLinks = data.edges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        kind: e.kind,
        subtype: e.subtype,
        weight: e.weight,
        origin: e.origin,
      }));

    if (selected && !nodeIds.has(selected.id)) selected = null;
    if (focusId && !nodeIds.has(focusId)) focusId = null;

    updateRender();
    sim?.stop();
    sim = forceSimulation(rawNodes)
      .force(
        "link",
        forceLink<GNode, GLink>(rawLinks)
          .id((d) => d.id)
          .distance((l) => {
            if (l.kind === "affiliation") return 70;
            if (l.kind === "presence") return 125;
            return 105;
          })
          .strength(0.35),
      )
      .force(
        "charge",
        forceManyBody<GNode>().strength((d) => (d.kind === "faction" ? -650 : -300)),
      )
      .force("center", forceCenter(width / 2, height / 2))
      .force("collide", forceCollide<GNode>((d) => d.radius + 10))
      .force("cluster", cluster ? factionClusterForce : null)
      .on("tick", () => {
        const now = performance.now();
        if (now - lastTick < 32) return;
        lastTick = now;
        updateRender();
      })
      .on("end", updateRender);
    sim.alpha(1).restart();
  }

  async function load() {
    const token = ++loadToken;
    if (!projectId) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      const graph = await api.getWikiGraph(projectId);
      if (token === loadToken) {
        data = graph;
        build();
      }
    } catch (e) {
      if (token === loadToken) error = String(e);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void projectId;
    void refreshKey;
    void load();
    return () => sim?.stop();
  });

  $effect(() => {
    void cluster;
    if (!sim) return;
    sim.force("cluster", cluster ? factionClusterForce : null);
    sim.alpha(0.6).restart();
  });

  function toGraph(event: PointerEvent): { x: number; y: number } {
    if (!svgEl) return { x: 0, y: 0 };
    const rect = svgEl.getBoundingClientRect();
    const vx = ((event.clientX - rect.left) / rect.width) * width;
    const vy = ((event.clientY - rect.top) / rect.height) * height;
    return { x: (vx - panX) / zoom, y: (vy - panY) / zoom };
  }

  function nodeDown(event: PointerEvent, id: string) {
    event.stopPropagation();
    const node = rawNodes.find((n) => n.id === id);
    if (!node) return;
    nodeDragged = false;
    dragNode = node;
    node.fx = node.x;
    node.fy = node.y;
    sim?.alphaTarget(0.25).restart();
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
  }

  function pointerMove(event: PointerEvent) {
    if (dragNode) {
      nodeDragged = true;
      const p = toGraph(event);
      dragNode.fx = p.x;
      dragNode.fy = p.y;
      return;
    }
    if (panning && svgEl) {
      const rect = svgEl.getBoundingClientRect();
      const dx = ((event.clientX - panStart.x) / rect.width) * width;
      const dy = ((event.clientY - panStart.y) / rect.height) * height;
      panX = panStart.panX + dx;
      panY = panStart.panY + dy;
    }
  }

  function pointerUp() {
    if (dragNode) {
      dragNode.fx = null;
      dragNode.fy = null;
      dragNode = null;
      sim?.alphaTarget(0);
    }
    panning = false;
  }

  function backgroundDown(event: PointerEvent) {
    if (event.button !== 0) return;
    panning = true;
    panStart = { x: event.clientX, y: event.clientY, panX, panY };
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    if (!svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const vx = ((event.clientX - rect.left) / rect.width) * width;
    const vy = ((event.clientY - rect.top) / rect.height) * height;
    const factor = event.deltaY < 0 ? 1.1 : 0.9;
    const next = Math.min(2.5, Math.max(0.4, zoom * factor));
    const gx = (vx - panX) / zoom;
    const gy = (vy - panY) / zoom;
    panX = vx - gx * next;
    panY = vy - gy * next;
    zoom = next;
  }

  function resetView() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function selectNode(node: RNode) {
    if (nodeDragged) return;
    selected = node;
  }

  function clearFocus() {
    focusId = null;
  }

  function isolate(node: RNode) {
    focusId = node.id;
    selected = node;
  }

  function isolateSelected() {
    if (!selected) return;
    if (focusId === selected.id) {
      focusId = null;
      return;
    }
    isolate(selected);
  }

  function openCharacter() {
    if (!selected || selected.kind !== "character") return;
    onselectcharacter(selected.id.replace(/^char:/, ""));
  }

  function hexagon(radius: number): string {
    return Array.from({ length: 6 }, (_, i) => {
      const angle = (Math.PI / 3) * i - Math.PI / 2;
      return `${Math.cos(angle) * radius},${Math.sin(angle) * radius}`;
    }).join(" ");
  }

  function detailLabel(kind: GraphNodeKind): string {
    return kind === "character" ? "Personaje" : kind === "faction" ? "Facción" : "Lugar";
  }
</script>

<div class="world-graph">
  <div class="graph-toolbar">
    <input
      class="input graph-search"
      bind:value={query}
      placeholder="Buscar en el grafo…"
      aria-label="Buscar en el grafo"
    />
    <div class="graph-filters" role="group" aria-label="Tipos de nodo">
      <label class="graph-toggle">
        <input type="checkbox" bind:checked={showKinds.character} /> Personajes
      </label>
      <label class="graph-toggle">
        <input type="checkbox" bind:checked={showKinds.faction} /> Facciones
      </label>
      <label class="graph-toggle">
        <input type="checkbox" bind:checked={showKinds.location} /> Lugares
      </label>
    </div>
    <button class="btn" onclick={resetView}>Centrar</button>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={cluster} /> Agrupar por facción
    </label>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showLabels} /> Etiquetas
    </label>
    {#if focusId}
      <button class="btn" onclick={clearFocus}>Ver todo</button>
    {/if}
  </div>

  <div class="graph-toolbar graph-toolbar-secondary">
    <span class="graph-filter-title">Aristas:</span>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showEdges.relationships} /> Relaciones
    </label>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showEdges.affiliations} /> Afiliaciones
    </label>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showEdges.presence} /> Presencia
    </label>
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showEdges.entities} /> Alianzas y territorio
    </label>
    <span class="graph-stats">
      {#if stats}
        {stats.characters} personajes · {stats.factions} facciones · {stats.locations} lugares ·
        {stats.relationships + stats.affiliations + stats.presence + stats.entity_relations} vínculos
      {:else}
        Cargando…
      {/if}
    </span>
  </div>

  {#if error}
    <p class="graph-error" role="alert">{error}</p>
  {:else if loading && !data}
    <p class="graph-muted">Cargando el grafo del mundo…</p>
  {:else if !data || data.nodes.length === 0}
    <div class="graph-empty">
      <p>Analiza el manuscrito para construir el grafo de personajes, facciones y lugares.</p>
    </div>
  {:else}
    <div class="graph-main">
      <div class="graph-canvas">
        <svg
          bind:this={svgEl}
          viewBox={`0 0 ${width} ${height}`}
          role="img"
          aria-label="Grafo del mundo con personajes, facciones, lugares y sus relaciones"
          onpointerdown={backgroundDown}
          onpointermove={pointerMove}
          onpointerup={pointerUp}
          onpointercancel={pointerUp}
          onpointerleave={pointerUp}
          onwheel={onWheel}
        >
          <g transform={`translate(${panX},${panY}) scale(${zoom})`}>
            {#each visibleLinks as link (link.id)}
              <line
                class="edge edge-{link.kind}"
                class:ai={link.origin === "ai"}
                class:dim={matchedIds &&
                  (!matchedIds.has(link.sourceId) || !matchedIds.has(link.targetId))}
                x1={link.x1}
                y1={link.y1}
                x2={link.x2}
                y2={link.y2}
                style:stroke-width={1 + Math.min(3, link.weight / 4)}
              >
                <title>
                  {link.kind}{link.subtype ? ` · ${link.subtype}` : ""}
                  {link.origin === "ai" ? " · sugerencia IA" : ""}
                  {link.origin === "derived" ? " · derivado de escenas" : ""}
                </title>
              </line>
            {/each}

            {#each visibleNodes as node (node.id)}
              <g
                class="node node-{node.kind}"
                class:dim={matchedIds && !matchedIds.has(node.id)}
                class:selected={selected?.id === node.id}
                transform={`translate(${node.x},${node.y})`}
                onpointerdown={(e) => nodeDown(e, node.id)}
                onclick={() => selectNode(node)}
                role="button"
                tabindex="0"
                aria-label={`${node.label}, ${detailLabel(node.kind)}`}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    selectNode(node);
                  }
                }}
              >
                {#if node.kind === "character"}
                  <circle
                    r={node.radius}
                    class:protagonist={node.protagonist}
                    style:fill={node.factionId ? factionColor(node.factionId) : "var(--text-muted)"}
                  />
                {:else if node.kind === "faction"}
                  <polygon
                    points={hexagon(node.radius)}
                    style:fill={factionColor(node.id)}
                    class="faction-shape"
                  />
                {:else}
                  <rect
                    x={-node.radius}
                    y={-node.radius * 0.75}
                    width={node.radius * 2}
                    height={node.radius * 1.5}
                    rx="3"
                  />
                {/if}
                {#if showLabels}
                  <text class="label" y={node.radius + 14} text-anchor="middle">
                    {node.label}
                  </text>
                {/if}
                <title>
                  {node.label} · {detailLabel(node.kind)}
                  {node.role ? ` · ${node.role}` : ""} · {node.degree} vínculo(s)
                </title>
              </g>
            {/each}
          </g>
        </svg>
      </div>

      {#if selected}
        <aside class="graph-inspector" aria-label="Detalle del nodo">
          <header class="inspector-head">
            <span class="inspector-kind">{detailLabel(selected.kind)}</span>
            <button
              class="btn-icon"
              title="Cerrar detalle"
              onclick={() => (selected = null)}
            >×</button>
          </header>
          <h4>{selected.label}</h4>
          {#if selected.role}
            <p class="inspector-role">{selected.role}</p>
          {/if}
          <dl class="inspector-meta">
            <div><dt>Menciones</dt><dd>{selected.mentions}</dd></div>
            <div><dt>Vínculos</dt><dd>{selected.degree}</dd></div>
            {#if selected.factionLabels.length > 0}
              <div><dt>Facciones</dt><dd>{selected.factionLabels.join(", ")}</dd></div>
            {/if}
          </dl>
          <div class="inspector-actions">
            {#if selected.kind === "character"}
              <button class="btn btn-primary" onclick={openCharacter}>Abrir ficha</button>
            {/if}
            <button class="btn" onclick={isolateSelected}>
              {focusId === selected.id ? "Entorno aislado" : "Aislar entorno"}
            </button>
          </div>
        </aside>
      {/if}
    </div>

    <div class="graph-legend">
      <span><i class="mark char"></i>personaje</span>
      <span><i class="mark faction"></i>facción</span>
      <span><i class="mark location"></i>lugar</span>
      <span><i class="line relationship"></i>relación</span>
      <span><i class="line affiliation"></i>afiliación</span>
      <span><i class="line presence"></i>presencia (derivada)</span>
      <span><i class="line entity"></i>alianza / territorio</span>
      {#if data.stats.relationships === 0 && data.stats.affiliations === 0}
        <span class="graph-warning">
          Aún no hay vínculos; añádelos en la pestaña Vínculos.
        </span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .world-graph {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 520px;
    padding: 12px 16px 16px;
  }

  .graph-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .graph-toolbar-secondary {
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }

  .graph-search {
    flex: 1;
    min-width: 180px;
    max-width: 300px;
  }

  .graph-filters {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .graph-filter-title {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .graph-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .graph-toggle input {
    accent-color: var(--accent);
  }

  .graph-stats {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .graph-main {
    flex: 1;
    min-height: 380px;
    display: flex;
    gap: 10px;
  }

  .graph-canvas {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    overflow: hidden;
    touch-action: none;
    cursor: grab;
  }

  .graph-canvas:active {
    cursor: grabbing;
  }

  .graph-canvas svg {
    width: 100%;
    height: 100%;
    display: block;
    user-select: none;
  }

  .edge {
    stroke: var(--border-strong);
    stroke-opacity: 0.55;
  }

  .edge-relationship {
    stroke: var(--border-strong);
  }

  .edge-affiliation {
    stroke: var(--accent);
    stroke-opacity: 0.7;
    stroke-dasharray: 5 3;
  }

  .edge-presence {
    stroke: var(--text-muted);
    stroke-opacity: 0.35;
    stroke-dasharray: 2 5;
  }

  .edge-territory {
    stroke: var(--text-muted);
    stroke-opacity: 0.6;
    stroke-dasharray: 8 4;
  }

  .edge-alliance {
    stroke: #4f9d69;
    stroke-opacity: 0.8;
  }

  .edge-conflict {
    stroke: var(--danger, #d64545);
    stroke-opacity: 0.8;
    stroke-dasharray: 4 3;
  }

  .edge.ai {
    stroke-opacity: 0.55;
  }

  .edge.dim {
    stroke-opacity: 0.08;
  }

  .node {
    cursor: pointer;
    outline: none;
  }

  .node circle,
  .node rect {
    fill: var(--text-muted);
    fill-opacity: 0.85;
    stroke: var(--bg-elevated);
    stroke-width: 2;
    transition: fill-opacity 0.15s;
  }

  .node circle.protagonist {
    stroke: var(--accent);
    stroke-width: 2.5;
  }

  .node .faction-shape {
    fill-opacity: 0.9;
    stroke: var(--bg-elevated);
    stroke-width: 2;
  }

  .node:hover circle,
  .node:hover rect,
  .node:hover .faction-shape {
    fill-opacity: 1;
    stroke: var(--accent);
  }

  .node.selected circle,
  .node.selected rect,
  .node.selected .faction-shape {
    stroke: var(--accent);
    stroke-width: 3;
  }

  .node.dim {
    opacity: 0.18;
  }

  .node:focus-visible circle,
  .node:focus-visible rect,
  .node:focus-visible .faction-shape {
    stroke: var(--accent);
    stroke-width: 3;
  }

  .label {
    font-family: var(--font-ui);
    font-size: 11px;
    fill: var(--text);
    paint-order: stroke;
    stroke: var(--bg-elevated);
    stroke-width: 3px;
    stroke-linejoin: round;
    pointer-events: none;
  }

  .graph-inspector {
    width: 220px;
    flex: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }

  .inspector-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .inspector-kind {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .graph-inspector h4 {
    margin: 0;
    font-size: 15px;
  }

  .inspector-role {
    margin: 0;
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
  }

  .inspector-meta {
    margin: 4px 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .inspector-meta div {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
  }

  .inspector-meta dt {
    color: var(--text-muted);
  }

  .inspector-meta dd {
    margin: 0;
    text-align: right;
  }

  .inspector-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: auto;
  }

  .graph-legend {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 11px;
    color: var(--text-muted);
  }

  .mark {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 5px;
    vertical-align: middle;
    background: var(--text-muted);
  }

  .mark.char {
    border-radius: 50%;
  }

  .mark.faction {
    clip-path: polygon(50% 0, 93% 25%, 93% 75%, 50% 100%, 7% 75%, 7% 25%);
    background: var(--accent);
  }

  .mark.location {
    border-radius: 2px;
    background: var(--border-strong);
  }

  .line {
    display: inline-block;
    width: 18px;
    height: 1px;
    margin-right: 5px;
    vertical-align: middle;
    background: var(--border-strong);
  }

  .line.affiliation {
    background: repeating-linear-gradient(90deg, var(--accent) 0 4px, transparent 4px 7px);
  }

  .line.presence {
    background: repeating-linear-gradient(90deg, var(--text-muted) 0 2px, transparent 2px 5px);
  }

  .line.entity {
    background: repeating-linear-gradient(90deg, var(--text-muted) 0 6px, transparent 6px 9px);
  }

  .graph-warning {
    color: var(--warning, var(--text-muted));
  }

  .graph-muted {
    margin: 0;
    padding: 20px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .graph-error {
    margin: 0;
    padding: 12px;
    color: var(--danger, #d64545);
    font-size: 12px;
  }

  .graph-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 320px;
    color: var(--text-muted);
    text-align: center;
    padding: 24px;
  }

  @media (max-width: 760px) {
    .graph-main {
      flex-direction: column;
    }

    .graph-inspector {
      width: auto;
      max-height: 220px;
    }
  }
</style>
