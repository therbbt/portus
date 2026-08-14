<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let terminalFontFamily: string;
  export let terminalFontSize: number;

  const dispatch = createEventDispatcher<{
    save: { terminalFontFamily: string; terminalFontSize: number };
    cancel: void;
  }>();

  let fontFamily = terminalFontFamily;
  let fontSize = terminalFontSize;
  let panelEl: HTMLDivElement;

  $: canSubmit = fontFamily.trim().length > 0 && fontSize >= 8 && fontSize <= 32;

  function submit() {
    if (!canSubmit) return;
    dispatch("save", { terminalFontFamily: fontFamily.trim(), terminalFontSize: fontSize });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    }
  }

  function handleOutsideClick(event: MouseEvent) {
    if (panelEl && !panelEl.contains(event.target as Node)) {
      dispatch("cancel");
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" on:mousedown={handleOutsideClick}>
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="Settings">
    <header>
      <h2>Settings</h2>
      <button class="close" aria-label="Close" title="Close" on:click={() => dispatch("cancel")}>
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <path d="M2 2l12 12M14 2L2 14" />
        </svg>
      </button>
    </header>

    <div class="content">
      <section class="card">
        <span class="section-title">Terminal</span>
        <label class="field">
          <span>Font family</span>
          <input type="text" bind:value={fontFamily} placeholder="JetBrains Mono" />
        </label>
        <label class="field">
          <span>Font size</span>
          <input type="number" bind:value={fontSize} min="8" max="32" />
        </label>
        <p class="hint">Applies to every open and future terminal tab.</p>
      </section>
    </div>

    <div class="actions">
      <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
      <button class="btn primary" disabled={!canSubmit} on:click={submit}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: var(--window-shadow-margin);
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .panel {
    width: min(380px, 90vw);
    background: var(--surface-2);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-4) 0;
    flex-shrink: 0;
  }

  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--fg-secondary);
    padding: 0;
    cursor: pointer;
  }
  .close:hover {
    background: var(--surface-4);
    color: var(--fg-primary);
  }

  .content {
    padding: var(--space-4);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .section-title {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-tertiary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--fg-secondary);
  }

  input {
    background: var(--surface-1);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.8rem;
  }
  input:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .hint {
    margin: 0;
    font-size: 0.68rem;
    color: var(--fg-tertiary);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: 0 var(--space-4) var(--space-4);
  }

  .btn {
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    color: var(--fg-primary);
    font-size: 0.78rem;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--surface-4);
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn.primary:disabled {
    background: var(--surface-4);
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
</style>
