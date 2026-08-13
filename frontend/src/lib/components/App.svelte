<script lang="ts">
  import TitleBar from "./TitleBar.svelte";
  import HostTree from "./HostTree.svelte";
  import TabStrip from "./TabStrip.svelte";
  import Terminal from "./Terminal.svelte";
  import EmptyMainArea from "./EmptyMainArea.svelte";
  import type { Protocol, SessionState } from "../bridge";

  interface Tab {
    id: string; // local tab id, stable across the session's lifetime
    protocol: Protocol;
    title: string;
    state: SessionState;
    sessionId: string | null;
  }

  let tabs: Tab[] = [];
  let activeTabId: string | null = null;
  let nextTabNum = 1;

  function newShellTab() {
    const id = crypto.randomUUID();
    const tab: Tab = {
      id,
      protocol: "shell",
      title: `Local Shell ${nextTabNum++}`,
      state: "connecting",
      sessionId: null,
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

  function onClosed(id: string) {
    closeTab(id);
  }
</script>

<div class="app-shell">
  <TitleBar />
  <div class="body">
    <HostTree on:newShell={newShellTab} />
    <div class="main">
      <TabStrip
        tabs={tabs.map((t) => ({ id: t.id, title: t.title, state: t.state }))}
        activeId={activeTabId}
        on:select={(e) => selectTab(e.detail.id)}
        on:close={(e) => closeTab(e.detail.id)}
        on:new={newShellTab}
      />
      <div class="session-area">
        {#each tabs as tab (tab.id)}
          <Terminal
            protocol={tab.protocol}
            active={tab.id === activeTabId}
            on:state={(e) => onState(tab.id, e.detail)}
            on:closed={() => onClosed(tab.id)}
          />
        {/each}
        {#if tabs.length === 0}
          <EmptyMainArea on:newShell={newShellTab} />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .app-shell {
    height: 100%;
    display: flex;
    flex-direction: column;
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
