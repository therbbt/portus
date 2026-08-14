<script lang="ts">
  import type { ResizerDescriptor } from "../panes";

  export let descriptor: ResizerDescriptor;
  /** The tab's overall content container — its real pixel size along the
   * split's axis, times descriptor.axisLength / 100, is the actual split's
   * pixel size a drag delta gets turned into a fraction of. Bound by the
   * parent via bind:this, so it's only actually read once a drag starts. */
  export let containerEl: HTMLElement | undefined;
  export let onResize: (splitId: string, sizes: number[]) => void;

  const MIN_SIZE = 0.1;

  let isResizing = false;

  function startResize(event: MouseEvent) {
    if (!containerEl) return;
    event.preventDefault();
    isResizing = true;
    const { direction, index, sizes, axisLength, splitId } = descriptor;
    const cursorClass = direction === "row" ? "resizing-pane-col" : "resizing-pane-row";
    document.body.classList.add(cursorClass);

    const containerRect = containerEl.getBoundingClientRect();
    const totalAxisPixels = direction === "row" ? containerRect.width : containerRect.height;
    const splitAxisPixels = (totalAxisPixels * axisLength) / 100;
    const startPos = direction === "row" ? event.clientX : event.clientY;
    const startSizes = [...sizes];

    function handleMove(moveEvent: MouseEvent) {
      const pos = direction === "row" ? moveEvent.clientX : moveEvent.clientY;
      const delta = (pos - startPos) / splitAxisPixels;
      const a = startSizes[index] + delta;
      const b = startSizes[index + 1] - delta;
      if (a < MIN_SIZE || b < MIN_SIZE) return;
      const next = [...startSizes];
      next[index] = a;
      next[index + 1] = b;
      onResize(splitId, next);
    }

    function handleUp() {
      isResizing = false;
      document.body.classList.remove(cursorClass);
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    }

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
  }

  $: style =
    descriptor.direction === "row"
      ? `left: ${descriptor.position}%; top: ${descriptor.crossStart}%; height: ${descriptor.crossLength}%;`
      : `top: ${descriptor.position}%; left: ${descriptor.crossStart}%; width: ${descriptor.crossLength}%;`;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="pane-resizer"
  class:row={descriptor.direction === "row"}
  class:column={descriptor.direction === "column"}
  class:active={isResizing}
  {style}
  on:mousedown={startResize}
></div>

<style>
  .pane-resizer {
    position: absolute;
    z-index: 10;
    background: transparent;
  }
  .pane-resizer.row {
    width: 5px;
    margin-left: -2px;
    cursor: col-resize;
  }
  .pane-resizer.column {
    height: 5px;
    margin-top: -2px;
    cursor: row-resize;
  }
  .pane-resizer:hover,
  .pane-resizer.active {
    background: var(--accent);
  }
</style>
