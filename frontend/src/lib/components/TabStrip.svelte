<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { SessionState } from "../bridge";

  export let tabs: Array<{ id: string; title: string; state: SessionState }> = [];
  export let activeId: string | null = null;

  const dispatch = createEventDispatcher<{
    select: { id: string };
    close: { id: string };
    rename: { id: string; title: string };
    new: void;
  }>();

  let editingId: string | null = null;
  let editValue = "";

  function startEdit(tab: { id: string; title: string }) {
    editingId = tab.id;
    editValue = tab.title;
  }

  function commitEdit(id: string) {
    if (editingId !== id) return;
    editingId = null;
    dispatch("rename", { id, title: editValue });
  }

  function cancelEdit() {
    editingId = null;
  }

  function handleEditKeydown(event: KeyboardEvent, id: string) {
    if (event.key === "Enter") {
      event.preventDefault();
      commitEdit(id);
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelEdit();
    }
  }

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="tabstrip">
  <div class="tabs">
    {#each tabs as tab (tab.id)}
      <div
        class="tab"
        class:active={tab.id === activeId}
        role="tab"
        tabindex="0"
        aria-selected={tab.id === activeId}
        on:click={() => dispatch("select", { id: tab.id })}
        on:keydown={(e) => e.key === "Enter" && dispatch("select", { id: tab.id })}
      >
        <span class="status-dot" data-state={tab.state}></span>
        {#if editingId === tab.id}
          <input
            class="tab-title-input"
            bind:value={editValue}
            use:focusAndSelect
            on:click|stopPropagation
            on:dblclick|stopPropagation
            on:keydown|stopPropagation={(e) => handleEditKeydown(e, tab.id)}
            on:blur={() => commitEdit(tab.id)}
          />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="tab-title" on:dblclick|stopPropagation={() => startEdit(tab)}>{tab.title}</span>
        {/if}
        <span
          class="tab-close"
          role="button"
          tabindex="0"
          aria-label="Close tab"
          title="Close tab"
          on:click|stopPropagation={() => dispatch("close", { id: tab.id })}
          on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("close", { id: tab.id })}
        >
          ×
        </span>
      </div>
    {/each}
  </div>
  <button class="new-tab" aria-label="New tab" title="New local shell tab" on:click={() => dispatch("new")}>+</button>
</div>

<style>
  .tabstrip {
    /* No height/background of its own — it now lives inside App.svelte's
       unified .action-bar, which is the single source of truth for both. */
    display: flex;
    align-items: stretch;
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
    color: var(--fg-secondary);
    cursor: pointer;
    white-space: nowrap;
    font-size: 12.5px;
    user-select: none;
  }
  .tab:hover {
    background: var(--surface-2);
    color: var(--fg-primary);
  }
  .tab.active {
    background: var(--surface-2);
    color: var(--fg-primary);
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
  .tab-title-input {
    background: var(--surface-0);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-sm);
    font: inherit;
    padding: 1px 4px;
    width: 12ch;
  }
  .tab-title-input:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
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
