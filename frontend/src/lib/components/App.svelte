<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./TitleBar.svelte";
  import ResizeHandles from "./ResizeHandles.svelte";
  import HostTree from "./HostTree.svelte";
  import SidebarResizer from "./SidebarResizer.svelte";
  import TabStrip from "./TabStrip.svelte";
  import PaneGrid from "./PaneGrid.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import SshConnectDialog from "./SshConnectDialog.svelte";
  import SerialConnectDialog from "./SerialConnectDialog.svelte";
  import ShellConnectDialog from "./ShellConnectDialog.svelte";
  import RdpConnectDialog from "./RdpConnectDialog.svelte";
  import SftpPanel from "./SftpPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import ShortcutsPanel from "./ShortcutsPanel.svelte";
  import NewConnectionMenu from "./NewConnectionMenu.svelte";
  import ContextMenu, { type ContextMenuItem } from "./ContextMenu.svelte";
  import type {
    Protocol,
    SessionState,
    SessionOptions,
    SshConnectOptions,
    SerialConnectOptions,
    ShellConnectOptions,
    RdpConnectOptions,
    SaveHostInput,
    Host,
    Group,
    PortusConfig,
    TerminalColors,
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
  import { terminalAppearanceVersion } from "../terminalAppearance";
  import {
    collectPaneIds,
    insertSplit,
    removePane,
    updateSplitSizes,
    usedShellNumbers,
    type PaneLayout,
    type PaneState,
    type SplitDirection,
  } from "../panes";

  interface Tab {
    id: string; // local tab id, stable across the session's lifetime
    title: string;
    /** Once the user renames a tab, session-driven title updates stop overwriting it. */
    renamed?: boolean;
    layout: PaneLayout;
    /** Which pane in this tab is the split target / shows the accent focus
     * outline — distinct from `active`-ness, which is about the owning tab. */
    activePaneId: string;
  }

  let tabs: Tab[] = [];
  let panes: Record<string, PaneState> = {};
  let activeTabId: string | null = null;
  let showSshDialog = false;
  let showSerialDialog = false;
  let showShellPresetDialog = false;
  let showRdpDialog = false;
  let showSftpPanel = false;
  let showSettingsPanel = false;
  let showShortcutsPanel = false;
  let contextMenu: { x: number; y: number; items: ContextMenuItem[] } | null = null;
  let hosts: Host[] = [];
  let groups: Group[] = [];
  let editingSshHost: Host | null = null;
  let editingSerialHost: Host | null = null;
  let editingShellHost: Host | null = null;
  let sidebarWidth = 260;
  let config: PortusConfig | null = null;

  $: activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  $: activePane = activeTab ? (panes[activeTab.activePaneId] ?? null) : null;
  $: activeSshOptions = activePane?.protocol === "ssh" ? (activePane.options as SshConnectOptions) : null;

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

  // Maps each TerminalColors field to the CSS custom property it overrides.
  // Only the properties an actual per-machine override exists for get
  // touched — everything else stays at tokens.css's xterm.js-matching
  // defaults, and removeProperty() (rather than leaving a stale inline
  // value) is what makes "reset to default" in Settings actually work.
  const ANSI_COLOR_CSS_VARS: Record<keyof TerminalColors, string> = {
    black: "--ansi-black",
    red: "--ansi-red",
    green: "--ansi-green",
    yellow: "--ansi-yellow",
    blue: "--ansi-blue",
    magenta: "--ansi-magenta",
    cyan: "--ansi-cyan",
    white: "--ansi-white",
    brightBlack: "--ansi-bright-black",
    brightRed: "--ansi-bright-red",
    brightGreen: "--ansi-bright-green",
    brightYellow: "--ansi-bright-yellow",
    brightBlue: "--ansi-bright-blue",
    brightMagenta: "--ansi-bright-magenta",
    brightCyan: "--ansi-bright-cyan",
    brightWhite: "--ansi-bright-white",
  };

  function applyTerminalColorVars(colors: TerminalColors) {
    for (const key of Object.keys(ANSI_COLOR_CSS_VARS) as Array<keyof TerminalColors>) {
      const cssVar = ANSI_COLOR_CSS_VARS[key];
      const value = colors[key];
      if (value) {
        document.documentElement.style.setProperty(cssVar, value);
      } else {
        document.documentElement.style.removeProperty(cssVar);
      }
    }
  }

  onMount(async () => {
    try {
      config = await getConfig();
      hosts = config.hosts;
      groups = config.groups;
      applyTerminalFontVars(config.settings);
      applyTerminalColorVars(config.settings.terminalColors);
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

  async function onSaveSettings(detail: { terminalFontFamily: string; terminalFontSize: number; terminalColors: TerminalColors }) {
    showSettingsPanel = false;
    const next: PortusConfig = config ?? {
      schemaVersion: 2,
      groups: [],
      hosts,
      settings: { terminalFontFamily: "JetBrains Mono", terminalFontSize: 14, terminalColors: {} },
    };
    next.settings = detail;
    await saveConfig(next);
    config = next;
    applyTerminalFontVars(next.settings);
    applyTerminalColorVars(next.settings.terminalColors);
    // Pushes the new font/colors into every already-open terminal too, not
    // just ones opened from here on.
    terminalAppearanceVersion.update((n) => n + 1);
  }

  function createPane(protocol: Protocol, title: string, options: SessionOptions, hostId?: string, shellNumber?: number): string {
    const id = crypto.randomUUID();
    panes = { ...panes, [id]: { id, protocol, title, state: "connecting", options, hostId, shellNumber } };
    return id;
  }

  function newShellTab() {
    const shellNumber = nextAvailableNumber(usedShellNumbers(panes));
    const title = `Terminal ${shellNumber}`;
    const paneId = createPane("shell", title, undefined, undefined, shellNumber);
    const tabId = crypto.randomUUID();
    tabs = [...tabs, { id: tabId, title, layout: { type: "leaf", paneId }, activePaneId: paneId }];
    activeTabId = tabId;
  }

  function openTab(protocol: Protocol, title: string, options: SessionOptions, hostId?: string) {
    const paneId = createPane(protocol, title, options, hostId);
    const tabId = crypto.randomUUID();
    tabs = [...tabs, { id: tabId, title, layout: { type: "leaf", paneId }, activePaneId: paneId }];
    activeTabId = tabId;
  }

  /** Splits the active tab's active pane, opening a fresh local shell in
   * the new half — matches hitting "+" for a new tab, just landing beside
   * the current pane instead of in a new tab. */
  function splitActivePane(direction: SplitDirection) {
    if (!activeTab) return;
    const shellNumber = nextAvailableNumber(usedShellNumbers(panes));
    const title = `Terminal ${shellNumber}`;
    const newPaneId = createPane("shell", title, undefined, undefined, shellNumber);
    const newLayout = insertSplit(activeTab.layout, activeTab.activePaneId, direction, newPaneId, crypto.randomUUID());
    const tabId = activeTab.id;
    tabs = tabs.map((t) => (t.id === tabId ? { ...t, layout: newLayout, activePaneId: newPaneId } : t));
  }

  function focusPane(tabId: string, paneId: string) {
    tabs = tabs.map((t) => (t.id === tabId ? { ...t, activePaneId: paneId } : t));
  }

  function resizeSplit(tabId: string, splitId: string, sizes: number[]) {
    tabs = tabs.map((t) => (t.id === tabId ? { ...t, layout: updateSplitSizes(t.layout, splitId, sizes) } : t));
  }

  function findTabIdForPane(paneId: string): string | null {
    return tabs.find((t) => collectPaneIds(t.layout).includes(paneId))?.id ?? null;
  }

  /** Closing a tab's last remaining pane closes the tab itself — same
   * fallback-active-tab selection as the plain tab-strip close always had. */
  function closeTab(id: string) {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;

    const ids = collectPaneIds(tabs[idx].layout);
    const restPanes = { ...panes };
    for (const paneId of ids) delete restPanes[paneId];
    panes = restPanes;

    tabs = tabs.filter((t) => t.id !== id);
    if (activeTabId === id) {
      const fallback = tabs[idx] ?? tabs[idx - 1] ?? tabs[0];
      activeTabId = fallback ? fallback.id : null;
    }
  }

  function closePane(tabId: string, paneId: string) {
    const tab = tabs.find((t) => t.id === tabId);
    if (!tab) return;

    const remainingIds = collectPaneIds(tab.layout).filter((id) => id !== paneId);
    if (remainingIds.length === 0) {
      closeTab(tabId);
      return;
    }

    const newLayout = removePane(tab.layout, paneId);
    if (!newLayout) return; // unreachable given the remainingIds check above
    const newActivePaneId = tab.activePaneId === paneId ? remainingIds[remainingIds.length - 1] : tab.activePaneId;
    tabs = tabs.map((t) => (t.id === tabId ? { ...t, layout: newLayout, activePaneId: newActivePaneId } : t));

    const restPanes = { ...panes };
    delete restPanes[paneId];
    panes = restPanes;
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
    showShellPresetDialog = false;
    editingSshHost = null;
    editingSerialHost = null;
    editingShellHost = null;
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
    } else if (host.protocol === "shell") {
      editingShellHost = host;
      showShellPresetDialog = true;
    }
  }

  async function onShellPresetConnect(detail: { options: ShellConnectOptions; save: SaveHostInput | null }) {
    showShellPresetDialog = false;
    editingShellHost = null;
    const { options, save } = detail;
    const title = save?.name ?? options.shellCommand ?? "Terminal";
    // save.id is generated client-side by ShellConnectDialog, so it's known
    // here immediately rather than only after the save round-trip resolves
    // — opening the tab doesn't need to wait on that.
    openTab("shell", title, options, save?.id ?? undefined);

    if (save) {
      const result = await saveHost(save);
      hosts = result.hosts;
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
    } else if (host.protocol === "shell") {
      const options: ShellConnectOptions = { shellCommand: host.shellCommand ?? null, workingDir: host.workingDir ?? null };
      openTab("shell", host.name, options, host.id);
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

  function onPaneState(paneId: string, state: SessionState) {
    const pane = panes[paneId];
    if (!pane) return;
    panes = { ...panes, [paneId]: { ...pane, state } };
  }

  function onPaneTitle(paneId: string, title: string) {
    // A manual rename wins permanently over whatever the session reports.
    const pane = panes[paneId];
    if (!pane || pane.renamed) return;
    panes = { ...panes, [paneId]: { ...pane, title } };
  }

  function onRename(id: string, title: string) {
    const trimmed = title.trim();
    if (!trimmed) return;
    tabs = tabs.map((t) => (t.id === id ? { ...t, title: trimmed, renamed: true } : t));
  }

  function onPaneClosed(paneId: string) {
    const tabId = findTabIdForPane(paneId);
    if (tabId) closePane(tabId, paneId);
  }

  function toggleSftpPanel() {
    showSftpPanel = !showSftpPanel;
  }

  // Ctrl+Shift+<key> rather than a plain Ctrl+<key> — xterm.js's default
  // keymap doesn't turn Ctrl+Shift combos into shell control sequences, so
  // this is safe to reserve at the app level even while a terminal pane has
  // keyboard focus, the same convention Windows Terminal/GNOME Terminal use.
  function handleKeydown(event: KeyboardEvent) {
    if (!event.ctrlKey || !event.shiftKey) return;
    const key = event.key.toLowerCase();
    if (key === "d") {
      event.preventDefault();
      splitActivePane("row");
    } else if (key === "e") {
      event.preventDefault();
      splitActivePane("column");
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app-shell">
  <ResizeHandles />
  <TitleBar
    splitDisabled={!activeTab}
    on:splitRow={() => splitActivePane("row")}
    on:splitColumn={() => splitActivePane("column")}
    on:showShortcuts={() => (showShortcutsPanel = true)}
    on:showSettings={openSettingsPanel}
  />
  <div class="action-bar">
    <div class="action-bar-sidebar" style="width: {sidebarWidth}px; min-width: {sidebarWidth}px">
      <NewConnectionMenu
        on:newSsh={openSshDialog}
        on:newRdp={openRdpDialog}
        on:newSerial={openSerialDialog}
        on:newShell={newShellTab}
        on:newShellPreset={() => (showShellPresetDialog = true)}
      />
    </div>
    <div class="action-bar-main">
      <TabStrip
        tabs={tabs.map((t) => ({ id: t.id, title: t.title, state: panes[t.activePaneId]?.state ?? "disconnected" }))}
        activeId={activeTabId}
        on:select={(e) => selectTab(e.detail.id)}
        on:close={(e) => closeTab(e.detail.id)}
        on:rename={(e) => onRename(e.detail.id, e.detail.title)}
        on:new={newShellTab}
      />
      {#if activePane?.protocol === "ssh"}
        <button class="files-btn" class:active={showSftpPanel} on:click={toggleSftpPanel}>Files</button>
      {/if}
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
          <div class="tab-panes" class:hidden={tab.id !== activeTabId}>
            <PaneGrid
              node={tab.layout}
              {panes}
              activePaneId={tab.activePaneId}
              active={tab.id === activeTabId}
              showHeader={collectPaneIds(tab.layout).length > 1}
              onFocusPane={(paneId) => focusPane(tab.id, paneId)}
              onClosePane={(paneId) => closePane(tab.id, paneId)}
              onResizeSplit={(splitId, sizes) => resizeSplit(tab.id, splitId, sizes)}
              {onPaneState}
              {onPaneTitle}
              {onPaneClosed}
            />
          </div>
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
  {#if showShellPresetDialog}
    <ShellConnectDialog
      editHost={editingShellHost}
      {groups}
      on:connect={(e) => onShellPresetConnect(e.detail)}
      on:save={(e) => onSaveHostOnly(e.detail)}
      on:cancel={() => {
        showShellPresetDialog = false;
        editingShellHost = null;
      }}
    />
  {/if}
  {#if showRdpDialog}
    <RdpConnectDialog on:connect={(e) => onRdpConnect(e.detail)} on:cancel={() => (showRdpDialog = false)} />
  {/if}
  {#if showSftpPanel && activeSshOptions && activePane}
    <SftpPanel
      options={activeSshOptions}
      title={activePane.title}
      on:close={() => (showSftpPanel = false)}
    />
  {/if}
  {#if showSettingsPanel && config}
    <SettingsPanel
      terminalFontFamily={config.settings.terminalFontFamily}
      terminalFontSize={config.settings.terminalFontSize}
      terminalColors={config.settings.terminalColors}
      on:save={(e) => onSaveSettings(e.detail)}
      on:cancel={() => (showSettingsPanel = false)}
    />
  {/if}
  {#if showShortcutsPanel}
    <ShortcutsPanel on:cancel={() => (showShortcutsPanel = false)} />
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
  .session-area {
    flex: 1;
    position: relative;
    display: flex;
    min-height: 0;
  }
  .tab-panes {
    position: absolute;
    inset: 0;
    display: flex;
  }
  .tab-panes.hidden {
    display: none;
  }
</style>
