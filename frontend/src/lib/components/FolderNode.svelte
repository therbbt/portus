<script context="module" lang="ts">
  export type DropZone = "before" | "inside" | "after";
  export type DropTarget = { kind: "session" | "group" | "root"; id: string; zone: DropZone };

  export interface FolderNodeActions {
    toggleFolder(group: Group): void;
    openFolderMenu(event: MouseEvent, group: Group): void;
    startRenameFolder(group: Group): void;
    commitRenameFolder(id: string, name: string): void;
    cancelRenameFolder(): void;
    connect(session: SavedSession): void;
    openSessionMenu(event: MouseEvent, session: SavedSession): void;
    startDragSession(event: DragEvent, session: SavedSession): void;
    startDragGroup(event: DragEvent, group: Group): void;
    endDrag(): void;
    dragOverSession(event: DragEvent, session: SavedSession): void;
    dragOverGroup(event: DragEvent, group: Group): void;
    drop(event: DragEvent): void;
  }
</script>

<script lang="ts">
  import type { SavedSession, Group } from "../bridge";

  // Recursive tree node: one folder row, plus (unless collapsed) its own
  // child folders via <svelte:self> and its own direct sessions. Mirrors
  // FlashPad's TreeNode.svelte structure - a callback-bag prop (`actions`)
  // rather than dispatched events, since a custom event only reaches this
  // component's direct parent, not further up through every recursion
  // level the way a plain function call does.
  export let group: Group;
  export let allGroups: Group[];
  export let allSessions: SavedSession[];
  export let depth: number;
  export let renamingGroupId: string | null;
  export let draggingSessionId: string | null;
  export let draggingGroupId: string | null;
  export let dropTarget: DropTarget | null;
  export let protocolLabel: Record<SavedSession["protocol"], string>;
  export let focusAndSelect: (node: HTMLInputElement) => void;
  export let actions: FolderNodeActions;

  let renameValue = group.name;
  $: if (renamingGroupId === group.id) renameValue = group.name;

  $: childFolders = allGroups.filter((g) => g.parentId === group.id).sort((a, b) => a.sortOrder - b.sortOrder);
  $: childSessions = allSessions.filter((s) => s.groupId === group.id).sort((a, b) => a.sortOrder - b.sortOrder);

  const BASE_INDENT_REM = 0.4;
  const INDENT_STEP_REM = 1.1;
  $: folderIndent = `${BASE_INDENT_REM + depth * INDENT_STEP_REM}rem`;
  $: sessionIndent = `${BASE_INDENT_REM + (depth + 1) * INDENT_STEP_REM}rem`;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<li
  class="folder-row"
  style="padding-left: {folderIndent}"
  class:dragging={draggingGroupId === group.id}
  class:drop-before={dropTarget?.kind === "group" && dropTarget.id === group.id && dropTarget.zone === "before"}
  class:drop-inside={dropTarget?.kind === "group" && dropTarget.id === group.id && dropTarget.zone === "inside"}
  class:drop-after={dropTarget?.kind === "group" && dropTarget.id === group.id && dropTarget.zone === "after"}
  draggable="true"
  on:dragstart={(e) => actions.startDragGroup(e, group)}
  on:dragend={actions.endDrag}
  on:dragover={(e) => actions.dragOverGroup(e, group)}
  on:drop={actions.drop}
  on:contextmenu|preventDefault|stopPropagation={(e) => actions.openFolderMenu(e, group)}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="chevron-btn" on:click={() => actions.toggleFolder(group)}>
    <svg class="chevron" class:open={!group.collapsed} width="10" height="10" viewBox="0 0 10 10">
      <path d="M3 1 L7 5 L3 9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </span>
  <svg class="folder-icon" width="17" height="17" viewBox="0 0 16 16">
    <path fill="currentColor" d="M1.5 3A1.5 1.5 0 0 1 3 1.5h3.17a1.5 1.5 0 0 1 1.06.44l.83.82H13A1.5 1.5 0 0 1 14.5 4.26V12.5A1.5 1.5 0 0 1 13 14H3a1.5 1.5 0 0 1-1.5-1.5V3Z" />
  </svg>
  {#if renamingGroupId === group.id}
    <input
      class="rename-input"
      bind:value={renameValue}
      use:focusAndSelect
      on:click|stopPropagation
      on:keydown|stopPropagation={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          actions.commitRenameFolder(group.id, renameValue);
        } else if (e.key === "Escape") {
          e.preventDefault();
          actions.cancelRenameFolder();
        }
      }}
      on:blur={() => actions.commitRenameFolder(group.id, renameValue)}
    />
  {:else}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span class="folder-name" on:click={() => actions.toggleFolder(group)} on:dblclick|stopPropagation={() => actions.startRenameFolder(group)}>
      {group.name}
    </span>
  {/if}
</li>
{#if !group.collapsed}
  {#each childFolders as child (child.id)}
    <svelte:self
      group={child}
      {allGroups}
      {allSessions}
      depth={depth + 1}
      {renamingGroupId}
      {draggingSessionId}
      {draggingGroupId}
      {dropTarget}
      {protocolLabel}
      {focusAndSelect}
      {actions}
    />
  {/each}
  {#each childSessions as session (session.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <li
      class="session-row"
      style="padding-left: {sessionIndent}"
      class:dragging={draggingSessionId === session.id}
      class:drop-before={dropTarget?.kind === "session" && dropTarget.id === session.id && dropTarget.zone === "before"}
      class:drop-after={dropTarget?.kind === "session" && dropTarget.id === session.id && dropTarget.zone === "after"}
      draggable="true"
      on:dragstart={(e) => actions.startDragSession(e, session)}
      on:dragend={actions.endDrag}
      on:dragover={(e) => actions.dragOverSession(e, session)}
      on:drop={actions.drop}
      on:contextmenu|preventDefault|stopPropagation={(e) => actions.openSessionMenu(e, session)}
    >
      <button class="session-main" title={`${protocolLabel[session.protocol]} · ${session.address}`} on:click={() => actions.connect(session)}>
        <svg class="session-icon" width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
          <path d="M4.5 6.5L7 9L4.5 11.5" />
          <line x1="8.5" y1="11.5" x2="11.5" y2="11.5" />
        </svg>
        <span class="session-name">{session.name}</span>
      </button>
    </li>
  {/each}
{/if}

<style>
  .session-row,
  .folder-row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .session-row:hover,
  .folder-row:hover {
    background: var(--surface-2);
  }
  .session-row.dragging,
  .folder-row.dragging {
    opacity: 0.5;
  }
  .session-row.drop-before,
  .folder-row.drop-before {
    box-shadow: inset 0 2px 0 0 var(--accent);
  }
  .session-row.drop-after,
  .folder-row.drop-after {
    box-shadow: inset 0 -2px 0 0 var(--accent);
  }
  .folder-row.drop-inside {
    background: var(--surface-2);
    outline: 1px dashed var(--accent);
    outline-offset: -1px;
  }
  .session-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.22rem 0.4rem;
    background: transparent;
    border: none;
    color: var(--fg-primary);
    cursor: pointer;
    text-align: left;
  }
  .session-icon {
    flex-shrink: 0;
    color: var(--fg-secondary);
  }
  .session-name {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .folder-row {
    padding-top: 0.22rem;
    padding-bottom: 0.22rem;
    padding-right: 0.4rem;
    gap: 0.35rem;
    cursor: pointer;
    user-select: none;
  }
  .chevron-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    margin: -2px;
    padding: 2px;
  }
  .chevron {
    flex-shrink: 0;
    color: var(--fg-secondary);
    transition: transform 0.1s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .folder-icon {
    flex-shrink: 0;
    color: #e8a33d;
  }
  .folder-name {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The chevron/name controls on a folder row are click targets, not text
     inputs — the global accent focus ring (tokens.css's *:focus-visible)
     reads as an unexpected flash of green on a click here rather than
     useful keyboard-nav feedback. The rename input right below keeps its
     own ring since typing feedback there IS useful. */
  .folder-row .chevron-btn:focus-visible,
  .folder-row .folder-name:focus-visible {
    outline: none;
    box-shadow: none;
  }
  .rename-input {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    font-family: inherit;
    background: transparent;
    color: var(--fg-primary);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 0 0.2rem;
  }
  .rename-input:focus-visible {
    outline: none;
    box-shadow: none;
  }
</style>
