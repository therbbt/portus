// Pure data model + tree operations for split-screen panes within a tab.
// Kept free of Svelte/DOM so the tree logic (the part actually worth
// getting wrong) can be unit-tested without mounting a component.

import type { Protocol, SessionOptions, SessionState } from "./bridge";

export interface PaneState {
  id: string;
  protocol: Protocol;
  options?: SessionOptions;
  title: string;
  state: SessionState;
  /** Set only when opening a saved session — see Terminal.svelte's savedSessionId prop. */
  savedSessionId?: string;
  /** Reserves a slot in the app-wide "Terminal N" sequence; freed when the pane closes. */
  shellNumber?: number;
  /** Once the user renames a pane, session-driven title updates stop overwriting it. */
  renamed?: boolean;
}

export type SplitDirection = "row" | "column";

export type PaneLayout =
  | { type: "leaf"; paneId: string }
  | { type: "split"; id: string; direction: SplitDirection; children: PaneLayout[]; sizes: number[] };

export function collectPaneIds(node: PaneLayout): string[] {
  if (node.type === "leaf") return [node.paneId];
  return node.children.flatMap(collectPaneIds);
}

/** Replaces the leaf for `targetPaneId` with a new split containing the
 * original leaf and a new leaf for `newPaneId`, in that order. No-op
 * (returns `node` unchanged, same reference) if `targetPaneId` isn't found —
 * callers should always pass a real pane id, but this keeps a stale one
 * from silently corrupting the tree. */
export function insertSplit(
  node: PaneLayout,
  targetPaneId: string,
  direction: SplitDirection,
  newPaneId: string,
  newSplitId: string,
): PaneLayout {
  if (node.type === "leaf") {
    if (node.paneId !== targetPaneId) return node;
    return {
      type: "split",
      id: newSplitId,
      direction,
      children: [node, { type: "leaf", paneId: newPaneId }],
      sizes: [0.5, 0.5],
    };
  }
  return { ...node, children: node.children.map((c) => insertSplit(c, targetPaneId, direction, newPaneId, newSplitId)) };
}

/** Removes the leaf for `paneId` from the tree. A split left with only one
 * child collapses into that child directly, so the tree never accumulates
 * pointless single-child split wrappers. Returns `null` only when removing
 * `paneId` would empty the whole tree — callers with a multi-pane tab
 * should never actually see that; closing the tab's last pane is handled
 * one level up instead of by this function. Sizes are redistributed evenly
 * on removal rather than preserving relative proportions, for simplicity. */
export function removePane(node: PaneLayout, paneId: string): PaneLayout | null {
  if (node.type === "leaf") {
    return node.paneId === paneId ? null : node;
  }
  const remaining = node.children.map((c) => removePane(c, paneId)).filter((c): c is PaneLayout => c !== null);
  if (remaining.length === 0) return null;
  if (remaining.length === 1) return remaining[0];
  const evenSize = 1 / remaining.length;
  return { ...node, children: remaining, sizes: remaining.map(() => evenSize) };
}

/** Updates the sizes of the split identified by `splitId`, wherever it is
 * in the tree. No-op (same shape) if no split with that id is found. */
export function updateSplitSizes(node: PaneLayout, splitId: string, sizes: number[]): PaneLayout {
  if (node.type === "leaf") return node;
  if (node.id === splitId) return { ...node, sizes };
  return { ...node, children: node.children.map((c) => updateSplitSizes(c, splitId, sizes)) };
}

/** Every shellNumber currently in use across all panes (any tab), for
 * nextAvailableNumber() to pick the next free "Terminal N" slot from. */
export function usedShellNumbers(panes: Record<string, PaneState>): number[] {
  return Object.values(panes)
    .map((p) => p.shellNumber)
    .filter((n): n is number => n !== undefined);
}

// --- Layout math -----------------------------------------------------------
// Deliberately pure percentage math, not DOM measurement — kept separate
// from rendering so a split (which changes a leaf's position in its
// parent's children array, and therefore its {#each} key) never has to
// tear down and remount the actual Terminal/RdpView component for a pane
// that already existed. The component layer renders a FLAT list keyed only
// by paneId (which never changes for a pane's lifetime) and positions each
// one absolutely using the rect this computes — layout changes become pure
// style updates instead of remounts.

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

const FULL_RECT: Rect = { left: 0, top: 0, width: 100, height: 100 };

/** Every pane's on-screen rect (in % of the tab's content area), keyed by paneId. */
export function computePaneRects(node: PaneLayout, rect: Rect = FULL_RECT): Record<string, Rect> {
  if (node.type === "leaf") {
    return { [node.paneId]: rect };
  }
  const rects: Record<string, Rect> = {};
  let offset = 0;
  for (let i = 0; i < node.children.length; i++) {
    const size = node.sizes[i] ?? 1 / node.children.length;
    const childRect: Rect =
      node.direction === "row"
        ? { left: rect.left + offset * rect.width, top: rect.top, width: size * rect.width, height: rect.height }
        : { left: rect.left, top: rect.top + offset * rect.height, width: rect.width, height: size * rect.height };
    Object.assign(rects, computePaneRects(node.children[i], childRect));
    offset += size;
  }
  return rects;
}

export interface ResizerDescriptor {
  splitId: string;
  direction: SplitDirection;
  /** Sits between children[index] and children[index + 1]. */
  index: number;
  sizes: number[];
  /** % of the tab container, along the split axis, where the divider line sits. */
  position: number;
  /** % of the tab container — where the divider's cross-axis span starts. */
  crossStart: number;
  /** % of the tab container — how far the divider's cross-axis span runs. */
  crossLength: number;
  /** % of the tab container's size along the split axis — how a resizer
   * converts a drag's pixel delta into a size fraction (the container's
   * actual pixel size along that axis, times this / 100, is the split's
   * own pixel size to divide the delta by). */
  axisLength: number;
}

/** Every resize-handle in the tree (one per gap between adjacent children
 * of every split, at any depth), with everything needed to both place it
 * and drive its drag math. Safe to render from a plain keyed {#each} that
 * gets destroyed/recreated on structural changes — a divider holds no
 * session state, just a transient drag gesture that can't span a
 * structural change in the first place. */
export function collectResizers(node: PaneLayout, rect: Rect = FULL_RECT): ResizerDescriptor[] {
  if (node.type === "leaf") return [];
  const result: ResizerDescriptor[] = [];
  const childRects: Rect[] = [];
  let offset = 0;
  for (let i = 0; i < node.children.length; i++) {
    const size = node.sizes[i] ?? 1 / node.children.length;
    const childRect: Rect =
      node.direction === "row"
        ? { left: rect.left + offset * rect.width, top: rect.top, width: size * rect.width, height: rect.height }
        : { left: rect.left, top: rect.top + offset * rect.height, width: rect.width, height: size * rect.height };
    childRects.push(childRect);
    result.push(...collectResizers(node.children[i], childRect));
    offset += size;
  }
  for (let i = 0; i < node.children.length - 1; i++) {
    const position = node.direction === "row" ? childRects[i].left + childRects[i].width : childRects[i].top + childRects[i].height;
    result.push({
      splitId: node.id,
      direction: node.direction,
      index: i,
      sizes: node.sizes,
      position,
      crossStart: node.direction === "row" ? rect.top : rect.left,
      crossLength: node.direction === "row" ? rect.height : rect.width,
      axisLength: node.direction === "row" ? rect.width : rect.height,
    });
  }
  return result;
}
