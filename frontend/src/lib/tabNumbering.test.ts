import { describe, expect, it } from "vitest";
import { nextAvailableNumber } from "./tabNumbering";

describe("nextAvailableNumber", () => {
  it("returns 1 when nothing is used", () => {
    expect(nextAvailableNumber([])).toBe(1);
  });

  it("fills the lowest gap", () => {
    expect(nextAvailableNumber([1, 2, 3, 5])).toBe(4);
    expect(nextAvailableNumber([2, 3])).toBe(1);
  });

  it("reuses numbers freed by closing all tabs instead of counting up forever", () => {
    // Open 5 local shell tabs.
    let used: number[] = [];
    for (let i = 0; i < 5; i++) {
      used.push(nextAvailableNumber(used));
    }
    expect(used).toEqual([1, 2, 3, 4, 5]);

    // Close all of them.
    used = [];

    // The next tab should be "Local Shell 1" again, not 6.
    expect(nextAvailableNumber(used)).toBe(1);
  });

  it("reuses only the number freed by closing a single tab", () => {
    const used = [1, 2, 4, 5]; // tab numbered 3 was closed
    expect(nextAvailableNumber(used)).toBe(3);
  });
});
