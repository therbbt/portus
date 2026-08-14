<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import type { SshConnectOptions, AuthInput, Host, Group, SaveHostInput } from "../bridge";
  import { resolveHostSecret } from "../bridge";

  /** When set, prefills the form from an existing saved host and treats
   * submit as an edit (saveHost overwrites it in place) rather than a new
   * save. Password auth can be left blank to keep the stored password
   * without retyping it — private-key path/passphrase are always sent as
   * typed, same as creating a new host. */
  export let editHost: Host | null = null;
  export let groups: Group[] = [];

  const dispatch = createEventDispatcher<{
    connect: { options: SshConnectOptions; save: SaveHostInput | null };
    save: SaveHostInput;
    cancel: void;
  }>();

  let host = "";
  let port = 22;
  let username = "";
  let authMethod: "password" | "privateKey" = "password";
  let password = "";
  let keyPath = "";
  let passphrase = "";
  let saveName = "";
  let groupId = "";

  let panelEl: HTMLDivElement;

  $: isEditing = !!editHost;
  // Blank password only means "keep the stored one" when the host was
  // already password-authed — switching tabs while editing means there's
  // nothing old to fall back to, so a real value is required either way.
  $: originalIsPassword = isEditing && editHost?.auth.type === "password";
  $: passwordUnchanged = originalIsPassword && authMethod === "password" && password.length === 0;

  onMount(() => {
    if (!editHost) return;
    host = editHost.address;
    port = editHost.port ?? 22;
    username = editHost.username ?? "";
    saveName = editHost.name;
    groupId = editHost.groupId ?? "";
    if (editHost.auth.type === "privateKey") {
      authMethod = "privateKey";
      keyPath = editHost.auth.path ?? "";
    } else {
      authMethod = "password";
    }
  });

  // The core fields needed either to connect or to save — a name is only
  // required for saving, not for a one-off connection.
  $: coreValid =
    host.trim().length > 0 &&
    username.trim().length > 0 &&
    (authMethod === "password" ? password.length > 0 || passwordUnchanged : keyPath.trim().length > 0);
  $: canConnect = coreValid;
  $: canSave = coreValid && saveName.trim().length > 0;

  function buildAuthInput(): AuthInput {
    if (authMethod === "password") {
      return passwordUnchanged ? { type: "unchanged" } : { type: "password", password };
    }
    return { type: "privateKey", path: keyPath.trim(), passphrase: passphrase || null };
  }

  function buildSaveInput(authInput: AuthInput): SaveHostInput {
    return {
      id: editHost?.id,
      name: saveName.trim(),
      groupId: groupId || null,
      protocol: "ssh",
      address: host.trim(),
      port,
      username: username.trim(),
      auth: authInput,
    };
  }

  async function connect() {
    if (!canConnect) return;
    const authInput = buildAuthInput();
    let connectAuth: SshConnectOptions["auth"];
    if (authInput.type === "unchanged" && editHost) {
      const secret = await resolveHostSecret(editHost.id).catch(() => null);
      connectAuth = { type: "password", password: secret ?? "" };
    } else if (authMethod === "password") {
      connectAuth = { type: "password", password };
    } else {
      connectAuth = { type: "privateKey", path: keyPath.trim(), passphrase: passphrase || null };
    }
    const options: SshConnectOptions = { host: host.trim(), port, username: username.trim(), auth: connectAuth };
    dispatch("connect", {
      options,
      save: canSave ? buildSaveInput(authInput) : null,
    });
  }

  function saveOnly() {
    if (!canSave) return;
    dispatch("save", buildSaveInput(buildAuthInput()));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    } else if (event.key === "Enter" && (event.metaKey || event.ctrlKey || document.activeElement?.tagName !== "INPUT")) {
      event.preventDefault();
      connect();
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label={isEditing ? "Edit SSH connection" : "New SSH connection"}>
    <h2 class="title">{isEditing ? "Edit SSH connection" : "New SSH connection"}</h2>

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
        <input type="password" bind:value={password} placeholder={originalIsPassword ? "Leave blank to keep existing password" : ""} />
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
      {#if isEditing}
        <p class="hint">Retype the passphrase to keep it — it isn't carried over automatically.</p>
      {/if}
    {/if}

    <div class="row split">
      <label class="field grow">
        <span>Name (to save it)</span>
        <input type="text" bind:value={saveName} placeholder={username && host ? `${username}@${host}` : "My server"} />
      </label>
      <label class="field narrow">
        <span>Folder</span>
        <select bind:value={groupId}>
          <option value="">None</option>
          {#each groups as group (group.id)}
            <option value={group.id}>{group.name}</option>
          {/each}
        </select>
      </label>
    </div>

    <p class="hint">
      {saveName.trim()
        ? "The credential above goes in your OS keychain, not this config file."
        : "Leave the name blank for a one-off connection that isn't saved."}
    </p>

    <div class="actions">
      <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
      <button class="btn" disabled={!canSave} on:click={saveOnly}>Save</button>
      <button class="btn primary" disabled={!canConnect} on:click={connect}>Connect</button>
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

  input,
  select {
    background: var(--surface-1);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.8rem;
  }
  input:focus-visible,
  select:focus-visible {
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
