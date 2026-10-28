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
  import type { Character, Relationship } from "$lib/types";

  let {
    characters,
    relationships,
    onselectcharacter,
  }: {
    characters: Character[];
    relationships: Relationship[];
    onselectcharacter: (id: string) => void;
  } = $props();

  const width = 900;
  const height = 560;

  interface GNode extends SimulationNodeDatum {
    id: string;
    label: string;
    role: string | null;
    degree: number;
    radius: number;
    protagonist: boolean;
  }

  interface GLink extends SimulationLinkDatum<GNode> {
    id: string;
    kind: string | null;
    weight: number;
  }

  interface RNode {
    id: string;
    label: string;
    role: string | null;
    degree: number;
    radius: number;
    protagonist: boolean;
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
    kind: string | null;
    weight: number;
  }

  let rawNodes: GNode[] = [];
  let rawLinks: GLink[] = [];
  let nodes = $state<RNode[]>([]);
  let links = $state<RLink[]>([]);
  let sim: Simulation<GNode, GLink> | undefined;

  let query = $state("");
  let showLabels = $state(true);
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let svgEl = $state<SVGSVGElement | undefined>(undefined);

  let dragNode: GNode | null = null;
  let nodeDragged = false;
  let panning = false;
  let panStart = { x: 0, y: 0, panX: 0, panY: 0 };
  let lastTick = 0;

  const matchedIds = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    return new Set(
      nodes.filter((n) => n.label.toLowerCase().includes(q)).map((n) => n.id),
    );
  });

  function build() {
    const degree = new Map<string, number>();
    for (const r of relationships) {
      degree.set(r.character_a_id, (degree.get(r.character_a_id) ?? 0) + 1);
      degree.set(r.character_b_id, (degree.get(r.character_b_id) ?? 0) + 1);
    }

    rawNodes = characters.map((c) => {
      const d = degree.get(c.id) ?? 0;
      return {
        id: c.id,
        label: c.name,
        role: c.role,
        degree: d,
        radius: 10 + Math.min(16, d * 2.5),
        protagonist: (c.role ?? "").toLowerCase().includes("protagon"),
      };
    });

    const ids = new Set(rawNodes.map((n) => n.id));
    rawLinks = relationships
      .filter((r) => ids.has(r.character_a_id) && ids.has(r.character_b_id))
      .map((r) => ({
        id: r.id,
        source: r.character_a_id,
        target: r.character_b_id,
        kind: r.kind,
        weight: Math.max(1, r.intensity ?? 1),
      }));

    updateRender();
    sim?.stop();
    sim = forceSimulation(rawNodes)
      .force(
        "link",
        forceLink<GNode, GLink>(rawLinks)
          .id((d) => d.id)
          .distance(105)
          .strength(0.4),
      )
      .force("charge", forceManyBody().strength(-320))
      .force("center", forceCenter(width / 2, height / 2))
      .force("collide", forceCollide<GNode>((d) => d.radius + 12))
      .on("tick", () => {
        const now = performance.now();
        if (now - lastTick < 32) return;
        lastTick = now;
        updateRender();
      })
      .on("end", updateRender);
    sim.alpha(1).restart();
  }

  function updateRender() {
    nodes = rawNodes.map((n) => ({
      id: n.id,
      label: n.label,
      role: n.role,
      degree: n.degree,
      radius: n.radius,
      protagonist: n.protagonist,
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
          weight: l.weight,
        };
      });
  }

  $effect(() => {
    // Reconstruye cuando cambia el análisis (personajes/relaciones).
    void characters;
    void relationships;
    build();
    return () => sim?.stop();
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
    panStart = {
      x: event.clientX,
      y: event.clientY,
      panX,
      panY,
    };
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

  function selectNode(id: string) {
    if (nodeDragged) return;
    onselectcharacter(id);
  }
</script>

<div class="world-graph">
  <div class="graph-toolbar">
    <input
      class="input graph-search"
      bind:value={query}
      placeholder="Buscar personaje…"
      aria-label="Buscar personaje en el grafo"
    />
    <label class="graph-toggle">
      <input type="checkbox" bind:checked={showLabels} />
      Etiquetas
    </label>
    <button class="btn" onclick={resetView}>Centrar</button>
    <span class="graph-stats">
      {characters.length} personajes · {relationships.length} relaciones
    </span>
  </div>

  {#if characters.length === 0}
    <div class="graph-empty">
      <p>Analiza el manuscrito para construir el grafo de personajes.</p>
    </div>
  {:else}
    <div class="graph-canvas">
      <svg
        bind:this={svgEl}
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label="Grafo de relaciones entre personajes"
        onpointerdown={backgroundDown}
        onpointermove={pointerMove}
        onpointerup={pointerUp}
        onpointercancel={pointerUp}
        onpointerleave={pointerUp}
        onwheel={onWheel}
      >
        <g transform={`translate(${panX},${panY}) scale(${zoom})`}>
          {#each links as link (link.id)}
            <line
              class="edge"
              class:dim={matchedIds &&
                (!matchedIds.has(link.sourceId) ||
                  !matchedIds.has(link.targetId))}
              x1={link.x1}
              y1={link.y1}
              x2={link.x2}
              y2={link.y2}
              style:stroke-width={1 + Math.min(4, link.weight / 3)}
            >
              <title>
                {link.kind ? link.kind : "relación"} · intensidad {link.weight}
              </title>
            </line>
          {/each}

          {#each nodes as node (node.id)}
            <g
              class="node"
              class:dim={matchedIds && !matchedIds.has(node.id)}
              transform={`translate(${node.x},${node.y})`}
              onpointerdown={(e) => nodeDown(e, node.id)}
              onclick={() => selectNode(node.id)}
              role="button"
              tabindex="0"
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  selectNode(node.id);
                }
              }}
            >
              <circle
                r={node.radius}
                class:protagonist={node.protagonist}
              />
              {#if showLabels}
                <text
                  class="label"
                  y={node.radius + 14}
                  text-anchor="middle"
                >{node.label}</text>
              {/if}
              <title>
                {node.label}{node.role ? ` · ${node.role}` : ""} · {node.degree}
                relación(es)
              </title>
            </g>
          {/each}
        </g>
      </svg>
    </div>

    <div class="graph-legend">
      <span><i class="dot protagonist"></i> protagonista</span>
      <span><i class="dot secondary"></i> secundario</span>
      <span><i class="edge-mark"></i> relación</span>
      {#if relationships.length === 0}
        <span class="graph-warning">
          Aún no hay relaciones; analiza más escenas o revisa la pestaña Relaciones.
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
    min-height: 480px;
    padding: 12px 16px 16px;
  }

  .graph-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .graph-search {
    flex: 1;
    min-width: 180px;
    max-width: 320px;
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

  .graph-canvas {
    flex: 1;
    min-height: 380px;
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

  .edge.dim {
    stroke-opacity: 0.08;
  }

  .node {
    cursor: pointer;
  }

  .node circle {
    fill: var(--text-muted);
    fill-opacity: 0.85;
    stroke: var(--bg-elevated);
    stroke-width: 2;
    transition: fill 0.15s, fill-opacity 0.15s;
  }

  .node circle.protagonist {
    fill: var(--accent);
  }

  .node:hover circle {
    fill-opacity: 1;
    stroke: var(--accent);
  }

  .node.dim {
    opacity: 0.18;
  }

  .node:focus-visible {
    outline: none;
  }

  .node:focus-visible circle {
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

  .graph-legend {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 11px;
    color: var(--text-muted);
  }

  .dot {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    margin-right: 5px;
    vertical-align: middle;
  }

  .dot.protagonist {
    background: var(--accent);
  }

  .dot.secondary {
    background: var(--text-muted);
  }

  .edge-mark {
    display: inline-block;
    width: 18px;
    height: 1px;
    background: var(--border-strong);
    margin-right: 5px;
    vertical-align: middle;
  }

  .graph-warning {
    color: var(--warning, var(--text-muted));
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
</style>
