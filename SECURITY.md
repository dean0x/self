# Security Policy

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via GitHub Security Advisories:
<https://github.com/dean0x/self/security/advisories/new>

GitHub will notify the maintainer and keep the report confidential until a fix is ready.

## Supported versions

The latest `0.x` release receives security fixes. No backports are made to older versions.

## What this tool does

`self` is a local CLI. It:

- Writes files to `~/.claude/` (preamble block, agent definitions, settings.json permissions) and `~/.self/` (corpus skeleton, git-tracked learning data)
- Runs `git -C ~/.self` for local version control of the corpus
- Makes **no network calls** and collects **no telemetry**

All data stays on the local machine.
