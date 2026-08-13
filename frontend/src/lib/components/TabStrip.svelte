<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { SessionState } from "../bridge";

  export let tabs: Array<{ id: string; title: string; state: SessionState }> = [];
  export let activeId: string | null = null;

  const dispatch = createEventDispatcher<{
    select: { id: string };
    close: { id: string };
    new: void;
  }>();
</script>

<div class="tabstrip">
  <div class="tabs">
    {#each tabs as tab (tab.id)}
      <button
        class="tab"
        class:active={tab.id === activeId}
        on:click={() => dispatch("select", { id: tab.id })}
      >
        <span class="status-dot" data-state={tab.state}></span>
        <span class="tab-title">{tab.title}</span>
        <span
          class="tab-close"
          role="button"
          tabindex="0"
          aria-label="Close tab"
          on:click|stopPropagation={() => dispatch("close", { id: tab.id })}
          on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("close", { id: tab.id })}
        >
          ×
        </span>
      </button>
    {/each}
  </div>
  <button class="new-tab" aria-label="New tab" on:click={() => dispatch("new")}>+</button>
</div>

<style>
  .tabstrip {
    height: var(--tabstrip-height);
    display: flex;
    align-items: stretch;
    background: var(--surface-1);
  }
  .tabs {
    display: flex;
    flex: 1;
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--fg-secondary);
    cursor: pointer;
    white-space: nowrap;
    font-size: 12.5px;
  }
  .tab:hover {
    background: var(--surface-2);
    color: var(--fg-primary);
  }
  .tab.active {
    background: var(--surface-2);
    color: var(--fg-primary);
    border-bottom-color: var(--accent);
  }
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--status-disconnected);
    flex-shrink: 0;
  }
  .status-dot[data-state="connecting"] {
    background: var(--status-connecting);
  }
  .status-dot[data-state="connected"] {
    background: var(--status-connected);
  }
  .status-dot[data-state="disconnected"] {
    background: var(--status-disconnected);
  }
  .tab-close {
    color: var(--fg-tertiary);
    padding: 0 2px;
    border-radius: var(--radius-sm);
    line-height: 1;
  }
  .tab-close:hover {
    color: var(--fg-primary);
    background: var(--surface-4);
  }
  .new-tab {
    width: 36px;
    background: transparent;
    border: none;
    color: var(--fg-tertiary);
    cursor: pointer;
    font-size: 16px;
  }
  .new-tab:hover {
    color: var(--fg-primary);
    background: var(--surface-2);
  }
</style>
