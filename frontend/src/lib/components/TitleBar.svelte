<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import RingMark from "./RingMark.svelte";
  import SplitMenu from "./SplitMenu.svelte";

  /** Disables the Split button when there's no active tab to split. */
  export let splitDisabled = false;

  const appWindow = getCurrentWindow();
  const dispatch = createEventDispatcher<{
    splitRow: void;
    splitColumn: void;
    showShortcuts: void;
    showSettings: void;
  }>();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="titlebar" data-tauri-drag-region on:dblclick={() => appWindow.toggleMaximize()}>
  <div class="brand" data-tauri-drag-region>
    <RingMark size={14} color="var(--fg-secondary)" />
    <span data-tauri-drag-region>Portus</span>
  </div>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="toolbar-group">
    <SplitMenu disabled={splitDisabled} on:splitRow={() => dispatch("splitRow")} on:splitColumn={() => dispatch("splitColumn")} />
    <button class="toolbar-btn" aria-label="Keyboard shortcuts" on:click={() => dispatch("showShortcuts")}>
      <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round">
        <circle cx="8" cy="8" r="6.5" />
        <path d="M6.1 6.2a1.9 1.9 0 1 1 2.7 1.7c-.7.35-.9.7-.9 1.4" stroke-linejoin="round" />
        <circle cx="8" cy="11.4" r="0.15" fill="currentColor" />
      </svg>
      <span>Shortcuts</span>
    </button>
    <button class="toolbar-btn" aria-label="Settings" on:click={() => dispatch("showSettings")}>
      <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="8" cy="8" r="2.2" />
        <path d="M8 2v1.6M8 12.4V14M14 8h-1.6M3.6 8H2M12.13 3.87l-1.13 1.13M4.99 11.01l-1.13 1.13M12.13 12.13l-1.13-1.13M4.99 4.99 3.87 3.87" />
      </svg>
      <span>Settings</span>
    </button>
  </div>
  <div class="window-controls">
    <button class="win-btn" aria-label="Minimize" on:click={() => appWindow.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><line x1="1" y1="9" x2="9" y2="9" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="win-btn" aria-label="Maximize" on:click={() => appWindow.toggleMaximize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" stroke="currentColor" stroke-width="1.2" fill="none" /></svg>
    </button>
    <button class="win-btn close" aria-label="Close" on:click={() => appWindow.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10">
        <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.2" />
        <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: var(--titlebar-height);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    background: var(--surface-1);
    border-bottom: 1px solid var(--hairline);
    -webkit-app-region: drag;
    user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.6rem;
    color: var(--fg-secondary);
    font-size: 0.72rem;
    font-weight: 500;
  }
  .spacer {
    flex: 1;
  }
  .toolbar-group {
    -webkit-app-region: no-drag;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 0.3rem;
  }
  /* Same shape/tier as NewConnectionMenu's and SplitMenu's .toolbar-btn —
     these two are plain actions rather than dropdowns, so no caret. */
  .toolbar-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    height: 22px;
    border: none;
    border-radius: var(--radius-sm);
    padding: 0 0.4rem;
    background: transparent;
    color: var(--fg-secondary);
    font-size: 0.8rem;
    line-height: 1;
    cursor: pointer;
  }
  .toolbar-btn:hover {
    background: var(--surface-2);
    color: var(--fg-primary);
  }
  .window-controls {
    display: flex;
    height: 100%;
  }
  .win-btn {
    -webkit-app-region: no-drag;
    width: 40px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--fg-secondary);
    cursor: pointer;
  }
  .win-btn:hover {
    background: var(--surface-3);
    color: var(--fg-primary);
  }
  .win-btn.close:hover {
    background: #ef4444;
    color: #fff;
  }
</style>
