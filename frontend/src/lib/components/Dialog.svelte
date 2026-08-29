<script lang="ts">
  import { createEventDispatcher } from "svelte";

  // Shared overlay/panel chrome factored out of NewSessionDialog.svelte -
  // the four existing per-protocol connect dialogs (SshConnectDialog etc.)
  // each still carry their own copy of this exact same pattern
  // (overlay + panel + role="dialog" + Escape/outside-click-to-cancel);
  // this is only used by the new dialog, not a retrofit of those four.
  //
  // Enter-to-submit is deliberately NOT handled here - it means something
  // different per dialog (which button counts as "submit", whether typing
  // in a text field should swallow a bare Enter) and stays local to
  // whichever component uses this wrapper, same as it already varies
  // slightly between the existing dialogs today.
  export let label: string;
  export let width = "380px";

  const dispatch = createEventDispatcher<{ cancel: void }>();

  let panelEl: HTMLDivElement;

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    }
  }

  function handleOutsideClick(event: MouseEvent) {
    if (panelEl && !panelEl.contains(event.target as Node)) {
      dispatch("cancel");
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" on:mousedown={handleOutsideClick}>
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label={label} style="width: min({width}, 90vw);">
    <slot />
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: var(--window-shadow-margin);
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .panel {
    background: var(--surface-2);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
</style>
