<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./TitleBar.svelte";
  import ResizeHandles from "./ResizeHandles.svelte";
  import HostTree from "./HostTree.svelte";
  import SidebarResizer from "./SidebarResizer.svelte";
  import TabStrip from "./TabStrip.svelte";
  import Terminal from "./Terminal.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import SshConnectDialog from "./SshConnectDialog.svelte";
  import SerialConnectDialog from "./SerialConnectDialog.svelte";
  import RdpConnectDialog from "./RdpConnectDialog.svelte";
  import RdpView from "./RdpView.svelte";
  import SftpPanel from "./SftpPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import NewConnectionMenu from "./NewConnectionMenu.svelte";
  import ContextMenu, { type ContextMenuItem } from "./ContextMenu.svelte";
  import type {
    Protocol,
    SessionState,
    SessionOptions,
    SshConnectOptions,
    SerialConnectOptions,
    RdpConnectOptions,
    SaveHostInput,
    Host,
    Group,
    PortusConfig,
  } from "../bridge";
  import {
    getConfig,
    saveConfig,
    saveHost,
    deleteHost,
    resolveHostSecret,
    saveGroup,
    deleteGroup,
    setGroupCollapsed,
  } from "../bridge";
  import { nextAvailableNumber } from "../tabNumbering";

  interface Tab {
    id: string; // local tab id, stable across the session's lifetime
    protocol: Protocol;
    title: string;
    state: SessionState;
    sessionId: string | null;
    options?: SessionOptions;
    /** Reserves a slot in the "Terminal N" sequence; freed when the tab closes. */
    shellNumber?: number;
    /** Once the user renames a tab, session-driven title updates stop overwriting it. */
    renamed?: boolean;
  }

  let tabs: Tab[] = [];
  let activeTabId: string | null = null;
  let showSshDialog = false;
  let showSerialDialog = false;
  let showRdpDialog = false;
  let showSftpPanel = false;
  let showSettingsPanel = false;
  let contextMenu: { x: number; y: number; items: ContextMenuItem[] } | null = null;
  let hosts: Host[] = [];
  let groups: Group[] = [];
  let editingSshHost: Host | null = null;
  let editingSerialHost: Host | null = null;
  let sidebarWidth = 260;
  let config: PortusConfig | null = null;

  $: activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  $: activeSshOptions = activeTab?.protocol === "ssh" ? (activeTab.options as SshConnectOptions) : null;

  // CSS generic family keywords (monospace, ui-monospace, ...) must stay
  // unquoted — quoting one turns it into a request for an actual font
  // literally named e.g. "monospace" instead of invoking the browser's
  // built-in generic-family fallback, which is the whole point of using one.
  const CSS_GENERIC_FONT_FAMILIES = new Set([
    "monospace",
    "ui-monospace",
    "serif",
    "sans-serif",
    "system-ui",
    "cursive",
    "fantasy",
  ]);

  // Keeps a fallback chain after the chosen family — replacing --font-mono
  // with just that one name (no fallbacks) meant that if it isn't actually
  // installed, the browser silently substitutes a proportional font while
  // xterm.js still lays out cells at the fixed width it assumed for a
  // monospace font, producing visibly gapped text.
  function applyTerminalFontVars(settings: PortusConfig["settings"]) {
    const family = settings.terminalFontFamily.trim();
    const primary = CSS_GENERIC_FONT_FAMILIES.has(family) ? family : `"${family.replace(/"/g, '\\"')}"`;
    document.documentElement.style.setProperty("--font-mono", `${primary}, ui-monospace, Consolas, monospace`);
    document.documentElement.style.setProperty("--font-size-terminal", `${settings.terminalFontSize}px`);
  }

  onMount(async () => {
    try {
      config = await getConfig();
      hosts = config.hosts;
      groups = config.groups;
      applyTerminalFontVars(config.settings);
    } catch {
      // No persisted config yet (first launch) — the rail just stays empty.
    }

    // Terminal-first: land on a working shell rather than the empty state,
    // same as opening a real terminal app.
    newShellTab();
  });

  function openSettingsPanel() {
    showSettingsPanel = true;
  }

  async function onSaveSettings(detail: { terminalFontFamily: string; terminalFontSize: number }) {
    showSettingsPanel = false;
    const next: PortusConfig = config ?? {
      schemaVersion: 2,
      groups: [],
      hosts,
      settings: { terminalFontFamily: "JetBrains Mono", terminalFontSize: 14 },
    };
    next.settings = detail;
    await saveConfig(next);
    config = next;
    applyTerminalFontVars(next.settings);
  }

  function newShellTab() {
    const id = crypto.randomUUID();
    const usedNumbers = tabs.map((t) => t.shellNumber).filter((n): n is number => n !== undefined);
    const shellNumber = nextAvailableNumber(usedNumbers);
    const tab: Tab = {
      id,
      protocol: "shell",
      title: `Terminal ${shellNumber}`,
      state: "connecting",
      sessionId: null,
      shellNumber,
    };
    tabs = [...tabs, tab];
    activeTabId = id;
  }

  function openTab(protocol: Protocol, title: string, options: SessionOptions) {
    const id = crypto.randomUUID();
    const tab: Tab = { id, protocol, title, state: "connecting", sessionId: null, options };
    tabs = [...tabs, tab];
    activeTabId = id;
  }

  function openSshDialog() {
    showSshDialog = true;
  }

  async function onSshConnect(detail: { options: SshConnectOptions; save: SaveHostInput | null }) {
    showSshDialog = false;
    editingSshHost = null;
    const { options, save } = detail;
    openTab("ssh", `${options.username}@${options.host}`, options);

    if (save) {
      const result = await saveHost(save);
      hosts = result.hosts;
    }
  }

  function openSerialDialog() {
    showSerialDialog = true;
  }

  async function onSerialConnect(detail: { options: SerialConnectOptions; save: SaveHostInput | null }) {
    showSerialDialog = false;
    editingSerialHost = null;
    const { options, save } = detail;
    openTab("serial", options.portName, options);

    if (save) {
      const result = await saveHost(save);
      hosts = result.hosts;
    }
  }

  /** From a connect dialog's standalone "Save" button — persists the host
   * without opening a tab or connecting to it at all. */
  async function onSaveHostOnly(input: SaveHostInput) {
    showSshDialog = false;
    showSerialDialog = false;
    editingSshHost = null;
    editingSerialHost = null;
    const result = await saveHost(input);
    hosts = result.hosts;
  }

  function onEditHost(host: Host) {
    if (host.protocol === "ssh") {
      editingSshHost = host;
      showSshDialog = true;
    } else if (host.protocol === "serial") {
      editingSerialHost = host;
      showSerialDialog = true;
    }
  }

  async function onCreateFolder(name: string) {
    const result = await saveGroup({ name });
    groups = result.groups;
  }

  async function onRenameFolder(id: string, name: string) {
    const existing = groups.find((g) => g.id === id);
    const result = await saveGroup({ id, name, parentId: existing?.parentId ?? null });
    groups = result.groups;
  }

  async function onDeleteFolder(group: Group) {
    const result = await deleteGroup(group.id);
    groups = result.groups;
    hosts = result.hosts;
  }

  async function onToggleFolder(group: Group) {
    const collapsed = !group.collapsed;
    groups = groups.map((g) => (g.id === group.id ? { ...g, collapsed } : g));
    await setGroupCollapsed(group.id, collapsed);
  }

  function openRdpDialog() {
    showRdpDialog = true;
  }

  function onRdpConnect(options: RdpConnectOptions) {
    showRdpDialog = false;
    openTab("rdp", `${options.username}@${options.host}`, options);
  }

  async function connectToSavedHost(host: Host) {
    const secret = await resolveHostSecret(host.id).catch(() => null);

    if (host.protocol === "ssh") {
      const auth: SshConnectOptions["auth"] =
        host.auth.type === "privateKey"
          ? { type: "privateKey", path: host.auth.path ?? "", passphrase: secret }
          : { type: "password", password: secret ?? "" };
      const options: SshConnectOptions = {
        host: host.address,
        port: host.port ?? undefined,
        username: host.username ?? "",
        auth,
      };
      openTab("ssh", host.name, options);
    } else if (host.protocol === "serial") {
      const options: SerialConnectOptions = { portName: host.address, baudRate: host.baudRate ?? undefined };
      openTab("serial", host.name, options);
    } else if (host.protocol === "rdp") {
      // No UI saves an RDP host yet (see RdpConnectDialog) — this only
      // matters for a hand-edited config.json, which the format allows.
      const options: RdpConnectOptions = {
        host: host.address,
        port: host.port ?? undefined,
        username: host.username ?? "",
        password: secret ?? "",
      };
      openTab("rdp", host.name, options);
    }
  }

  async function onDeleteHost(host: Host) {
    const config = await deleteHost(host.id);
    hosts = config.hosts;
  }

  function selectTab(id: string) {
    activeTabId = id;
  }

  function closeTab(id: string) {
    const idx = tabs.findIndex((t) => t.id === id);
    tabs = tabs.filter((t) => t.id !== id);
    if (activeTabId === id) {
      const fallback = tabs[idx] ?? tabs[idx - 1] ?? tabs[0];
      activeTabId = fallback ? fallback.id : null;
    }
  }

  function onState(id: string, state: SessionState) {
    tabs = tabs.map((t) => (t.id === id ? { ...t, state } : t));
  }

  function onTitle(id: string, title: string) {
    // A manual rename wins permanently over whatever the session reports.
    tabs = tabs.map((t) => (t.id === id && !t.renamed ? { ...t, title } : t));
  }

  function onRename(id: string, title: string) {
    const trimmed = title.trim();
    if (!trimmed) return;
    tabs = tabs.map((t) => (t.id === id ? { ...t, title: trimmed, renamed: true } : t));
  }

  function onClosed(id: string) {
    closeTab(id);
  }

  function toggleSftpPanel() {
    showSftpPanel = !showSftpPanel;
  }
</script>

<div class="app-shell">
  <ResizeHandles />
  <TitleBar />
  <div class="action-bar">
    <div class="action-bar-sidebar" style="width: {sidebarWidth}px; min-width: {sidebarWidth}px">
      <NewConnectionMenu
        on:newSsh={openSshDialog}
        on:newRdp={openRdpDialog}
        on:newSerial={openSerialDialog}
        on:newShell={newShellTab}
      />
    </div>
    <div class="action-bar-main">
      <TabStrip
        tabs={tabs.map((t) => ({ id: t.id, title: t.title }))}
        activeId={activeTabId}
        on:select={(e) => selectTab(e.detail.id)}
        on:close={(e) => closeTab(e.detail.id)}
        on:rename={(e) => onRename(e.detail.id, e.detail.title)}
        on:new={newShellTab}
      />
      {#if activeTab?.protocol === "ssh"}
        <button class="files-btn" class:active={showSftpPanel} on:click={toggleSftpPanel}>Files</button>
      {/if}
      <button class="settings-btn" aria-label="Settings" title="Settings" on:click={openSettingsPanel}>
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="8" cy="8" r="2.2" />
          <path d="M8 2v1.6M8 12.4V14M14 8h-1.6M3.6 8H2M12.13 3.87l-1.13 1.13M4.99 11.01l-1.13 1.13M12.13 12.13l-1.13-1.13M4.99 4.99 3.87 3.87" />
        </svg>
      </button>
    </div>
  </div>
  <div class="body">
    <HostTree
      {hosts}
      {groups}
      width={sidebarWidth}
      on:connect={(e) => connectToSavedHost(e.detail)}
      on:deleteHost={(e) => onDeleteHost(e.detail)}
      on:editHost={(e) => onEditHost(e.detail)}
      on:createFolder={(e) => onCreateFolder(e.detail.name)}
      on:renameFolder={(e) => onRenameFolder(e.detail.id, e.detail.name)}
      on:deleteFolder={(e) => onDeleteFolder(e.detail)}
      on:toggleFolder={(e) => onToggleFolder(e.detail)}
      on:openContextMenu={(e) => (contextMenu = e.detail)}
    />
    <SidebarResizer bind:width={sidebarWidth} />
    <div class="main">
      <div class="session-area">
        {#each tabs as tab (tab.id)}
          {#if tab.protocol === "rdp"}
            <RdpView
              options={tab.options as RdpConnectOptions}
              active={tab.id === activeTabId}
              on:state={(e) => onState(tab.id, e.detail)}
              on:closed={() => onClosed(tab.id)}
            />
          {:else}
            <Terminal
              protocol={tab.protocol}
              options={tab.options}
              active={tab.id === activeTabId}
              on:state={(e) => onState(tab.id, e.detail)}
              on:title={(e) => onTitle(tab.id, e.detail.title)}
              on:closed={() => onClosed(tab.id)}
            />
          {/if}
        {/each}
        {#if tabs.length === 0}
          <EmptyMainArea
            on:newShell={newShellTab}
            on:newSsh={openSshDialog}
            on:newSerial={openSerialDialog}
            on:newRdp={openRdpDialog}
          />
        {/if}
      </div>
    </div>
  </div>

  {#if showSshDialog}
    <SshConnectDialog
      editHost={editingSshHost}
      {groups}
      on:connect={(e) => onSshConnect(e.detail)}
      on:save={(e) => onSaveHostOnly(e.detail)}
      on:cancel={() => {
        showSshDialog = false;
        editingSshHost = null;
      }}
    />
  {/if}
  {#if showSerialDialog}
    <SerialConnectDialog
      editHost={editingSerialHost}
      {groups}
      on:connect={(e) => onSerialConnect(e.detail)}
      on:save={(e) => onSaveHostOnly(e.detail)}
      on:cancel={() => {
        showSerialDialog = false;
        editingSerialHost = null;
      }}
    />
  {/if}
  {#if showRdpDialog}
    <RdpConnectDialog on:connect={(e) => onRdpConnect(e.detail)} on:cancel={() => (showRdpDialog = false)} />
  {/if}
  {#if showSftpPanel && activeSshOptions && activeTab}
    <SftpPanel
      options={activeSshOptions}
      title={activeTab.title}
      on:close={() => (showSftpPanel = false)}
    />
  {/if}
  {#if showSettingsPanel && config}
    <SettingsPanel
      terminalFontFamily={config.settings.terminalFontFamily}
      terminalFontSize={config.settings.terminalFontSize}
      on:save={(e) => onSaveSettings(e.detail)}
      on:cancel={() => (showSettingsPanel = false)}
    />
  {/if}
  {#if contextMenu}
    <ContextMenu x={contextMenu.x} y={contextMenu.y} items={contextMenu.items} onClose={() => (contextMenu = null)} />
  {/if}
</div>

<style>
  .app-shell {
    /* The Tauri window is transparent and exactly window-shadow-margin
       larger than this on every side, so the box-shadow below renders into
       that sliver against the desktop instead of a hard rectangle. */
    position: fixed;
    inset: var(--window-shadow-margin);
    display: flex;
    flex-direction: column;
    background: var(--surface-0);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
  }
  /* One continuous bar spanning the full window, like FlashPad's
     ActionToolbar — the sidebar label and the tab strip used to be two
     separate, visually disconnected bars sitting side by side; now they're
     zones within a single row, split at the same width as the sidebar
     below so the seam lines up with the resizer. */
  .action-bar {
    flex-shrink: 0;
    display: flex;
    align-items: stretch;
    height: var(--tabstrip-height);
    background: var(--surface-1);
    border-bottom: 1px solid var(--hairline);
  }
  .action-bar-sidebar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 0.4rem 0 0.3rem;
  }
  .action-bar-main {
    flex: 1;
    display: flex;
    align-items: stretch;
    min-width: 0;
  }
  .action-bar-main :global(.tabstrip) {
    flex: 1;
    min-width: 0;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .files-btn {
    flex-shrink: 0;
    align-self: center;
    margin-right: var(--space-2);
    padding: 0.3rem 0.7rem;
    background: var(--surface-2);
    color: var(--fg-secondary);
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.72rem;
    cursor: pointer;
  }
  .files-btn:hover {
    background: var(--surface-3);
    color: var(--fg-primary);
  }
  .files-btn.active {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .settings-btn {
    flex-shrink: 0;
    align-self: center;
    margin-right: var(--space-2);
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: var(--fg-tertiary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
  }
  .settings-btn:hover {
    background: var(--surface-3);
    color: var(--fg-primary);
  }
  .session-area {
    flex: 1;
    position: relative;
    display: flex;
    min-height: 0;
  }
  .session-area :global(.terminal-host),
  .session-area :global(.rdp-view) {
    position: absolute;
    inset: 0;
  }
</style>
