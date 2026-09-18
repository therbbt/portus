<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import "@xterm/xterm/css/xterm.css";
  import {
    openSession,
    newSessionId,
    writeSession,
    resizeSession,
    closeSession,
    subscribeSession,
    trustHostKey,
    type Protocol,
    type SessionEvent,
    type SessionState,
    type SessionOptions,
  } from "../bridge";
  import { terminalAppearanceVersion } from "../terminalAppearance";
  import {
    highlightTerminalOutput,
    setHighlightGreen,
    setHighlightGet,
    setHighlightUrl,
    setHighlightIpv6,
  } from "../terminalHighlight";
  import Dialog from "./Dialog.svelte";

  export let protocol: Protocol = "shell";
  export let options: SessionOptions = undefined;
  export let active = true;
  /** Set only when this tab was opened from a saved session — see openSession. */
  export let savedSessionId: string | undefined = undefined;

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
  /** Persists across `data` events (not recreated per-chunk) so a multi-byte
   * UTF-8 character split across two chunks decodes correctly instead of
   * producing a replacement character at the boundary. */
  let decoder: TextDecoder;

  /** Set while an SSH connection is refused because the server's host key
   * doesn't match what Portus recorded last time — the changed-host-key
   * confirm prompt renders while this is non-null. `null` the rest of the
   * time (the vastly more common case: this never happens on a session that
   * never hit a mismatch). */
  let hostKeyMismatch: { hostId: string; fingerprint: string; keyBase64: string } | null = null;
  let trustingHostKey = false;
  let trustHostKeyError: string | null = null;

  // fit() forces a synchronous layout read, and xterm's own onResize
  // handler (below) already tells the backend when cols/rows actually
  // change — so this only measures and resizes the grid, it never talks to
  // the backend itself. During a live window drag a ResizeObserver can fire
  // far more often than the browser paints, and each fit() was previously
  // paired with its own redundant resizeSession() call on top of the one
  // onResize already makes; both together were enough IPC + layout thrash
  // per tick to make content visibly stop updating mid-drag.
  //
  // The `active` guard matters for a different reason: the ResizeObserver
  // below watches `container` unconditionally, and a hidden tab's
  // container (display:none) reports a 0x0 content rect — without this
  // guard, fit() would resize the terminal down to a degenerate size while
  // it's off-screen, corrupting xterm's tracked cursor position (it stays
  // wrong even once the tab is shown and refit back to full size, since
  // the buffer reflow through that 0x0 intermediate state doesn't
  // reconstruct it correctly). That's what a stray blinking cursor sitting
  // mid-prompt after switching tabs turned out to be.
  function applyFit() {
    if (!term || !sessionId || !active) return;
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

  // Single source of truth for both the terminal's initial construction and
  // any later live refresh (see the $: block below) — reads the same
  // --font-mono/--font-size-terminal/--ansi-* custom properties either way,
  // so a Settings change reaches this terminal the moment it's saved
  // instead of only affecting tabs opened afterward.
  function readAppearance() {
    const rootStyle = getComputedStyle(document.documentElement);
    const cssVar = (name: string) => rootStyle.getPropertyValue(name).trim();
    // Keeps the colorizer's dedicated (non-ANSI) colors in sync with the
    // current theme (light/dark, or a future palette swap) — see
    // tokens.css's --highlight-* custom properties and terminalHighlight.ts's
    // own comment on why these can't just be an ANSI code like everything
    // else it colors.
    setHighlightGreen(cssVar("--highlight-green"));
    setHighlightGet(cssVar("--highlight-get"));
    setHighlightUrl(cssVar("--highlight-url"));
    setHighlightIpv6(cssVar("--highlight-ipv6"));
    return {
      fontFamily: cssVar("--font-mono"),
      fontSize: parseInt(cssVar("--font-size-terminal")) || 14,
      theme: {
        background: cssVar("--surface-0"),
        foreground: cssVar("--fg-primary"),
        cursor: cssVar("--accent"),
        selectionBackground: cssVar("--accent-dim"),
        // Per-machine overrides (Settings) land on these custom properties
        // via App.svelte's applyTerminalColorVars — tokens.css's --ansi-*
        // defaults (xterm.js's own built-in palette) otherwise.
        black: cssVar("--ansi-black"),
        red: cssVar("--ansi-red"),
        green: cssVar("--ansi-green"),
        yellow: cssVar("--ansi-yellow"),
        blue: cssVar("--ansi-blue"),
        magenta: cssVar("--ansi-magenta"),
        cyan: cssVar("--ansi-cyan"),
        white: cssVar("--ansi-white"),
        brightBlack: cssVar("--ansi-bright-black"),
        brightRed: cssVar("--ansi-bright-red"),
        brightGreen: cssVar("--ansi-bright-green"),
        brightYellow: cssVar("--ansi-bright-yellow"),
        brightBlue: cssVar("--ansi-bright-blue"),
        brightMagenta: cssVar("--ansi-bright-magenta"),
        brightCyan: cssVar("--ansi-bright-cyan"),
        brightWhite: cssVar("--ansi-bright-white"),
      },
    };
  }

  function applyLiveAppearance() {
    if (!term) return;
    const appearance = readAppearance();
    term.options.fontFamily = appearance.fontFamily;
    term.options.fontSize = appearance.fontSize;
    term.options.theme = appearance.theme;
    // A font-size change moves cell metrics, so the grid needs re-fitting
    // the same way a container resize does.
    fitAddon?.fit();
  }

  function handleEvent(event: SessionEvent) {
    switch (event.type) {
      case "data":
        term.write(highlightTerminalOutput(decoder.decode(new Uint8Array(event.data), { stream: true })));
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
      case "host_key_mismatch":
        hostKeyMismatch = { hostId: event.hostId, fingerprint: event.fingerprint, keyBase64: event.keyBase64 };
        break;
    }
  }

  /** Re-runs the same connect steps `onMount` performed originally — safe to
   * repeat because `term.onData`/`onResize` below close over `sessionId` by
   * reference, so they pick up the new id without being re-registered. */
  async function reconnect() {
    await sub?.unlisten();
    decoder = new TextDecoder();
    sessionId = newSessionId();
    // Subscribed before the session exists on the backend at all — see
    // openSession's doc comment for why a local shell needs this ordering.
    sub = await subscribeSession(sessionId, handleEvent);
    dispatch("ready", { sessionId });
    await openSession(sessionId, protocol, options, savedSessionId);
    void resizeSession(sessionId, term.cols, term.rows);
  }

  async function trustAndReconnect() {
    if (!hostKeyMismatch) return;
    trustingHostKey = true;
    trustHostKeyError = null;
    try {
      await trustHostKey(hostKeyMismatch.hostId, hostKeyMismatch.keyBase64);
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

  onMount(async () => {
    decoder = new TextDecoder();
    const appearance = readAppearance();
    term = new Terminal({
      fontFamily: appearance.fontFamily,
      fontSize: appearance.fontSize,
      theme: appearance.theme,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    // Default WebLinksAddon behavior is a bare `window.open(uri)`, which
    // inside the Tauri webview doesn't reliably reach the OS's actual
    // default browser — routing through the opener plugin's `openUrl`
    // does, the same way a native "open in browser" menu item would.
    term.loadAddon(new WebLinksAddon((_event, uri) => void openUrl(uri)));
    // Ctrl+Shift+A rather than plain Ctrl+A — matches this app's own
    // convention (see App.svelte's split-pane shortcuts) of using
    // Ctrl+Shift+<key> for anything the app wants to reserve at the
    // keyboard level, precisely because the plain combo already means
    // something to the shell. Plain Ctrl+A is readline's "move to start of
    // line," used constantly in everyday shell/vim editing — binding it to
    // select-all instead would silently break that in every session.
    // Returning `false` tells xterm.js to stop processing the event
    // (i.e. don't also send it to the shell as input) once handled here.
    term.attachCustomKeyEventHandler((event) => {
      if (event.type === "keydown" && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "a") {
        term.selectAll();
        return false;
      }
      return true;
    });
    term.open(container);
    fitAddon.fit();

    sessionId = newSessionId();
    // Subscribed before the session exists on the backend at all — see
    // openSession's doc comment for why a local shell needs this ordering.
    sub = await subscribeSession(sessionId, handleEvent);
    dispatch("ready", { sessionId });
    await openSession(sessionId, protocol, options, savedSessionId);

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

  // Becoming the visible tab can reveal a stale size (it was 0x0 while
  // hidden) — fit() corrects the cell grid, but xterm.js also positions a
  // hidden <textarea> (its real keyboard-input target, kept aligned with
  // the cursor cell for IME support) using those same now-stale metrics.
  // If that textarea doesn't get repositioned before it's focused again,
  // the browser's native text caret can render at a leftover pixel
  // position that happens to land mid-word in the prompt text instead of
  // at the actual cursor — term.focus() after the fit forces xterm to
  // resync it, and doubles as switching tabs actually being ready to type
  // in immediately.
  $: if (active && term && fitAddon) {
    requestAnimationFrame(() => {
      applyFit();
      term.focus();
    });
  }

  // Fires once redundantly right after onMount creates `term` (harmless —
  // just re-applies what construction already used), then again on every
  // real settings change afterward.
  $: if (term && $terminalAppearanceVersion >= 0) {
    applyLiveAppearance();
  }
</script>

<div class="terminal-host" class:hidden={!active} bind:this={container}></div>

{#if hostKeyMismatch}
  <Dialog label="SSH host key changed" width="440px" on:cancel={dismissHostKeyMismatch}>
    <h2 class="title">SSH host key changed</h2>
    <p class="body">
      The key presented by <strong>{hostKeyMismatch.hostId}</strong> doesn't match the one Portus recorded last
      time (fingerprint now <code>SHA256:{hostKeyMismatch.fingerprint}</code>).
    </p>
    <p class="body">
      This is expected if the server was reinstalled or rebuilt. It can also mean the connection is being
      intercepted — only continue if you're sure the new key is legitimate.
    </p>
    {#if trustHostKeyError}
      <p class="error">{trustHostKeyError}</p>
    {/if}
    <div class="actions">
      <button class="btn" disabled={trustingHostKey} on:click={dismissHostKeyMismatch}>Cancel</button>
      <button class="btn danger" disabled={trustingHostKey} on:click={trustAndReconnect}>
        {trustingHostKey ? "Connecting…" : "Trust new key & reconnect"}
      </button>
    </div>
  </Dialog>
{/if}

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
  /* xterm.js keeps a hidden <textarea> (its real keyboard-input target,
     positioned over the cursor cell for IME support) that's supposed to be
     fully invisible via opacity:0/width:0/height:0 in its own CSS —
     WebKitGTK (this app's Linux webview) doesn't reliably honor that for a
     focused element's native blinking caret, so it can render through at
     wherever xterm last positioned it, looking like a stray cursor sitting
     in the wrong spot after switching tabs. caret-color is the direct,
     standards-based way to suppress that specific rendering without
     touching the element's actual (working) input capture. Also strip our
     own global *:focus-visible ring (tokens.css) for the same element —
     programmatically focusing it (see the $: block above) can trigger that
     heuristic too, and a teal box-shadow ring reads just as much like a
     misplaced cursor as a native caret would. */
  .terminal-host :global(.xterm-helper-textarea) {
    caret-color: transparent;
  }
  .terminal-host :global(.xterm-helper-textarea:focus-visible) {
    outline: none !important;
    box-shadow: none !important;
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
