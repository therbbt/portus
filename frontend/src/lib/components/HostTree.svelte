<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";

  export let hosts: Array<{ id: string; name: string }> = [];

  const dispatch = createEventDispatcher<{ newShell: void; newSsh: void; newSerial: void }>();
</script>

<aside class="rail">
  <div class="rail-header">
    <span class="rail-title">Hosts</span>
  </div>

  {#if hosts.length === 0}
    <div class="empty-state">
      <RingMark size={40} />
      <p class="empty-title">No saved hosts yet</p>
      <p class="empty-subtitle">Hosts you add will show up here, grouped and collapsible.</p>
    </div>
  {:else}
    <ul class="host-list">
      {#each hosts as host (host.id)}
        <li class="host-row">{host.name}</li>
      {/each}
    </ul>
  {/if}

  <div class="rail-footer">
    <button class="new-shell-btn" on:click={() => dispatch("newSsh")}>
      + SSH connection
    </button>
    <button class="new-shell-btn" on:click={() => dispatch("newSerial")}>
      + Serial connection
    </button>
    <button class="new-shell-btn" on:click={() => dispatch("newShell")}>
      + Local shell
    </button>
  </div>
</aside>

<style>
  .rail {
    width: var(--rail-width);
    min-width: var(--rail-width);
    background: var(--surface-1);
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .rail-header {
    padding: var(--space-4) var(--space-4) var(--space-2);
  }
  .rail-title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-tertiary);
  }
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-5);
    opacity: 0.85;
  }
  .empty-title {
    margin: var(--space-2) 0 0;
    color: var(--fg-secondary);
    font-size: 13px;
  }
  .empty-subtitle {
    margin: 0;
    color: var(--fg-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .host-list {
    list-style: none;
    margin: 0;
    padding: 0 var(--space-2);
    overflow-y: auto;
    flex: 1;
  }
  .host-row {
    padding: 0.22rem 0.4rem;
    border-radius: var(--radius-sm);
    color: var(--fg-primary);
    cursor: pointer;
    font-size: 0.78rem;
  }
  .host-row:hover {
    background: var(--surface-3);
  }
  .rail-footer {
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .new-shell-btn {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--surface-2);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 13px;
    text-align: left;
  }
  .new-shell-btn:hover {
    background: var(--surface-3);
  }
  .new-shell-btn:active {
    background: var(--surface-4);
  }
</style>
