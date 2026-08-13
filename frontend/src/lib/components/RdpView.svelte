<script lang="ts">
  import { onDestroy, onMount, createEventDispatcher } from "svelte";
  import type { RdpConnectOptions, RdpEvent, SessionState } from "../bridge";
  import { rdpConnect, rdpDisconnect, subscribeRdp } from "../bridge";
  import RingMark from "./RingMark.svelte";

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
    }
  }

  onMount(async () => {
    try {
      rdpId = await rdpConnect(options);
      sub = await subscribeRdp(rdpId, handleEvent);
    } catch (e) {
      status = "error";
      statusMessage = String(e);
      dispatch("state", "disconnected");
    }
  });

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
</style>
