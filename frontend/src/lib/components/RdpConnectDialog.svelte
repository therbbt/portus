<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { RdpConnectOptions } from "../bridge";

  const dispatch = createEventDispatcher<{
    connect: RdpConnectOptions;
    cancel: void;
  }>();

  let host = "";
  let port = 3389;
  let username = "";
  let password = "";
  let domain = "";

  let panelEl: HTMLDivElement;

  $: canSubmit = host.trim().length > 0 && username.trim().length > 0;

  function submit() {
    if (!canSubmit) return;
    dispatch("connect", {
      host: host.trim(),
      port,
      username: username.trim(),
      password,
      domain: domain.trim() || null,
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    } else if (event.key === "Enter" && (event.metaKey || event.ctrlKey || document.activeElement?.tagName !== "INPUT")) {
      event.preventDefault();
      submit();
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="New RDP connection">
    <h2 class="title">New RDP connection</h2>

    <div class="row split">
      <label class="field grow">
        <span>Host</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={host} placeholder="example.com" autofocus />
      </label>
      <label class="field narrow">
        <span>Port</span>
        <input type="number" bind:value={port} min="1" max="65535" />
      </label>
    </div>

    <label class="field">
      <span>Username</span>
      <input type="text" bind:value={username} placeholder="Administrator" />
    </label>

    <label class="field">
      <span>Password</span>
      <input type="password" bind:value={password} />
    </label>

    <label class="field">
      <span>Domain (optional)</span>
      <input type="text" bind:value={domain} />
    </label>

    <p class="hint">View-only for now — no keyboard or mouse input is sent yet.</p>

    <div class="actions">
      <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
      <button class="btn primary" disabled={!canSubmit} on:click={submit}>Connect</button>
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
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .row.split {
    display: flex;
    gap: var(--space-2);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--fg-secondary);
  }
  .field.grow {
    flex: 1;
  }
  .field.narrow {
    width: 90px;
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
    margin-top: var(--space-1);
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
