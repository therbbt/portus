<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let disabled = false;

  const dispatch = createEventDispatcher<{
    splitRow: void;
    splitColumn: void;
  }>();

  const MENU_WIDTH = 220;

  let open = false;
  let buttonEl: HTMLButtonElement;
  let menuEl: HTMLDivElement | undefined;
  let menuX = 0;
  let menuY = 0;

  function toggle() {
    if (disabled) return;
    if (open) {
      open = false;
      return;
    }
    const rect = buttonEl.getBoundingClientRect();
    // Right-aligned to the button rather than left-aligned like New's menu
    // — this sits near the window's right edge (beside Shortcuts/Settings
    // and the window controls), where a left-aligned menu would overflow.
    menuX = rect.right - MENU_WIDTH;
    menuY = rect.bottom + 4;
    open = true;
  }

  function pick(event: "splitRow" | "splitColumn") {
    open = false;
    dispatch(event);
  }

  function handleOutsideClick(event: MouseEvent) {
    const target = event.target as Node;
    if (!buttonEl.contains(target) && !(menuEl && menuEl.contains(target))) {
      open = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      open = false;
    }
  }
</script>

<svelte:window on:mousedown={open ? handleOutsideClick : undefined} on:keydown={open ? handleKeydown : undefined} />

<button class="toolbar-btn" class:active={open} bind:this={buttonEl} on:click={toggle} aria-label="Split pane" {disabled}>
  <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
    <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
    <line x1="8" y1="2.5" x2="8" y2="13.5" />
  </svg>
  <span>Split</span>
  <svg class="caret" width="7" height="7" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
    <path d="M2.5 3.5L5 6.5L7.5 3.5" />
  </svg>
</button>

{#if open}
  <div class="menu" bind:this={menuEl} style="left: {menuX}px; top: {menuY}px; min-width: {MENU_WIDTH}px;" role="menu">
    <button class="item" role="menuitem" on:click={() => pick("splitRow")}>
      <span>Split right</span>
      <span class="shortcut">Ctrl+Shift+D</span>
    </button>
    <button class="item" role="menuitem" on:click={() => pick("splitColumn")}>
      <span>Split down</span>
      <span class="shortcut">Ctrl+Shift+E</span>
    </button>
  </div>
{/if}

<style>
  /* Matches FlashPad's ActionToolbar .toolbar-btn (see NewConnectionMenu.svelte
     for the same pattern applied to the sidebar's "New" button). */
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
  .toolbar-btn:hover,
  .toolbar-btn.active {
    background: var(--surface-2);
    color: var(--fg-primary);
  }
  .toolbar-btn:disabled {
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
  .toolbar-btn .caret {
    opacity: 0.7;
  }

  .menu {
    position: fixed;
    z-index: 1000;
    background: var(--surface-1);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    text-align: left;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .item:hover {
    background: var(--surface-2);
  }
  .shortcut {
    font-size: 0.68rem;
    color: var(--fg-tertiary);
  }
</style>
