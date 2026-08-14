<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";
  import type { Host, Group } from "../bridge";

  export let hosts: Host[] = [];
  export let groups: Group[] = [];
  export let width = 260;

  const dispatch = createEventDispatcher<{
    connect: Host;
    deleteHost: Host;
    editHost: Host;
    toggleFolder: Group;
    createFolder: { name: string };
    renameFolder: { id: string; name: string };
    deleteFolder: Group;
  }>();

  // "echo" is a debug-only session kind, never a real saved host's
  // protocol — included here only so this satisfies the shared `Protocol` type.
  const protocolLabel: Record<Host["protocol"], string> = {
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
  $: ungroupedHosts = hosts.filter((h) => !h.groupId);
  const hostsIn = (groupId: string, allHosts: Host[]) => allHosts.filter((h) => h.groupId === groupId);

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
</script>

<aside class="rail" style="width: {width}px; min-width: {width}px">
  {#if hosts.length === 0 && groups.length === 0 && !creatingFolder}
    <div class="empty-state">
      <RingMark size={40} />
      <p class="empty-title">No saved hosts yet</p>
      <p class="empty-subtitle">Save a connection from the SSH or serial dialog and it'll show up here.</p>
    </div>
  {:else}
    <ul class="host-list">
      {#if creatingFolder}
        <li class="folder-row">
          <span class="chevron-spacer"></span>
          <svg class="folder-icon" width="14" height="14" viewBox="0 0 16 16">
            <path fill="currentColor" d="M1.5 3A1.5 1.5 0 0 1 3 1.5h3.17a1.5 1.5 0 0 1 1.06.44l.83.82H13A1.5 1.5 0 0 1 14.5 4.26V12.5A1.5 1.5 0 0 1 13 14H3a1.5 1.5 0 0 1-1.5-1.5V3Z" />
          </svg>
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
        <li class="folder-row">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="chevron-btn" on:click={() => dispatch("toggleFolder", group)}>
            <svg class="chevron" class:open={!group.collapsed} width="10" height="10" viewBox="0 0 10 10">
              <path d="M3 1 L7 5 L3 9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </span>
          <svg class="folder-icon" width="14" height="14" viewBox="0 0 16 16">
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
          <span
            class="row-action"
            role="button"
            tabindex="0"
            aria-label={`Rename ${group.name}`}
            title={`Rename ${group.name}`}
            on:click|stopPropagation={() => startRenameFolder(group)}
            on:keydown|stopPropagation={(e) => e.key === "Enter" && startRenameFolder(group)}
          >
            <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
              <path d="M11 2l3 3-8 8-3.5.5.5-3.5z" />
            </svg>
          </span>
          <span
            class="row-action"
            role="button"
            tabindex="0"
            aria-label={`Delete ${group.name}`}
            title={`Delete ${group.name} (hosts inside move to the root)`}
            on:click|stopPropagation={() => dispatch("deleteFolder", group)}
            on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("deleteFolder", group)}
          >
            ×
          </span>
        </li>
        {#if !group.collapsed}
          {#each hostsIn(group.id, hosts) as host (host.id)}
            <li class="host-row nested">
              <button class="host-main" on:click={() => dispatch("connect", host)}>
                <span class="host-name">{host.name}</span>
                <span class="host-meta">{protocolLabel[host.protocol]} · {host.address}</span>
              </button>
              {#if host.protocol === "ssh" || host.protocol === "serial"}
                <span
                  class="row-action"
                  role="button"
                  tabindex="0"
                  aria-label={`Edit ${host.name}`}
                  title={`Edit ${host.name}`}
                  on:click|stopPropagation={() => dispatch("editHost", host)}
                  on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("editHost", host)}
                >
                  <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M11 2l3 3-8 8-3.5.5.5-3.5z" />
                  </svg>
                </span>
              {/if}
              <span
                class="row-action"
                role="button"
                tabindex="0"
                aria-label={`Delete ${host.name}`}
                title={`Delete ${host.name}`}
                on:click|stopPropagation={() => dispatch("deleteHost", host)}
                on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("deleteHost", host)}
              >
                ×
              </span>
            </li>
          {/each}
        {/if}
      {/each}
      {#each ungroupedHosts as host (host.id)}
        <li class="host-row">
          <button class="host-main" on:click={() => dispatch("connect", host)}>
            <span class="host-name">{host.name}</span>
            <span class="host-meta">{protocolLabel[host.protocol]} · {host.address}</span>
          </button>
          {#if host.protocol === "ssh" || host.protocol === "serial"}
            <span
              class="row-action"
              role="button"
              tabindex="0"
              aria-label={`Edit ${host.name}`}
              title={`Edit ${host.name}`}
              on:click|stopPropagation={() => dispatch("editHost", host)}
              on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("editHost", host)}
            >
              <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                <path d="M11 2l3 3-8 8-3.5.5.5-3.5z" />
              </svg>
            </span>
          {/if}
          <span
            class="row-action"
            role="button"
            tabindex="0"
            aria-label={`Delete ${host.name}`}
            title={`Delete ${host.name}`}
            on:click|stopPropagation={() => dispatch("deleteHost", host)}
            on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("deleteHost", host)}
          >
            ×
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="rail-footer">
    <button class="new-shell-btn" on:click={startCreateFolder}>
      + Folder
    </button>
  </div>
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
  .host-list {
    list-style: none;
    margin: 0;
    /* Top padding for breathing room now that the rail has no header of its
       own — that job moved to App.svelte's unified action bar. */
    padding: var(--space-2) var(--space-2) 0;
    overflow-y: auto;
    flex: 1;
  }
  .host-row,
  .folder-row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .host-row:hover,
  .folder-row:hover {
    background: var(--surface-3);
  }
  .host-row.nested {
    padding-left: 1.1rem;
  }
  .host-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 0.3rem 0.4rem;
    background: transparent;
    border: none;
    color: var(--fg-primary);
    cursor: pointer;
    text-align: left;
  }
  .host-name {
    font-size: 0.78rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .host-meta {
    font-size: 0.66rem;
    color: var(--fg-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .folder-row {
    padding: 0.3rem 0.4rem;
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
    color: var(--fg-tertiary);
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
  .row-action {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 0.4rem;
    color: var(--fg-tertiary);
    opacity: 0;
    border-radius: var(--radius-sm);
    line-height: 1;
  }
  .host-row:hover .row-action,
  .folder-row:hover .row-action {
    opacity: 1;
  }
  .row-action:hover {
    color: var(--fg-primary);
    background: var(--surface-4);
  }
  .rename-input {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    font-family: inherit;
    background: var(--surface-0);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-sm);
    padding: 1px 4px;
  }
  .rename-input:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .rail-footer {
    /* Matches FlashPad's sidebar Footer padding exactly (0.5rem 0.9rem) —
       this is Portus's structural equivalent (bottom-of-sidebar bar), just
       stacked instead of a single row since it holds several action
       buttons instead of a search box. */
    padding: 0.5rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .new-shell-btn {
    width: 100%;
    padding: 0.35rem 0.6rem;
    background: var(--surface-2);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.75rem;
    text-align: left;
  }
  .new-shell-btn:hover {
    background: var(--surface-3);
  }
  .new-shell-btn:active {
    background: var(--surface-4);
  }
</style>
