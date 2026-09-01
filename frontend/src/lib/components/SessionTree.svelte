<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";
  import type { ContextMenuItem } from "./ContextMenu.svelte";
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

  // Folders can nest in the data model (Group.parentId), but nothing in the
  // UI creates a nested one yet — only top-level folders are rendered, same
  // as FlashPad's folders-of-notes before you factor in sub-notes.
  $: rootGroups = groups.filter((g) => !g.parentId).sort((a, b) => a.name.localeCompare(b.name));
  $: ungroupedSessions = sessions.filter((s) => !s.groupId);
  const sessionsIn = (groupId: string, allSessions: SavedSession[]) => allSessions.filter((s) => s.groupId === groupId);

  let creatingFolder = false;
  let newFolderName = "";

  function startCreateFolder() {
    creatingFolder = true;
    newFolderName = "";
  }

  function commitCreateFolder() {
    const name = newFolderName.trim();
    creatingFolder = false;
    if (!name) return;
    dispatch("createFolder", { name });
  }

  function cancelCreateFolder() {
    creatingFolder = false;
  }

  let renamingGroupId: string | null = null;
  let renameValue = "";

  function startRenameFolder(group: Group) {
    renamingGroupId = group.id;
    renameValue = group.name;
  }

  function commitRenameFolder(id: string) {
    if (renamingGroupId !== id) return;
    renamingGroupId = null;
    const name = renameValue.trim();
    if (!name) return;
    dispatch("renameFolder", { id, name });
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
    <ul class="session-list">
      {#if creatingFolder}
        <li class="folder-row">
          <span class="chevron-spacer"></span>
          <!-- No folder icon yet here — pairing that (fairly saturated
               orange) icon with the input's own accent border made naming
               a brand-new folder read as two strong colors firing at once.
               The icon shows up once the folder actually exists (the
               {#each rootGroups} row below this one). -->
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
      {#each rootGroups as group (group.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <li class="folder-row" on:contextmenu|preventDefault|stopPropagation={(e) => openFolderMenu(e, group)}>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="chevron-btn" on:click={() => dispatch("toggleFolder", group)}>
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
                  commitRenameFolder(group.id);
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  cancelRenameFolder();
                }
              }}
              on:blur={() => commitRenameFolder(group.id)}
            />
          {:else}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span class="folder-name" on:click={() => dispatch("toggleFolder", group)} on:dblclick|stopPropagation={() => startRenameFolder(group)}>
              {group.name}
            </span>
          {/if}
        </li>
        {#if !group.collapsed}
          {#each sessionsIn(group.id, sessions) as session (session.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <li class="session-row nested" on:contextmenu|preventDefault|stopPropagation={(e) => openSessionMenu(e, session)}>
              <button class="session-main" title={`${protocolLabel[session.protocol]} · ${session.address}`} on:click={() => dispatch("connect", session)}>
                <svg class="session-icon" width="15" height="15" viewBox="0 0 16 16" fill="currentColor">
                  <rect x="1" y="1.5" width="9.5" height="7" rx="1" />
                  <rect x="4.5" y="9" width="2.5" height="1" rx="0.3" />
                  <circle cx="12.3" cy="8.7" r="1.7" />
                  <path d="M9 14.8a3.3 3.3 0 0 1 6.6 0Z" />
                </svg>
                <span class="session-name">{session.name}</span>
              </button>
            </li>
          {/each}
        {/if}
      {/each}
      {#each ungroupedSessions as session (session.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <li class="session-row" on:contextmenu|preventDefault|stopPropagation={(e) => openSessionMenu(e, session)}>
          <button class="session-main" title={`${protocolLabel[session.protocol]} · ${session.address}`} on:click={() => dispatch("connect", session)}>
            <span class="session-name">{session.name}</span>
          </button>
        </li>
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
  .session-row.nested {
    /* Matches FlashPad's TreeNode indent step (depth * 14px, roughly 30px
       for a leaf one level in) rather than Portus's previous, noticeably
       shallower 1.1rem. */
    padding-left: 1.75rem;
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
  .chevron-spacer {
    flex-shrink: 0;
    width: 14px;
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
