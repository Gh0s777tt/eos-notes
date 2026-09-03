# E-OS Notes

Crimson-themed notes app for [E-OS](https://gitlab.com/e-os/e-os) — the first
E-OS original application.

- **UI:** [Slint](https://slint.dev) with the software renderer over the winit
  orbital backend (no GPU required — renders under Orbital on Redox/E-OS).
- **Storage:** SQLite in WAL mode (`rusqlite`, bundled), one database at
  `$HOME/.local/share/eos-notes/notes.db`.
- Sidebar with substring search, autosaving editor (title + body), Crimson
  palette (`#0c0202` / `#e50914`).

## Download

E-OS Notes is built for E-OS first, and it is also downloadable on its own. Two archives are
produced by `packaging/release.sh` and published as pipeline artefacts by the `package-windows`
and `package-linux` jobs:

| system | archive | contents |
|---|---|---|
| Windows x86_64 | `eos-notes-<ver>-x86_64-pc-windows-gnu.zip` | `eos-notes.exe`, `LICENSE`, `README.md`, `assets/` |
| Linux x86_64 | `eos-notes-<ver>-x86_64-unknown-linux-gnu.tar.gz` | `eos-notes`, `LICENSE`, `README.md`, `assets/` |

Each archive is accompanied by a `.sha256` file over the archive itself — the file a person
actually downloads.

**Three things stated rather than left to be discovered.**

1. **The archives are not signed.** Signing product downloads needs a key that a human generates
   and holds outside this repository, so the checksum is all there is today. A checksum proves the
   download was not corrupted in transit; it does not prove who built it.
2. **Linux needs fontconfig present at runtime.** The build deliberately `dlopen`s libfontconfig
   instead of linking it, which is what makes the cross build possible at all; a system without it
   will start and then fail to find fonts. Every mainstream desktop distribution has it.
3. **The GUI needs a windowing backend compiled in**, and until 2026-09-03 the host build had
   none. That binary compiled, packaged and passed `--selftest`, then panicked the moment a window
   was opened: *"No default Slint platform was selected, and no Slint platform was initialized"*.
   The `host-backend` feature is the fix, and the no-backend build now refuses with a message
   instead of pretending to be a GUI application.

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
