# Permission Autonomy Research

Status: current research snapshot for the ThinWedge `:power-user` preset.

## Conclusion

The best low-friction safety model is bounded autonomy: let the agent act freely inside an explicit sandbox, prompt when it crosses a trust boundary, deny known-catastrophic actions, and log what happened. This avoids asking yes/no for every command without turning the environment into an unrestricted YOLO shell.

For ThinWedge, the recommended power-user shape is:

```toml
approval_policy = "on-request"
approvals_reviewer = "auto_review"
default_permissions = ":power-user"
```

`on-request` keeps the approval path available for boundary crossings. `auto_review` reduces approval fatigue for policy-compliant sandbox escalations. `:power-user` keeps workspace-write behavior, protects workspace metadata, denies obvious secret reads, and keeps network restricted unless an explicit policy enables it.

## Product Patterns

- A version-controlled-folder agent pattern uses workspace writes plus on-request approvals, and separates filesystem sandboxing from approval policy with explicit configuration surfaces. Sources: https://developers.openai.com/codex/agent-approvals-security, https://developers.openai.com/codex/config-reference.
- Command-prefix rules with deny precedence are a useful pattern for auditable shell policy. ThinWedge already mirrors this through `thinwedge-execpolicy`, so the implementation should extend existing prefix-rule policy rather than inventing a parallel shell classifier. Source: https://developers.openai.com/codex/rules.
- Claude Code uses `allow`, `ask`, and `deny` permission lists plus permission modes such as `default`, `acceptEdits`, `plan`, `auto`, and `bypassPermissions`. Its sandboxing docs frame sandbox boundaries as the way to reduce repeated command prompts while preserving safety. Sources: https://code.claude.com/docs/en/permissions, https://code.claude.com/docs/en/permission-modes, https://code.claude.com/docs/en/sandboxing.
- Windsurf Cascade exposes terminal auto-execution levels: disabled, allowlist-only, auto, and turbo. It also has command allow and deny lists, with deny taking precedence. This supports the same model: auto-run trusted routine commands, keep denies authoritative, and leave high-impact actions gated. Source: https://docs.windsurf.com/windsurf/terminal.
- Cline exposes auto-approve categories for project reads, project edits, safe commands, all commands, browser, MCP, and YOLO mode. Its docs explicitly distinguish common safe examples such as tests and status commands from installs and destructive commands. Source: https://docs.cline.bot/features/auto-approve.
- Continue CLI separates read-only operation from writable/editable/tool permissions with explicit allow and exclude controls. This supports preserving a read-only fallback for untrusted or non-version-controlled directories. Source: https://docs.continue.dev/cli/tool-permissions.
- Gemini CLI has approval modes, sandbox configuration, sandbox allowed paths, sandbox network access, and trusted-folder behavior. It disables tool auto-acceptance in untrusted safe mode. Source: https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/settings.md.
- GitHub Copilot's cloud coding agent uses a firewall model and recommends dependency-oriented allowlists. The relevant ThinWedge lesson is that network should be a separately controlled capability, not implied by file-write autonomy. Source: https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/customize-the-agent-firewall.
- OWASP agent security guidance emphasizes least privilege, action previews, audit trails, rollback, and explicit approval for high-impact actions. Approval dialogs should not be the only safety boundary. Sources: https://cheatsheetseries.owasp.org/cheatsheets/AI_Agent_Security_Cheat_Sheet.html, https://owasp.org/www-community/attacks/Lies_in_the_Loop.

## ThinWedge Policy

Default power-user behavior:

- Allow repo-local edits, tests, builds, formatters, read-only inspection, and non-network local commands inside the sandbox.
- Prompt for package installs, arbitrary network commands, SSH, cloud CLIs, Docker/Kubernetes, database migrations, long-running services, git publish/history operations, and destructive filesystem operations.
- Forbid obvious catastrophic or exfiltration-oriented commands: root/home deletion, broad permission changes, privilege escalation, disabling security controls, direct secret-file uploads, and direct edits to protected metadata.
- Keep `.git`, `.agents`, and `.thinwedge` protected under writable roots.
- Deny reads of common secret patterns by default: `.env`, secret, token, pem, and private-key-like paths.
- Keep network restricted unless the user selects an explicit network profile or grants a command-specific permission.

## Implementation Notes

The `:power-user` built-in should be opt-in and should not change existing `:workspace`, `:read-only`, or `:danger-no-sandbox` behavior. It should reuse `PermissionProfile::workspace_write_with` so metadata carveouts and additional writable roots keep the current semantics, then overlay deny-read glob entries and force network to restricted.

The command policy should remain in `thinwedge-execpolicy` prefix rules. Prefix rules are easier to audit than shell-string regex and already merge by strictest decision, which matches the researched products' deny-precedence pattern.
