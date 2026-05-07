## Contributing

ThinWedge accepts public contributions through an open but gated process. The
goal is to make useful outside work welcome while keeping the CLI stable,
secure, and maintainable for finance and operations users.

Before opening a pull request, start with an issue unless the change is a small
documentation fix. Use the issue to agree on the problem, expected behavior, and
rough approach. PRs that introduce unrelated scope or bypass an unresolved
design discussion may be closed or redirected.

### Good first contributions

- Reproducible bug reports with logs, platform details, and a minimal example.
- Documentation fixes that make install, configuration, or contributor setup
  easier to follow.
- Small bug fixes linked to an accepted issue.
- Focused tests that document existing behavior or catch a known regression.

Large feature work, release automation, permission-policy changes, and security
or sandboxing changes need maintainer discussion before implementation.

### Development workflow

1. Fork the repository and create a topic branch from `main`, for example
   `fix/terminal-resize`.
2. Keep changes focused. Use separate PRs for unrelated fixes.
3. Link the issue in the PR description.
4. Add or update tests when behavior changes.
5. Update user-facing docs or help text when user-visible behavior changes.
6. Run the relevant local checks before marking the PR ready for review.

Common local commands:

```bash
# Rust workspace checks
cd thinwedge-rs
cargo test -p <crate-you-touched>

# Root repository checks
cd ..
just fmt
just fix -p <crate-you-touched>
python3 scripts/check_public_release_readiness.py
```

If you do not have `just` installed, see [`docs/install.md`](./install.md) for
the full setup path. Prefer targeted tests while developing, then run the
broader checks that match the files you touched.

### Project logic

Start with [`docs/project-logic.md`](./project-logic.md) before changing code.
It explains how the repository is organized, how the Rust CLI, npm wrapper,
tool runtime, local state, and release pipeline fit together, and where common
changes usually belong.

### Pull request requirements

Every external PR must satisfy these gates before merge:

- The PR links to an issue, except for trivial documentation-only fixes.
- The PR template is filled out with what changed, why it changed, and how it
  was tested.
- The contributor license agreement check has passed.
- Required CI checks pass.
- At least one maintainer approves the change.
- CODEOWNERS review is satisfied when the touched files require it.
- Review conversations are resolved.

Maintainers may ask contributors to split large PRs, add tests, adjust public
API shape, or move design discussion back to the issue before review continues.

### Model metadata updates

When a change updates model catalogs or model metadata (`/models` payloads,
presets, or fixtures):

- Set `input_modalities` explicitly for any model that does not support images.
- Keep compatibility defaults in mind: omitted `input_modalities` currently
  implies text + image support.
- Ensure client surfaces that accept images, such as TUI paste or attach flows,
  consume the same capability signal.
- Add or update tests that cover unsupported-image behavior and warning paths.

### Review and merge process

One maintainer will be the primary reviewer. Maintainers squash-merge accepted
PRs into `main`; release publishing remains maintainer-controlled and is
triggered by release tags, not by contributor PRs.

Please keep commits understandable while the PR is under review. They do not
need to be perfectly polished because the final merge is squashed.

### Contributor license agreement

All contributors must accept the CLA. The process is lightweight:

1. Open your pull request.
2. Paste the following comment, or reply `recheck` if you have signed before:

   ```text
   I have read the CLA Document and I hereby sign the CLA
   ```

3. The CLA Assistant bot records your signature and marks the status check as
   passed.

### Security and responsible AI

Do not open public issues or PRs for suspected vulnerabilities, exploitable
sandbox escapes, credential leaks, or similar sensitive reports. Follow
[`SECURITY.md`](../SECURITY.md) instead.

For non-sensitive support and design questions, use GitHub Issues. GitHub
Discussions are not enabled for this repository.
