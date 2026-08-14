<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher<{
    newSsh: void;
    newRdp: void;
    newSerial: void;
    newShell: void;
    newShellPreset: void;
  }>();

  let open = false;
  let buttonEl: HTMLButtonElement;
  let menuEl: HTMLDivElement | undefined;
  let menuX = 0;
  let menuY = 0;

  function toggle() {
    if (open) {
      open = false;
      return;
    }
    const rect = buttonEl.getBoundingClientRect();
    menuX = rect.left;
    menuY = rect.bottom + 4;
    open = true;
  }

  function pick(event: "newSsh" | "newRdp" | "newSerial" | "newShell" | "newShellPreset") {
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

<button class="toolbar-btn" class:active={open} bind:this={buttonEl} on:click={toggle} aria-label="New connection">
  <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
    <path d="M8 3v10M3 8h10" />
  </svg>
  <span>New</span>
  <svg class="caret" width="7" height="7" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
    <path d="M2.5 3.5L5 6.5L7.5 3.5" />
  </svg>
</button>

{#if open}
  <div class="menu" bind:this={menuEl} style="left: {menuX}px; top: {menuY}px;" role="menu">
    <button class="item" role="menuitem" on:click={() => pick("newSsh")}>SSH connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newRdp")}>RDP connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newSerial")}>Serial connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newShell")}>Local shell</button>
    <button class="item" role="menuitem" on:click={() => pick("newShellPreset")}>Terminal preset…</button>
  </div>
{/if}

<style>
  /* Matches FlashPad's ActionToolbar .toolbar-btn exactly — not just shape
     (icon + label + caret, 22px tall) but tier: FlashPad's --muted/--panel-2
     map to Portus's --fg-secondary/--surface-2, NOT --fg-tertiary/--surface-3
     (those are a shade dimmer/lighter and were the actual reason this
     looked off — same shape, wrong tier). */
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
  .toolbar-btn .caret {
    opacity: 0.7;
  }

  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 170px;
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
    display: block;
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
</style>
