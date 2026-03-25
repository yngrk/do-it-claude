use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

const DEFAULT_CLAUDE_MD: &str = r#"# AI Dev Team — Engineering Manager

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
- Always summarize results for the user"#;

const DEFAULT_AGENT_CODER: &str = r#"---
name: coder
description: Writes and edits code. Use when implementation work is needed — features, bug fixes, or refactoring. Spawn this agent when the manager needs hands-on coding done.
tools: Read, Edit, Write, Bash, Glob, Grep
model: sonnet
---

You are a **Coder Agent** on an AI dev team, managed by the Engineering Manager.

Your job:
- Implement features and fixes as instructed
- Write clean, idiomatic code following existing project conventions
- Read existing code before making changes
- Report what you changed and any blockers back to the manager

Rules:
- Do not over-engineer. Build exactly what was asked.
- Read files before editing them.
- Prefer editing existing files over creating new ones.
- Do not add error handling, comments, or abstractions beyond what the task requires."#;

const DEFAULT_AGENT_REVIEWER: &str = r#"---
name: reviewer
description: Reviews code quality, correctness, and best practices. Use proactively after code changes are made. Read-only — does not modify files.
tools: Read, Glob, Grep
model: sonnet
---

You are a **Reviewer Agent** on an AI dev team, managed by the Engineering Manager.

Your job:
- Review code for correctness, clarity, security issues, and best practices
- Provide specific, actionable feedback with file and line references
- Flag bugs, edge cases, and potential improvements
- Return a structured review report to the manager

Output format:
- **Summary**: 1–2 sentence overall assessment
- **Issues**: Bulleted list of specific problems (file:line — description)
- **Suggestions**: Optional improvements
- **Verdict**: APPROVED / NEEDS CHANGES"#;

const DEFAULT_AGENT_TESTER: &str = r#"---
name: tester
description: Writes and runs tests. Use when test coverage is needed for new code, or when test failures need investigation.
tools: Read, Edit, Write, Bash, Glob, Grep
model: sonnet
---

You are a **Tester Agent** on an AI dev team, managed by the Engineering Manager.

Your job:
- Write unit, integration, or end-to-end tests as instructed
- Run existing tests and report failures
- Investigate test failures and diagnose root causes
- Return a test report with pass/fail counts and any failure details

Rules:
- Follow the existing test framework and conventions in the project
- Test behavior, not implementation details
- Do not fix bugs — report them to the manager if found"#;

const DEFAULT_AGENT_RESEARCHER: &str = r#"---
name: researcher
description: Gathers information from the codebase, web, or documentation. Use when background research, API investigation, or technology evaluation is needed before implementation.
tools: Read, Glob, Grep, WebSearch, WebFetch
model: sonnet
---

You are a **Researcher Agent** on an AI dev team, managed by the Engineering Manager.

Your job:
- Investigate codebases to understand structure and patterns
- Research external APIs, libraries, and documentation
- Evaluate technology options and tradeoffs
- Return a concise findings report to the manager

Output format:
- **Question**: What was investigated
- **Findings**: Key facts and relevant code/API references
- **Recommendation**: Best approach based on findings (if applicable)"#;

const DEFAULT_AGENT_ARCHITECT: &str = r#"---
name: architect
description: Designs system structure and implementation plans. Use when high-level architecture decisions are needed before coding begins, or when a complex feature needs a design spec.
tools: Read, Glob, Grep
model: opus
---

You are an **Architect Agent** on an AI dev team, managed by the Engineering Manager.

Your job:
- Design system structure, data models, and component boundaries
- Create implementation plans with clear steps for the coder agent to follow
- Identify technical risks and constraints
- Return a design document to the manager

Output format:
- **Goal**: What is being designed
- **Design**: Architecture overview, key components, data flow
- **Implementation Plan**: Ordered steps for the coder to follow
- **Risks**: Technical risks or open questions"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub system_prompt: Option<String>,
    pub mode_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub tag: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub exit_code: Option<i32>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLog {
    pub id: String,
    pub task_id: String,
    pub content: String,
    pub log_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub id: String,
    pub task_id: String,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeFile {
    pub id: String,
    pub mode_id: String,
    pub file_path: String,
    pub content: String,
}

pub type DbConn = Arc<Mutex<Connection>>;

pub fn init_db(app_dir: &std::path::Path) -> Result<Connection> {
    let db_path = app_dir.join("do-it-claude.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            tag TEXT,
            status TEXT NOT NULL DEFAULT 'backlog',
            sort_order INTEGER NOT NULL DEFAULT 0,
            exit_code INTEGER,
            created_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS task_logs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            content TEXT NOT NULL,
            log_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS task_messages (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            message_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS prompt_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS mode_files (
            id TEXT PRIMARY KEY,
            mode_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            content TEXT NOT NULL,
            FOREIGN KEY (mode_id) REFERENCES prompt_templates(id) ON DELETE CASCADE
        );
    ")?;

    // Migration: add tag column to existing databases
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN tag TEXT", []);

    // Task updated_at for tracking modifications
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN updated_at TEXT", []);

    // Migration: add system_prompt and mode_id to projects
    let _ = conn.execute("ALTER TABLE projects ADD COLUMN system_prompt TEXT", []);
    let _ = conn.execute("ALTER TABLE projects ADD COLUMN mode_id TEXT", []);

    // Seed default mode if not present
    let has_default: bool = conn.query_row(
        "SELECT COUNT(*) FROM prompt_templates",
        [],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;

    if !has_default {
        let mode_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO prompt_templates (id, name, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&mode_id, "Software Dev Team", "AI Engineering Manager mode with subagent team", &now, &now],
        )?;

        // Seed mode files
        let seed_files: Vec<(&str, &str)> = vec![
            ("CLAUDE.md", DEFAULT_CLAUDE_MD),
            (".claude/agents/coder.md", DEFAULT_AGENT_CODER),
            (".claude/agents/reviewer.md", DEFAULT_AGENT_REVIEWER),
            (".claude/agents/tester.md", DEFAULT_AGENT_TESTER),
            (".claude/agents/researcher.md", DEFAULT_AGENT_RESEARCHER),
            (".claude/agents/architect.md", DEFAULT_AGENT_ARCHITECT),
        ];
        for (path, content) in seed_files {
            conn.execute(
                "INSERT INTO mode_files (id, mode_id, file_path, content) VALUES (?1, ?2, ?3, ?4)",
                params![Uuid::new_v4().to_string(), &mode_id, path, content],
            )?;
        }
    }

    Ok(conn)
}

pub fn create_project(conn: &Connection, name: &str, path: &str) -> Result<Project> {
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        path: path.to_string(),
        created_at: Utc::now().to_rfc3339(),
        system_prompt: None,
        mode_id: None,
    };
    conn.execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![project.id, project.name, project.path, project.created_at],
    )?;
    Ok(project)
}

pub fn get_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, path, created_at, system_prompt, mode_id FROM projects ORDER BY created_at DESC")?;
    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
            system_prompt: row.get(4)?,
            mode_id: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(projects)
}

pub fn get_project_by_id(conn: &Connection, id: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, path, created_at, system_prompt, mode_id FROM projects WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
            system_prompt: row.get(4)?,
            mode_id: row.get(5)?,
        })
    })?;
    match rows.next() {
        Some(Ok(p)) => Ok(Some(p)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn delete_project(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM task_messages WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1)", params![id])?;
    conn.execute("DELETE FROM task_logs WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1)", params![id])?;
    conn.execute("DELETE FROM tasks WHERE project_id = ?1", params![id])?;
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn update_project_system_prompt(conn: &Connection, id: &str, system_prompt: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE projects SET system_prompt = ?1 WHERE id = ?2",
        params![system_prompt, id],
    )?;
    Ok(())
}

pub fn update_project_mode(conn: &Connection, id: &str, mode_id: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE projects SET mode_id = ?1 WHERE id = ?2",
        params![mode_id, id],
    )?;
    Ok(())
}

pub fn create_task(conn: &Connection, project_id: &str, title: &str, description: &str, tag: Option<&str>) -> Result<Task> {
    let max_order: i32 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) FROM tasks WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;

    let task = Task {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        tag: tag.map(|t| t.to_string()),
        status: "backlog".to_string(),
        sort_order: max_order + 1,
        exit_code: None,
        created_at: Utc::now().to_rfc3339(),
        started_at: None,
        completed_at: None,
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    conn.execute(
        "INSERT INTO tasks (id, project_id, title, description, tag, status, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![task.id, task.project_id, task.title, task.description, task.tag, task.status, task.sort_order, task.created_at, task.updated_at],
    )?;
    Ok(task)
}

pub fn get_tasks(conn: &Connection, project_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at, updated_at FROM tasks WHERE project_id = ?1 ORDER BY sort_order ASC"
    )?;
    let tasks = stmt.query_map(params![project_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            tag: row.get(4)?,
            status: row.get(5)?,
            sort_order: row.get(6)?,
            exit_code: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(tasks)
}

pub fn get_task_by_id(conn: &Connection, id: &str) -> Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at, updated_at FROM tasks WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            tag: row.get(4)?,
            status: row.get(5)?,
            sort_order: row.get(6)?,
            exit_code: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    match rows.next() {
        Some(Ok(task)) => Ok(Some(task)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn update_task(conn: &Connection, id: &str, title: Option<&str>, description: Option<&str>, tag: Option<Option<&str>>) -> Result<Task> {
    if let Some(t) = title {
        conn.execute("UPDATE tasks SET title = ?1 WHERE id = ?2", params![t, id])?;
    }
    if let Some(d) = description {
        conn.execute("UPDATE tasks SET description = ?1 WHERE id = ?2", params![d, id])?;
    }
    if let Some(t) = tag {
        conn.execute("UPDATE tasks SET tag = ?1 WHERE id = ?2", params![t, id])?;
    }
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE tasks SET updated_at = ?1 WHERE id = ?2", params![now, id])?;
    let mut stmt = conn.prepare("SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at, updated_at FROM tasks WHERE id = ?1")?;
    stmt.query_row(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            tag: row.get(4)?,
            status: row.get(5)?,
            sort_order: row.get(6)?,
            exit_code: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM task_messages WHERE task_id = ?1", params![id])?;
    conn.execute("DELETE FROM task_logs WHERE task_id = ?1", params![id])?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn move_task(conn: &Connection, id: &str, new_status: &str, new_sort_order: i32) -> Result<Task> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = ?1, sort_order = ?2, updated_at = ?3 WHERE id = ?4",
        params![new_status, new_sort_order, now, id],
    )?;
    let mut stmt = conn.prepare("SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at, updated_at FROM tasks WHERE id = ?1")?;
    stmt.query_row(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            tag: row.get(4)?,
            status: row.get(5)?,
            sort_order: row.get(6)?,
            exit_code: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })
}

pub fn set_task_in_progress(conn: &Connection, id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = 'in_progress', started_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn set_task_completed(conn: &Connection, id: &str, exit_code: i32) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let status = if exit_code == 0 { "done" } else { "failed" };
    conn.execute(
        "UPDATE tasks SET status = ?1, exit_code = ?2, completed_at = ?3 WHERE id = ?4",
        params![status, exit_code, now, id],
    )?;
    Ok(())
}

pub fn get_next_queued_task(conn: &Connection, project_id: &str) -> Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at, updated_at FROM tasks WHERE project_id = ?1 AND status = 'queued' ORDER BY sort_order ASC LIMIT 1"
    )?;
    let mut rows = stmt.query_map(params![project_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            tag: row.get(4)?,
            status: row.get(5)?,
            sort_order: row.get(6)?,
            exit_code: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    match rows.next() {
        Some(Ok(task)) => Ok(Some(task)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn add_task_log(conn: &Connection, task_id: &str, content: &str, log_type: &str) -> Result<TaskLog> {
    let log = TaskLog {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        content: content.to_string(),
        log_type: log_type.to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    conn.execute(
        "INSERT INTO task_logs (id, task_id, content, log_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![log.id, log.task_id, log.content, log.log_type, log.created_at],
    )?;
    Ok(log)
}

pub fn get_task_logs(conn: &Connection, task_id: &str) -> Result<Vec<TaskLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, content, log_type, created_at FROM task_logs WHERE task_id = ?1 ORDER BY created_at ASC"
    )?;
    let logs = stmt.query_map(params![task_id], |row| {
        Ok(TaskLog {
            id: row.get(0)?,
            task_id: row.get(1)?,
            content: row.get(2)?,
            log_type: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(logs)
}

pub fn add_task_message(conn: &Connection, task_id: &str, role: &str, content: &str, message_type: &str) -> Result<TaskMessage> {
    let message = TaskMessage {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        message_type: message_type.to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    conn.execute(
        "INSERT INTO task_messages (id, task_id, role, content, message_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![message.id, message.task_id, message.role, message.content, message.message_type, message.created_at],
    )?;
    Ok(message)
}

pub fn get_task_messages(conn: &Connection, task_id: &str) -> Result<Vec<TaskMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, role, content, message_type, created_at FROM task_messages WHERE task_id = ?1 ORDER BY created_at ASC"
    )?;
    let messages = stmt.query_map(params![task_id], |row| {
        Ok(TaskMessage {
            id: row.get(0)?,
            task_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            message_type: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(messages)
}

pub fn get_templates(conn: &Connection) -> Result<Vec<PromptTemplate>> {
    let mut stmt = conn.prepare("SELECT id, name, content, created_at, updated_at FROM prompt_templates ORDER BY created_at ASC")?;
    let templates = stmt.query_map([], |row| {
        Ok(PromptTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(templates)
}

pub fn create_template(conn: &Connection, name: &str, content: &str) -> Result<PromptTemplate> {
    let now = Utc::now().to_rfc3339();
    let template = PromptTemplate {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        content: content.to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO prompt_templates (id, name, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![template.id, template.name, template.content, template.created_at, template.updated_at],
    )?;
    Ok(template)
}

pub fn update_template(conn: &Connection, id: &str, name: &str, content: &str) -> Result<PromptTemplate> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE prompt_templates SET name = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
        params![name, content, now, id],
    )?;
    let mut stmt = conn.prepare("SELECT id, name, content, created_at, updated_at FROM prompt_templates WHERE id = ?1")?;
    stmt.query_row(params![id], |row| {
        Ok(PromptTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })
}

pub fn delete_template(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM mode_files WHERE mode_id = ?1", params![id])?;
    conn.execute("DELETE FROM prompt_templates WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn create_mode_file(conn: &Connection, mode_id: &str, file_path: &str, content: &str) -> Result<ModeFile> {
    let file = ModeFile {
        id: Uuid::new_v4().to_string(),
        mode_id: mode_id.to_string(),
        file_path: file_path.to_string(),
        content: content.to_string(),
    };
    conn.execute(
        "INSERT INTO mode_files (id, mode_id, file_path, content) VALUES (?1, ?2, ?3, ?4)",
        params![file.id, file.mode_id, file.file_path, file.content],
    )?;
    Ok(file)
}

pub fn get_mode_files(conn: &Connection, mode_id: &str) -> Result<Vec<ModeFile>> {
    let mut stmt = conn.prepare("SELECT id, mode_id, file_path, content FROM mode_files WHERE mode_id = ?1 ORDER BY file_path ASC")?;
    let files = stmt.query_map(params![mode_id], |row| {
        Ok(ModeFile {
            id: row.get(0)?,
            mode_id: row.get(1)?,
            file_path: row.get(2)?,
            content: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(files)
}

pub fn delete_mode_files(conn: &Connection, mode_id: &str) -> Result<()> {
    conn.execute("DELETE FROM mode_files WHERE mode_id = ?1", params![mode_id])?;
    Ok(())
}
