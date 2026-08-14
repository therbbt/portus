<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import type { ShellConnectOptions, Host, Group, SaveHostInput } from "../bridge";

  /** When set, prefills the form from an existing saved host and treats
   * submit as an edit (saveHost overwrites it in place) rather than a new
   * save. Shell presets carry no credential, so there's nothing to retype. */
  export let editHost: Host | null = null;
  export let groups: Group[] = [];

  const dispatch = createEventDispatcher<{
    connect: { options: ShellConnectOptions; save: SaveHostInput | null };
    save: SaveHostInput;
    cancel: void;
  }>();

  let shellCommand = "";
  let workingDir = "";
  let saveName = "";
  let groupId = "";
  let panelEl: HTMLDivElement;

  $: isEditing = !!editHost;

  onMount(() => {
    if (!editHost) return;
    shellCommand = editHost.shellCommand ?? "";
    workingDir = editHost.workingDir ?? "";
    saveName = editHost.name;
    groupId = editHost.groupId ?? "";
  });

  // Both fields are optional (blank = use the system default), so there's
  // nothing to require beyond a name to actually save it.
  $: canConnect = true;
  $: canSave = saveName.trim().length > 0;

  function buildSaveInput(): SaveHostInput {
    // Generated client-side (rather than left for save_host to fill in)
    // so the caller knows the real host id immediately, synchronously —
    // needed to open the tab with the right id for scrollback keying
    // without waiting on the save round-trip first.
    return {
      id: editHost?.id ?? crypto.randomUUID(),
      name: saveName.trim(),
      groupId: groupId || null,
      protocol: "shell",
      // Not meaningful for shell, but Host.address is required — shown in
      // the sidebar's meta line, so this doubles as the display summary.
      address: shellCommand.trim() || "$SHELL",
      auth: { type: "none" },
      shellCommand: shellCommand.trim() || null,
      workingDir: workingDir.trim() || null,
    };
  }

  function connect() {
    if (!canConnect) return;
    const options: ShellConnectOptions = {
      shellCommand: shellCommand.trim() || null,
      workingDir: workingDir.trim() || null,
    };
    dispatch("connect", { options, save: canSave ? buildSaveInput() : null });
  }

  function saveOnly() {
    if (!canSave) return;
    dispatch("save", buildSaveInput());
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    } else if (event.key === "Enter" && document.activeElement?.tagName !== "SELECT") {
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label={isEditing ? "Edit terminal preset" : "New terminal preset"}>
    <h2 class="title">{isEditing ? "Edit terminal preset" : "New terminal preset"}</h2>

    <label class="field">
      <span>Shell command (optional)</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={shellCommand} placeholder="$SHELL" autofocus />
    </label>

    <label class="field">
      <span>Working directory (optional)</span>
      <input type="text" bind:value={workingDir} placeholder="$HOME" />
    </label>

    <div class="row split">
      <label class="field grow">
        <span>Name (to save it)</span>
        <input type="text" bind:value={saveName} placeholder="My terminal" />
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
    {#if !saveName.trim()}
      <p class="hint">
        Leave the name blank for a one-off terminal that isn't saved. A saved terminal also keeps its scrollback
        across restarts.
      </p>
    {/if}

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
    width: min(360px, 90vw);
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
