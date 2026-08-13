<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { SshConnectOptions, SaveRequest } from "../bridge";

  const dispatch = createEventDispatcher<{
    connect: { options: SshConnectOptions; save: SaveRequest | null };
    cancel: void;
  }>();

  let host = "";
  let port = 22;
  let username = "";
  let authMethod: "password" | "privateKey" = "password";
  let password = "";
  let keyPath = "";
  let passphrase = "";
  let saveConnection = false;
  let saveName = "";

  let panelEl: HTMLDivElement;

  $: canSubmit =
    host.trim().length > 0 &&
    username.trim().length > 0 &&
    (authMethod === "password" ? password.length > 0 : keyPath.trim().length > 0) &&
    (!saveConnection || saveName.trim().length > 0);

  function submit() {
    if (!canSubmit) return;
    const options: SshConnectOptions = {
      host: host.trim(),
      port,
      username: username.trim(),
      auth:
        authMethod === "password"
          ? { type: "password", password }
          : { type: "privateKey", path: keyPath.trim(), passphrase: passphrase || null },
    };
    dispatch("connect", { options, save: saveConnection ? { name: saveName.trim() } : null });
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="New SSH connection">
    <h2 class="title">New SSH connection</h2>

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
      <input type="text" bind:value={username} placeholder="root" />
    </label>

    <div class="auth-toggle" role="tablist" aria-label="Authentication method">
      <button
        type="button"
        class="toggle-btn"
        class:active={authMethod === "password"}
        role="tab"
        aria-selected={authMethod === "password"}
        on:click={() => (authMethod = "password")}
      >
        Password
      </button>
      <button
        type="button"
        class="toggle-btn"
        class:active={authMethod === "privateKey"}
        role="tab"
        aria-selected={authMethod === "privateKey"}
        on:click={() => (authMethod = "privateKey")}
      >
        Private key
      </button>
    </div>

    {#if authMethod === "password"}
      <label class="field">
        <span>Password</span>
        <input type="password" bind:value={password} />
      </label>
    {:else}
      <label class="field">
        <span>Key path</span>
        <input type="text" bind:value={keyPath} placeholder="~/.ssh/id_ed25519" />
      </label>
      <label class="field">
        <span>Passphrase (optional)</span>
        <input type="password" bind:value={passphrase} />
      </label>
    {/if}

    <label class="checkbox-field">
      <input type="checkbox" bind:checked={saveConnection} />
      <span>Save this connection</span>
    </label>
    {#if saveConnection}
      <label class="field">
        <span>Name</span>
        <input type="text" bind:value={saveName} placeholder={username && host ? `${username}@${host}` : "My server"} />
      </label>
    {/if}

    <p class="hint">
      {saveConnection
        ? "The credential above goes in your OS keychain, not this config file."
        : "Credentials aren't saved — you'll be asked again next time."}
    </p>

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

  .auth-toggle {
    display: flex;
    background: var(--surface-1);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
  }
  .toggle-btn {
    flex: 1;
    padding: 0.3rem 0.5rem;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--fg-secondary);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .toggle-btn.active {
    background: var(--surface-3);
    color: var(--fg-primary);
  }

  .checkbox-field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--fg-secondary);
    cursor: pointer;
  }
  .checkbox-field input {
    padding: 0;
    accent-color: var(--accent);
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
