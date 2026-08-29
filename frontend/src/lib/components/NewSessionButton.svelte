<script lang="ts">
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher<{ open: void }>();
</script>

<button class="toolbar-btn" on:click={() => dispatch("open")} aria-label="New session">
  <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
    <path d="M8 3v10M3 8h10" />
  </svg>
  <span>New</span>
</button>

<style>
  /* Matches FlashPad's ActionToolbar .toolbar-btn exactly — not just shape
     (icon + label, 22px tall) but tier: FlashPad's --muted/--panel-2 map
     to Portus's --fg-secondary/--surface-2, NOT --fg-tertiary/--surface-3
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
  .toolbar-btn:hover {
    background: var(--surface-2);
    color: var(--fg-primary);
  }
</style>
