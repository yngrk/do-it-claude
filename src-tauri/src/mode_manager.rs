use std::path::{Path, PathBuf};
use std::fs;

fn backup_dir(app_dir: &Path, project_id: &str) -> PathBuf {
    app_dir.join("backups").join(project_id)
}

fn presets_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("presets")
}

fn skills_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("skills")
}

fn agents_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("agents")
}

/// List available preset names (folder names inside presets/)
pub fn list_presets(app_dir: &Path) -> Result<Vec<String>, String> {
    let dir = presets_dir(app_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("Failed to read presets dir: {}", e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Load a preset: backup project config, then copy preset folder contents into project
pub fn load_preset(app_dir: &Path, project_id: &str, project_path: &Path, preset_name: &str) -> Result<(), String> {
    let src = presets_dir(app_dir).join(preset_name);
    if !src.exists() {
        return Err(format!("Preset '{}' not found", preset_name));
    }

    // Backup original config if no backup exists yet
    if !backup_dir(app_dir, project_id).exists() {
        backup_project_config(app_dir, project_id, project_path)?;
    }

    // Copy preset contents into project
    copy_dir_recursive(&src, project_path)?;

    Ok(())
}

/// List available skills (folder names in skills/)
pub fn list_skills(app_dir: &Path) -> Result<Vec<String>, String> {
    let dir = skills_dir(app_dir);
    if !dir.exists() { return Ok(vec![]); }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// List available agents (.md files in agents/)
pub fn list_agents(app_dir: &Path) -> Result<Vec<String>, String> {
    let dir = agents_dir(app_dir);
    if !dir.exists() { return Ok(vec![]); }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".md") {
                names.push(name.trim_end_matches(".md").to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Install a skill into project's .claude/skills/<name>/
pub fn install_skill(app_dir: &Path, project_path: &Path, skill_name: &str) -> Result<(), String> {
    let src = skills_dir(app_dir).join(skill_name);
    if !src.exists() {
        return Err(format!("Skill '{}' not found", skill_name));
    }
    let dst = project_path.join(".claude").join("skills").join(skill_name);
    fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
    copy_dir_recursive(&src, &dst)?;
    Ok(())
}

/// Install an agent into project's .claude/agents/<name>.md
pub fn install_agent(app_dir: &Path, project_path: &Path, agent_name: &str) -> Result<(), String> {
    let src = agents_dir(app_dir).join(format!("{}.md", agent_name));
    if !src.exists() {
        return Err(format!("Agent '{}' not found", agent_name));
    }
    let dst_dir = project_path.join(".claude").join("agents");
    fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
    fs::copy(&src, dst_dir.join(format!("{}.md", agent_name))).map_err(|e| e.to_string())?;
    Ok(())
}

/// Restore project config from backup (undo template load)
pub fn restore_backup(app_dir: &Path, project_id: &str, project_path: &Path) -> Result<(), String> {
    let backup = backup_dir(app_dir, project_id);
    if !backup.exists() {
        return Ok(());
    }

    // Remove CLAUDE.md if it exists
    let claude_md = project_path.join("CLAUDE.md");
    if claude_md.exists() {
        let _ = fs::remove_file(&claude_md);
    }

    // Remove .claude/agents/ if it exists
    let agents_dir = project_path.join(".claude").join("agents");
    if agents_dir.exists() {
        let _ = fs::remove_dir_all(&agents_dir);
    }

    // Restore from backup
    copy_dir_recursive(&backup, project_path)?;

    // Remove backup
    let _ = fs::remove_dir_all(&backup);

    Ok(())
}

/// Seed the default "Software Dev Team" preset and individual agents/skills
pub fn seed_defaults(app_dir: &Path) {
    // Seed presets
    let presets = presets_dir(app_dir);
    let _ = fs::create_dir_all(&presets);
    let default_preset = presets.join("Software Dev Team");
    if !default_preset.exists() {
        let _ = fs::create_dir_all(default_preset.join(".claude").join("agents"));
        let _ = fs::write(default_preset.join("CLAUDE.md"), DEFAULT_CLAUDE_MD);
        let _ = fs::write(default_preset.join(".claude/agents/coder.md"), DEFAULT_AGENT_CODER);
        let _ = fs::write(default_preset.join(".claude/agents/reviewer.md"), DEFAULT_AGENT_REVIEWER);
        let _ = fs::write(default_preset.join(".claude/agents/tester.md"), DEFAULT_AGENT_TESTER);
        let _ = fs::write(default_preset.join(".claude/agents/researcher.md"), DEFAULT_AGENT_RESEARCHER);
        let _ = fs::write(default_preset.join(".claude/agents/architect.md"), DEFAULT_AGENT_ARCHITECT);
    }

    // Seed individual agents
    let agents = agents_dir(app_dir);
    let _ = fs::create_dir_all(&agents);
    let agent_files = [
        ("coder.md", DEFAULT_AGENT_CODER),
        ("reviewer.md", DEFAULT_AGENT_REVIEWER),
        ("tester.md", DEFAULT_AGENT_TESTER),
        ("researcher.md", DEFAULT_AGENT_RESEARCHER),
        ("architect.md", DEFAULT_AGENT_ARCHITECT),
    ];
    for (name, content) in agent_files {
        let path = agents.join(name);
        if !path.exists() {
            let _ = fs::write(path, content);
        }
    }

    // Create skills dir (empty by default)
    let skills = skills_dir(app_dir);
    let _ = fs::create_dir_all(&skills);
}

// --- Internal helpers ---

fn backup_project_config(app_dir: &Path, project_id: &str, project_path: &Path) -> Result<(), String> {
    let backup = backup_dir(app_dir, project_id);
    fs::create_dir_all(&backup).map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let claude_md = project_path.join("CLAUDE.md");
    if claude_md.exists() {
        fs::copy(&claude_md, backup.join("CLAUDE.md"))
            .map_err(|e| format!("Failed to backup CLAUDE.md: {}", e))?;
    }

    let claude_dir = project_path.join(".claude");
    if claude_dir.exists() {
        copy_dir_recursive(&claude_dir, &backup.join(".claude"))
            .map_err(|e| format!("Failed to backup .claude/: {}", e))?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("mkdir failed: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read_dir failed: {}", e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

// --- Default template content ---

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
- Always summarize results for the user
"#;

const DEFAULT_AGENT_CODER: &str = r#"---
name: coder
description: Writes and edits code. Use when implementation work is needed — features, bug fixes, or refactoring.
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
- Do not add error handling, comments, or abstractions beyond what the task requires.
"#;

const DEFAULT_AGENT_REVIEWER: &str = r#"---
name: reviewer
description: Reviews code quality, correctness, and best practices. Read-only — does not modify files.
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
- **Verdict**: APPROVED / NEEDS CHANGES
"#;

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
- Do not fix bugs — report them to the manager if found
"#;

const DEFAULT_AGENT_RESEARCHER: &str = r#"---
name: researcher
description: Gathers information from the codebase, web, or documentation. Use when background research or API investigation is needed.
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
- **Recommendation**: Best approach based on findings (if applicable)
"#;

const DEFAULT_AGENT_ARCHITECT: &str = r#"---
name: architect
description: Designs system structure and implementation plans. Use when high-level architecture decisions are needed.
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
- **Risks**: Technical risks or open questions
"#;
