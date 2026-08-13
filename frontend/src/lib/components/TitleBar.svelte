<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import RingMark from "./RingMark.svelte";

  const appWindow = getCurrentWindow();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="titlebar" data-tauri-drag-region on:dblclick={() => appWindow.toggleMaximize()}>
  <div class="brand" data-tauri-drag-region>
    <RingMark size={14} color="var(--fg-secondary)" />
    <span data-tauri-drag-region>Portus</span>
  </div>
  <div class="spacer" data-tauri-drag-region></div>
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
    background: #e5484d;
    color: #fff;
  }
</style>
