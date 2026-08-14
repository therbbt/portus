# Portus

A lightweight, cross-platform, multi-protocol connection manager and
terminal. One window, tabbed sessions over SSH, serial, and a local shell,
plus SFTP file transfer and view-only RDP — built in Rust (Tauri) and
Svelte, with a UI deliberately kept close to a plain terminal app rather
than a heavyweight IDE.

## What it does

- **Local shell** tabs backed by a real PTY (portable-pty), with automatic
  tab numbering that reuses freed slots and double-click-to-rename.
- **SSH** sessions (russh) with password or private-key authentication and
  trust-on-first-use host-key verification, tracked in a `known_hosts.json`
  sibling to the config file.
- **Serial** connections (serialport) with auto-detected available ports
  and common baud-rate presets.
- **SFTP** file browsing and transfer over an existing SSH connection —
  list, upload/download, delete, create/remove directories — in a side
  panel next to the terminal.
- **RDP** (IronRDP) — connects and renders the remote framebuffer; currently
  view-only, no keyboard/mouse input forwarding yet.
- **Saved hosts**: save a connection's address/credentials from any connect
  dialog, reconnect with one click from the sidebar, and edit a saved
  host's details later without retyping its password (only its handle is
  kept in the OS keychain — password/passphrase can be left blank on edit
  to keep the stored value).
- **Settings** for the terminal's font family and size, applied to every
  new tab opened afterward.
- A window, sidebar, and unified action bar styled to match FlashPad, a
  sibling app, including the resizable sidebar, transparent rounded window
  chrome, and matching accent color.

Not yet implemented: telnet (scaffolded crate, no working session yet),
RDP input forwarding, and host groups/folders in the sidebar.

## Layout

```
crates/portus-core     the Session trait, event/command types, config, keychain
crates/portus-shell     portable-pty: local shell Session
crates/portus-ssh       russh: SSH Session + host-key (TOFU) verification
crates/portus-serial    serialport: serial Session
crates/portus-sftp      SFTP over an SSH session
crates/portus-rdp       ironrdp: view-only RDP client
crates/portus-telnet    telnet Session (scaffolded, not yet implemented)
app/portus              Tauri app: commands, event bridge, window
frontend/               Svelte + xterm.js
```

The app crate only ever depends on `portus-core`'s `Session` trait — never on
a protocol crate's internals directly. Every protocol lives behind that
trait in its own crate, and the on-disk config is a single hand-editable
`config.json`; secrets are never written to it, only opaque keychain handles
are.

## Development

Requires Rust, Node, and (on Linux) `webkit2gtk-4.1` plus a Secret
Service provider (GNOME Keyring, KWallet) for the keychain backend.

```sh
npm install --prefix frontend
npm run tauri --prefix frontend -- dev
```

(`tauri.conf.json` lives in `app/portus`, not next to `frontend/package.json`,
so the `tauri` script `cd`s there before invoking the CLI — that's also why
a bare `npx tauri dev` from `frontend/` won't find the config.)

`cargo check --workspace` and `cd frontend && npx svelte-check` cover the
Rust and frontend sides independently. Each protocol crate also has its own
integration tests that spin up a real counterpart (a local sshd, a virtual
serial pair, etc.) rather than mocking the protocol.
