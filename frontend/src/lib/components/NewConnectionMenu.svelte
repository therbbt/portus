<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher<{
    newSsh: void;
    newRdp: void;
    newSerial: void;
    newShell: void;
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

  function pick(event: "newSsh" | "newRdp" | "newSerial" | "newShell") {
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

<button class="new-btn" class:active={open} bind:this={buttonEl} on:click={toggle} aria-label="New connection" title="New connection">
  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
    <path d="M8 2v12M2 8h12" />
  </svg>
</button>

{#if open}
  <div class="menu" bind:this={menuEl} style="left: {menuX}px; top: {menuY}px;" role="menu">
    <button class="item" role="menuitem" on:click={() => pick("newSsh")}>SSH connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newRdp")}>RDP connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newSerial")}>Serial connection</button>
    <button class="item" role="menuitem" on:click={() => pick("newShell")}>Local shell</button>
  </div>
{/if}

<style>
  .new-btn {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: var(--fg-tertiary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .new-btn:hover,
  .new-btn.active {
    background: var(--surface-3);
    color: var(--fg-primary);
  }

  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 170px;
    background: var(--surface-2);
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
    padding: 0.4rem 0.55rem;
    font-size: 0.78rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .item:hover {
    background: var(--surface-3);
  }
</style>
