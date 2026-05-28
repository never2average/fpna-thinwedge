# Getting Started With ThinWedge

ThinWedge is an open-source CLI for technical finance workflows. It is meant for
people who work close to finance models, repositories, databases, infrastructure,
and agent-assisted analysis.

## Install

Install the published npm package:

```shell
npm install -g @never2average-does-npm/cli
```

Confirm the command resolves:

```shell
thinwedge --version
```

If you prefer standalone binaries, use the latest GitHub release:

https://github.com/never2average/fpna-thinwedge/releases/latest

## Authenticate

ThinWedge uses an OpenRouter-compatible provider token.

```shell
thinwedge login
```

You can also set the token first:

```shell
export OPENROUTER_API_KEY=...
thinwedge login
```

Optional prompts can configure capabilities such as Artificial Analysis, RunPod,
AWS profile and region values, and Neon DB sandbox metadata.

## First Useful Commands

Run a one-shot task in a repository:

```shell
thinwedge exec "summarize this repository and identify the finance or data workflows it contains"
```

Start the interactive terminal UI:

```shell
thinwedge
```

Open DB sandbox setup help:

```shell
thinwedge db-sandbox --help
```

## What To Try First

Good first workflows:

- ask ThinWedge to summarize a finance, analytics, or infra repository,
- ask it to inspect a spreadsheet/modeling codebase and produce a TODO list,
- run `/goal` in the TUI for a longer investigation,
- configure a Neon-backed DB sandbox and run a dry-run preflight,
- compare LLM or cloud cost options before selecting a model or VM.

## Report Feedback

Open an issue with:

- OS and CPU architecture,
- Node and npm versions,
- exact install command,
- exact failure output,
- the finance, data, infra, or DB-sandbox workflow you wanted to run.

Issues:

https://github.com/never2average/fpna-thinwedge/issues
