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
  | { type: "error"; message: string };

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
  workingDir?: string | null;
}

export type SessionOptions = SshConnectOptions | SerialConnectOptions | RdpConnectOptions | ShellConnectOptions | undefined;

/** `hostId` is set only when this tab is a saved host's session — it's what
 * unlocks scrollback persistence for a saved shell preset on the backend
 * (see portus_core::scrollback). An ad-hoc tab has no stable identity to
 * persist scrollback under, so it's omitted for those. */
export async function openSession(protocol: Protocol, options?: SessionOptions, hostId?: string): Promise<string> {
  return invoke<string>("session_open", { protocol, options: options ?? null, hostId: hostId ?? null });
}

export async function listSerialPorts(): Promise<string[]> {
  return invoke<string[]>("list_serial_ports");
}

// --- Saved hosts -----------------------------------------------------------
// Mirrors portus-core's Config/Host/AuthMethod. AuthMethodDto's handle
// fields are opaque keychain references, not secrets — never used directly
// as a password. Call resolveHostSecret() to get the real value back.

export interface AuthMethodDto {
  type: "none" | "password" | "privateKey";
  credentialHandle?: string;
  path?: string;
  passphraseHandle?: string | null;
}

export interface Host {
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
  workingDir?: string | null;
}

export interface Group {
  id: string;
  name: string;
  parentId?: string | null;
  collapsed: boolean;
}

export interface PortusConfig {
  schemaVersion: number;
  groups: Group[];
  hosts: Host[];
  settings: { terminalFontFamily: string; terminalFontSize: number };
}

export async function getConfig(): Promise<PortusConfig> {
  return invoke<PortusConfig>("get_config");
}

export async function saveConfig(config: PortusConfig): Promise<void> {
  await invoke("save_config", { config });
}

/** What save_host expects for the auth half — the raw secret, not a handle.
 * `unchanged` reuses whatever the host being edited already has stored,
 * without touching the keychain — lets an edit dialog leave the credential
 * field blank instead of forcing a retype on every edit. */
export type AuthInput =
  | { type: "none" }
  | { type: "unchanged" }
  | { type: "password"; password: string }
  | { type: "privateKey"; path: string; passphrase?: string | null };

export interface SaveHostInput {
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
  workingDir?: string | null;
}

export async function saveHost(input: SaveHostInput): Promise<PortusConfig> {
  return invoke<PortusConfig>("save_host", { ...input });
}

export async function deleteHost(hostId: string): Promise<PortusConfig> {
  return invoke<PortusConfig>("delete_host", { hostId });
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

/** Pulls a saved host's password/passphrase back out of the keychain. `null`
 * if the host has no stored credential (AuthMethod::None, or a private key
 * with no saved passphrase). */
export async function resolveHostSecret(hostId: string): Promise<string | null> {
  return invoke<string | null>("resolve_host_secret", { hostId });
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
