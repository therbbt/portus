/** Matches a `sudo`-style interactive password prompt sitting at the end of
 * freshly-arrived terminal output — `[sudo] password for bob: ` (Linux's
 * usual sudo), a bare `Password: ` (su, some sudo builds/locales), or a
 * variant with different trailing whitespace. Anchored to the *end* of the
 * string (no `m` flag, so `$` means end-of-string, not end-of-line) since
 * this is meant to run against a short rolling tail of recent output, not
 * search the whole scrollback for the word appearing anywhere.
 *
 * Used to decide whether to auto-type a saved SSH password back — see
 * Terminal.svelte's own, much longer comment on why that's a deliberate,
 * knowingly-accepted trade-off (a malicious/compromised remote host could
 * spoof this same pattern to get its own password typed somewhere
 * unintended) rather than something to broaden carelessly. Keep this
 * narrow: a prompt pattern that's too permissive directly widens that
 * attack surface, not just the false-positive rate. */
const SUDO_PASSWORD_PROMPT = /(\[sudo\]\s+)?password(\s+for\s+\S+)?:\s*$/i;

export function looksLikeSudoPasswordPrompt(text: string): boolean {
  return SUDO_PASSWORD_PROMPT.test(text);
}
