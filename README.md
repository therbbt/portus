# Portus

A lightweight, cross-platform, multi-protocol connection manager and terminal.
One window, tabbed sessions over SSH, RDP, Telnet, serial, and a local shell,
with SFTP file transfer.

## Layout

```
crates/portus-core     the Session trait, event/command types, config, keychain
crates/portus-shell     portable-pty: local shell Session
crates/portus-ssh       russh: SSH Session (Milestone 3, stubbed)
crates/portus-telnet    telnet Session (Milestone 4, stubbed)
crates/portus-serial    serialport Session (Milestone 4, stubbed)
crates/portus-rdp       ironrdp Session (Milestone 6, stubbed)
crates/portus-sftp      SFTP over an SSH session (Milestone 5, stubbed)
app/portus              Tauri app: commands, event bridge, window
frontend/               Svelte + xterm.js
```

The app crate only ever depends on `portus-core`'s `Session` trait — never on
a protocol crate's internals directly.

## Development

Requires Rust, Node, and (on Linux) `webkit2gtk-4.1`.

```sh
npm install --prefix frontend
npm run tauri --prefix frontend -- dev
```

(`tauri.conf.json` lives in `app/portus`, not next to `frontend/package.json`,
so the `tauri` script `cd`s there before invoking the CLI — that's also why
a bare `npx tauri dev` from `frontend/` won't find the config.)

`cargo check --workspace` and `cd frontend && npx svelte-check` cover the
Rust and frontend sides independently.

## Status

Milestone 1 (skeleton) and Milestone 2 (local shell + xterm.js) are done:
the command/event bridge, FlashPad-matched theme tokens, and a local-shell
tab backed by a real PTY all work end to end. SSH/Telnet/Serial/RDP/SFTP are
scaffolded as isolated crates with `Session` stubs, pending their own
milestones.
