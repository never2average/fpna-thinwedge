## Why ThinWedge Is Open Source

ThinWedge is open source because finance automation needs trust at the same
level as the systems it touches.

The CLI is meant to work inside real repositories, spreadsheets, databases,
cloud accounts, model workflows, and local workspaces. Those workflows can
involve sensitive files, shell commands, database sandboxes, cost data, and
approval boundaries. Users should be able to inspect the code, review the
permission model, run it locally, file issues in public, and fork the project if
the maintainers make choices that do not fit their environment.

## What We Mean By Always Open Source

The ThinWedge CLI core will remain open source. The public repository, local
agent runtime, CLI/TUI surfaces, sandbox and approval model, FP&A tool wiring,
and contributor documentation are intended to stay public under the Apache-2.0
license.

We may build paid hosted services, support, managed integrations, or enterprise
deployment help around ThinWedge. Those services should be additive. They should
not require converting the local CLI core into a closed-source product or making
public users dependent on an opaque hosted control plane for basic local use.

The practical commitment is:

- The current public code remains available under Apache-2.0.
- Future ThinWedge CLI core releases are intended to remain Apache-2.0.
- Core security, sandboxing, release, and contribution docs stay public.
- Public users should be able to install, run, inspect, build, fork, and patch
  the CLI without asking us for permission.
- Hosted services may exist, but the local-first CLI should remain useful on its
  own.

## Why This Matters For Finance Workflows

Technical finance operators often sit between FP&A, data engineering, infra,
and software teams. They need tools that can explain themselves.

Open source makes that possible:

- **Security review:** approvals, sandboxing, and secret-handling behavior can be
  inspected instead of trusted as marketing copy.
- **Auditability:** finance teams can see how local state, logs, DB sandbox
  metadata, and tool calls are handled.
- **Extensibility:** contributors can add connectors, examples, probes, skills,
  and provider support without waiting for a vendor roadmap.
- **Portability:** teams can keep using or forking the CLI if their infra,
  compliance, or provider choices diverge from ours.
- **Credibility:** early users can verify what works today and challenge what is
  missing in public.

## License And Attribution

ThinWedge is licensed under [Apache-2.0](../LICENSE). It includes software
derived from [OpenAI Codex](https://github.com/openai/codex), which is also
Apache-2.0 licensed. ThinWedge preserves the required notices in
[NOTICE](../NOTICE) and [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

ThinWedge is not affiliated with or endorsed by OpenAI. See
[License](./license.md) for the detailed attribution and trademark boundary.
