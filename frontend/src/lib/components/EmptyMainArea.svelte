<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";

  const dispatch = createEventDispatcher<{ newShell: void; newSsh: void }>();
</script>

<div class="empty-main">
  <RingMark size={64} />
  <p class="title">No sessions open</p>
  <div class="actions">
    <button class="cta" on:click={() => dispatch("newSsh")}>Connect over SSH</button>
    <button class="cta secondary" on:click={() => dispatch("newShell")}>Open a local shell</button>
  </div>
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
  .actions {
    display: flex;
    gap: var(--space-2);
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
  .cta.secondary {
    background: var(--surface-2);
    color: var(--fg-primary);
    font-weight: 500;
  }
  .cta.secondary:hover {
    background: var(--surface-3);
    filter: none;
  }
</style>
