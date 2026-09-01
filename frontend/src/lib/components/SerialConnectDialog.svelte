<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import type { SerialConnectOptions, SavedSession, Group, SaveSessionInput } from "../bridge";
  import { listSerialPorts } from "../bridge";
  import FolderSelect from "./FolderSelect.svelte";

  /** When set, prefills the form from an existing saved session and treats
   * submit as an edit (saveSession overwrites it in place) rather than a new
   * save. Serial sessions carry no credential, so there's nothing to retype. */
  export let editSession: SavedSession | null = null;
  export let groups: Group[] = [];

  const dispatch = createEventDispatcher<{
    connect: { options: SerialConnectOptions; save: SaveSessionInput | null };
    save: SaveSessionInput;
    cancel: void;
  }>();

  const commonBaudRates = [9600, 19200, 38400, 57600, 115200];

  let portName = "";
  let baudRate = 9600;
  let availablePorts: string[] = [];
  let saveName = "";
  let groupId = "";
  let panelEl: HTMLDivElement;

  $: isEditing = !!editSession;

  onMount(async () => {
    if (editSession) {
      portName = editSession.address;
      baudRate = editSession.baudRate ?? 9600;
      saveName = editSession.name;
      groupId = editSession.groupId ?? "";
    }

    try {
      availablePorts = await listSerialPorts();
      if (!portName && availablePorts.length > 0) {
        portName = availablePorts[0];
      }
    } catch {
      // No ports, or the command isn't reachable yet — the input still
      // accepts a manually typed device path either way.
    }
  });

  $: coreValid = portName.trim().length > 0 && baudRate > 0;
  $: canConnect = coreValid;
  $: canSave = coreValid && saveName.trim().length > 0;

  function buildSaveInput(): SaveSessionInput {
    return {
      id: editSession?.id,
      name: saveName.trim(),
      groupId: groupId || null,
      protocol: "serial",
      address: portName.trim(),
      baudRate,
      auth: { type: "none" },
    };
  }

  function connect() {
    if (!canConnect) return;
    const options: SerialConnectOptions = { portName: portName.trim(), baudRate };
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label={isEditing ? "Edit serial session" : "New serial session"}>
    <h2 class="title">{isEditing ? "Edit serial session" : "New serial session"}</h2>

    <label class="field">
      <span>Port</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={portName}
        list="portus-serial-ports"
        placeholder="/dev/ttyUSB0"
        autofocus
      />
      <datalist id="portus-serial-ports">
        {#each availablePorts as port (port)}
          <option value={port}></option>
        {/each}
      </datalist>
    </label>

    {#if availablePorts.length === 0}
      <p class="hint">No serial ports detected — you can still type a device path directly.</p>
    {/if}

    <label class="field">
      <span>Baud rate</span>
      <input type="number" bind:value={baudRate} min="1" list="portus-baud-rates" />
      <datalist id="portus-baud-rates">
        {#each commonBaudRates as rate (rate)}
          <option value={rate}></option>
        {/each}
      </datalist>
    </label>

    <div class="row split">
      <label class="field grow">
        <span>Name (to save it)</span>
        <input type="text" bind:value={saveName} placeholder={portName || "My device"} />
      </label>
      <label class="field narrow">
        <span>Folder</span>
        <FolderSelect {groups} bind:value={groupId} />
      </label>
    </div>
    {#if !saveName.trim()}
      <p class="hint">Leave the name blank for a one-off connection that isn't saved.</p>
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
    width: min(340px, 90vw);
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
