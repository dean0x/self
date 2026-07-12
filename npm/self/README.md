# @dean0x/self

**self** is a continuous learning layer for [Claude Code](https://claude.ai/code). It manages a persistent memory store (`~/.self`) where observations, skills, and run history accumulate across sessions — letting Claude improve its own behavior over time.

## Install

```sh
npm install -g @dean0x/self
```

Then initialise once:

```sh
self init
```

## Commands

| Command | Description |
|---------|-------------|
| `self init [--reset]` | Create `~/.self`, seed the corpus, install adapters |
| `self status` | Show registry counts, skill stats, and run trends |
| `self doctor` | Audit installation health (exits 1 if findings) |
| `self uninstall` | Remove the marker block and agent files |

## Source

<https://github.com/dean0x/self>
