<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./TitleBar.svelte";
  import ResizeHandles from "./ResizeHandles.svelte";
  import SessionTree from "./SessionTree.svelte";
  import SidebarResizer from "./SidebarResizer.svelte";
  import TabStrip from "./TabStrip.svelte";
  import PaneGrid from "./PaneGrid.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import SshConnectDialog from "./SshConnectDialog.svelte";
  import SerialConnectDialog from "./SerialConnectDialog.svelte";
  import ShellConnectDialog from "./ShellConnectDialog.svelte";
  import NewSessionDialog from "./NewSessionDialog.svelte";
  import SftpPanel from "./SftpPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import ShortcutsPanel from "./ShortcutsPanel.svelte";
  import NewSessionButton from "./NewSessionButton.svelte";
  import SaveAsDialog from "./SaveAsDialog.svelte";
  import ContextMenu, { type ContextMenuItem } from "./ContextMenu.svelte";
  import type {
    Protocol,
    SessionState,
    SessionOptions,
    SshConnectOptions,
    SerialConnectOptions,
    ShellConnectOptions,
    RdpConnectOptions,
    SaveSessionInput,
    SavedSession,
    Group,
    PortusConfig,
    TerminalColors,
  } from "../bridge";
  import {
    getConfig,
    saveConfig,
    saveSession,
    deleteSession,
    resolveSessionSecret,
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
  // Create-only - see NewSessionDialog.svelte. Editing an existing saved
  // session still goes through the three flags above (SSH/Serial/Shell -
  // RDP has no edit path, so it has no equivalent here at all anymore).
  let showNewSessionDialog = false;
  let newSessionInitialType: "ssh" | "rdp" | "shell" | "serial" = "ssh";
  // "Save as session" from a tab's right-click menu — saves whichever pane
  // is currently active within that tab (the one with the split focus
  // outline), regardless of whether it's a split tab or a single pane.
  let showSaveAsDialog = false;
  let saveAsPane: PaneState | null = null;
  let saveAsSuggestedName = "";
  let showSftpPanel = false;
  let showSettingsPanel = false;
  let showShortcutsPanel = false;
  let contextMenu: { x: number; y: number; items: ContextMenuItem[] } | null = null;
  let sessions: SavedSession[] = [];
  let groups: Group[] = [];
  let editingSshSession: SavedSession | null = null;
  let editingSerialSession: SavedSession | null = null;
  let editingShellSession: SavedSession | null = null;
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
      sessions = config.sessions;
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
      schemaVersion: 3,
      groups: [],
      sessions,
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

  function createPane(protocol: Protocol, title: string, options: SessionOptions, savedSessionId?: string, shellNumber?: number): string {
    const id = crypto.randomUUID();
    panes = { ...panes, [id]: { id, protocol, title, state: "connecting", options, savedSessionId, shellNumber } };
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

  function openTab(protocol: Protocol, title: string, options: SessionOptions, savedSessionId?: string) {
    const paneId = createPane(protocol, title, options, savedSessionId);
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

  async function onSshConnect(detail: { options: SshConnectOptions; save: SaveSessionInput | null }) {
    showSshDialog = false;
    editingSshSession = null;
    const { options, save } = detail;
    openTab("ssh", `${options.username}@${options.host}`, options);

    if (save) {
      const result = await saveSession(save);
      sessions = result.sessions;
    }
  }

  async function onSerialConnect(detail: { options: SerialConnectOptions; save: SaveSessionInput | null }) {
    showSerialDialog = false;
    editingSerialSession = null;
    const { options, save } = detail;
    openTab("serial", options.portName, options);

    if (save) {
      const result = await saveSession(save);
      sessions = result.sessions;
    }
  }

  /** From a connect dialog's standalone "Save" button — persists the
   * session without opening a tab or connecting to it at all. */
  async function onSaveSessionOnly(input: SaveSessionInput) {
    showSshDialog = false;
    showSerialDialog = false;
    showShellPresetDialog = false;
    showNewSessionDialog = false;
    editingSshSession = null;
    editingSerialSession = null;
    editingShellSession = null;
    const result = await saveSession(input);
    sessions = result.sessions;
  }

  async function onNewSessionConnect(detail: { protocol: Protocol; title: string; options: SessionOptions; save: SaveSessionInput | null }) {
    showNewSessionDialog = false;
    const { protocol, title, options, save } = detail;
    openTab(protocol, title, options, save?.id ?? undefined);

    if (save) {
      const result = await saveSession(save);
      sessions = result.sessions;
    }
  }

  function onEditSession(session: SavedSession) {
    if (session.protocol === "ssh") {
      editingSshSession = session;
      showSshDialog = true;
    } else if (session.protocol === "serial") {
      editingSerialSession = session;
      showSerialDialog = true;
    } else if (session.protocol === "shell") {
      editingShellSession = session;
      showShellPresetDialog = true;
    }
  }

  async function onShellPresetConnect(detail: { options: ShellConnectOptions; save: SaveSessionInput | null }) {
    showShellPresetDialog = false;
    editingShellSession = null;
    const { options, save } = detail;
    const title = save?.name ?? options.shellCommand ?? "Terminal";
    // save.id is generated client-side by ShellConnectDialog, so it's known
    // here immediately rather than only after the save round-trip resolves
    // — opening the tab doesn't need to wait on that.
    openTab("shell", title, options, save?.id ?? undefined);

    if (save) {
      const result = await saveSession(save);
      sessions = result.sessions;
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
    sessions = result.sessions;
  }

  async function onToggleFolder(group: Group) {
    const collapsed = !group.collapsed;
    groups = groups.map((g) => (g.id === group.id ? { ...g, collapsed } : g));
    await setGroupCollapsed(group.id, collapsed);
  }

  async function connectToSavedSession(session: SavedSession) {
    const secret = await resolveSessionSecret(session.id).catch(() => null);

    if (session.protocol === "ssh") {
      const auth: SshConnectOptions["auth"] =
        session.auth.type === "privateKey"
          ? { type: "privateKey", path: session.auth.path ?? "", passphrase: secret }
          : { type: "password", password: secret ?? "" };
      const options: SshConnectOptions = {
        host: session.address,
        port: session.port ?? undefined,
        username: session.username ?? "",
        auth,
      };
      openTab("ssh", session.name, options);
    } else if (session.protocol === "serial") {
      const options: SerialConnectOptions = { portName: session.address, baudRate: session.baudRate ?? undefined };
      openTab("serial", session.name, options);
    } else if (session.protocol === "shell") {
      const options: ShellConnectOptions = { shellCommand: session.shellCommand ?? null, workingDir: session.workingDir ?? null };
      openTab("shell", session.name, options, session.id);
    } else if (session.protocol === "rdp") {
      const options: RdpConnectOptions = {
        host: session.address,
        port: session.port ?? undefined,
        username: session.username ?? "",
        password: secret ?? "",
      };
      openTab("rdp", session.name, options);
    }
  }

  async function onDeleteSession(session: SavedSession) {
    const config = await deleteSession(session.id);
    sessions = config.sessions;
  }

  function onSaveTabAsSession(tabId: string) {
    const tab = tabs.find((t) => t.id === tabId);
    const pane = tab ? panes[tab.activePaneId] : null;
    if (!pane) return;
    saveAsPane = pane;
    saveAsSuggestedName = tab?.title ?? "";
    showSaveAsDialog = true;
  }

  /** Builds a SaveSessionInput from a live pane's own connect options —
   * `null` for protocols with nothing sensible to save (echo, telnet).
   * Unlike the connect dialogs' own save paths, this reads a credential (if
   * any) straight out of the pane's in-memory options rather than needing
   * it retyped, since that's exactly what was used to establish the
   * connection in the first place. */
  function buildSaveInputFromPane(pane: PaneState, name: string, groupId: string | null): SaveSessionInput | null {
    if (pane.protocol === "ssh") {
      const options = pane.options as SshConnectOptions;
      return { name, groupId, protocol: "ssh", address: options.host, port: options.port, username: options.username, auth: options.auth };
    }
    if (pane.protocol === "rdp") {
      const options = pane.options as RdpConnectOptions;
      return {
        name,
        groupId,
        protocol: "rdp",
        address: options.host,
        port: options.port,
        username: options.username,
        auth: { type: "password", password: options.password },
      };
    }
    if (pane.protocol === "serial") {
      const options = pane.options as SerialConnectOptions;
      return { name, groupId, protocol: "serial", address: options.portName, baudRate: options.baudRate, auth: { type: "none" } };
    }
    if (pane.protocol === "shell") {
      const options = (pane.options as ShellConnectOptions | undefined) ?? {};
      return {
        id: crypto.randomUUID(),
        name,
        groupId,
        protocol: "shell",
        address: options.shellCommand ?? "$SHELL",
        auth: { type: "none" },
        shellCommand: options.shellCommand ?? null,
        workingDir: options.workingDir ?? null,
      };
    }
    return null;
  }

  async function onSaveAsSubmit(detail: { name: string; groupId: string | null }) {
    showSaveAsDialog = false;
    const pane = saveAsPane;
    saveAsPane = null;
    if (!pane) return;
    const input = buildSaveInputFromPane(pane, detail.name, detail.groupId);
    if (!input) return;
    const result = await saveSession(input);
    sessions = result.sessions;
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
    const pane = panes[paneId];
    // A session that never got past "connecting" failed outright — bad
    // host, wrong credentials, unreachable server. Auto-closing here would
    // hide the error Terminal.svelte just wrote into the pane before
    // there's any chance to read it, so leave it open (flipping the dot to
    // "disconnected" so it doesn't look stuck) and let the user close it
    // manually instead. A session that did connect and later disconnects
    // still auto-closes, unchanged.
    if (pane && pane.state === "connecting") {
      panes = { ...panes, [paneId]: { ...pane, state: "disconnected" } };
      return;
    }
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
      <NewSessionButton
        on:open={() => {
          newSessionInitialType = "ssh";
          showNewSessionDialog = true;
        }}
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
        on:saveAs={(e) => onSaveTabAsSession(e.detail.id)}
        on:openContextMenu={(e) => (contextMenu = e.detail)}
      />
      {#if activePane?.protocol === "ssh"}
        <button class="files-btn" class:active={showSftpPanel} on:click={toggleSftpPanel}>Files</button>
      {/if}
    </div>
  </div>
  <div class="body">
    <SessionTree
      {sessions}
      {groups}
      width={sidebarWidth}
      on:connect={(e) => connectToSavedSession(e.detail)}
      on:deleteSession={(e) => onDeleteSession(e.detail)}
      on:editSession={(e) => onEditSession(e.detail)}
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
            on:newShell={() => {
              newSessionInitialType = "shell";
              showNewSessionDialog = true;
            }}
            on:newSsh={() => {
              newSessionInitialType = "ssh";
              showNewSessionDialog = true;
            }}
            on:newSerial={() => {
              newSessionInitialType = "serial";
              showNewSessionDialog = true;
            }}
            on:newRdp={() => {
              newSessionInitialType = "rdp";
              showNewSessionDialog = true;
            }}
          />
        {/if}
      </div>
    </div>
  </div>

  {#if showSshDialog}
    <SshConnectDialog
      editSession={editingSshSession}
      {groups}
      on:connect={(e) => onSshConnect(e.detail)}
      on:save={(e) => onSaveSessionOnly(e.detail)}
      on:cancel={() => {
        showSshDialog = false;
        editingSshSession = null;
      }}
    />
  {/if}
  {#if showSerialDialog}
    <SerialConnectDialog
      editSession={editingSerialSession}
      {groups}
      on:connect={(e) => onSerialConnect(e.detail)}
      on:save={(e) => onSaveSessionOnly(e.detail)}
      on:cancel={() => {
        showSerialDialog = false;
        editingSerialSession = null;
      }}
    />
  {/if}
  {#if showShellPresetDialog}
    <ShellConnectDialog
      editSession={editingShellSession}
      {groups}
      on:connect={(e) => onShellPresetConnect(e.detail)}
      on:save={(e) => onSaveSessionOnly(e.detail)}
      on:cancel={() => {
        showShellPresetDialog = false;
        editingShellSession = null;
      }}
    />
  {/if}
  {#if showNewSessionDialog}
    <NewSessionDialog
      {groups}
      initialType={newSessionInitialType}
      on:connect={(e) => onNewSessionConnect(e.detail)}
      on:save={(e) => onSaveSessionOnly(e.detail)}
      on:cancel={() => (showNewSessionDialog = false)}
    />
  {/if}
  {#if showSaveAsDialog}
    <SaveAsDialog
      {groups}
      suggestedName={saveAsSuggestedName}
      on:save={(e) => onSaveAsSubmit(e.detail)}
      on:cancel={() => {
        showSaveAsDialog = false;
        saveAsPane = null;
      }}
    />
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
