import { describe, expect, it } from "vitest";
import { looksLikeSudoPasswordPrompt } from "./sudoPasswordPrompt";

describe("looksLikeSudoPasswordPrompt", () => {
  it("matches real sudo/su prompts", () => {
    expect(looksLikeSudoPasswordPrompt("[sudo] password for bob: ")).toBe(true);
    expect(looksLikeSudoPasswordPrompt("[sudo] password for bob:")).toBe(true);
    expect(looksLikeSudoPasswordPrompt("Password: ")).toBe(true);
    expect(looksLikeSudoPasswordPrompt("password:")).toBe(true);
    // A prompt can arrive as the tail of a larger chunk that also has
    // earlier output in it (e.g. the command echo before the prompt).
    expect(looksLikeSudoPasswordPrompt("$ sudo apt upgrade\n[sudo] password for bob: ")).toBe(true);
  });

  it("does not match the word appearing mid-line, or not at the end", () => {
    expect(looksLikeSudoPasswordPrompt("password: wrong, try again\n")).toBe(false);
    expect(looksLikeSudoPasswordPrompt("Changed password for bob.")).toBe(false);
    expect(looksLikeSudoPasswordPrompt("cat /etc/password")).toBe(false);
    expect(looksLikeSudoPasswordPrompt("")).toBe(false);
  });

  it("is anchored to the end of the string, not mid-buffer", () => {
    // A real prompt earlier in the buffer, but more output since - should
    // not still match once something else has been printed after it.
    expect(looksLikeSudoPasswordPrompt("[sudo] password for bob: \n$ ")).toBe(false);
  });
});
