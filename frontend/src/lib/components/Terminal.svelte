<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";
  import {
    openSession,
    writeSession,
    resizeSession,
    closeSession,
    subscribeSession,
    type Protocol,
    type SessionEvent,
    type SessionState,
    type SessionOptions,
  } from "../bridge";

  export let protocol: Protocol = "shell";
  export let options: SessionOptions = undefined;
  export let active = true;
  /** Set only when this tab was opened from a saved host — see openSession. */
  export let hostId: string | undefined = undefined;

  const dispatch = createEventDispatcher<{
    state: SessionState;
    closed: { reason: string | null };
    ready: { sessionId: string };
    title: { title: string };
  }>();

  let container: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let sessionId: string | null = null;
  let sub: { unlisten(): Promise<void> } | null = null;
  let resizeObserver: ResizeObserver;
  let resizeRaf: number | null = null;

  // fit() forces a synchronous layout read, and xterm's own onResize
  // handler (below) already tells the backend when cols/rows actually
  // change — so this only measures and resizes the grid, it never talks to
  // the backend itself. During a live window drag a ResizeObserver can fire
  // far more often than the browser paints, and each fit() was previously
  // paired with its own redundant resizeSession() call on top of the one
  // onResize already makes; both together were enough IPC + layout thrash
  // per tick to make content visibly stop updating mid-drag.
  function applyFit() {
    if (!term || !sessionId) return;
    fitAddon.fit();
  }

  // Coalesces bursts of ResizeObserver callbacks (common during a live
  // native window resize) down to at most one fit() per animation frame.
  function scheduleFit() {
    if (resizeRaf !== null) return;
    resizeRaf = requestAnimationFrame(() => {
      resizeRaf = null;
      applyFit();
    });
  }

  function handleEvent(event: SessionEvent) {
    switch (event.type) {
      case "data":
        term.write(new Uint8Array(event.data));
        break;
      case "state_changed":
        dispatch("state", event.state);
        break;
      case "title_changed":
        dispatch("title", { title: event.title });
        break;
      case "closed":
        dispatch("closed", { reason: event.reason });
        break;
      case "error":
        term.write(`\r\n\x1b[31m[portus] ${event.message}\x1b[0m\r\n`);
        break;
    }
  }

  onMount(async () => {
    term = new Terminal({
      fontFamily: getComputedStyle(document.documentElement).getPropertyValue("--font-mono").trim(),
      fontSize: parseInt(getComputedStyle(document.documentElement).getPropertyValue("--font-size-terminal")) || 14,
      theme: {
        background: getComputedStyle(document.documentElement).getPropertyValue("--surface-0").trim(),
        foreground: getComputedStyle(document.documentElement).getPropertyValue("--fg-primary").trim(),
        cursor: getComputedStyle(document.documentElement).getPropertyValue("--accent").trim(),
        selectionBackground: getComputedStyle(document.documentElement).getPropertyValue("--accent-dim").trim(),
      },
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.open(container);
    fitAddon.fit();

    sessionId = await openSession(protocol, options, hostId);
    dispatch("ready", { sessionId });

    sub = await subscribeSession(sessionId, handleEvent);

    term.onData((data) => {
      if (sessionId) void writeSession(sessionId, new TextEncoder().encode(data));
    });

    // Resize propagation: the PTY (or remote protocol) must learn about
    // every grid change, not just the initial one — this is the classic bug
    // (resizing one side but not the other) the milestone calls out.
    term.onResize(({ cols, rows }) => {
      if (sessionId) void resizeSession(sessionId, cols, rows);
    });

    resizeObserver = new ResizeObserver(() => scheduleFit());
    resizeObserver.observe(container);

    void resizeSession(sessionId, term.cols, term.rows);
  });

  onDestroy(() => {
    if (resizeRaf !== null) cancelAnimationFrame(resizeRaf);
    resizeObserver?.disconnect();
    void sub?.unlisten();
    if (sessionId) void closeSession(sessionId);
    term?.dispose();
  });

  $: if (active && term && fitAddon) {
    // Becoming the visible tab can reveal a stale size (it was 0x0 while hidden).
    requestAnimationFrame(applyFit);
  }
</script>

<div class="terminal-host" class:hidden={!active} bind:this={container}></div>

<style>
  .terminal-host {
    width: 100%;
    height: 100%;
    padding: var(--space-2);
    background: var(--surface-0);
  }
  .terminal-host.hidden {
    display: none;
  }
  .terminal-host :global(.xterm) {
    height: 100%;
  }
</style>
