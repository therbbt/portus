<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import type { SshConnectOptions, SftpDirEntry } from "../bridge";
  import {
    sftpConnect,
    sftpList,
    sftpReadFile,
    sftpWriteFile,
    sftpRemoveFile,
    sftpCreateDir,
    sftpRemoveDir,
    sftpDisconnect,
  } from "../bridge";
  import RingMark from "./RingMark.svelte";

  export let options: SshConnectOptions;
  export let title: string;

  const dispatch = createEventDispatcher<{ close: void }>();

  let sftpId: string | null = null;
  let currentPath = ".";
  let entries: SftpDirEntry[] = [];
  let loading = true;
  let error: string | null = null;
  let fileInput: HTMLInputElement;
  let newFolderOpen = false;
  let newFolderName = "";

  function joinPath(base: string, name: string): string {
    if (base === ".") return name;
    return base.endsWith("/") ? base + name : `${base}/${name}`;
  }

  function parentPath(path: string): string {
    if (path === ".") return ".";
    const idx = path.lastIndexOf("/");
    return idx <= 0 ? "." : path.slice(0, idx);
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB", "TB"];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit++;
    }
    return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
  }

  async function load(path: string) {
    if (!sftpId) return;
    loading = true;
    error = null;
    try {
      entries = await sftpList(sftpId, path);
      currentPath = path;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      sftpId = await sftpConnect(options);
      await load(".");
    } catch (e) {
      error = String(e);
      loading = false;
    }
  });

  onDestroy(() => {
    if (sftpId) void sftpDisconnect(sftpId);
  });

  function openEntry(entry: SftpDirEntry) {
    if (entry.isDir) void load(joinPath(currentPath, entry.name));
  }

  async function deleteEntry(entry: SftpDirEntry) {
    if (!sftpId) return;
    if (!confirm(`Delete ${entry.isDir ? "folder" : "file"} "${entry.name}"?`)) return;
    const path = joinPath(currentPath, entry.name);
    try {
      if (entry.isDir) {
        await sftpRemoveDir(sftpId, path);
      } else {
        await sftpRemoveFile(sftpId, path);
      }
      await load(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function downloadEntry(entry: SftpDirEntry) {
    if (!sftpId) return;
    try {
      const bytes = await sftpReadFile(sftpId, joinPath(currentPath, entry.name));
      const blob = new Blob([bytes.buffer as ArrayBuffer]);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = entry.name;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = String(e);
    }
  }

  function triggerUpload() {
    fileInput?.click();
  }

  async function handleFileInputChange() {
    if (!sftpId || !fileInput.files) return;
    try {
      for (const file of Array.from(fileInput.files)) {
        const bytes = new Uint8Array(await file.arrayBuffer());
        await sftpWriteFile(sftpId, joinPath(currentPath, file.name), bytes);
      }
      await load(currentPath);
    } catch (e) {
      error = String(e);
    } finally {
      fileInput.value = "";
    }
  }

  async function confirmNewFolder() {
    if (!sftpId || !newFolderName.trim()) return;
    try {
      await sftpCreateDir(sftpId, joinPath(currentPath, newFolderName.trim()));
      newFolderName = "";
      newFolderOpen = false;
      await load(currentPath);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="sftp-panel">
  <div class="panel-header">
    <div class="header-text">
      <span class="panel-title">Files</span>
      <span class="panel-subtitle">{title}</span>
    </div>
    <button class="icon-btn" aria-label="Close" on:click={() => dispatch("close")}>×</button>
  </div>

  <div class="toolbar">
    <button class="toolbar-btn" disabled={currentPath === "."} on:click={() => load(parentPath(currentPath))}>
      ↑ Up
    </button>
    <span class="path">{currentPath === "." ? "~" : currentPath}</span>
    <button class="toolbar-btn" on:click={() => (newFolderOpen = !newFolderOpen)}>+ Folder</button>
    <button class="toolbar-btn" on:click={triggerUpload}>↥ Upload</button>
    <input
      bind:this={fileInput}
      type="file"
      multiple
      class="hidden-input"
      on:change={handleFileInputChange}
    />
  </div>

  {#if newFolderOpen}
    <div class="new-folder-row">
      <input
        type="text"
        placeholder="Folder name"
        bind:value={newFolderName}
        on:keydown={(e) => e.key === "Enter" && confirmNewFolder()}
      />
      <button class="toolbar-btn" on:click={confirmNewFolder}>Create</button>
    </div>
  {/if}

  {#if error}
    <p class="error-text">{error}</p>
  {/if}

  <div class="entry-list">
    {#if loading}
      <div class="loading-state">
        <RingMark size={32} spinning />
      </div>
    {:else if entries.length === 0}
      <p class="empty-text">Empty directory</p>
    {:else}
      {#each entries as entry (entry.name)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="entry-row" class:dir={entry.isDir} on:dblclick={() => openEntry(entry)}>
          <span class="entry-icon">{entry.isDir ? "📁" : "📄"}</span>
          <button class="entry-name" on:click={() => openEntry(entry)}>{entry.name}</button>
          <span class="entry-size">{entry.isDir ? "" : formatSize(entry.size)}</span>
          {#if !entry.isDir}
            <span
              class="entry-action"
              role="button"
              tabindex="0"
              aria-label={`Download ${entry.name}`}
              on:click|stopPropagation={() => downloadEntry(entry)}
              on:keydown|stopPropagation={(e) => e.key === "Enter" && downloadEntry(entry)}
            >
              ↓
            </span>
          {/if}
          <span
            class="entry-action"
            role="button"
            tabindex="0"
            aria-label={`Delete ${entry.name}`}
            on:click|stopPropagation={() => deleteEntry(entry)}
            on:keydown|stopPropagation={(e) => e.key === "Enter" && deleteEntry(entry)}
          >
            ×
          </span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .sftp-panel {
    position: fixed;
    top: var(--titlebar-height);
    right: 0;
    bottom: 0;
    width: min(400px, 90vw);
    background: var(--surface-1);
    box-shadow: -16px 0 40px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    z-index: 900;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
  }
  .header-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .panel-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }
  .panel-subtitle {
    font-size: 0.68rem;
    color: var(--fg-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-btn {
    background: transparent;
    border: none;
    color: var(--fg-tertiary);
    font-size: 1.1rem;
    line-height: 1;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--surface-3);
    color: var(--fg-primary);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-4) var(--space-3);
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 0.72rem;
    color: var(--fg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .toolbar-btn {
    background: var(--surface-2);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.3rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .toolbar-btn:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .toolbar-btn:disabled {
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
  .hidden-input {
    display: none;
  }

  .new-folder-row {
    display: flex;
    gap: var(--space-2);
    padding: 0 var(--space-4) var(--space-3);
  }
  .new-folder-row input {
    flex: 1;
    background: var(--surface-2);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.3rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.75rem;
  }

  .error-text {
    margin: 0 var(--space-4) var(--space-3);
    font-size: 0.72rem;
    color: var(--status-error);
  }

  .entry-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--space-2) var(--space-3);
  }
  .loading-state {
    display: flex;
    justify-content: center;
    padding: var(--space-5);
  }
  .empty-text {
    text-align: center;
    color: var(--fg-tertiary);
    font-size: 0.75rem;
    padding: var(--space-5);
  }

  .entry-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0.3rem 0.5rem;
    border-radius: var(--radius-sm);
  }
  .entry-row:hover {
    background: var(--surface-2);
  }
  .entry-icon {
    flex-shrink: 0;
    font-size: 0.85rem;
  }
  .entry-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--fg-primary);
    font-size: 0.76rem;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0;
  }
  .entry-row.dir .entry-name {
    cursor: pointer;
  }
  .entry-size {
    flex-shrink: 0;
    font-size: 0.68rem;
    color: var(--fg-tertiary);
    width: 56px;
    text-align: right;
  }
  .entry-action {
    flex-shrink: 0;
    color: var(--fg-tertiary);
    padding: 0 4px;
    border-radius: var(--radius-sm);
    line-height: 1;
    opacity: 0;
  }
  .entry-row:hover .entry-action {
    opacity: 1;
  }
  .entry-action:hover {
    color: var(--fg-primary);
    background: var(--surface-4);
  }
</style>
