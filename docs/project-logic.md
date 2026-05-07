# Project Logic

This document is the contributor map for ThinWedge. Read it before changing
code so you can place work in the right subsystem and choose the right tests.

## Product shape

ThinWedge is a local FP&A agent terminal. Users install a CLI, run a terminal UI
or one-shot command, authenticate with an OpenRouter-compatible token, and let
the agent work inside a local repository or modeling workspace.

The product is local-first:

- User state lives under `THINWEDGE_HOME`.
- Authentication, logs, thread history, goals, and local tool state stay on the
  user's machine.
- Shell execution, file edits, sandboxing, approvals, and tool calls are routed
  through the Rust runtime.
- Published npm packages are thin wrappers around platform-specific native
  binaries built by CI.

## Repository layout

- `thinwedge-rs/` is the main Rust workspace. Most CLI, TUI, agent runtime,
  tool, sandbox, and local-state changes belong here.
- `thinwedge-cli/` builds and packages the npm wrapper that installs the native
  binaries for each supported platform.
- `sdk/` contains language SDKs and runtime support packages.
- `docs/` contains user, contributor, release, security, and architecture
  documentation.
- `.github/` contains pull request checks, issue templates, release workflows,
  CODEOWNERS, and helper scripts.
- `.circleci/config.yml` builds and publishes full multi-platform CLI releases
  from `rust-v*` tags.

## Runtime flow

At a high level, ThinWedge flows like this:

1. A user starts `thinwedge`, `thinwedge exec`, or another CLI command.
2. The CLI loads configuration, auth, workspace context, and persisted session
   state from `THINWEDGE_HOME`.
3. The TUI or non-interactive command creates an agent session.
4. The agent runtime decides whether to answer, inspect files, run commands,
   update plans, request approvals, call tools, or coordinate side agents.
5. Tool calls pass through local permission and sandbox boundaries before they
   affect the filesystem, shell, network, or external services.
6. Thread history, goal state, logs, and relevant local mirrors are persisted so
   work can resume.

Keep these boundaries intact. UI code should not bypass runtime permission
checks, and low-level tool code should not make product-policy decisions that
belong in the agent or configuration layer.

## Major subsystems

- **CLI and TUI:** command entrypoints, interactive chat, slash commands,
  status panes, permissions prompts, and user-visible workflows.
- **Agent runtime:** session state, model calls, tool orchestration, side
  agents, review behavior, goals, and planning behavior.
- **Tools:** shell execution, file operations, patch application, MCP resources,
  plugin tools, cost tools, and FP&A/statistical-model tools.
- **Sandboxing and approvals:** local execution boundaries, escalation prompts,
  filesystem policy, and command safety behavior.
- **Persistence:** configuration, auth, logs, thread history, goal state, and
  local mirrors under `THINWEDGE_HOME`.
- **Packaging and release:** npm staging, platform-native binaries, GitHub
  release assets, and CircleCI tag publishing.

## Where common changes belong

- CLI command behavior usually starts in `thinwedge-rs/cli` or the relevant
  command crate, then updates docs and packaging tests if user-visible.
- TUI interaction changes usually start in `thinwedge-rs/tui` and need
  terminal-state or snapshot-style tests when practical.
- Agent/tool behavior usually belongs in the relevant runtime or tool crate
  under `thinwedge-rs/`, with tests for permission and failure paths.
- npm install or binary-layout changes usually belong in `thinwedge-cli/` and
  must preserve the platform package contract used by CircleCI.
- Release automation changes must keep tag-only publishing and must not allow
  contributor PRs to publish packages.

## Contributor safety rules

- Do not add secrets, tokens, private paths, or generated credentials.
- Do not weaken sandboxing, approval prompts, or filesystem restrictions without
  a linked design discussion.
- Do not make release publishing run on pull requests.
- Keep public docs aligned with the actual supported install path and CI gates.
- Prefer small, testable PRs over broad refactors.
