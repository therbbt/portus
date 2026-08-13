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
  import type {
    Protocol,
    SessionState,
    SessionOptions,
    SshConnectOptions,
    SerialConnectOptions,
    RdpConnectOptions,
    SaveRequest,
    Host,
  } from "../bridge";
  import { getConfig, saveHost, deleteHost, resolveHostSecret } from "../bridge";
  import { nextAvailableNumber } from "../tabNumbering";

  interface Tab {
    id: string; // local tab id, stable across the session's lifetime
    protocol: Protocol;
    title: string;
    state: SessionState;
    sessionId: string | null;
    options?: SessionOptions;
    /** Reserves a slot in the "Local Shell N" sequence; freed when the tab closes. */
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
  let hosts: Host[] = [];
  let sidebarWidth = 260;

  $: activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  $: activeSshOptions = activeTab?.protocol === "ssh" ? (activeTab.options as SshConnectOptions) : null;

  onMount(async () => {
    try {
      const config = await getConfig();
      hosts = config.hosts;
    } catch {
      // No persisted config yet (first launch) — the rail just stays empty.
    }
  });

  function newShellTab() {
    const id = crypto.randomUUID();
    const usedNumbers = tabs.map((t) => t.shellNumber).filter((n): n is number => n !== undefined);
    const shellNumber = nextAvailableNumber(usedNumbers);
    const tab: Tab = {
      id,
      protocol: "shell",
      title: `Local Shell ${shellNumber}`,
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

  async function onSshConnect(detail: { options: SshConnectOptions; save: SaveRequest | null }) {
    showSshDialog = false;
    const { options, save } = detail;
    openTab("ssh", `${options.username}@${options.host}`, options);

    if (save) {
      const config = await saveHost({
        name: save.name,
        protocol: "ssh",
        address: options.host,
        port: options.port,
        username: options.username,
        auth:
          options.auth.type === "password"
            ? { type: "password", password: options.auth.password }
            : { type: "privateKey", path: options.auth.path, passphrase: options.auth.passphrase },
      });
      hosts = config.hosts;
    }
  }

  function openSerialDialog() {
    showSerialDialog = true;
  }

  async function onSerialConnect(detail: { options: SerialConnectOptions; save: SaveRequest | null }) {
    showSerialDialog = false;
    const { options, save } = detail;
    openTab("serial", options.portName, options);

    if (save) {
      const config = await saveHost({
        name: save.name,
        protocol: "serial",
        address: options.portName,
        baudRate: options.baudRate,
        auth: { type: "none" },
      });
      hosts = config.hosts;
    }
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
  <div class="body">
    <HostTree
      {hosts}
      width={sidebarWidth}
      on:newShell={newShellTab}
      on:newSsh={openSshDialog}
      on:newSerial={openSerialDialog}
      on:newRdp={openRdpDialog}
      on:connect={(e) => connectToSavedHost(e.detail)}
      on:deleteHost={(e) => onDeleteHost(e.detail)}
    />
    <SidebarResizer bind:width={sidebarWidth} />
    <div class="main">
      <div class="tabstrip-row">
        <TabStrip
          tabs={tabs.map((t) => ({ id: t.id, title: t.title, state: t.state }))}
          activeId={activeTabId}
          on:select={(e) => selectTab(e.detail.id)}
          on:close={(e) => closeTab(e.detail.id)}
          on:rename={(e) => onRename(e.detail.id, e.detail.title)}
          on:new={newShellTab}
        />
        {#if activeTab?.protocol === "ssh"}
          <button class="files-btn" class:active={showSftpPanel} on:click={toggleSftpPanel}>Files</button>
        {/if}
      </div>
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
    <SshConnectDialog on:connect={(e) => onSshConnect(e.detail)} on:cancel={() => (showSshDialog = false)} />
  {/if}
  {#if showSerialDialog}
    <SerialConnectDialog on:connect={(e) => onSerialConnect(e.detail)} on:cancel={() => (showSerialDialog = false)} />
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
  .tabstrip-row {
    display: flex;
    align-items: stretch;
    background: var(--surface-1);
  }
  .tabstrip-row :global(.tabstrip) {
    flex: 1;
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
  .session-area :global(.terminal-host),
  .session-area :global(.rdp-view) {
    position: absolute;
    inset: 0;
  }
</style>
