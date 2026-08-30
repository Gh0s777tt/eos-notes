# Changelog

All notable changes to this repository. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**How this file was built.** Reconstructed from `git log` on 2026-08-30. Every entry names the
commit that introduced it. This repository carries **no tags**, so all work sits under
`[Unreleased]`; it is released as part of the E-OS image, versioned by the orchestrator.

## [Unreleased]

### Added

- feat: E-OS Notes v1 — Crimson notes app (Slint + SQLite/WAL) ([`20146a302`](https://gitlab.com/e-os/eos-notes/-/commit/20146a302), 2026-07-17)
- feat: drive Slint 1.17 over a custom orbclient platform backend ([`19e1bb64e`](https://gitlab.com/e-os/eos-notes/-/commit/19e1bb64e), 2026-07-17)

### Changed

- refactor: use the shared eos-ui crate for the Slint-on-Orbital backend ([`9f9eae6e7`](https://gitlab.com/e-os/eos-notes/-/commit/9f9eae6e7), 2026-07-18)

### Fixed

- fix: pin the Redox-proven Slint 1.1.1 and gate the GUI behind a feature ([`30edcbd11`](https://gitlab.com/e-os/eos-notes/-/commit/30edcbd11), 2026-07-17)
- fix: register the image's DejaVu fonts with fontique at startup ([`829ac5e62`](https://gitlab.com/e-os/eos-notes/-/commit/829ac5e62), 2026-07-18)
- fix: default DISPLAY to the orbital scheme for shell launches ([`5ca5c4903`](https://gitlab.com/e-os/eos-notes/-/commit/5ca5c4903), 2026-07-18)
- fix: accept typed text via orbital TextInput events ([`bad75e570`](https://gitlab.com/e-os/eos-notes/-/commit/bad75e570), 2026-07-18)
