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

/** Emitted alongside a connect dialog's `connect` event when the user
 * checked "save this connection" — just the display name, since everything
 * else needed to build the saved `Host` is already in the connect options. */
export interface SaveRequest {
  name: string;
}

export type SessionOptions = SshConnectOptions | SerialConnectOptions | undefined;

export async function openSession(protocol: Protocol, options?: SessionOptions): Promise<string> {
  return invoke<string>("session_open", { protocol, options: options ?? null });
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

/** What save_host expects for the auth half — the raw secret, not a handle. */
export type AuthInput =
  | { type: "none" }
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
}

export async function saveHost(input: SaveHostInput): Promise<PortusConfig> {
  return invoke<PortusConfig>("save_host", { ...input });
}

export async function deleteHost(hostId: string): Promise<PortusConfig> {
  return invoke<PortusConfig>("delete_host", { hostId });
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

export interface SessionSubscription {
  unlisten(): Promise<void>;
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
