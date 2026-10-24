<script lang="ts">
  import type { SpellReport } from "$lib/types";

  let {
    report,
    installing,
    oninstall,
    onreplace,
    onclose,
  }: {
    report: SpellReport | null;
    installing: boolean;
    oninstall: (code: string) => void;
    onreplace: (word: string, suggestion: string) => void;
    onclose: () => void;
  } = $props();
</script>

{#if report}
  <div class="spell-overlay">
    <button class="spell-backdrop" aria-label="Cerrar" onclick={onclose}></button>

    <div class="spell-dialog" role="dialog" aria-modal="true">
      <header class="spell-head">
        <div>
          <h2>Ortografía</h2>
          {#if report.language_name}
            <span class="spell-lang">{report.language_name}</span>
          {/if}
        </div>
        <button class="btn-icon" title="Cerrar" onclick={onclose}>×</button>
      </header>

      <div class="spell-body">
        {#if !report.language}
          <p class="spell-hint">No se pudo detectar el idioma del texto.</p>
        {:else if !report.installed}
          <div class="spell-install">
            <p>
              No hay diccionario instalado para <strong>{report.language_name}</strong>.
              Se descargará una sola vez (pocos MB).
            </p>
            <button
              class="btn btn-primary"
              onclick={() => oninstall(report.language)}
              disabled={installing}
            >
              {installing ? "Instalando…" : "Instalar diccionario"}
            </button>
          </div>
        {:else if report.issues.length === 0}
          <p class="spell-ok">Sin faltas de ortografía. ✦</p>
        {:else}
          <p class="spell-count">{report.issues.length} posible(s) falta(s)</p>
          <ul class="spell-list">
            {#each report.issues as issue (issue.word)}
              <li>
                <span class="spell-word">{issue.word}</span>
                <div class="spell-suggs">
                  {#if issue.suggestions.length > 0}
                    {#each issue.suggestions as s (s)}
                      <button class="sugg" onclick={() => onreplace(issue.word, s)}>{s}</button>
                    {/each}
                  {:else}
                    <span class="spell-nosugg">Sin sugerencias</span>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .spell-overlay {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .spell-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    border: none;
  }

  .spell-dialog {
    position: relative;
    width: min(520px, 94vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-md);
    overflow: hidden;
  }

  .spell-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .spell-head h2 {
    font-family: var(--font-serif);
    font-size: 19px;
    margin: 0;
  }

  .spell-lang {
    font-size: 12px;
    color: var(--text-muted);
  }

  .spell-body {
    overflow-y: auto;
    padding: 16px 20px;
  }

  .spell-hint,
  .spell-ok {
    color: var(--text-muted);
    font-size: 13px;
    margin: 0;
  }

  .spell-ok {
    color: var(--accent);
  }

  .spell-install {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .spell-install p {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.6;
  }

  .spell-count {
    margin: 0 0 10px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .spell-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .spell-list li {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .spell-word {
    font-weight: 600;
    color: var(--danger);
    font-size: 14px;
  }

  .spell-suggs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .sugg {
    font: inherit;
    font-size: 12px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
  }

  .sugg:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .spell-nosugg {
    font-size: 12px;
    color: var(--text-faint);
  }
</style>
