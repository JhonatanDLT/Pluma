<script lang="ts">
  let {
    ollamaAvailable,
    onchat,
  }: {
    ollamaAvailable: boolean;
    onchat: (message: string) => Promise<string>;
  } = $props();

  type Msg = { role: "user" | "assistant"; text: string };
  let messages = $state<Msg[]>([]);
  let input = $state("");
  let busy = $state(false);
  let listEl = $state<HTMLDivElement | undefined>(undefined);

  function scrollToBottom() {
    queueMicrotask(() => {
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    });
  }

  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    input = "";
    messages = [...messages, { role: "user", text }];
    busy = true;
    scrollToBottom();
    try {
      const reply = await onchat(text);
      messages = [...messages, { role: "assistant", text: reply }];
    } catch (e) {
      messages = [
        ...messages,
        { role: "assistant", text: "Error: " + (typeof e === "string" ? e : String(e)) },
      ];
    } finally {
      busy = false;
      scrollToBottom();
    }
  }
</script>

<div class="assistant-panel">
  <div class="messages" bind:this={listEl}>
    {#if messages.length === 0}
      <p class="hint">
        Pregunta al asistente sobre tu novela: ideas de trama, nombres, estructura,
        correcciones… Responde con un LLM local (Ollama).
      </p>
    {/if}
    {#each messages as m, i (i)}
      <div class="msg" class:user={m.role === "user"}>
        <span class="bubble">{m.text}</span>
      </div>
    {/each}
    {#if busy}
      <div class="msg"><span class="bubble thinking">Escribiendo…</span></div>
    {/if}
  </div>

  {#if !ollamaAvailable}
    <p class="note">
      Ollama no detectado. Instala <strong>Ollama</strong> y un modelo (p. ej.
      <code>ollama pull qwen2.5:1.5b</code>) para usar el asistente.
    </p>
  {/if}

  <div class="composer">
    <input
      class="input"
      placeholder="Pregunta al asistente…"
      bind:value={input}
      disabled={!ollamaAvailable || busy}
      onkeydown={(e) => e.key === "Enter" && send()}
    />
    <button
      class="btn btn-primary"
      onclick={send}
      disabled={!ollamaAvailable || busy || !input.trim()}
    >Enviar</button>
  </div>
</div>

<style>
  .assistant-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.6;
  }

  .msg {
    display: flex;
  }

  .msg.user {
    justify-content: flex-end;
  }

  .bubble {
    max-width: 85%;
    padding: 8px 12px;
    border-radius: 12px;
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--bg-hover);
    color: var(--text);
  }

  .msg.user .bubble {
    background: var(--accent-soft);
    color: var(--text);
  }

  .thinking {
    opacity: 0.6;
    font-style: italic;
  }

  .note {
    margin: 0;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 11px;
    line-height: 1.5;
  }

  .note code {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .composer {
    display: flex;
    gap: 6px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }
</style>
