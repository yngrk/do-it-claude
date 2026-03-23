# Do It Claude

A desktop app that turns the Claude Code CLI into an automated task execution pipeline. Organize coding tasks on a Kanban board, queue them up, and let Claude Code execute them one by one — unattended.

## Why

Claude Code is powerful but sequential and manual. If you have 20 tasks — write tests, refactor code, fix bugs, add docs — you run them one at a time. Do It Claude eliminates the waiting: describe all tasks upfront, queue them, and they run back to back.

## Features

### Task Management
- **Kanban board** with Backlog and Queued columns — drag tasks between them
- **Sequential execution** — tasks run one at a time per project
- **Live output** — watch Claude's work in real-time in the current task box
- **Cancel & revert** — kill a running task and automatically `git reset --hard` to the pre-task state
- **Pause queue** — stop after the current task finishes without killing it
- **Session persistence** — Claude keeps context across tasks via `--resume`, so it doesn't re-analyze your codebase every time

### Multi-Project Support
- **Browser-style tabs** — each project gets its own tab, switch instantly
- **Independent queues** — projects run in parallel, tasks within a project run sequentially
- **Per-project stats** — token usage, cost, and task counts stored in the project folder (`.do-it-claude-stats.json`)

### Stats & Cost Tracking
- **Real-time stats** — output tokens, input tokens, total cost (USD), turns, tasks completed/failed
- **Persisted to project folder** — stats survive app restarts, project removal and re-adding
- **Token formatting** — large numbers displayed as 1.2k, 3.5M
- Powered by Claude's `--output-format stream-json` for accurate usage data

### Project Settings
- **Effort level** — low, medium, high, max (controls Claude's reasoning depth)
- **Max turns** — cap iterations per task
- **Context reset** — clear Claude's session to start fresh
- **Git detection** — warns if project isn't a git repo, offers one-click `git init`

### Integrated Terminal
- **Embedded terminal** in a resizable bottom panel (like VS Code)
- **Multiple sessions** per project with iTerm-style pill tabs
- **Persistent across tab switches** — terminals keep running when you switch projects
- Full PTY support via `portable-pty` with xterm.js rendering

### Themes
- **Dark** — high contrast black & white with colored status accents
- **Light** — clean light mode with proper contrast
- **Unhinged** — random colors every 2 seconds, confetti explosions, flashbangs, and screen earthquakes on every click

### Settings
- Claude CLI auto-detection with version display and recheck
- Theme selector
- About section

## Architecture

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri v2 — native window, system webview, <15 MB |
| Frontend | Vue 3 + TypeScript + Vite |
| State | Pinia |
| Terminal | xterm.js + @xterm/addon-fit |
| Drag & Drop | SortableJS (with `forceFallback` for Tauri compatibility) |
| Backend | Rust (runs in Tauri process) |
| Database | SQLite via rusqlite |
| PTY | portable-pty |
| IPC | Tauri commands + events (no HTTP) |

## Getting Started

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and in your PATH

### Install & Run

```bash
# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build for production
npm run tauri build
```

### Verify Setup

1. Open the app
2. Go to Settings (cog icon in the status bar)
3. Check that Claude CLI shows "Found" with the path and version
4. Create a project by clicking the `+` button and selecting a folder
5. Add tasks to the Backlog, drag them to Queued, and hit play

## How It Works

1. **Projects** point at a local code folder
2. **Tasks** are prompts sent to Claude Code (`claude -p "your prompt" --dangerously-skip-permissions`)
3. The queue picks up the first queued task, runs Claude in the project directory, captures output, and moves to the next
4. Claude's session is preserved across tasks (`--resume`) so context carries over
5. Git state is snapshotted before each task — cancel reverts all changes

## Key Design Decisions

- **Automation-first** — Claude runs non-interactive with `--dangerously-skip-permissions`. Tasks are self-contained prompts.
- **One process per project** — tasks within a project run sequentially (they may depend on each other). Different projects run in parallel.
- **Stats in the project folder** — `.do-it-claude-stats.json` travels with the project, survives app reinstalls.
- **`forceFallback: true`** on SortableJS — Tauri's webview intercepts HTML5 drag events; pointer-event fallback is required.
- **Stream JSON parsing** — Claude output is parsed from `--output-format stream-json --verbose` to extract both text content and usage statistics.

## Version

0.2.0

## License

MIT
