<script lang="ts" context="module">
  export interface ContextMenuItem {
    label: string;
    action?: () => void;
    danger?: boolean;
    separator?: boolean;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";

  export let x: number;
  export let y: number;
  export let items: ContextMenuItem[];
  export let onClose: () => void;

  let menuEl: HTMLDivElement;
  let adjustedX = x;
  let adjustedY = y;

  const clampToViewport = () => {
    if (!menuEl) return;
    const rect = menuEl.getBoundingClientRect();
    const margin = 6;
    adjustedX = Math.min(x, window.innerWidth - rect.width - margin);
    adjustedY = Math.min(y, window.innerHeight - rect.height - margin);
    adjustedX = Math.max(adjustedX, margin);
    adjustedY = Math.max(adjustedY, margin);
  };

  const runItem = (item: ContextMenuItem) => {
    if (item.disabled || item.separator) return;
    item.action?.();
    onClose();
  };

  const handleOutside = (event: MouseEvent) => {
    if (menuEl && !menuEl.contains(event.target as Node)) {
      onClose();
    }
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  };

  onMount(() => {
    clampToViewport();
    window.addEventListener("mousedown", handleOutside, true);
    window.addEventListener("keydown", handleKeydown, true);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("mousedown", handleOutside, true);
      window.removeEventListener("keydown", handleKeydown, true);
      window.removeEventListener("blur", onClose);
    };
  });
</script>

<div class="menu" bind:this={menuEl} style="left: {adjustedX}px; top: {adjustedY}px;" role="menu">
  {#each items as item, index (item.label + index)}
    {#if item.separator}
      <div class="separator"></div>
    {:else}
      <button class="item" class:danger={item.danger} class:disabled={item.disabled} on:click={() => runItem(item)} disabled={item.disabled}>
        {item.label}
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 170px;
    /* One tier lighter than the sidebar it usually pops up over
       (--surface-1) so it reads as a distinct floating card rather than
       blending into whatever's behind it. */
    background: var(--surface-2);
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

  .item:hover:not(.disabled) {
    background: var(--surface-3);
  }

  .item.danger {
    color: var(--status-error);
  }

  .item.disabled {
    opacity: 0.4;
    cursor: default;
  }

  .separator {
    height: 1px;
    background: var(--hairline);
    margin: 0.25rem 0.2rem;
  }
</style>
