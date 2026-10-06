<script lang="ts">
  import { onDestroy, onMount, createEventDispatcher } from "svelte";
  import type { RdpConnectOptions, RdpEvent, SessionState } from "../bridge";
  import { rdpConnect, rdpDisconnect, subscribeRdp, trustRdpHostKey } from "../bridge";
  import RingMark from "./RingMark.svelte";
  import Dialog from "./Dialog.svelte";

  export let options: RdpConnectOptions;
  export let active = true;

  const dispatch = createEventDispatcher<{
    closed: { reason: string | null };
    state: SessionState;
  }>();

  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let rdpId: string | null = null;
  let sub: { unlisten(): Promise<void> } | null = null;
  let status: "connecting" | "connected" | "disconnected" | "error" = "connecting";
  let statusMessage = "";
  let desktopSize: { width: number; height: number } | null = null;

  /** Set while an RDP connection is refused because the server's
   * certificate doesn't match what Portus recorded last time — the
   * changed-certificate confirm prompt renders while this is non-null.
   * `null` the rest of the time (the vastly more common case: this never
   * happens on a host that never hit a mismatch). Mirrors Terminal.svelte's
   * own hostKeyMismatch for SSH. */
  let hostKeyMismatch: { hostId: string; fingerprint: string; keyBase64: string } | null = null;
  let trustingHostKey = false;
  let trustHostKeyError: string | null = null;

  function handleEvent(event: RdpEvent) {
    switch (event.type) {
      case "connected": {
        desktopSize = { width: event.width, height: event.height };
        status = "connected";
        dispatch("state", "connected");
        if (canvas) {
          canvas.width = event.width;
          canvas.height = event.height;
          const ctx = canvas.getContext("2d");
          if (ctx) {
            ctx.fillStyle = "#000";
            ctx.fillRect(0, 0, event.width, event.height);
          }
        }
        break;
      }
      case "frame": {
        const ctx = canvas?.getContext("2d");
        if (!ctx) break;
        const img = new Image();
        img.onload = () => ctx.drawImage(img, event.x, event.y);
        img.src = `data:image/png;base64,${event.pngBase64}`;
        break;
      }
      case "error": {
        status = "error";
        statusMessage = event.message;
        dispatch("state", "disconnected");
        break;
      }
      case "disconnected": {
        status = "disconnected";
        statusMessage = event.reason ?? "";
        dispatch("state", "disconnected");
        dispatch("closed", { reason: event.reason });
        break;
      }
      case "host_key_mismatch": {
        status = "error";
        hostKeyMismatch = { hostId: event.hostId, fingerprint: event.fingerprint, keyBase64: event.keyBase64 };
        dispatch("state", "disconnected");
        break;
      }
    }
  }

  /** Re-runs the same connect steps `onMount` performs originally, reusing
   * the same canvas/container - exported so PaneGrid can call it on a
   * specific pane's RdpView instance from the tab strip's right-click
   * "Reconnect" (see App.svelte). Unlike Terminal.svelte's session_open,
   * there's no backend-emitted "connecting" state for RDP to pick this up
   * automatically, so `state` is dispatched explicitly here. */
  export async function reconnect() {
    await sub?.unlisten();
    sub = null;
    status = "connecting";
    statusMessage = "";
    hostKeyMismatch = null;
    dispatch("state", "connecting");
    try {
      rdpId = await rdpConnect(options);
      sub = await subscribeRdp(rdpId, handleEvent);
    } catch (e) {
      status = "error";
      statusMessage = String(e);
      dispatch("state", "disconnected");
    }
  }

  async function trustAndReconnect() {
    if (!hostKeyMismatch) return;
    trustingHostKey = true;
    trustHostKeyError = null;
    try {
      await trustRdpHostKey(hostKeyMismatch.hostId, hostKeyMismatch.keyBase64);
      hostKeyMismatch = null;
      await reconnect();
    } catch (e) {
      trustHostKeyError = e instanceof Error ? e.message : String(e);
    } finally {
      trustingHostKey = false;
    }
  }

  function dismissHostKeyMismatch() {
    hostKeyMismatch = null;
    trustHostKeyError = null;
  }

  onMount(reconnect);

  onDestroy(() => {
    void sub?.unlisten();
    if (rdpId) void rdpDisconnect(rdpId);
  });
</script>

<div class="rdp-view" class:hidden={!active} bind:this={container}>
  <div class="canvas-scroll">
    <canvas bind:this={canvas}></canvas>
  </div>
  {#if status !== "connected"}
    <div class="status-overlay">
      <RingMark size={40} spinning={status === "connecting"} />
      <p class="status-text">
        {#if status === "connecting"}
          Connecting to {options.host}…
        {:else if status === "error"}
          {statusMessage || "Connection error"}
        {:else if status === "disconnected"}
          Disconnected{statusMessage ? `: ${statusMessage}` : ""}
        {/if}
      </p>
    </div>
  {/if}
</div>

{#if hostKeyMismatch}
  <Dialog label="RDP certificate changed" width="440px" on:cancel={dismissHostKeyMismatch}>
    <h2 class="title">RDP certificate changed</h2>
    <p class="body">
      The certificate presented by <strong>{hostKeyMismatch.hostId}</strong> doesn't match the one Portus recorded
      last time (fingerprint now <code>SHA256:{hostKeyMismatch.fingerprint}</code>).
    </p>
    <p class="body">
      This is expected if the server was reinstalled or rebuilt. It can also mean the connection is being
      intercepted — only continue if you're sure the new certificate is legitimate.
    </p>
    {#if trustHostKeyError}
      <p class="error">{trustHostKeyError}</p>
    {/if}
    <div class="actions">
      <button class="btn" disabled={trustingHostKey} on:click={dismissHostKeyMismatch}>Cancel</button>
      <button class="btn danger" disabled={trustingHostKey} on:click={trustAndReconnect}>
        {trustingHostKey ? "Connecting…" : "Trust new certificate & reconnect"}
      </button>
    </div>
  </Dialog>
{/if}

<style>
  .rdp-view {
    width: 100%;
    height: 100%;
    background: var(--surface-0);
  }
  .rdp-view.hidden {
    display: none;
  }
  .canvas-scroll {
    width: 100%;
    height: 100%;
    overflow: auto;
    display: flex;
    align-items: flex-start;
    justify-content: flex-start;
  }
  canvas {
    display: block;
    image-rendering: pixelated;
  }
  .status-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    background: var(--surface-0);
  }
  .status-text {
    margin: 0;
    color: var(--fg-secondary);
    font-size: 13px;
    max-width: 80%;
    text-align: center;
  }

  .title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }
  .body {
    margin: 0;
    font-size: 0.78rem;
    color: var(--fg-secondary);
    line-height: 1.4;
  }
  .error {
    margin: 0;
    font-size: 0.72rem;
    color: var(--status-error);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }
  .btn {
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    color: var(--fg-primary);
    font-size: 0.78rem;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--surface-4);
  }
  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .btn.danger {
    background: var(--status-error);
    color: #fff;
    font-weight: 600;
  }
  .btn.danger:hover {
    filter: brightness(1.08);
  }
</style>
