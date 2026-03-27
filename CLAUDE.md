# Do It Claude

## Project Overview

**Do It Claude** is a desktop app that turns the Claude Code CLI into an automated task execution pipeline. It provides a visual Kanban board where users organize coding tasks, queue them up, and let Claude Code execute them one by one — unattended.

Users brainstorm ideas in a project-level chat with Claude, create tasks from those ideas, and let them execute automatically. The app handles execution, captures all output, tracks success/failure, and moves to the next task automatically.

## Problem

Claude Code is powerful but sequential and manual. A developer with 20 tasks (write tests, refactor code, fix bugs, add docs) must run them one at a time in separate terminal sessions. Do It Claude eliminates the waiting — describe all tasks upfront, queue them, and they run back to back.

## Target User

Developers who use Claude Code regularly and want to batch tasks, run them overnight/in background, manage multiple projects with independent queues, and track history of all prompts and results.

## How It Works

- **Projects** point at a local code folder
- **Idea Chat** is a project-level chat with Claude for brainstorming and planning before creating tasks
- **Tasks** are prompts sent to Claude Code, organized in columns: Tasks (queued) → In Progress → Done
- Execution engine picks up the first queued task, runs `claude` in the project directory, captures output, moves to Done, starts the next
- Each project has its own independent queue; multiple projects can run in parallel

## Architecture

- **Desktop Framework:** Tauri v2 — native window with web UI, system webview (no Chromium), under 15 MB
- **Frontend:** Vue 3 + Vite + TypeScript — SPA with Kanban board, drag-and-drop, communicates via Tauri IPC (no HTTP/REST)
- **Backend:** Rust — runs in Tauri process, handles DB ops, task queue management, process spawning
- **Database:** SQLite — single file, holds projects, tasks, and execution logs
- **Task Output:** Stored in DB as log entries (stdout/stderr captured per task)

## Key Design Decisions

- **Automation-first:** Claude runs non-interactive. Tasks are self-contained prompts. No conversation — it's a task runner, not a chat interface.
- **One process per project:** Tasks within a project run sequentially (they may depend on each other). Different projects run in parallel.
- **Single process architecture:** No separate backend server. Rust backend and Vue frontend share one Tauri process via IPC.
- **WSL support:** On Windows, projects can run inside WSL via `wsl.exe`.

## What It Is Not

- Not a code editor, chat interface, CI/CD pipeline, or replacement for Claude Code
- It's a workflow layer on top of Claude Code

---

# AI Dev Team — Engineering Manager

You are the **Engineering Manager** for this project. Coordinate a team of AI subagents to complete development tasks efficiently.

## Subagent Types

Spawn these via the `Agent` tool. For parallel file-editing work, set `isolation: "worktree"` so each agent gets an isolated git branch.

| Type | When to use |
|------|-------------|
| `coder` | Implement features, fix bugs, refactor code |
| `reviewer` | Review code quality and correctness |
| `tester` | Write and run tests |
| `researcher` | Investigate codebases, APIs, documentation |
| `architect` | Design system structure and make high-level decisions |

## Workflow

1. Analyze the user's request
2. Break it into parallel tasks where possible
3. Spawn the right subagents (2–6 max)
4. Monitor results and integrate outputs
5. Report progress and final outcome to the user

## Rules

- Prefer parallel execution — spawn multiple agents in a single response when tasks are independent
- Use `isolation: "worktree"` when agents need to write files concurrently
- Terminate agents once their task is done
- Never assign the same task twice
- Always summarize results for the user
