use crate::db::{self, DbConn};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Resolve the full path to the claude binary by checking common locations
/// and the user's shell PATH (which Tauri doesn't inherit on macOS).
pub fn resolve_claude_path() -> String {
    // Check common locations first
    let candidates = [
        "/opt/homebrew/bin/claude",
        "/usr/local/bin/claude",
        "/usr/bin/claude",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }

    // Try to get PATH from user's shell
    if let Ok(output) = std::process::Command::new("/bin/zsh")
        .args(["-ilc", "echo $PATH"])
        .output()
    {
        if let Ok(shell_path) = String::from_utf8(output.stdout) {
            let shell_path = shell_path.trim();
            for dir in shell_path.split(':') {
                let candidate = format!("{}/claude", dir);
                if std::path::Path::new(&candidate).exists() {
                    return candidate;
                }
            }
        }
    }

    // Fallback — hope it's in PATH
    "claude".to_string()
}

pub fn resolve_codex_path() -> String {
    let candidates = [
        "/opt/homebrew/bin/codex",
        "/usr/local/bin/codex",
        "/usr/bin/codex",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    if let Ok(output) = std::process::Command::new("/bin/zsh")
        .args(["-ilc", "echo $PATH"])
        .output()
    {
        if let Ok(shell_path) = String::from_utf8(output.stdout) {
            let shell_path = shell_path.trim();
            for dir in shell_path.split(':') {
                let candidate = format!("{}/codex", dir);
                if std::path::Path::new(&candidate).exists() {
                    return candidate;
                }
            }
        }
    }
    "codex".to_string()
}

pub fn resolve_cli_path(provider: &str) -> String {
    match provider {
        "codex" => resolve_codex_path(),
        _ => resolve_claude_path(),
    }
}

/// Get the user's shell PATH for child processes
pub fn get_shell_path() -> Option<String> {
    std::process::Command::new("/bin/zsh")
        .args(["-ilc", "echo $PATH"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

pub type RunningProcesses = Arc<Mutex<HashMap<String, RunningTask>>>;
pub type StopFlags = Arc<std::sync::Mutex<HashMap<String, bool>>>;
pub type SessionStore = Arc<std::sync::Mutex<HashMap<String, String>>>; // logical session key -> session/thread id
pub type ActiveQueues = Arc<std::sync::Mutex<HashMap<String, bool>>>;

pub fn new_active_queues() -> ActiveQueues {
    Arc::new(std::sync::Mutex::new(HashMap::new()))
}

/// Atomically check-and-set: returns true if the queue was newly marked active
/// (caller should start the loop). Returns false if already active (caller should not).
pub fn try_mark_active(active_queues: &ActiveQueues, project_id: &str) -> bool {
    let mut active = active_queues.lock().unwrap();
    if active.get(project_id) == Some(&true) {
        return false;
    }
    active.insert(project_id.to_string(), true);
    true
}

pub fn new_session_store() -> SessionStore {
    Arc::new(std::sync::Mutex::new(HashMap::new()))
}

fn codex_session_key(project_id: &str) -> String {
    format!("queue-project-{}", project_id)
}

pub struct RunningTask {
    pub child: tokio::process::Child,
    pub git_hash: Option<String>,
    pub project_path: String,
}

pub fn new_stop_flags() -> StopFlags {
    Arc::new(std::sync::Mutex::new(HashMap::new()))
}

#[derive(Clone, Serialize)]
struct QueueStoppedPayload {
    project_id: String,
}

#[derive(Clone, Serialize)]
struct TaskStartedPayload {
    task_id: String,
    project_id: String,
}

#[derive(Clone, Serialize)]
struct TaskCompletedPayload {
    task_id: String,
    project_id: String,
    exit_code: i32,
    status: String,
    task_title: String,
    project_name: String,
}

#[derive(Clone, Serialize)]
struct TaskOutputPayload {
    task_id: String,
    content: String,
    log_type: String,
}

pub fn new_running_processes() -> RunningProcesses {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn start_queue(
    app: AppHandle,
    db: DbConn,
    processes: RunningProcesses,
    stop_flags: StopFlags,
    sessions: SessionStore,
    active_queues: ActiveQueues,
    project_id: String,
    provider: String,
) {
    // Clear stop flag on start
    {
        let mut flags = stop_flags.lock().unwrap();
        flags.remove(&project_id);
    }

    // active flag was already set by try_mark_active in the command handler

    loop {
        // Check stop flag before picking up next task
        {
            let flags = stop_flags.lock().unwrap();
            if flags.get(&project_id) == Some(&true) {
                break;
            }
        }

        let task = {
            let conn = db.lock().unwrap();
            db::get_next_queued_task(&conn, &project_id).ok().flatten()
        };

        let task = match task {
            Some(t) => t,
            None => break,
        };

        {
            let conn = db.lock().unwrap();
            let _ = db::set_task_in_progress(&conn, &task.id);
            let _ = db::set_task_provider(&conn, &task.id, &provider);
        }

        let _ = app.emit(
            "task-started",
            TaskStartedPayload {
                task_id: task.id.clone(),
                project_id: project_id.clone(),
            },
        );

        let project = {
            let conn = db.lock().unwrap();
            db::get_project_by_id(&conn, &project_id).ok().flatten()
        };

        let project = match project {
            Some(p) => p,
            None => break,
        };

        let project_path = project.path.clone();
        let normalized_task_description = crate::context_generator::normalize_project_local_paths(
            &task.description,
            &project_path,
        );

        // Generate codebase map for this task (refreshed per task so new/modified files are reflected)
        let codebase_map = crate::context_generator::generate_codebase_map(&project_path);

        // Snapshot git HEAD before running the task
        let git_hash = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&project_path)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        let cli_bin = resolve_cli_path(&provider);
        let shell_path = get_shell_path();

        let mut cmd = Command::new(&cli_bin);
        let existing_session = {
            let store = sessions.lock().unwrap();
            store.get(&codex_session_key(&project_id)).cloned()
        };

        // Auto-select model if not specified
        let effective_model = match &task.model {
            Some(m) if !m.is_empty() => m.clone(),
            _ => crate::context_generator::auto_select_model(
                &task.description,
                task.tag.as_deref(),
                &provider,
            ),
        };

        match provider.as_str() {
            "codex" => {
                if let Some(ref session_id) = existing_session {
                    cmd.arg("exec").arg("resume").arg(session_id).arg("--json");

                    cmd.arg("-m").arg(&effective_model);

                    // Prompt must come last for codex exec resume
                    cmd.arg(&normalized_task_description);
                } else {
                    cmd.arg("exec")
                        .arg("--yolo")
                        .arg("--json")
                        .arg("-C")
                        .arg(&project_path);

                    cmd.arg("-m").arg(&effective_model);

                    // Build developer instructions with codebase map for context
                    let mut dev_instructions = Vec::new();
                    if !codebase_map.is_empty() {
                        dev_instructions.push(codebase_map.clone());
                    }
                    if let Some(ref prompt) = project.system_prompt {
                        if !prompt.trim().is_empty() {
                            dev_instructions.push(
                                crate::context_generator::normalize_project_local_paths(
                                    prompt,
                                    &project_path,
                                ),
                            );
                        }
                    }
                    if !dev_instructions.is_empty() {
                        cmd.arg("-c").arg(format!(
                            "developer_instructions={}",
                            dev_instructions.join("\n\n")
                        ));
                    }

                    // Prompt must come last for codex exec
                    cmd.arg(&normalized_task_description);
                }
            }
            _ => {
                cmd.arg("-p")
                    .arg(&task.description)
                    .arg("--dangerously-skip-permissions")
                    .arg("--output-format")
                    .arg("stream-json")
                    .arg("--verbose");

                let max_turns = task.max_turns.unwrap_or_else(|| {
                    crate::context_generator::estimate_max_turns(
                        &task.description,
                        task.tag.as_deref(),
                    )
                });
                cmd.arg("--max-turns").arg(max_turns.to_string());

                cmd.arg("--model").arg(&effective_model);

                // Build system prompt with codebase map for context + caching
                let mut system_parts = Vec::new();
                if !codebase_map.is_empty() {
                    system_parts.push(codebase_map.clone());
                }
                if let Some(ref prompt) = project.system_prompt {
                    if !prompt.trim().is_empty() {
                        system_parts.push(prompt.clone());
                    }
                }
                if !system_parts.is_empty() {
                    cmd.arg("--append-system-prompt")
                        .arg(system_parts.join("\n\n"));
                }
            }
        }

        cmd.current_dir(&project_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Pass the user's shell PATH so claude can find tools (git, node, etc.)
        if let Some(ref path) = shell_path {
            cmd.env("PATH", path);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!(
                    "Failed to spawn {}: {}",
                    if provider == "codex" {
                        "codex"
                    } else {
                        "claude"
                    },
                    e
                );
                {
                    let conn = db.lock().unwrap();
                    let _ = db::add_task_log(&conn, &task.id, &error_msg, "stderr");
                    let _ = db::set_task_completed(&conn, &task.id, 1);
                }
                let _ = app.emit(
                    "task-output",
                    TaskOutputPayload {
                        task_id: task.id.clone(),
                        content: error_msg,
                        log_type: "stderr".to_string(),
                    },
                );
                let _ = app.emit(
                    "task-completed",
                    TaskCompletedPayload {
                        task_id: task.id.clone(),
                        project_id: project_id.clone(),
                        exit_code: 1,
                        status: "failed".to_string(),
                        task_title: task.title.clone(),
                        project_name: project.name.clone(),
                    },
                );
                continue;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        {
            let mut procs = processes.lock().await;
            procs.insert(
                project_id.clone(),
                RunningTask {
                    child,
                    git_hash,
                    project_path: project_path.clone(),
                },
            );
        }

        let task_id = task.id.clone();
        let provider_clone = provider.clone();

        if let Some(stdout) = stdout {
            let app_clone = app.clone();
            let db_clone = db.clone();
            let task_id_clone = task_id.clone();
            let sessions_clone = sessions.clone();
            let task_key = task_id.clone();
            let project_session_key = codex_session_key(&project_id);
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Try to parse as JSON event from stream-json format
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        if provider_clone == "codex" {
                            if event_type == "thread.started" {
                                if let Some(thread_id) =
                                    json.get("thread_id").and_then(|v| v.as_str())
                                {
                                    let mut store = sessions_clone.lock().unwrap();
                                    store
                                        .insert(project_session_key.clone(), thread_id.to_string());
                                }
                            } else if event_type == "session.started" {
                                if let Some(session_id) =
                                    json.get("session_id").and_then(|v| v.as_str())
                                {
                                    let mut store = sessions_clone.lock().unwrap();
                                    store.insert(
                                        project_session_key.clone(),
                                        session_id.to_string(),
                                    );
                                }
                            }
                            if event_type.contains("output_text.delta") {
                                if let Some(delta) = json.get("delta").and_then(|v| v.as_str()) {
                                    let conn = db_clone.lock().unwrap();
                                    let _ =
                                        db::add_task_log(&conn, &task_id_clone, delta, "stdout");
                                    let _ = app_clone.emit(
                                        "task-output",
                                        TaskOutputPayload {
                                            task_id: task_id_clone.clone(),
                                            content: delta.to_string(),
                                            log_type: "stdout".to_string(),
                                        },
                                    );
                                }
                            }
                            // Skip output_text.done — deltas already logged above
                        } else {
                            match event_type {
                                "system" => {
                                    if let Some(sid) =
                                        json.get("session_id").and_then(|v| v.as_str())
                                    {
                                        let mut store = sessions_clone.lock().unwrap();
                                        store.insert(task_key.clone(), sid.to_string());
                                    }
                                }
                                "assistant" => {
                                    if let Some(content) = json.pointer("/message/content") {
                                        if let Some(arr) = content.as_array() {
                                            for item in arr {
                                                if item.get("type").and_then(|v| v.as_str())
                                                    == Some("text")
                                                {
                                                    if let Some(text) =
                                                        item.get("text").and_then(|v| v.as_str())
                                                    {
                                                        let conn = db_clone.lock().unwrap();
                                                        let _ = db::add_task_log(
                                                            &conn,
                                                            &task_id_clone,
                                                            text,
                                                            "stdout",
                                                        );
                                                        let _ = app_clone.emit(
                                                            "task-output",
                                                            TaskOutputPayload {
                                                                task_id: task_id_clone.clone(),
                                                                content: text.to_string(),
                                                                log_type: "stdout".to_string(),
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                "result" => {
                                    if let Some(result_text) =
                                        json.get("result").and_then(|v| v.as_str())
                                    {
                                        let conn = db_clone.lock().unwrap();
                                        let _ = db::add_task_log(
                                            &conn,
                                            &task_id_clone,
                                            result_text,
                                            "stdout",
                                        );
                                    }
                                    let input_tokens = json
                                        .pointer("/usage/input_tokens")
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        + json
                                            .pointer("/usage/cache_read_input_tokens")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0)
                                        + json
                                            .pointer("/usage/cache_creation_input_tokens")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0);
                                    let output_tokens = json
                                        .pointer("/usage/output_tokens")
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0);
                                    if input_tokens > 0 || output_tokens > 0 {
                                        let conn = db_clone.lock().unwrap();
                                        let _ = db::set_task_token_usage(
                                            &conn,
                                            &task_id_clone,
                                            input_tokens,
                                            output_tokens,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        // Non-JSON output, log as plain text
                        {
                            let conn = db_clone.lock().unwrap();
                            let _ = db::add_task_log(&conn, &task_id_clone, &line, "stdout");
                        }
                        let _ = app_clone.emit(
                            "task-output",
                            TaskOutputPayload {
                                task_id: task_id_clone.clone(),
                                content: line,
                                log_type: "stdout".to_string(),
                            },
                        );
                    }
                }
            });
        }

        if let Some(stderr) = stderr {
            let app_clone = app.clone();
            let db_clone = db.clone();
            let task_id_clone = task_id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    {
                        let conn = db_clone.lock().unwrap();
                        let _ = db::add_task_log(&conn, &task_id_clone, &line, "stderr");
                    }
                    let _ = app_clone.emit(
                        "task-output",
                        TaskOutputPayload {
                            task_id: task_id_clone.clone(),
                            content: line,
                            log_type: "stderr".to_string(),
                        },
                    );
                }
            });
        }

        let exit_code = {
            let mut procs = processes.lock().await;
            if let Some(mut running) = procs.remove(&project_id) {
                match running.child.wait().await {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(_) => 1,
                }
            } else {
                {
                    let conn = db.lock().unwrap();
                    let _ = db::set_task_completed(&conn, &task_id, 130);
                }
                let _ = app.emit(
                    "task-completed",
                    TaskCompletedPayload {
                        task_id: task_id.clone(),
                        project_id: project_id.clone(),
                        exit_code: 130,
                        status: "failed".to_string(),
                        task_title: task.title.clone(),
                        project_name: project.name.clone(),
                    },
                );
                break;
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        {
            let conn = db.lock().unwrap();
            let _ = db::set_task_completed(&conn, &task_id, exit_code);
        }

        let status = if exit_code == 0 { "done" } else { "failed" };
        let _ = app.emit(
            "task-completed",
            TaskCompletedPayload {
                task_id: task_id.clone(),
                project_id: project_id.clone(),
                exit_code,
                status: status.to_string(),
                task_title: task.title.clone(),
                project_name: project.name.clone(),
            },
        );
    }

    {
        let mut active = active_queues.lock().unwrap();
        active.remove(&project_id);
    }

    let _ = app.emit(
        "queue-stopped",
        QueueStoppedPayload {
            project_id: project_id.clone(),
        },
    );
}

pub async fn stop_queue(processes: RunningProcesses, project_id: &str) {
    let mut procs = processes.lock().await;
    if let Some(mut running) = procs.remove(project_id) {
        let _ = running.child.kill().await;
    }
}

pub fn pause_queue(stop_flags: &StopFlags, project_id: &str) {
    let mut flags = stop_flags.lock().unwrap();
    flags.insert(project_id.to_string(), true);
}

pub async fn cancel_and_revert(
    processes: RunningProcesses,
    project_id: &str,
) -> Result<(), String> {
    let mut procs = processes.lock().await;
    if let Some(mut running) = procs.remove(project_id) {
        let _ = running.child.kill().await;
        // Wait for process to fully exit
        let _ = running.child.wait().await;

        if let Some(hash) = &running.git_hash {
            // Reset to pre-task state
            let output = std::process::Command::new("git")
                .args(["reset", "--hard", hash])
                .current_dir(&running.project_path)
                .output()
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("git reset failed: {}", stderr));
            }

            // Clean untracked files that claude may have created
            let _ = std::process::Command::new("git")
                .args(["clean", "-fd"])
                .current_dir(&running.project_path)
                .output();
        }
    }
    Ok(())
}
