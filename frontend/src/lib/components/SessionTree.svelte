<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";
  import type { ContextMenuItem } from "./ContextMenu.svelte";
  import FolderNode, { type DropTarget, type DropZone, type FolderNodeActions, type TreeEntry, mergeEntries } from "./FolderNode.svelte";
  import type { SavedSession, Group } from "../bridge";

  export let sessions: SavedSession[] = [];
  export let groups: Group[] = [];
  export let width = 260;

  const dispatch = createEventDispatcher<{
    connect: SavedSession;
    deleteSession: SavedSession;
    editSession: SavedSession;
    toggleFolder: Group;
    createFolder: { name: string };
    renameFolder: { id: string; name: string };
    deleteFolder: Group;
    // The menu itself renders from App.svelte, same as every other overlay
    // in this app (connect dialogs, Settings) — a fixed-position popup
    // shouldn't be nested inside a flex-item component's own render tree.
    openContextMenu: { x: number; y: number; items: ContextMenuItem[] };
    reorderSession: { id: string; groupId: string | null; sortOrder: number };
    reorderGroup: { id: string; parentId: string | null; sortOrder: number };
  }>();

  // "echo" is a debug-only session kind, never a real saved session's
  // protocol — included here only so this satisfies the shared `Protocol` type.
  const protocolLabel: Record<SavedSession["protocol"], string> = {
    ssh: "SSH",
    serial: "Serial",
    shell: "Shell",
    telnet: "Telnet",
    rdp: "RDP",
    echo: "Echo",
  };

  // Folders can nest arbitrarily deep (Group.parentId) — rendered via
  // FolderNode.svelte's own <svelte:self> recursion below. A folder and a
  // loose session can sit side by side at the same level in whatever order
  // dragging put them in, rather than every folder always rendering before
  // any session — see mergeEntries().
  $: rootEntries = mergeEntries(groups, sessions, null);

  let creatingFolder = false;
  let newFolderName = "";

  function startCreateFolder() {
    creatingFolder = true;
    newFolderName = "";
  }

  function commitCreateFolder() {
    // Without this guard, pressing Enter commits once from the keydown
    // handler below, which sets creatingFolder=false and removes the
    // input from the DOM - removing a focused element fires a native
    // blur, so on:blur then commits a second time too. Some browser
    // engines (WebView2 on Windows, apparently, not WebKitGTK on Linux)
    // fire that blur synchronously enough for both to actually go
    // through, dispatching "createFolder" twice for one Enter press.
    if (!creatingFolder) return;
    const name = newFolderName.trim();
    creatingFolder = false;
    if (!name) return;
    dispatch("createFolder", { name });
  }

  function cancelCreateFolder() {
    creatingFolder = false;
  }

  let renamingGroupId: string | null = null;

  function startRenameFolder(group: Group) {
    renamingGroupId = group.id;
  }

  function commitRenameFolder(id: string, name: string) {
    if (renamingGroupId !== id) return;
    renamingGroupId = null;
    const trimmed = name.trim();
    if (!trimmed) return;
    dispatch("renameFolder", { id, name: trimmed });
  }

  function cancelRenameFolder() {
    renamingGroupId = null;
  }

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function openBackgroundMenu(event: MouseEvent) {
    dispatch("openContextMenu", {
      x: event.clientX,
      y: event.clientY,
      items: [{ label: "New folder", action: startCreateFolder }],
    });
  }

  function openFolderMenu(event: MouseEvent, group: Group) {
    dispatch("openContextMenu", {
      x: event.clientX,
      y: event.clientY,
      items: [
        { label: "New folder", action: startCreateFolder },
        { label: "", separator: true },
        { label: "Rename", action: () => startRenameFolder(group) },
        { label: "Delete", danger: true, action: () => dispatch("deleteFolder", group) },
      ],
    });
  }

  function openSessionMenu(event: MouseEvent, session: SavedSession) {
    // RDP has no edit dialog yet (see NewSessionDialog.svelte) — a saved
    // RDP session can only be deleted and re-created, not edited in place.
    const canEdit = session.protocol === "ssh" || session.protocol === "serial" || session.protocol === "shell";
    dispatch("openContextMenu", {
      x: event.clientX,
      y: event.clientY,
      items: [
        ...(canEdit ? [{ label: "Edit", action: () => dispatch("editSession", session) }, { label: "", separator: true }] : []),
        { label: "Delete", danger: true, action: () => dispatch("deleteSession", session) },
      ],
    });
  }

  // --- Drag and drop ---------------------------------------------------
  // Mirrors FlashPad's TreeNode.svelte pattern (drop zone = before/inside/
  // after based on cursor position within the target row). A session row
  // only ever accepts before/after (reordering among its current
  // siblings) — a folder's "inside" zone is what files a session into it.
  // A folder row accepts all three: before/after reorders among its
  // current siblings, "inside" reparents it under the target folder.

  let draggingSession: SavedSession | null = null;
  let draggingGroup: Group | null = null;
  let dropTarget: DropTarget | null = null;

  function zoneFromEvent(event: DragEvent, allowInside: boolean): DropZone {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = (event.clientY - rect.top) / rect.height;
    if (!allowInside) return ratio < 0.5 ? "before" : "after";
    return ratio < 0.25 ? "before" : ratio > 0.75 ? "after" : "inside";
  }

  function sortOrderBetween(list: Array<{ sortOrder: number }>, targetIndex: number, zone: "before" | "after"): number {
    if (zone === "before") {
      const prev = list[targetIndex - 1];
      const target = list[targetIndex];
      return prev ? (prev.sortOrder + target.sortOrder) / 2 : target.sortOrder - 1;
    }
    const target = list[targetIndex];
    const next = list[targetIndex + 1];
    return next ? (target.sortOrder + next.sortOrder) / 2 : target.sortOrder + 1;
  }

  function appendSortOrder(list: Array<{ sortOrder: number }>): number {
    return list.length ? Math.max(...list.map((item) => item.sortOrder)) + 1 : 0;
  }

  /** Is `candidateId` equal to, or nested somewhere inside, `ancestorId`?
   * Guards against dropping a folder into itself or one of its own
   * descendants, which would otherwise create a cycle. */
  function isDescendantOrSelf(candidateId: string, ancestorId: string): boolean {
    let current: Group | undefined = groups.find((g) => g.id === candidateId);
    while (current) {
      if (current.id === ancestorId) return true;
      current = current.parentId ? groups.find((g) => g.id === current!.parentId) : undefined;
    }
    return false;
  }

  function startDragSession(event: DragEvent, session: SavedSession) {
    draggingSession = session;
    draggingGroup = null;
    event.dataTransfer?.setData("text/plain", session.id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function startDragGroup(event: DragEvent, group: Group) {
    draggingGroup = group;
    draggingSession = null;
    event.dataTransfer?.setData("text/plain", group.id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function endDrag() {
    draggingSession = null;
    draggingGroup = null;
    dropTarget = null;
  }

  function dragOverSession(event: DragEvent, session: SavedSession) {
    if (draggingSession?.id === session.id) return;
    if (!draggingSession && !draggingGroup) return;
    event.preventDefault();
    event.stopPropagation();
    // A session never has an "inside" zone - it can't contain anything -
    // but a dragged folder can still land before/after it as a sibling.
    dropTarget = { kind: "session", id: session.id, zone: zoneFromEvent(event, false) };
  }

  function dragOverGroup(event: DragEvent, group: Group) {
    if (draggingGroup) {
      if (draggingGroup.id === group.id) return;
      if (isDescendantOrSelf(group.id, draggingGroup.id)) return;
    } else if (!draggingSession) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    // Both a dragged session and a dragged folder can go "inside" now.
    dropTarget = { kind: "group", id: group.id, zone: zoneFromEvent(event, true) };
  }

  function dragOverRoot(event: DragEvent) {
    if (!draggingSession && !draggingGroup) return;
    event.preventDefault();
    dropTarget = { kind: "root", id: "__root__", zone: "after" };
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    const target = dropTarget;
    const session = draggingSession;
    const group = draggingGroup;
    endDrag();
    if (!target) return;

    const dispatchMove = (parentId: string | null, sortOrder: number) => {
      if (group && parentId !== null && isDescendantOrSelf(parentId, group.id)) return; // would create a cycle
      if (session) dispatch("reorderSession", { id: session.id, groupId: parentId, sortOrder });
      else if (group) dispatch("reorderGroup", { id: group.id, parentId, sortOrder });
    };

    if (target.kind === "root") {
      dispatchMove(null, appendSortOrder(mergeEntries(groups, sessions, null).map((e) => e.item)));
      return;
    }

    if (target.kind === "group" && target.zone === "inside") {
      const targetGroup = groups.find((g) => g.id === target.id);
      if (!targetGroup) return;
      dispatchMove(targetGroup.id, appendSortOrder(mergeEntries(groups, sessions, targetGroup.id).map((e) => e.item)));
      return;
    }

    // Before/after a session or a folder row: become a sibling at that
    // target's own level, positioned relative to it - a folder and a
    // session dropped near each other both just mean "put me here."
    const targetParentId =
      target.kind === "session" ? (sessions.find((s) => s.id === target.id)?.groupId ?? null) : (groups.find((g) => g.id === target.id)?.parentId ?? null);
    const siblings = mergeEntries(groups, sessions, targetParentId).map((e) => e.item);
    const idx = siblings.findIndex((item) => item.id === target.id);
    if (idx === -1) return;
    const zone = target.zone === "inside" ? "after" : target.zone; // "inside" only ever reachable on a session target
    dispatchMove(targetParentId, sortOrderBetween(siblings, idx, zone));
  }

  const actions: FolderNodeActions = {
    toggleFolder: (group) => dispatch("toggleFolder", group),
    openFolderMenu,
    startRenameFolder,
    commitRenameFolder,
    cancelRenameFolder,
    connect: (session) => dispatch("connect", session),
    openSessionMenu,
    startDragSession,
    startDragGroup,
    endDrag,
    dragOverSession,
    dragOverGroup,
    drop: handleDrop,
  };
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<aside class="rail" style="width: {width}px; min-width: {width}px" on:contextmenu|preventDefault={openBackgroundMenu}>
  {#if sessions.length === 0 && groups.length === 0 && !creatingFolder}
    <div class="empty-state">
      <RingMark size={40} />
      <p class="empty-title">No saved sessions yet</p>
      <p class="empty-subtitle">Save a connection from the SSH or serial dialog and it'll show up here.</p>
    </div>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <ul
      class="session-list"
      class:drop-root={dropTarget?.kind === "root"}
      on:dragover={dragOverRoot}
      on:drop={handleDrop}
      on:dragleave={() => {
        if (dropTarget?.kind === "root") dropTarget = null;
      }}
    >
      {#if creatingFolder}
        <li class="folder-row">
          <span class="chevron-spacer"></span>
          <!-- No folder icon yet here — pairing that (fairly saturated
               orange) icon with the input's own accent border made naming
               a brand-new folder read as two strong colors firing at once.
               The icon shows up once the folder actually exists. -->
          <input
            class="rename-input"
            bind:value={newFolderName}
            placeholder="Folder name"
            use:focusAndSelect
            on:keydown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                commitCreateFolder();
              } else if (e.key === "Escape") {
                e.preventDefault();
                cancelCreateFolder();
              }
            }}
            on:blur={commitCreateFolder}
          />
        </li>
      {/if}
      {#each rootEntries as entry (entry.item.id)}
        {#if entry.kind === "group"}
          <FolderNode
            group={entry.item}
            allGroups={groups}
            allSessions={sessions}
            depth={0}
            {renamingGroupId}
            draggingSessionId={draggingSession?.id ?? null}
            draggingGroupId={draggingGroup?.id ?? null}
            {dropTarget}
            {protocolLabel}
            {focusAndSelect}
            {actions}
          />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <li
            class="session-row"
            class:dragging={draggingSession?.id === entry.item.id}
            class:drop-before={dropTarget?.kind === "session" && dropTarget.id === entry.item.id && dropTarget.zone === "before"}
            class:drop-after={dropTarget?.kind === "session" && dropTarget.id === entry.item.id && dropTarget.zone === "after"}
            draggable="true"
            on:dragstart={(e) => startDragSession(e, entry.item)}
            on:dragend={endDrag}
            on:dragover={(e) => dragOverSession(e, entry.item)}
            on:drop={handleDrop}
            on:contextmenu|preventDefault|stopPropagation={(e) => openSessionMenu(e, entry.item)}
          >
            <button class="session-main" title={`${protocolLabel[entry.item.protocol]} · ${entry.item.address}`} on:click={() => dispatch("connect", entry.item)}>
              <svg class="session-icon" width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
                <path d="M4.5 6.5L7 9L4.5 11.5" />
                <line x1="8.5" y1="11.5" x2="11.5" y2="11.5" />
              </svg>
              <span class="session-name">{entry.item.name}</span>
            </button>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .rail {
    flex-shrink: 0;
    background: var(--surface-1);
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-5);
    opacity: 0.85;
  }
  .empty-title {
    margin: var(--space-2) 0 0;
    color: var(--fg-secondary);
    font-size: 13px;
  }
  .empty-subtitle {
    margin: 0;
    color: var(--fg-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .session-list {
    list-style: none;
    margin: 0;
    /* Top padding for breathing room now that the rail has no header of its
       own — that job moved to App.svelte's unified action bar. */
    padding: var(--space-2) var(--space-2) 0;
    overflow-y: auto;
    flex: 1;
  }
  /* Dropping in the empty space below the last row un-files a session (or
     appends a folder) to the root — a faint inset line along the whole
     list edge is the only feedback needed since there's no specific row
     to highlight. */
  .session-list.drop-root {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
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
  .session-row.dragging {
    opacity: 0.5;
  }
  .session-row.drop-before {
    box-shadow: inset 0 2px 0 0 var(--accent);
  }
  .session-row.drop-after {
    box-shadow: inset 0 -2px 0 0 var(--accent);
  }
  .session-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    /* Matches folder-row's padding now that this is a single line like
       folder rows are — the protocol/address that used to sit below the
       name as a second line is now a hover tooltip instead (see the
       button's title attribute). */
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
    padding: 0.22rem 0.4rem;
    gap: 0.35rem;
    cursor: pointer;
    user-select: none;
  }
  .chevron-spacer {
    flex-shrink: 0;
    width: 14px;
  }
  .rename-input {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    font-family: inherit;
    /* No fill of its own — a --surface-1 background reliably looked like a
       mismatched patch against the row's own state (transparent normally,
       --surface-2 on hover), which is exactly the "background color that
       just happens" this was fixing. The border alone is the "you're
       editing" signal, same as FlashPad's version. */
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
