# Third-Party Notices

This file summarizes third-party code and binary components that are intentionally included in this repository or release packages. It is not a substitute for the full license texts in the referenced source trees.

## OpenAI Codex

- Use: ThinWedge is derived from OpenAI Codex.
- License: Apache License 2.0.
- Notice: OpenAI Codex, Copyright 2025 OpenAI.

## Ratatui

- Use: Derived terminal code in `thinwedge-rs/tui/src/custom_terminal.rs`.
- License: MIT.
- Notice: Copyright (c) 2016-2022 Florian Dehau; Copyright (c) 2023-2025 The Ratatui Developers.

## WezTerm

- Use: Derived Windows PTY code under `thinwedge-rs/utils/pty/src/win/`.
- License: MIT.
- Notice: Copyright (c) 2018-Present Wez Furlong.

## ripgrep

- Use: Prebuilt `rg` binaries are bundled in native npm packages from the manifest at `thinwedge-cli/bin/rg`.
- License: MIT OR Unlicense, per the upstream ripgrep project.
- Upstream: https://github.com/BurntSushi/ripgrep

## bubblewrap

- Use: Vendored Linux sandbox source under `thinwedge-rs/vendor/bubblewrap/`. Default public release builds do not link this code into distributed binaries; they require system `bwrap` on PATH.
- License: LGPL-2.0-or-later. The full license text is in `thinwedge-rs/vendor/bubblewrap/COPYING`.
- Notice: Copyright (C) 2016 Alexander Larsson and other bubblewrap contributors.
- Public release note: vendored bubblewrap embedding is opt-in via `THINWEDGE_ENABLE_VENDORED_BWRAP=1`; builds that enable it must satisfy the applicable LGPL obligations for those binaries.
