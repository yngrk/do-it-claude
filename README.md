# AI Dev Team

A multi-agent development setup for [Claude Code](https://marketplace.visualstudio.com/items?itemName=anthropic.claude-code). A **Manager** agent coordinates specialized **subagents** that build your project in parallel.

## Quick Start

1. Install the [Claude Code VSCode extension](https://marketplace.visualstudio.com/items?itemName=anthropic.claude-code)
2. Configure your Anthropic API key
3. Open this folder in VSCode and start a conversation

`CLAUDE.md` loads automatically and activates the manager. Just describe what you want built.

## Agents

| Agent | Model | Role |
|-------|-------|------|
| `architect` | Opus | Designs system structure and implementation plans |
| `coder` | Sonnet | Implements features, fixes bugs, refactors code |
| `reviewer` | Sonnet | Reviews code quality and correctness (read-only) |
| `tester` | Sonnet | Writes and runs tests, diagnoses failures |
| `researcher` | Sonnet | Investigates codebases, APIs, and documentation |

Agent definitions live in `.claude/agents/`. Each uses YAML frontmatter to configure its role, tools, and model.

## How It Works

You talk to the manager. The manager breaks your request into tasks, spawns the right agents, and reports back. You never interact with subagents directly.

When multiple agents need to write files simultaneously, they run in isolated [git worktrees](https://git-scm.com/docs/git-worktree) to avoid conflicts.

## Project Structure

```
├── CLAUDE.md              # Manager persona (loaded automatically)
├── project/               # Working directory for built applications
└── .claude/
    └── agents/
        ├── architect.md
        ├── coder.md
        ├── reviewer.md
        ├── tester.md
        └── researcher.md
```

## Customization

Edit files in `.claude/agents/` to modify agent behavior. Add new `.md` files to create new agent types — the manager picks them up automatically.

Key frontmatter fields: `name`, `description`, `tools`, `model` (`sonnet` | `opus` | `haiku`), `maxTurns`.
