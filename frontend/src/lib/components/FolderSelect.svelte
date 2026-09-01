<script lang="ts">
  import type { Group } from "../bridge";

  // A native <select>'s open option list can't be restyled — its hover/
  // selected highlight always uses the OS's own accent color (blue on most
  // systems), clashing with Portus's teal accent no matter what CSS is
  // applied to the closed control. This is a drop-in replacement (bind
  // `value` the same way) built from a plain trigger button + popup list,
  // matching the app's other dropdown menus (NewSessionMenu.svelte etc.)
  // instead of a native widget.
  export let groups: Group[] = [];
  export let value = "";

  let open = false;
  let triggerEl: HTMLButtonElement;
  let menuEl: HTMLDivElement | undefined;

  $: selectedName = groups.find((g) => g.id === value)?.name ?? "None";

  function toggle() {
    open = !open;
  }

  function pick(id: string) {
    value = id;
    open = false;
  }

  function handleOutsideClick(event: MouseEvent) {
    const target = event.target as Node;
    if (!triggerEl.contains(target) && !(menuEl && menuEl.contains(target))) {
      open = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      open = false;
    }
  }
</script>

<svelte:window on:mousedown={open ? handleOutsideClick : undefined} on:keydown={open ? handleKeydown : undefined} />

<div class="folder-select">
  <button type="button" class="trigger" bind:this={triggerEl} on:click={toggle} aria-haspopup="listbox" aria-expanded={open}>
    <span class="label">{selectedName}</span>
    <svg class="caret" width="8" height="8" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
      <path d="M2.5 3.5L5 6.5L7.5 3.5" />
    </svg>
  </button>
  {#if open}
    <div class="menu" bind:this={menuEl} role="listbox">
      <button type="button" class="option" class:active={value === ""} role="option" aria-selected={value === ""} on:click={() => pick("")}>
        None
      </button>
      {#each groups as group (group.id)}
        <button type="button" class="option" class:active={value === group.id} role="option" aria-selected={value === group.id} on:click={() => pick(group.id)}>
          {group.name}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .folder-select {
    position: relative;
  }
  .trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem;
    background: var(--surface-1);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.8rem;
    cursor: pointer;
    text-align: left;
  }
  .trigger:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .caret {
    flex-shrink: 0;
    color: var(--fg-secondary);
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 1001;
    min-width: 140px;
    max-height: 200px;
    overflow-y: auto;
    background: var(--surface-1);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .option {
    display: block;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    text-align: left;
    padding: 0.35rem 0.5rem;
    font-size: 0.78rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .option:hover {
    background: var(--surface-2);
  }
  .option.active {
    background: var(--surface-3);
  }
</style>
