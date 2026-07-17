# E-OS Notes

Crimson-themed notes app for [E-OS](https://gitlab.com/e-os/e-os) — the first
E-OS original application.

- **UI:** [Slint](https://slint.dev) with the software renderer over the winit
  orbital backend (no GPU required — renders under Orbital on Redox/E-OS).
- **Storage:** SQLite in WAL mode (`rusqlite`, bundled), one database at
  `$HOME/.local/share/eos-notes/notes.db`.
- Sidebar with substring search, autosaving editor (title + body), Crimson
  palette (`#0c0202` / `#e50914`).

## Headless self-test

`eos-notes --selftest` proves the storage layer without a display: it creates a
note, reopens the database, verifies the content and search, deletes the note,
and asserts `journal_mode == wal`, printing `EOS-NOTES-SELFTEST-OK`. Used by
boot probes and CI.

## Building

Built as an E-OS recipe (`recipes/gui/eos-notes` in the meta-repo) for
`aarch64-unknown-redox` / `x86_64-unknown-redox`. Host build for development:
`cargo build` (Linux/macOS — the window opens under the local winit backend).

## Hosting

Development and CI live on GitLab (`gitlab.com/e-os/eos-notes`);
`github.com/Gh0s777tt/eos-notes` is a read-only mirror the build recipes fetch
from. License: AGPL-3.0-or-later.
