<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import type {
    Protocol,
    SessionOptions,
    SshConnectOptions,
    SshAuth,
    RdpConnectOptions,
    ShellConnectOptions,
    SerialConnectOptions,
    SaveSessionInput,
    Group,
  } from "../bridge";
  import { listSerialPorts } from "../bridge";
  import Dialog from "./Dialog.svelte";

  // One popup for creating any session type - type is picked via the tab
  // strip below, not by which component gets mounted (that's how it used
  // to work: the old NewSessionMenu picked one of four separate dialogs).
  // Create-only: editing an existing saved SSH/serial/shell session still
  // goes through the original per-protocol dialogs (SshConnectDialog etc.)
  // unchanged - see the plan this was built from for why. RDP has no edit
  // dialog at all yet (SessionTree hides its edit icon for rdp rows) - a
  // saved RDP session can be created and connected to, but only deleted
  // and re-created, not edited in place.
  export let groups: Group[] = [];
  // Lets a specific entry point (e.g. EmptyMainArea's "Connect over RDP"
  // button) open straight onto the matching tab instead of always SSH.
  export let initialType: "ssh" | "rdp" | "shell" | "serial" = "ssh";

  const dispatch = createEventDispatcher<{
    connect: { protocol: Protocol; title: string; options: SessionOptions; save: SaveSessionInput | null };
    save: SaveSessionInput;
    cancel: void;
  }>();

  let activeType = initialType;

  // ---- SSH ----
  let sshHost = "";
  let sshPort = 22;
  let sshUsername = "";
  let sshAuthMethod: "password" | "privateKey" = "password";
  let sshPassword = "";
  let sshKeyPath = "";
  let sshPassphrase = "";

  // ---- RDP ----
  let rdpHost = "";
  let rdpPort = 3389;
  let rdpUsername = "";
  let rdpPassword = "";
  let rdpDomain = "";

  // ---- Terminal (protocol: "shell") ----
  let shellCommand = "";
  let workingDir = "";

  // ---- Serial ----
  let serialPortName = "";
  let serialBaudRate = 9600;
  let availablePorts: string[] = [];
  const commonBaudRates = [9600, 19200, 38400, 57600, 115200];

  // ---- Shared save fields (all four types) ----
  // Collapsed by default - showing the name/folder fields unconditionally
  // made this dialog feel a lot heavier than the single-protocol dialogs it
  // replaced, for the common case of just connecting without saving.
  let wantsToSave = false;
  let saveName = "";
  let groupId = "";

  onMount(async () => {
    try {
      availablePorts = await listSerialPorts();
      if (!serialPortName && availablePorts.length > 0) {
        serialPortName = availablePorts[0];
      }
    } catch {
      // No ports, or the command isn't reachable yet - the input still
      // accepts a manually typed device path either way.
    }
  });

  $: sshValid = sshHost.trim().length > 0 && sshUsername.trim().length > 0 && (sshAuthMethod === "password" ? sshPassword.length > 0 : sshKeyPath.trim().length > 0);
  $: rdpValid = rdpHost.trim().length > 0 && rdpUsername.trim().length > 0;
  $: serialValid = serialPortName.trim().length > 0 && serialBaudRate > 0;
  // Terminal: both fields are optional (blank = system default), so
  // there's nothing to require to connect.
  $: canConnect = activeType === "ssh" ? sshValid : activeType === "rdp" ? rdpValid : activeType === "serial" ? serialValid : true;
  $: canSave = wantsToSave && canConnect && saveName.trim().length > 0;

  function buildSshAuth(): SshAuth {
    return sshAuthMethod === "password" ? { type: "password", password: sshPassword } : { type: "privateKey", path: sshKeyPath.trim(), passphrase: sshPassphrase || null };
  }

  function connect() {
    if (!canConnect) return;

    if (activeType === "ssh") {
      const auth = buildSshAuth();
      const options: SshConnectOptions = { host: sshHost.trim(), port: sshPort, username: sshUsername.trim(), auth };
      const save: SaveSessionInput | null = canSave
        ? { name: saveName.trim(), groupId: groupId || null, protocol: "ssh", address: sshHost.trim(), port: sshPort, username: sshUsername.trim(), auth }
        : null;
      dispatch("connect", { protocol: "ssh", title: `${options.username}@${options.host}`, options, save });
    } else if (activeType === "rdp") {
      const options: RdpConnectOptions = { host: rdpHost.trim(), port: rdpPort, username: rdpUsername.trim(), password: rdpPassword, domain: rdpDomain.trim() || null };
      const save: SaveSessionInput | null = canSave
        ? {
            name: saveName.trim(),
            groupId: groupId || null,
            protocol: "rdp",
            address: rdpHost.trim(),
            port: rdpPort,
            username: rdpUsername.trim(),
            auth: { type: "password", password: rdpPassword },
          }
        : null;
      dispatch("connect", { protocol: "rdp", title: `${options.username}@${options.host}`, options, save });
    } else if (activeType === "shell") {
      const options: ShellConnectOptions = { shellCommand: shellCommand.trim() || null, workingDir: workingDir.trim() || null };
      // Generated client-side (rather than left for save_session to fill
      // in) so the caller knows the real saved-session id immediately,
      // synchronously - needed to open the tab with the right id for
      // scrollback keying without waiting on the save round-trip first.
      const save: SaveSessionInput | null = canSave
        ? {
            id: crypto.randomUUID(),
            name: saveName.trim(),
            groupId: groupId || null,
            protocol: "shell",
            // Not meaningful for shell, but SavedSession.address is
            // required - shown in the sidebar's meta line, so this
            // doubles as the display summary.
            address: options.shellCommand ?? "$SHELL",
            auth: { type: "none" },
            shellCommand: options.shellCommand,
            workingDir: options.workingDir,
          }
        : null;
      const title = save?.name ?? options.shellCommand ?? "Terminal";
      dispatch("connect", { protocol: "shell", title, options, save });
    } else {
      const options: SerialConnectOptions = { portName: serialPortName.trim(), baudRate: serialBaudRate };
      const save: SaveSessionInput | null = canSave
        ? { name: saveName.trim(), groupId: groupId || null, protocol: "serial", address: options.portName, baudRate: options.baudRate, auth: { type: "none" } }
        : null;
      dispatch("connect", { protocol: "serial", title: options.portName, options, save });
    }
  }

  function saveOnly() {
    if (!canSave) return;
    if (activeType === "ssh") {
      dispatch("save", { name: saveName.trim(), groupId: groupId || null, protocol: "ssh", address: sshHost.trim(), port: sshPort, username: sshUsername.trim(), auth: buildSshAuth() });
    } else if (activeType === "shell") {
      dispatch("save", {
        id: crypto.randomUUID(),
        name: saveName.trim(),
        groupId: groupId || null,
        protocol: "shell",
        address: shellCommand.trim() || "$SHELL",
        auth: { type: "none" },
        shellCommand: shellCommand.trim() || null,
        workingDir: workingDir.trim() || null,
      });
    } else if (activeType === "serial") {
      dispatch("save", { name: saveName.trim(), groupId: groupId || null, protocol: "serial", address: serialPortName.trim(), baudRate: serialBaudRate, auth: { type: "none" } });
    } else if (activeType === "rdp") {
      dispatch("save", {
        name: saveName.trim(),
        groupId: groupId || null,
        protocol: "rdp",
        address: rdpHost.trim(),
        port: rdpPort,
        username: rdpUsername.trim(),
        auth: { type: "password", password: rdpPassword },
      });
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey || document.activeElement?.tagName !== "INPUT")) {
      event.preventDefault();
      connect();
    }
  }

  const typeLabels: Record<typeof activeType, string> = { ssh: "SSH", rdp: "RDP", shell: "Terminal", serial: "Serial" };
</script>

<svelte:window on:keydown={handleKeydown} />

<Dialog label="New session" width="380px" on:cancel={() => dispatch("cancel")}>
  <h2 class="title">New session</h2>

  <div class="form">
  <div class="type-toggle" role="tablist" aria-label="Session type">
    {#each ["ssh", "rdp", "shell", "serial"] as const as type (type)}
      <button type="button" class="type-btn" class:active={activeType === type} role="tab" aria-selected={activeType === type} on:click={() => (activeType = type)}>
        {typeLabels[type]}
      </button>
    {/each}
  </div>

  {#if activeType === "ssh"}
    <div class="row split">
      <label class="field grow">
        <span>Host</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={sshHost} placeholder="example.com" autofocus />
      </label>
      <label class="field narrow">
        <span>Port</span>
        <input type="number" bind:value={sshPort} min="1" max="65535" />
      </label>
    </div>

    <label class="field">
      <span>Username</span>
      <input type="text" bind:value={sshUsername} placeholder="root" />
    </label>

    <div class="type-toggle" role="tablist" aria-label="Authentication method">
      <button type="button" class="type-btn" class:active={sshAuthMethod === "password"} role="tab" aria-selected={sshAuthMethod === "password"} on:click={() => (sshAuthMethod = "password")}>
        Password
      </button>
      <button type="button" class="type-btn" class:active={sshAuthMethod === "privateKey"} role="tab" aria-selected={sshAuthMethod === "privateKey"} on:click={() => (sshAuthMethod = "privateKey")}>
        Private key
      </button>
    </div>

    {#if sshAuthMethod === "password"}
      <label class="field">
        <span>Password</span>
        <input type="password" bind:value={sshPassword} />
      </label>
    {:else}
      <label class="field">
        <span>Key path</span>
        <input type="text" bind:value={sshKeyPath} placeholder="~/.ssh/id_ed25519" />
      </label>
      <label class="field">
        <span>Passphrase (optional)</span>
        <input type="password" bind:value={sshPassphrase} />
      </label>
    {/if}
  {:else if activeType === "rdp"}
    <div class="row split">
      <label class="field grow">
        <span>Host</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={rdpHost} placeholder="example.com" autofocus />
      </label>
      <label class="field narrow">
        <span>Port</span>
        <input type="number" bind:value={rdpPort} min="1" max="65535" />
      </label>
    </div>

    <label class="field">
      <span>Username</span>
      <input type="text" bind:value={rdpUsername} placeholder="Administrator" />
    </label>

    <label class="field">
      <span>Password</span>
      <input type="password" bind:value={rdpPassword} />
    </label>

    <label class="field">
      <span>Domain (optional)</span>
      <input type="text" bind:value={rdpDomain} />
    </label>

    <p class="hint">View-only for now — no keyboard or mouse input is sent yet.</p>
  {:else if activeType === "shell"}
    <label class="field">
      <span>Shell command (optional)</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={shellCommand} placeholder="$SHELL" autofocus />
    </label>

    <label class="field">
      <span>Working directory (optional)</span>
      <input type="text" bind:value={workingDir} placeholder="$HOME" />
    </label>
  {:else}
    <label class="field">
      <span>Port</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={serialPortName} list="portus-new-session-serial-ports" placeholder="/dev/ttyUSB0" autofocus />
      <datalist id="portus-new-session-serial-ports">
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
      <input type="number" bind:value={serialBaudRate} min="1" list="portus-new-session-baud-rates" />
      <datalist id="portus-new-session-baud-rates">
        {#each commonBaudRates as rate (rate)}
          <option value={rate}></option>
        {/each}
      </datalist>
    </label>
  {/if}

  <label class="save-toggle">
    <input type="checkbox" bind:checked={wantsToSave} />
    <span>Save this session</span>
  </label>
  {#if wantsToSave}
    <div class="row split">
      <label class="field grow">
        <span>Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={saveName}
          autofocus
          placeholder={activeType === "ssh" && sshUsername && sshHost
            ? `${sshUsername}@${sshHost}`
            : activeType === "rdp" && rdpUsername && rdpHost
              ? `${rdpUsername}@${rdpHost}`
              : "My session"}
        />
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
    <p class="hint">The credential above goes in your OS keychain, not this config file.</p>
  {/if}

  <div class="actions">
    <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
    {#if wantsToSave}
      <button class="btn" disabled={!canSave} on:click={saveOnly}>Save</button>
    {/if}
    <button class="btn primary" disabled={!canConnect} on:click={connect}>Connect</button>
  </div>
  </div>
</Dialog>

<style>
  .title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .form {
    display: flex;
    flex-direction: column;
    /* Tighter than Dialog.svelte's own --space-3 gap between its slot's
       direct children (title, this wrapper) - with this many stacked rows,
       that gap compounded into a noticeably heavier-feeling dialog than
       the single-protocol ones it replaced. */
    gap: 0.6rem;
  }

  .save-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--fg-secondary);
    cursor: pointer;
    user-select: none;
  }
  .save-toggle input {
    margin: 0;
    accent-color: var(--accent);
  }

  .type-toggle {
    display: flex;
    background: var(--surface-1);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
  }
  .type-btn {
    flex: 1;
    padding: 0.3rem 0.5rem;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--fg-secondary);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .type-btn.active {
    background: var(--surface-3);
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
  select {
    /* Without this, the native control keeps its own baked-in vertical
       padding on top of ours, rendering noticeably taller than a text
       input even with identical CSS padding — this is what actually made
       it look mismatched. The custom arrow replaces the one appearance:
       none removes. */
    appearance: none;
    -webkit-appearance: none;
    padding-right: 1.4rem;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 10 10'%3E%3Cpath d='M2.5 3.5L5 6.5L7.5 3.5' fill='none' stroke='%2397958d' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.45rem center;
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
