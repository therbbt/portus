// Thin wrapper around the Tauri command + event bridge described in the
// architecture doc: invoke("session_*", ...) to send commands down into a
// Session, listen("session:<id>:<kind>") to receive its events. This module
// only translates — it holds no session state of its own.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Protocol = "shell" | "echo" | "ssh" | "telnet" | "serial" | "rdp";

export type SessionState = "connecting" | "connected" | "disconnected";

export type SessionEvent =
  | { type: "data"; data: number[] }
  | { type: "title_changed"; title: string }
  | { type: "state_changed"; state: SessionState }
  | { type: "closed"; reason: string | null }
  | { type: "error"; message: string }
  /** The server's SSH host key doesn't match what Portus last recorded for
   * this host — a legitimate key rotation (server rebuilt) or an active
   * MITM look identical from here. The connection has already been
   * refused; `trustHostKey` is the only way to proceed. */
  | { type: "host_key_mismatch"; hostId: string; fingerprint: string; keyBase64: string };

export type SshAuth =
  | { type: "password"; password: string }
  | { type: "privateKey"; path: string; passphrase?: string | null };

export interface SshConnectOptions {
  host: string;
  port?: number;
  username: string;
  auth: SshAuth;
}

export interface SerialConnectOptions {
  portName: string;
  baudRate?: number;
}

export interface RdpConnectOptions {
  host: string;
  port?: number;
  username: string;
  password: string;
  domain?: string | null;
}

/** Both fields optional — an empty object behaves like the old hardcoded
 * $SHELL-in-$HOME default. Only meaningful for a saved shell preset; an
 * ad-hoc "Local shell" tab still opens with no options at all. */
export interface ShellConnectOptions {
  shellCommand?: string | null;
  /** Arguments passed to shellCommand — e.g. ["-d", "Ubuntu"] to launch a
   * specific WSL distro via wsl.exe. See listWslDistros(). */
  shellArgs?: string[] | null;
  workingDir?: string | null;
}

export type SessionOptions = SshConnectOptions | SerialConnectOptions | RdpConnectOptions | ShellConnectOptions | undefined;

/** The caller generates `sessionId` itself (see `newSessionId`) and must
 * subscribe to it (`subscribeSession`) *before* calling this — the backend
 * starts emitting `Connecting`/`Connected` the instant this command spawns
 * the session's task, and a local shell reaches `Connected` fast enough
 * that subscribing only after this resolved routinely missed it, leaving
 * its tab's status dot stuck on "connecting" forever (an SSH session's
 * slower handshake almost always won that race, which is why only local
 * shell tabs showed it). `savedSessionId` is set only when this tab is
 * opening a saved session — it's what unlocks scrollback persistence for a
 * saved shell preset on the backend (see portus_core::scrollback). An
 * ad-hoc tab has no stable identity to persist scrollback under, so it's
 * omitted for those. */
export async function openSession(
  sessionId: string,
  protocol: Protocol,
  options?: SessionOptions,
  savedSessionId?: string,
): Promise<void> {
  await invoke("session_open", { sessionId, protocol, options: options ?? null, savedSessionId: savedSessionId ?? null });
}

/** A session id the caller mints itself, before the session exists on the
 * backend at all — see `openSession` for why that ordering is required. */
export function newSessionId(): string {
  return crypto.randomUUID();
}

export async function listSerialPorts(): Promise<string[]> {
  return invoke<string[]>("list_serial_ports");
}

/** Names of the WSL distros installed on this machine (e.g. "Ubuntu"), for
 * offering them as quick local-shell presets. Always empty off Windows. */
export async function listWslDistros(): Promise<string[]> {
  return invoke<string[]>("list_wsl_distros");
}

// --- Saved sessions ----------------------------------------------------------
// Mirrors portus-core's Config/SavedSession/AuthMethod. AuthMethodDto's
// handle fields are opaque keychain references, not secrets — never used
// directly as a password. Call resolveSessionSecret() to get the real value
// back.

export interface AuthMethodDto {
  type: "none" | "password" | "privateKey";
  credentialHandle?: string;
  path?: string;
  passphraseHandle?: string | null;
}

export interface SavedSession {
  id: string;
  name: string;
  groupId?: string | null;
  protocol: Protocol;
  address: string;
  port?: number | null;
  username?: string | null;
  baudRate?: number | null;
  auth: AuthMethodDto;
  /** Shell-only. */
  shellCommand?: string | null;
  /** Shell-only. */
  shellArgs?: string[] | null;
  /** Shell-only. */
  workingDir?: string | null;
  /** Position among siblings sharing the same groupId, for drag-and-drop
   * reordering in the sidebar — see reorderSession(). */
  sortOrder: number;
}

export interface Group {
  id: string;
  name: string;
  parentId?: string | null;
  collapsed: boolean;
  /** Position among siblings sharing the same parentId — see reorderGroup(). */
  sortOrder: number;
  /** A short, user-set tag (e.g. "PROD") shown as a `- [SLUG]` suffix
   * after a tab's title for any saved session nested under this folder —
   * see TabStrip.svelte, which walks up to the *nearest* ancestor folder
   * that has one set. `undefined`/`null` means no suffix. */
  slug?: string | null;
}

/** Per-machine color overrides — every field optional, `undefined` meaning
 * "use the default for that color" (see tokens.css's matching custom
 * properties, which hold those defaults verbatim). Hex strings like
 * "#8ae234", produced by <input type="color">. Mostly the 16-slot ANSI
 * terminal palette plus a few dedicated (non-ANSI) highlight colors, but
 * also — despite the name — a full set of the app's own UI chrome colors
 * (windowBackground through statusError below, Settings' "App Interface"
 * group): they live in the same theme so one saved theme covers both
 * what a terminal's text looks like and how the app chrome around it
 * looks, rather than needing two separate systems. */
export interface TerminalColors {
  black?: string | null;
  red?: string | null;
  green?: string | null;
  yellow?: string | null;
  blue?: string | null;
  magenta?: string | null;
  cyan?: string | null;
  white?: string | null;
  brightBlack?: string | null;
  brightRed?: string | null;
  brightGreen?: string | null;
  brightYellow?: string | null;
  brightBlue?: string | null;
  brightMagenta?: string | null;
  brightCyan?: string | null;
  brightWhite?: string | null;
  /** Not one of the 16 ANSI slots — overrides `--highlight-green`, the
   * dedicated "success" color terminalHighlight.ts's own rules use (HTTP
   * 2xx, an executable file's permission bits, a BGP session that's
   * Established, ...), independent of whatever `brightGreen` (which
   * tracks the accent, see tokens.css) happens to be. */
  highlightGreen?: string | null;
  /** Overrides `--highlight-get`, an HTTP GET request's own color —
   * independent of `cyan`, which is IP addresses' alone. */
  highlightGet?: string | null;
  /** Overrides `--highlight-url`, a URL's own color — independent of
   * `blue`, which is MAC addresses' alone. */
  highlightUrl?: string | null;
  /** Overrides `--highlight-ipv6`, an IPv6 address's own color —
   * independent of `cyan`, which is IPv4 addresses' alone. */
  highlightIpv6?: string | null;
  /** Not a terminal color at all — overrides `--surface-1`, the sidebar
   * rail's background (also shared by the top action bar, the same
   * visual zone). */
  sidebarBackground?: string | null;
  /** Overrides `--folder-icon-color`, the sidebar's folder icon —
   * separate from `brightBlue`, which colors a directory's *name* inside
   * a terminal, not this icon. */
  folderIcon?: string | null;
  /** Overrides `--status-connected`, the color that marks a tab/pane/
   * session as SSH specifically — separate from the app's one `--accent`,
   * so this can be customized without also changing buttons, focus rings,
   * and the active tab indicator. */
  sshIndicator?: string | null;
  /** Overrides `--surface-0`, the app's outermost background — behind the
   * sidebar, tab strip, and every panel. */
  windowBackground?: string | null;
  /** Overrides `--surface-2` — raised panels: the tab strip itself,
   * dropdown menus, overlays. */
  panelBackground?: string | null;
  /** Overrides `--surface-3` — the hover state for sidebar rows, tabs,
   * and similar list items. */
  hoverBackground?: string | null;
  /** Overrides `--surface-4` — the active tab and a pressed button. */
  activeBackground?: string | null;
  /** Overrides `--fg-primary`, the app's main text color. */
  textPrimary?: string | null;
  /** Overrides `--fg-secondary`, muted text — session names, field labels. */
  textSecondary?: string | null;
  /** Overrides `--fg-tertiary`, the dimmest text — hints, secondary
   * labels, section titles. */
  textTertiary?: string | null;
  /** Overrides `--fg-disabled`, text on a disabled control. */
  textDisabled?: string | null;
  /** Overrides `--status-connecting`, the status dot/text while a
   * session is still connecting. */
  statusConnecting?: string | null;
  /** Overrides `--status-disconnected`, the status dot for a closed
   * session. */
  statusDisconnected?: string | null;
  /** Overrides `--status-error`, the status dot/text for a failed
   * session. */
  statusError?: string | null;
}

/** A saved, named terminal color scheme — the full swatch set exactly as
 * `TerminalColors` describes it, just under a name so it can be picked
 * back out of a list later instead of being the one always-active set of
 * overrides. `terminalColors` (below) is still what's actually applied to
 * every terminal at any given moment; a theme only ever *feeds* that when
 * explicitly picked in Settings, it doesn't get read from live. */
export interface Theme {
  id: string;
  name: string;
  colors: TerminalColors;
}

export interface PortusConfig {
  schemaVersion: number;
  groups: Group[];
  sessions: SavedSession[];
  settings: {
    terminalFontFamily: string;
    terminalFontSize: number;
    terminalColors: TerminalColors;
    themes: Theme[];
    /** Which saved theme (by id) Settings last loaded into terminalColors
     * — purely so re-opening Settings shows the right dropdown selection.
     * `null` means Default (there's no "Custom" state — colors are always
     * either Default's, fixed, or a real saved theme's). Never read by
     * anything that renders a terminal — terminalColors alone still
     * governs that. */
    activeThemeId: string | null;
  };
}

export async function getConfig(): Promise<PortusConfig> {
  return invoke<PortusConfig>("get_config");
}

export async function saveConfig(config: PortusConfig): Promise<void> {
  await invoke("save_config", { config });
}

/** What save_session expects for the auth half — the raw secret, not a
 * handle. `unchanged` reuses whatever the session being edited already has
 * stored, without touching the keychain — lets an edit dialog leave the
 * credential field blank instead of forcing a retype on every edit. */
export type AuthInput =
  | { type: "none" }
  | { type: "unchanged" }
  | { type: "password"; password: string }
  | { type: "privateKey"; path: string; passphrase?: string | null };

export interface SaveSessionInput {
  id?: string | null;
  name: string;
  groupId?: string | null;
  protocol: Protocol;
  address: string;
  port?: number | null;
  username?: string | null;
  baudRate?: number | null;
  auth: AuthInput;
  shellCommand?: string | null;
  shellArgs?: string[] | null;
  workingDir?: string | null;
}

export async function saveSession(input: SaveSessionInput): Promise<PortusConfig> {
  return invoke<PortusConfig>("save_session", { ...input });
}

export async function deleteSession(savedSessionId: string): Promise<PortusConfig> {
  return invoke<PortusConfig>("delete_session", { savedSessionId });
}

/** Drag-and-drop in the sidebar: files a saved session into `groupId` (or
 * the root, if `null`) and/or repositions it among its new siblings.
 * `sortOrder` is typically the midpoint between the two siblings the drop
 * landed between. */
export async function reorderSession(sessionId: string, groupId: string | null, sortOrder: number): Promise<PortusConfig> {
  return invoke<PortusConfig>("reorder_session", { sessionId, groupId, sortOrder });
}

// --- Groups (sidebar folders) ----------------------------------------------

export interface SaveGroupInput {
  id?: string | null;
  name: string;
  parentId?: string | null;
}

export async function saveGroup(input: SaveGroupInput): Promise<PortusConfig> {
  return invoke<PortusConfig>("save_group", { ...input });
}

export async function deleteGroup(groupId: string): Promise<PortusConfig> {
  return invoke<PortusConfig>("delete_group", { groupId });
}

export async function setGroupCollapsed(groupId: string, collapsed: boolean): Promise<PortusConfig> {
  return invoke<PortusConfig>("set_group_collapsed", { groupId, collapsed });
}

/** `slug: null` clears it. See `Group.slug`'s own doc comment. */
export async function setGroupSlug(groupId: string, slug: string | null): Promise<PortusConfig> {
  return invoke<PortusConfig>("set_group_slug", { groupId, slug });
}

/** Drag-and-drop in the sidebar: reparents a folder (or moves it to the
 * root, if `parentId` is `null`) and/or repositions it among its new
 * siblings — see reorderSession(). */
export async function reorderGroup(groupId: string, parentId: string | null, sortOrder: number): Promise<PortusConfig> {
  return invoke<PortusConfig>("reorder_group", { groupId, parentId, sortOrder });
}

/** Pulls a saved session's password/passphrase back out of the keychain.
 * `null` if the session has no stored credential (AuthMethod::None, or a
 * private key with no saved passphrase). */
export async function resolveSessionSecret(savedSessionId: string): Promise<string | null> {
  return invoke<string | null>("resolve_session_secret", { savedSessionId });
}

export async function writeSession(sessionId: string, data: Uint8Array): Promise<void> {
  await invoke("session_write", { sessionId, data: Array.from(data) });
}

export async function resizeSession(sessionId: string, cols: number, rows: number): Promise<void> {
  await invoke("session_resize", { sessionId, cols, rows });
}

export async function closeSession(sessionId: string): Promise<void> {
  await invoke("session_close", { sessionId });
}

/** Overwrites the stored SSH host key for `hostId` ("host:port") with
 * `keyBase64` — call only after the user has explicitly confirmed a
 * `host_key_mismatch` event is expected, then retry `openSession`. */
export async function trustHostKey(hostId: string, keyBase64: string): Promise<void> {
  await invoke("ssh_trust_host_key", { hostId, keyBase64 });
}

// --- SFTP --------------------------------------------------------------
// A file panel, not a terminal session — plain request/response calls
// rather than the session bridge's event stream. Runs over its own SSH
// connection (see portus-sftp), so it takes the same SshConnectOptions.

export interface SftpDirEntry {
  name: string;
  isDir: boolean;
  size: number;
}

export async function sftpConnect(options: SshConnectOptions): Promise<string> {
  return invoke<string>("sftp_connect", { options });
}

export async function sftpList(id: string, path: string): Promise<SftpDirEntry[]> {
  return invoke<SftpDirEntry[]>("sftp_list", { id, path });
}

export async function sftpReadFile(id: string, path: string): Promise<Uint8Array> {
  const bytes = await invoke<number[]>("sftp_read_file", { id, path });
  return new Uint8Array(bytes);
}

/** Streams the remote file straight to `localPath` server-side, rather
 * than round-tripping the whole thing through JS as `sftpReadFile` does —
 * `localPath` should come from a native save dialog (see SftpPanel.svelte),
 * not be typed by hand. */
export async function sftpDownloadFile(id: string, path: string, localPath: string): Promise<void> {
  await invoke("sftp_download_file", { id, path, localPath });
}

export async function sftpWriteFile(id: string, path: string, data: Uint8Array): Promise<void> {
  await invoke("sftp_write_file", { id, path, data: Array.from(data) });
}

export async function sftpRemoveFile(id: string, path: string): Promise<void> {
  await invoke("sftp_remove_file", { id, path });
}

export async function sftpCreateDir(id: string, path: string): Promise<void> {
  await invoke("sftp_create_dir", { id, path });
}

export async function sftpRemoveDir(id: string, path: string): Promise<void> {
  await invoke("sftp_remove_dir", { id, path });
}

export async function sftpDisconnect(id: string): Promise<void> {
  await invoke("sftp_disconnect", { id });
}

// --- RDP -----------------------------------------------------------------
// View-only for now: no write/resize commands, just connect/disconnect and
// a stream of decoded framebuffer updates. Deliberately not a SessionEvent
// — RDP is a framebuffer, not a byte stream, so it gets its own event shape
// and its own <canvas>-based view instead of xterm.js.

export interface RdpFrameUpdate {
  x: number;
  y: number;
  width: number;
  height: number;
  pngBase64: string;
}

export type RdpEvent =
  | { type: "connected"; width: number; height: number }
  | ({ type: "frame" } & RdpFrameUpdate)
  | { type: "disconnected"; reason: string | null }
  | { type: "error"; message: string };

export async function rdpConnect(options: RdpConnectOptions): Promise<string> {
  return invoke<string>("rdp_connect", { options });
}

export async function rdpDisconnect(id: string): Promise<void> {
  await invoke("rdp_disconnect", { id });
}

export interface SessionSubscription {
  unlisten(): Promise<void>;
}

/** Subscribes to every `rdp:<id>:*` channel and dispatches to `onEvent`. */
export async function subscribeRdp(id: string, onEvent: (event: RdpEvent) => void): Promise<SessionSubscription> {
  const kinds = ["connected", "frame", "disconnected", "error"];
  const unlistens: UnlistenFn[] = await Promise.all(
    kinds.map((kind) => listen(`rdp:${id}:${kind}`, (e) => onEvent(e.payload as RdpEvent))),
  );
  return {
    async unlisten() {
      for (const fn of unlistens) fn();
    },
  };
}

/** Subscribes to every `session:<id>:*` channel and dispatches to `onEvent`. */
export async function subscribeSession(
  sessionId: string,
  onEvent: (event: SessionEvent) => void,
): Promise<SessionSubscription> {
  const kinds: Array<[string, (payload: unknown) => SessionEvent]> = [
    ["data", (p) => p as SessionEvent],
    ["title", (p) => p as SessionEvent],
    ["state", (p) => p as SessionEvent],
    ["closed", (p) => p as SessionEvent],
    ["error", (p) => p as SessionEvent],
    ["host_key_mismatch", (p) => p as SessionEvent],
  ];

  const unlistens: UnlistenFn[] = await Promise.all(
    kinds.map(([kind, coerce]) =>
      listen(`session:${sessionId}:${kind}`, (e) => onEvent(coerce(e.payload))),
    ),
  );

  return {
    async unlisten() {
      for (const fn of unlistens) fn();
    },
  };
}
