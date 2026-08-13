<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";

  const dispatch = createEventDispatcher<{ newShell: void }>();
</script>

<div class="empty-main">
  <RingMark size={64} />
  <p class="title">No sessions open</p>
  <button class="cta" on:click={() => dispatch("newShell")}>Open a local shell</button>
</div>

<style>
  .empty-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    background: var(--surface-0);
  }
  .title {
    margin: 0;
    color: var(--fg-secondary);
    font-size: 13px;
  }
  .cta {
    padding: var(--space-2) var(--space-4);
    background: var(--accent);
    color: var(--accent-fg);
    border: none;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .cta:hover {
    filter: brightness(1.08);
  }
</style>
