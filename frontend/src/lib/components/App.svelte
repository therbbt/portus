<script lang="ts">
  import TitleBar from "./TitleBar.svelte";
  import HostTree from "./HostTree.svelte";
  import TabStrip from "./TabStrip.svelte";
  import Terminal from "./Terminal.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import SshConnectDialog from "./SshConnectDialog.svelte";
  import type { Protocol, SessionState, SshConnectOptions } from "../bridge";
  import { nextAvailableNumber } from "../tabNumbering";

  interface Tab {
    id: string; // local tab id, stable across the session's lifetime
    protocol: Protocol;
    title: string;
    state: SessionState;
    sessionId: string | null;
    options?: SshConnectOptions;
    /** Reserves a slot in the "Local Shell N" sequence; freed when the tab closes. */
    shellNumber?: number;
    /** Once the user renames a tab, session-driven title updates stop overwriting it. */
    renamed?: boolean;
  }

  let tabs: Tab[] = [];
  let activeTabId: string | null = null;
  let showSshDialog = false;

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

  function openSshDialog() {
    showSshDialog = true;
  }

  function onSshConnect(options: SshConnectOptions) {
    showSshDialog = false;
    const id = crypto.randomUUID();
    const tab: Tab = {
      id,
      protocol: "ssh",
      title: `${options.username}@${options.host}`,
      state: "connecting",
      sessionId: null,
      options,
    };
    tabs = [...tabs, tab];
    activeTabId = id;
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
    <HostTree on:newShell={newShellTab} on:newSsh={openSshDialog} />
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
          <EmptyMainArea on:newShell={newShellTab} on:newSsh={openSshDialog} />
        {/if}
      </div>
    </div>
  </div>

  {#if showSshDialog}
    <SshConnectDialog on:connect={(e) => onSshConnect(e.detail)} on:cancel={() => (showSshDialog = false)} />
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
