<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import RingMark from "./RingMark.svelte";
  import type { Host } from "../bridge";

  export let hosts: Host[] = [];
  export let width = 260;

  const dispatch = createEventDispatcher<{
    newShell: void;
    newSsh: void;
    newSerial: void;
    newRdp: void;
    connect: Host;
    deleteHost: Host;
  }>();

  // "echo" is a debug-only session kind, never a real saved host's
  // protocol — included here only so this satisfies the shared `Protocol` type.
  const protocolLabel: Record<Host["protocol"], string> = {
    ssh: "SSH",
    serial: "Serial",
    shell: "Shell",
    telnet: "Telnet",
    rdp: "RDP",
    echo: "Echo",
  };
</script>

<aside class="rail" style="width: {width}px; min-width: {width}px">
  <div class="rail-header">
    <span class="rail-title">Hosts</span>
  </div>

  {#if hosts.length === 0}
    <div class="empty-state">
      <RingMark size={40} />
      <p class="empty-title">No saved hosts yet</p>
      <p class="empty-subtitle">Save a connection from the SSH or serial dialog and it'll show up here.</p>
    </div>
  {:else}
    <ul class="host-list">
      {#each hosts as host (host.id)}
        <li class="host-row">
          <button class="host-main" on:click={() => dispatch("connect", host)}>
            <span class="host-name">{host.name}</span>
            <span class="host-meta">{protocolLabel[host.protocol]} · {host.address}</span>
          </button>
          <span
            class="host-delete"
            role="button"
            tabindex="0"
            aria-label={`Delete ${host.name}`}
            title={`Delete ${host.name}`}
            on:click|stopPropagation={() => dispatch("deleteHost", host)}
            on:keydown|stopPropagation={(e) => e.key === "Enter" && dispatch("deleteHost", host)}
          >
            ×
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="rail-footer">
    <button class="new-shell-btn" on:click={() => dispatch("newSsh")}>
      + SSH connection
    </button>
    <button class="new-shell-btn" on:click={() => dispatch("newRdp")}>
      + RDP connection
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
    flex-shrink: 0;
    background: var(--surface-1);
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .rail-header {
    /* Matches FlashPad's ActionToolbar exactly: height 30px, padding
       0 0.5rem (not Portus's own wider --space-4 horizontal rhythm), and
       the hairline bottom border — missed on the first pass, since
       Portus's chrome is tonal-separation-first everywhere else. */
    height: 30px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 0.5rem;
    border-bottom: 1px solid var(--hairline);
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
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .host-row:hover {
    background: var(--surface-3);
  }
  .host-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 0.3rem 0.4rem;
    background: transparent;
    border: none;
    color: var(--fg-primary);
    cursor: pointer;
    text-align: left;
  }
  .host-name {
    font-size: 0.78rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .host-meta {
    font-size: 0.66rem;
    color: var(--fg-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .host-delete {
    flex-shrink: 0;
    padding: 0 0.5rem;
    color: var(--fg-tertiary);
    opacity: 0;
    border-radius: var(--radius-sm);
    line-height: 1;
  }
  .host-row:hover .host-delete {
    opacity: 1;
  }
  .host-delete:hover {
    color: var(--fg-primary);
    background: var(--surface-4);
  }
  .rail-footer {
    /* Matches FlashPad's sidebar Footer padding exactly (0.5rem 0.9rem) —
       this is Portus's structural equivalent (bottom-of-sidebar bar), just
       stacked instead of a single row since it holds four action buttons
       instead of a search box. */
    padding: 0.5rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .new-shell-btn {
    width: 100%;
    padding: 0.35rem 0.6rem;
    background: var(--surface-2);
    color: var(--fg-primary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.75rem;
    text-align: left;
  }
  .new-shell-btn:hover {
    background: var(--surface-3);
  }
  .new-shell-btn:active {
    background: var(--surface-4);
  }
</style>
