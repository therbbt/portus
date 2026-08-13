<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import type { SerialConnectOptions, SaveRequest } from "../bridge";
  import { listSerialPorts } from "../bridge";

  const dispatch = createEventDispatcher<{
    connect: { options: SerialConnectOptions; save: SaveRequest | null };
    cancel: void;
  }>();

  const commonBaudRates = [9600, 19200, 38400, 57600, 115200];

  let portName = "";
  let baudRate = 9600;
  let availablePorts: string[] = [];
  let saveConnection = false;
  let saveName = "";
  let panelEl: HTMLDivElement;

  onMount(async () => {
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

  $: canSubmit = portName.trim().length > 0 && baudRate > 0 && (!saveConnection || saveName.trim().length > 0);

  function submit() {
    if (!canSubmit) return;
    const options: SerialConnectOptions = { portName: portName.trim(), baudRate };
    dispatch("connect", { options, save: saveConnection ? { name: saveName.trim() } : null });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    } else if (event.key === "Enter" && document.activeElement?.tagName !== "SELECT") {
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
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="New serial connection">
    <h2 class="title">New serial connection</h2>

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

    <label class="checkbox-field">
      <input type="checkbox" bind:checked={saveConnection} />
      <span>Save this connection</span>
    </label>
    {#if saveConnection}
      <label class="field">
        <span>Name</span>
        <input type="text" bind:value={saveName} placeholder={portName || "My device"} />
      </label>
    {/if}

    <div class="actions">
      <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
      <button class="btn primary" disabled={!canSubmit} on:click={submit}>Connect</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
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
