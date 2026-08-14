import { describe, expect, it } from "vitest";
import {
  collectPaneIds,
  collectResizers,
  computePaneRects,
  insertSplit,
  removePane,
  updateSplitSizes,
  usedShellNumbers,
  type PaneLayout,
  type PaneState,
} from "./panes";

const leaf = (paneId: string): PaneLayout => ({ type: "leaf", paneId });

describe("collectPaneIds", () => {
  it("returns just the pane id for a leaf", () => {
    expect(collectPaneIds(leaf("a"))).toEqual(["a"]);
  });

  it("flattens a nested tree in order", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    };
    expect(collectPaneIds(tree)).toEqual(["a", "b", "c"]);
  });
});

describe("insertSplit", () => {
  it("replaces the target leaf with a new split containing it and the new pane", () => {
    const result = insertSplit(leaf("a"), "a", "row", "b", "s1");
    expect(result).toEqual({
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("b")],
      sizes: [0.5, 0.5],
    });
  });

  it("finds and splits a leaf nested inside an existing split", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("b")],
      sizes: [0.5, 0.5],
    };
    const result = insertSplit(tree, "b", "column", "c", "s2");
    expect(result).toEqual({
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    });
  });

  it("is a no-op (same shape) when the target pane id doesn't exist", () => {
    const result = insertSplit(leaf("a"), "nonexistent", "row", "b", "s1");
    expect(result).toEqual(leaf("a"));
  });
});

describe("removePane", () => {
  it("removing the only leaf returns null", () => {
    expect(removePane(leaf("a"), "a")).toBeNull();
  });

  it("removing one of two children collapses the split into the remaining child", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("b")],
      sizes: [0.3, 0.7],
    };
    expect(removePane(tree, "b")).toEqual(leaf("a"));
  });

  it("removing one of three children keeps the split and redistributes sizes evenly", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("b"), leaf("c")],
      sizes: [0.2, 0.3, 0.5],
    };
    const result = removePane(tree, "b");
    expect(result).toEqual({
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("c")],
      sizes: [0.5, 0.5],
    });
  });

  it("removing a pane nested in a sub-split collapses just that sub-split", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    };
    const result = removePane(tree, "b");
    expect(result).toEqual({
      type: "split",
      id: "s1",
      direction: "row",
      children: [leaf("a"), leaf("c")],
      sizes: [0.5, 0.5],
    });
  });

  it("is a no-op (same shape) when the target pane id doesn't exist", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b")], sizes: [0.5, 0.5] };
    expect(removePane(tree, "nonexistent")).toEqual(tree);
  });
});

describe("updateSplitSizes", () => {
  it("updates the sizes of the matching split at the root", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b")], sizes: [0.5, 0.5] };
    expect(updateSplitSizes(tree, "s1", [0.3, 0.7])).toEqual({ ...tree, sizes: [0.3, 0.7] });
  });

  it("finds and updates a split nested deeper in the tree", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    };
    const result = updateSplitSizes(tree, "s2", [0.2, 0.8]);
    expect(result).toEqual({
      ...tree,
      children: [leaf("a"), { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.2, 0.8] }],
    });
  });

  it("is a no-op (same shape) when the split id doesn't exist", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b")], sizes: [0.5, 0.5] };
    expect(updateSplitSizes(tree, "nonexistent", [0.1, 0.9])).toEqual(tree);
  });
});

describe("computePaneRects", () => {
  it("gives a leaf the whole area", () => {
    expect(computePaneRects(leaf("a"))).toEqual({ a: { left: 0, top: 0, width: 100, height: 100 } });
  });

  it("splits a row 50/50 left-to-right", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b")], sizes: [0.5, 0.5] };
    expect(computePaneRects(tree)).toEqual({
      a: { left: 0, top: 0, width: 50, height: 100 },
      b: { left: 50, top: 0, width: 50, height: 100 },
    });
  });

  it("splits a column top-to-bottom using uneven sizes", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "column", children: [leaf("a"), leaf("b")], sizes: [0.3, 0.7] };
    expect(computePaneRects(tree)).toEqual({
      a: { left: 0, top: 0, width: 100, height: 30 },
      b: { left: 0, top: 30, width: 100, height: 70 },
    });
  });

  it("recurses correctly into a nested split", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    };
    expect(computePaneRects(tree)).toEqual({
      a: { left: 0, top: 0, width: 50, height: 100 },
      b: { left: 50, top: 0, width: 50, height: 50 },
      c: { left: 50, top: 50, width: 50, height: 50 },
    });
  });

  it("does not change a pane's rect key identity when the tree grows around it (regression: splitting must never remount an existing pane)", () => {
    const before = computePaneRects(leaf("a"));
    const after = computePaneRects(insertSplit(leaf("a"), "a", "row", "b", "s1"));
    // Same key "a" present in both — a flat {#each} keyed by paneId sees
    // this as an update to an existing item, not a remove+add.
    expect(Object.keys(before)).toEqual(["a"]);
    expect(Object.keys(after).sort()).toEqual(["a", "b"]);
  });
});

describe("collectResizers", () => {
  it("returns nothing for a single leaf", () => {
    expect(collectResizers(leaf("a"))).toEqual([]);
  });

  it("places one resizer at the boundary of a two-way row split", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b")], sizes: [0.4, 0.6] };
    const resizers = collectResizers(tree);
    expect(resizers).toHaveLength(1);
    expect(resizers[0]).toMatchObject({ splitId: "s1", direction: "row", index: 0, position: 40, crossStart: 0, crossLength: 100, axisLength: 100 });
  });

  it("places two resizers for a three-way split", () => {
    const tree: PaneLayout = { type: "split", id: "s1", direction: "row", children: [leaf("a"), leaf("b"), leaf("c")], sizes: [0.2, 0.3, 0.5] };
    const resizers = collectResizers(tree);
    expect(resizers.map((r) => r.position)).toEqual([20, 50]);
  });

  it("scopes a nested split's resizer to its own (smaller) rect", () => {
    const tree: PaneLayout = {
      type: "split",
      id: "s1",
      direction: "row",
      children: [
        leaf("a"),
        { type: "split", id: "s2", direction: "column", children: [leaf("b"), leaf("c")], sizes: [0.5, 0.5] },
      ],
      sizes: [0.5, 0.5],
    };
    const resizers = collectResizers(tree);
    const nested = resizers.find((r) => r.splitId === "s2");
    // s2's own rect is {left:50, top:0, width:50, height:100} — s1 (row)
    // only subdivides width, so s2 inherits the full height from s1's rect.
    // axisLength for a column split is that height, not the width.
    expect(nested).toMatchObject({ direction: "column", position: 50, crossStart: 50, crossLength: 50, axisLength: 100 });
  });
});

describe("usedShellNumbers", () => {
  const pane = (shellNumber?: number): PaneState => ({
    id: crypto.randomUUID(),
    protocol: "shell",
    title: "Terminal",
    state: "connected",
    shellNumber,
  });

  it("collects shellNumber across every pane, ignoring panes without one", () => {
    const panes: Record<string, PaneState> = {
      a: pane(1),
      b: pane(undefined),
      c: pane(3),
    };
    expect(usedShellNumbers(panes).sort()).toEqual([1, 3]);
  });

  it("returns an empty array for no panes", () => {
    expect(usedShellNumbers({})).toEqual([]);
  });
});
