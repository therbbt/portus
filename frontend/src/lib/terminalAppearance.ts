import { writable } from "svelte/store";

// Bumped by App.svelte whenever it applies new font/color CSS vars from
// Settings. Every open Terminal instance subscribes and reacts by
// re-reading those vars fresh via getComputedStyle and pushing the result
// into xterm.js's live theme/font options — so a settings change reaches
// tabs that were already open, not just ones created afterward.
export const terminalAppearanceVersion = writable(0);
