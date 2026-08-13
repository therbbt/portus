<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./TitleBar.svelte";
  import HostTree from "./HostTree.svelte";
  import TabStrip from "./TabStrip.svelte";
  import Terminal from "./Terminal.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import SshConnectDialog from "./SshConnectDialog.svelte";
  import SerialConnectDialog from "./SerialConnectDialog.svelte";
  import type {
    Protocol,
    SessionState,
    SessionOptions,
    SshConnectOptions,
    SerialConnectOptions,
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
  let hosts: Host[] = [];

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
</script>

<div class="app-shell">
  <TitleBar />
  <div class="body">
    <HostTree
      {hosts}
      on:newShell={newShellTab}
      on:newSsh={openSshDialog}
      on:newSerial={openSerialDialog}
      on:connect={(e) => connectToSavedHost(e.detail)}
      on:deleteHost={(e) => onDeleteHost(e.detail)}
    />
    <div class="main">
      <TabStrip
        tabs={tabs.map((t) => ({ id: t.id, title: t.title, state: t.state }))}
        activeId={activeTabId}
        on:select={(e) => selectTab(e.detail.id)}
        on:close={(e) => closeTab(e.detail.id)}
        on:rename={(e) => onRename(e.detail.id, e.detail.title)}
        on:new={newShellTab}
      />
      <div class="session-area">
        {#each tabs as tab (tab.id)}
          <Terminal
            protocol={tab.protocol}
            options={tab.options}
            active={tab.id === activeTabId}
            on:state={(e) => onState(tab.id, e.detail)}
            on:title={(e) => onTitle(tab.id, e.detail.title)}
            on:closed={() => onClosed(tab.id)}
          />
        {/each}
        {#if tabs.length === 0}
          <EmptyMainArea on:newShell={newShellTab} on:newSsh={openSshDialog} on:newSerial={openSerialDialog} />
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
</div>

<style>
  .app-shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
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
  .session-area {
    flex: 1;
    position: relative;
    display: flex;
    min-height: 0;
  }
  .session-area :global(.terminal-host) {
    position: absolute;
    inset: 0;
  }
</style>
