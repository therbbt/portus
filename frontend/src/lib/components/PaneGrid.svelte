<script lang="ts">
  import Terminal from "./Terminal.svelte";
  import RdpView from "./RdpView.svelte";
  import PaneResizer from "./PaneResizer.svelte";
  import { collectResizers, computePaneRects, type PaneLayout, type PaneState } from "../panes";
  import type { RdpConnectOptions, SessionState } from "../bridge";

  export let node: PaneLayout;
  export let panes: Record<string, PaneState>;
  export let activePaneId: string;
  /** Whether the tab that owns this grid is the one currently shown — passed
   * straight through to every pane's Terminal/RdpView `active` prop, same
   * value for every pane in the tree (it's about tab visibility, not which
   * pane has focus within it). */
  export let active: boolean;
  /** Only true once this tab actually has more than one pane — a
   * single-pane tab renders with zero extra chrome, identical to before
   * split panes existed. */
  export let showHeader: boolean;

  export let onFocusPane: (id: string) => void;
  export let onClosePane: (id: string) => void;
  export let onResizeSplit: (splitId: string, sizes: number[]) => void;
  export let onPaneState: (id: string, state: SessionState) => void;
  export let onPaneTitle: (id: string, title: string) => void;
  export let onPaneClosed: (id: string) => void;

  let containerEl: HTMLDivElement | undefined;

  // Pure percentage math, recomputed whenever the tree shape or sizes
  // change — deliberately NOT what drives which Terminal/RdpView components
  // exist. Those render from a flat {#each} keyed only by paneId (below),
  // which never changes for a pane's lifetime, so a split (which changes a
  // leaf's position in its parent's children array) only ever produces a
  // style update here, never a remount of an already-running session.
  $: paneRects = computePaneRects(node);
  $: resizers = collectResizers(node);
</script>

<div class="pane-grid" bind:this={containerEl}>
  {#each Object.entries(paneRects) as [paneId, rect] (paneId)}
    {@const pane = panes[paneId]}
    {#if pane}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="pane-leaf"
        class:focused={showHeader && paneId === activePaneId}
        style="left: {rect.left}%; top: {rect.top}%; width: {rect.width}%; height: {rect.height}%;"
        on:mousedown={() => onFocusPane(paneId)}
      >
        {#if showHeader}
          <div class="pane-header">
            <span class="pane-dot" data-state={pane.state}></span>
            <span class="pane-title">{pane.title}</span>
            <span
              class="pane-close"
              role="button"
              tabindex="0"
              aria-label="Close pane"
              title="Close pane"
              on:click|stopPropagation={() => onClosePane(paneId)}
              on:keydown|stopPropagation={(e) => e.key === "Enter" && onClosePane(paneId)}
            >
              ×
            </span>
          </div>
        {/if}
        <div class="pane-body">
          {#if pane.protocol === "rdp"}
            <RdpView
              options={pane.options as RdpConnectOptions}
              {active}
              on:state={(e) => onPaneState(paneId, e.detail)}
              on:closed={() => onPaneClosed(paneId)}
            />
          {:else}
            <Terminal
              protocol={pane.protocol}
              options={pane.options}
              savedSessionId={pane.savedSessionId}
              {active}
              on:state={(e) => onPaneState(paneId, e.detail)}
              on:title={(e) => onPaneTitle(paneId, e.detail.title)}
              on:closed={() => onPaneClosed(paneId)}
            />
          {/if}
        </div>
      </div>
    {/if}
  {/each}
  {#each resizers as r (r.splitId + "-" + r.index)}
    <PaneResizer descriptor={r} {containerEl} onResize={onResizeSplit} />
  {/each}
</div>

<style>
  .pane-grid {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .pane-leaf {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }
  .pane-leaf.focused {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .pane-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 22px;
    padding: 0 var(--space-2);
    background: var(--surface-1);
    border-bottom: 1px solid var(--hairline);
  }
  .pane-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--status-disconnected);
    flex-shrink: 0;
  }
  .pane-dot[data-state="connecting"] {
    background: var(--status-connecting);
  }
  .pane-dot[data-state="connected"] {
    background: var(--status-connected);
  }
  .pane-dot[data-state="disconnected"] {
    background: var(--status-disconnected);
  }
  .pane-title {
    flex: 1;
    min-width: 0;
    font-size: 0.7rem;
    color: var(--fg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pane-close {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 0.3rem;
    color: var(--fg-tertiary);
    border-radius: var(--radius-sm);
    line-height: 1;
  }
  .pane-close:hover {
    color: var(--fg-primary);
    background: var(--surface-4);
  }
  .pane-body {
    flex: 1;
    min-width: 0;
    min-height: 0;
    position: relative;
    display: flex;
  }
  .pane-body :global(.terminal-host),
  .pane-body :global(.rdp-view) {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
</style>
