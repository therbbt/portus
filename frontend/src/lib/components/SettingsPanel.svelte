<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import type { TerminalColors } from "../bridge";

  export let terminalFontFamily: string;
  export let terminalFontSize: number;
  export let terminalColors: TerminalColors;
  /** App.svelte owns the actual check (and opens the update dialog itself
   * if one's found) — this just triggers it and reports whether anything
   * came back, so this panel can show its own "you're up to date" message. */
  export let onCheckForUpdate: () => Promise<boolean>;

  const dispatch = createEventDispatcher<{
    save: { terminalFontFamily: string; terminalFontSize: number; terminalColors: TerminalColors };
    cancel: void;
  }>();

  // Portus's own default ANSI palette, not a generic stock one — must match
  // tokens.css's --ansi-* defaults exactly, for both the dark and light
  // media blocks there (see the comments in tokens.css for why each color
  // is what it is). Used both to pre-fill an unset swatch and, on save, to
  // detect "the user dragged this back to the default" so it's stored as
  // an actual reset (null) rather than an explicit override that just
  // happens to match.
  //
  // Can't just read the live --ansi-* custom properties via
  // getComputedStyle here instead of hardcoding them a second time — once
  // a color IS customized, App.svelte's applyTerminalColorVars sets it as
  // an inline style on <html>, which shadows tokens.css's rule in the
  // computed value. Reading "the default" would then return whatever's
  // currently overridden, not the actual default, defeating the one case
  // (reset) that needs the real default most.
  const DARK_DEFAULT_COLORS: Record<keyof TerminalColors, string> = {
    black: "#3a3a42",
    red: "#a1362b",
    green: "#2ba19a",
    yellow: "#a17e2b",
    blue: "#2b5ea1",
    magenta: "#862ba1",
    cyan: "#2b86a1",
    white: "#97958d",
    brightBlack: "#726f66",
    brightRed: "#d05a4d",
    brightGreen: "#4dd0c8",
    brightYellow: "#d0a94d",
    brightBlue: "#4d86d0",
    brightMagenta: "#b14dd0",
    brightCyan: "#4db1d0",
    brightWhite: "#ecebe7",
  };
  const LIGHT_DEFAULT_COLORS: Record<keyof TerminalColors, string> = {
    black: "#211d18",
    red: "#c03d30",
    green: "#1e7671",
    yellow: "#856721",
    blue: "#2e6bba",
    magenta: "#a632c8",
    cyan: "#23738b",
    white: "#6b6259",
    brightBlack: "#8f8579",
    brightRed: "#a7281b",
    brightGreen: "#1b746e",
    brightYellow: "#705412",
    brightBlue: "#1b56a4",
    brightMagenta: "#8e1daf",
    brightCyan: "#136076",
    brightWhite: "#e5e0d5",
  };
  const DEFAULT_COLORS =
    typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: light)").matches
      ? LIGHT_DEFAULT_COLORS
      : DARK_DEFAULT_COLORS;

  const SWATCHES: Array<{ key: keyof TerminalColors; label: string }> = [
    { key: "black", label: "Black" },
    { key: "red", label: "Red" },
    { key: "green", label: "Green" },
    { key: "yellow", label: "Yellow" },
    { key: "blue", label: "Blue" },
    { key: "magenta", label: "Magenta" },
    { key: "cyan", label: "Cyan" },
    { key: "white", label: "White" },
  ];
  const BRIGHT_SWATCHES: Array<{ key: keyof TerminalColors; label: string }> = [
    { key: "brightBlack", label: "Black" },
    { key: "brightRed", label: "Red" },
    { key: "brightGreen", label: "Green" },
    { key: "brightYellow", label: "Yellow" },
    { key: "brightBlue", label: "Blue" },
    { key: "brightMagenta", label: "Magenta" },
    { key: "brightCyan", label: "Cyan" },
    { key: "brightWhite", label: "White" },
  ];

  let fontFamily = terminalFontFamily;
  let fontSize = terminalFontSize;
  // Always holds a real hex per swatch (never null) so <input type="color">
  // always has something valid to show — an unset override just displays
  // the default until touched.
  let colors: Record<keyof TerminalColors, string> = { ...DEFAULT_COLORS, ...stripNulls(terminalColors) };
  let panelEl: HTMLDivElement;

  let appVersion = "";
  let checkingForUpdate = false;
  let updateCheckMessage = "";
  let updateCheckError = "";

  onMount(() => {
    void getVersion().then((v) => (appVersion = v));
  });

  async function checkForUpdates() {
    checkingForUpdate = true;
    updateCheckMessage = "";
    updateCheckError = "";
    try {
      const found = await onCheckForUpdate();
      // If one was found, App.svelte already opens the update dialog with
      // the changelog on top of this panel — nothing more to show here.
      if (!found) updateCheckMessage = "You're up to date.";
    } catch (err) {
      updateCheckError = err instanceof Error ? err.message : "Failed to check for updates.";
    } finally {
      checkingForUpdate = false;
    }
  }

  function stripNulls(input: TerminalColors): Partial<Record<keyof TerminalColors, string>> {
    const result: Partial<Record<keyof TerminalColors, string>> = {};
    for (const key of Object.keys(input) as Array<keyof TerminalColors>) {
      const value = input[key];
      if (value) result[key] = value;
    }
    return result;
  }

  function resetColors() {
    colors = { ...DEFAULT_COLORS };
  }

  $: canSubmit = fontFamily.trim().length > 0 && fontSize >= 8 && fontSize <= 32;

  function submit() {
    if (!canSubmit) return;
    // A swatch matching the default is stored as unset (null) rather than
    // an override that happens to equal it — keeps a config someone hasn't
    // touched clean, and makes "drag back to default" behave like a reset.
    const overrides: TerminalColors = {};
    for (const key of Object.keys(colors) as Array<keyof TerminalColors>) {
      if (colors[key].toLowerCase() !== DEFAULT_COLORS[key].toLowerCase()) {
        overrides[key] = colors[key];
      }
    }
    dispatch("save", { terminalFontFamily: fontFamily.trim(), terminalFontSize: fontSize, terminalColors: overrides });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("cancel");
    }
  }

  function handleOutsideClick(event: MouseEvent) {
    if (panelEl && !panelEl.contains(event.target as Node)) {
      dispatch("cancel");
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" on:mousedown={handleOutsideClick}>
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="Settings">
    <header>
      <h2>Settings</h2>
      <button class="close" aria-label="Close" title="Close" on:click={() => dispatch("cancel")}>
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <path d="M2 2l12 12M14 2L2 14" />
        </svg>
      </button>
    </header>

    <div class="content">
      <section class="card">
        <span class="section-title">Terminal</span>
        <label class="field">
          <span>Font family</span>
          <input type="text" bind:value={fontFamily} placeholder="monospace" />
        </label>
        <label class="field">
          <span>Font size</span>
          <input type="number" bind:value={fontSize} min="8" max="32" />
        </label>
        <p class="hint">
          Applies immediately to every open terminal, not just new ones. "monospace" uses your system's default
          monospace font — type a specific font name instead if you have one installed you'd rather use.
        </p>
      </section>

      <section class="card">
        <div class="colors-header">
          <span class="section-title">Terminal colors</span>
          <button class="reset-link" type="button" on:click={resetColors}>Reset to defaults</button>
        </div>
        <div class="swatch-row">
          {#each SWATCHES as swatch (swatch.key)}
            <label class="swatch" title={swatch.label}>
              <input type="color" bind:value={colors[swatch.key]} aria-label={swatch.label} />
              <span>{swatch.label}</span>
            </label>
          {/each}
        </div>
        <div class="swatch-row">
          {#each BRIGHT_SWATCHES as swatch (swatch.key)}
            <label class="swatch" title={`Bright ${swatch.label}`}>
              <input type="color" bind:value={colors[swatch.key]} aria-label={`Bright ${swatch.label}`} />
              <span>{swatch.label}</span>
            </label>
          {/each}
        </div>
        <p class="hint">
          Only changes this machine's config — never shared or synced anywhere. These are the ANSI colors a shell
          prompt or command output picks from; they don't affect the app's own UI colors.
        </p>
      </section>

      <section class="card">
        <span class="section-title">Updates</span>
        <div class="update-row">
          <span>{appVersion ? `Version ${appVersion}` : "Portus"}</span>
          <button class="btn" disabled={checkingForUpdate} on:click={checkForUpdates}>
            {checkingForUpdate ? "Checking…" : "Check for updates"}
          </button>
        </div>
        {#if updateCheckMessage}
          <p class="hint">{updateCheckMessage}</p>
        {/if}
        {#if updateCheckError}
          <p class="hint error">{updateCheckError}</p>
        {/if}
      </section>
    </div>

    <div class="actions">
      <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
      <button class="btn primary" disabled={!canSubmit} on:click={submit}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: var(--window-shadow-margin);
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .panel {
    width: min(420px, 90vw);
    max-height: 85vh;
    background: var(--surface-2);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-4) 0;
    flex-shrink: 0;
  }

  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--fg-secondary);
    padding: 0;
    cursor: pointer;
  }
  .close:hover {
    background: var(--surface-4);
    color: var(--fg-primary);
  }

  .content {
    padding: var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .section-title {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-tertiary);
  }

  .colors-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .reset-link {
    border: none;
    background: transparent;
    color: var(--fg-secondary);
    font-size: 0.68rem;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
  }
  .reset-link:hover {
    color: var(--fg-primary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--fg-secondary);
  }

  input[type="text"],
  input[type="number"] {
    background: var(--surface-1);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.8rem;
  }
  input[type="text"]:focus-visible,
  input[type="number"]:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .swatch-row {
    display: flex;
    gap: var(--space-2);
  }

  .swatch {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    cursor: pointer;
  }
  .swatch span {
    font-size: 0.62rem;
    color: var(--fg-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .swatch input[type="color"] {
    width: 100%;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    cursor: pointer;
  }
  /* Firefox/WebKitGTK both render a color swatch with its own inset
     border/padding by default — flattening it to a plain filled rectangle
     matches every other input in this app instead of looking like a
     native OS control dropped into a themed panel. */
  .swatch input[type="color"]::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .swatch input[type="color"]::-webkit-color-swatch {
    border: none;
    border-radius: var(--radius-sm);
  }

  .hint {
    margin: 0;
    font-size: 0.68rem;
    color: var(--fg-tertiary);
  }
  .hint.error {
    color: var(--status-error);
  }

  .update-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    font-size: 0.78rem;
    color: var(--fg-primary);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: 0 var(--space-4) var(--space-4);
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
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn.primary:disabled {
    background: var(--surface-4);
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
</style>
