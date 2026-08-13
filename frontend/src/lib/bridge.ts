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

export type SessionOptions = SshConnectOptions | SerialConnectOptions | undefined;

export async function openSession(protocol: Protocol, options?: SessionOptions): Promise<string> {
  return invoke<string>("session_open", { protocol, options: options ?? null });
}

export async function listSerialPorts(): Promise<string[]> {
  return invoke<string[]>("list_serial_ports");
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
