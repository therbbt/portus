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
  } from "../bridge";

  export let protocol: Protocol = "shell";
  export let active = true;

  const dispatch = createEventDispatcher<{
    state: SessionState;
    closed: { reason: string | null };
    ready: { sessionId: string };
  }>();

  let container: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let sessionId: string | null = null;
  let sub: { unlisten(): Promise<void> } | null = null;
  let resizeObserver: ResizeObserver;

  function applyFit() {
    if (!term || !sessionId) return;
    fitAddon.fit();
    resizeSession(sessionId, term.cols, term.rows);
  }

  function handleEvent(event: SessionEvent) {
    switch (event.type) {
      case "data":
        term.write(new Uint8Array(event.data));
        break;
      case "state_changed":
        dispatch("state", event.state);
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

    sessionId = await openSession(protocol);
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

    resizeObserver = new ResizeObserver(() => applyFit());
    resizeObserver.observe(container);

    void resizeSession(sessionId, term.cols, term.rows);
  });

  onDestroy(() => {
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
