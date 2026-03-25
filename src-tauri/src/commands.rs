use tauri::{AppHandle, Emitter, Manager, State};
use crate::db::{self, DbConn, Project, Task, TaskLog, TaskMessage, PromptTemplate};
use crate::executor::{self, RunningProcesses, StopFlags, SessionStore, ActiveQueues};
use crate::pty::{self, PtySessions};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tauri::command]
pub fn create_project(db: State<DbConn>, name: String, path: String) -> Result<Project, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_project(&conn, &name, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_projects(db: State<DbConn>) -> Result<Vec<Project>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_projects(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(db: State<DbConn>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::delete_project(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_project_path(path: String) -> bool {
    std::path::Path::new(&path).is_dir()
}

#[tauri::command]
pub fn check_git(path: String) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn init_git(path: String) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Initial commit so we have a HEAD to reset to
    let _ = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&path)
        .output();

    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit (Do It Claude)", "--allow-empty"])
        .current_dir(&path)
        .output();

    Ok(())
}

#[tauri::command]
pub fn create_task(db: State<DbConn>, project_id: String, title: String, description: String, tag: Option<String>) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_task(&conn, &project_id, &title, &description, tag.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks(db: State<DbConn>, project_id: String) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_tasks(&conn, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(db: State<DbConn>, id: String, title: Option<String>, description: Option<String>, tag: Option<Option<String>>) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let tag_ref = tag.as_ref().map(|t| t.as_deref());
    db::update_task(&conn, &id, title.as_deref(), description.as_deref(), tag_ref).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(db: State<DbConn>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::delete_task(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_task(db: State<DbConn>, id: String, new_status: String, new_sort_order: i32) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::move_task(&conn, &id, &new_status, new_sort_order).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task_logs(db: State<DbConn>, task_id: String) -> Result<Vec<TaskLog>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_task_logs(&conn, &task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task_messages(db: State<DbConn>, task_id: String) -> Result<Vec<TaskMessage>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_task_messages(&conn, &task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_queue(
    app: AppHandle,
    db: State<'_, DbConn>,
    processes: State<'_, RunningProcesses>,
    stop_flags: State<'_, StopFlags>,
    sessions: State<'_, SessionStore>,
    active_queues: State<'_, ActiveQueues>,
    project_id: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    let processes = processes.inner().clone();
    let stop_flags_inner = stop_flags.inner().clone();
    let sessions = sessions.inner().clone();
    let active_queues_inner = active_queues.inner().clone();

    if !executor::try_mark_active(&active_queues_inner, &project_id) {
        // Queue loop already running — just clear stop flag so it continues
        let mut flags = stop_flags_inner.lock().unwrap();
        flags.remove(&project_id);
        return Ok(());
    }

    // active flag is now set; spawn the loop
    tokio::spawn(async move {
        executor::start_queue(app, db, processes, stop_flags_inner, sessions, active_queues_inner, project_id).await;
    });

    Ok(())
}

#[tauri::command]
pub fn reset_session(
    db: State<DbConn>,
    sessions: State<SessionStore>,
    project_id: Option<String>,
    task_id: Option<String>,
) -> Result<(), String> {
    let task_ids = if let Some(task_id) = task_id {
        vec![task_id]
    } else if let Some(project_id) = project_id {
        let conn = db.lock().map_err(|e| e.to_string())?;
        db::get_tasks(&conn, &project_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
    } else {
        return Err("Either project_id or task_id is required".to_string());
    };

    let mut store = sessions.lock().map_err(|e| e.to_string())?;
    for task_id in task_ids {
        store.remove(&task_id);
    }
    Ok(())
}

#[tauri::command]
pub fn pause_queue(
    stop_flags: State<StopFlags>,
    project_id: String,
) -> Result<(), String> {
    executor::pause_queue(&stop_flags, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn stop_queue(
    processes: State<'_, RunningProcesses>,
    project_id: String,
) -> Result<(), String> {
    let processes = processes.inner().clone();
    executor::stop_queue(processes, &project_id).await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_and_revert(
    processes: State<'_, RunningProcesses>,
    project_id: String,
) -> Result<(), String> {
    let processes = processes.inner().clone();
    executor::cancel_and_revert(processes, &project_id).await
}

#[derive(serde::Serialize)]
pub struct ClaudeStatus {
    found: bool,
    path: String,
    version: String,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskChatStartedPayload {
    task_id: String,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskChatChunkPayload {
    task_id: String,
    content: String,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskChatCompletedPayload {
    task_id: String,
    project_id: String,
    project_name: String,
    task_title: String,
    message: TaskMessage,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskChatFailedPayload {
    task_id: String,
    error: String,
}

fn extract_assistant_text(json: &serde_json::Value) -> Vec<String> {
    let mut chunks = Vec::new();
    if let Some(content) = json.pointer("/message/content").and_then(|v| v.as_array()) {
        for item in content {
            if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        chunks.push(text.to_string());
                    }
                }
            }
        }
    }
    chunks
}

#[tauri::command]
pub async fn send_task_message(
    app: AppHandle,
    db: State<'_, DbConn>,
    sessions: State<'_, SessionStore>,
    processes: State<'_, RunningProcesses>,
    task_id: String,
    content: String,
) -> Result<TaskMessage, String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    let db_conn = db.inner().clone();
    let (task, project) = {
        let conn = db_conn.lock().map_err(|e| e.to_string())?;
        let task = db::get_task_by_id(&conn, &task_id).map_err(|e| e.to_string())?
            .ok_or_else(|| "Task not found".to_string())?;
        let project = db::get_project_by_id(&conn, &task.project_id).map_err(|e| e.to_string())?
            .ok_or_else(|| "Project not found".to_string())?;
        (task, project)
    };

    if processes.inner().lock().await.contains_key(&project.id) {
        return Err("Chat is unavailable while a task is actively running for this project".to_string());
    }

    let user_message = {
        let conn = db_conn.lock().map_err(|e| e.to_string())?;
        db::add_task_message(&conn, &task_id, "user", &content, "chat").map_err(|e| e.to_string())?
    };

    let _ = app.emit("task-chat-started", TaskChatStartedPayload {
        task_id: task_id.clone(),
    });

    let existing_session = {
        let session_id = sessions.inner().lock().map_err(|e| e.to_string())?.get(&task_id).cloned();
        session_id
    };

    let claude_bin = executor::resolve_claude_path();
    let shell_path = executor::get_shell_path();

    let mut cmd = tokio::process::Command::new(&claude_bin);
    cmd.arg("-p")
        .arg(&content)
        .arg("--dangerously-skip-permissions")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose");

    if let Some(ref sid) = existing_session {
        cmd.arg("--resume").arg(sid);
    }

    if let Some(ref prompt) = project.system_prompt {
        if !prompt.trim().is_empty() {
            cmd.arg("--append-system-prompt").arg(prompt);
        }
    }

    cmd.current_dir(&project.path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(ref path) = shell_path {
        cmd.env("PATH", path);
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture Claude stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture Claude stderr".to_string())?;

    let app_stdout = app.clone();
    let sessions_stdout = sessions.inner().clone();
    let task_id_stdout = task_id.clone();
    let stdout_handle = tokio::spawn(async move {
        let mut full_response = String::new();
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                match json.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                    "system" => {
                        if let Some(sid) = json.get("session_id").and_then(|v| v.as_str()) {
                            let mut store = sessions_stdout.lock().unwrap();
                            store.insert(task_id_stdout.clone(), sid.to_string());
                        }
                    }
                    "assistant" => {
                        for chunk in extract_assistant_text(&json) {
                            full_response.push_str(&chunk);
                            let _ = app_stdout.emit("task-chat-chunk", TaskChatChunkPayload {
                                task_id: task_id_stdout.clone(),
                                content: chunk,
                            });
                        }
                    }
                    "result" => {
                        if full_response.is_empty() {
                            if let Some(result_text) = json.get("result").and_then(|v| v.as_str()) {
                                full_response.push_str(result_text);
                                let _ = app_stdout.emit("task-chat-chunk", TaskChatChunkPayload {
                                    task_id: task_id_stdout.clone(),
                                    content: result_text.to_string(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        full_response
    });

    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut output = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            output.push(line);
        }
        output
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let assistant_content = stdout_handle.await.map_err(|e| e.to_string())?;
    let stderr_output = stderr_handle.await.map_err(|e| e.to_string())?;

    if !status.success() {
        let error = stderr_output.join("\n").trim().to_string();
        let error = if error.is_empty() { "Claude chat failed".to_string() } else { error };
        let _ = app.emit("task-chat-failed", TaskChatFailedPayload {
            task_id,
            error: error.clone(),
        });
        return Err(error);
    }

    let final_content = assistant_content.trim().to_string();
    if final_content.is_empty() {
        let error = "Claude returned an empty response".to_string();
        let _ = app.emit("task-chat-failed", TaskChatFailedPayload {
            task_id,
            error: error.clone(),
        });
        return Err(error);
    }

    let assistant_message = {
        let conn = db_conn.lock().map_err(|e| e.to_string())?;
        db::add_task_message(&conn, &user_message.task_id, "assistant", &final_content, "chat").map_err(|e| e.to_string())?
    };

    let _ = app.emit("task-chat-completed", TaskChatCompletedPayload {
        task_id: user_message.task_id.clone(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        task_title: task.title.clone(),
        message: assistant_message.clone(),
    });

    Ok(assistant_message)
}

#[tauri::command]
pub fn check_claude() -> ClaudeStatus {
    let path = crate::executor::resolve_claude_path();
    let is_absolute = std::path::Path::new(&path).is_absolute();

    if !is_absolute {
        return ClaudeStatus {
            found: false,
            path: String::new(),
            version: String::new(),
        };
    }

    let version = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| if o.status.success() {
            String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
        } else {
            None
        })
        .unwrap_or_default();

    ClaudeStatus {
        found: !version.is_empty(),
        path,
        version,
    }
}

#[tauri::command]
pub fn open_pty(
    app: AppHandle,
    sessions: State<PtySessions>,
    session_id: String,
    cwd: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    pty::spawn_pty(app, sessions.inner().clone(), session_id, cwd, cols, rows)
}

#[tauri::command]
pub fn write_pty(
    sessions: State<PtySessions>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    pty::write_to_pty(&sessions, &session_id, &data)
}

#[tauri::command]
pub fn resize_pty(
    sessions: State<PtySessions>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    pty::resize_pty(&sessions, &session_id, cols, rows)
}

#[tauri::command]
pub fn close_pty(
    sessions: State<PtySessions>,
    session_id: String,
) -> Result<(), String> {
    pty::close_pty(&sessions, &session_id)
}

#[derive(serde::Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct GitInfo {
    pub branch: String,
    pub changes: u32,
    pub commits: Vec<GitCommit>,
}

#[tauri::command]
pub fn get_git_info(path: String) -> Result<GitInfo, String> {
    // Get current branch
    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&path)
        .output()
        .map_err(|e| e.to_string())
        .and_then(|o| if o.status.success() {
            String::from_utf8(o.stdout).map(|s| s.trim().to_string()).map_err(|e| e.to_string())
        } else {
            Ok(String::from("unknown"))
        })?;

    // Count changes
    let changes = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&path)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count() as u32
        })
        .unwrap_or(0);

    // Recent commits
    let commits = std::process::Command::new("git")
        .args(["log", "--oneline", "-10", "--format=%h\t%s"])
        .current_dir(&path)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|line| {
                    let mut parts = line.splitn(2, '\t');
                    GitCommit {
                        hash: parts.next().unwrap_or("").to_string(),
                        message: parts.next().unwrap_or("").to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(GitInfo { branch, changes, commits })
}

fn open_folder(dir: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(dir).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(dir).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(dir).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_presets(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::list_presets(&app_dir)
}

// Backward-compatible command aliases for the existing frontend wiring.
#[tauri::command]
pub fn list_templates(app_handle: AppHandle) -> Result<Vec<String>, String> {
    list_presets(app_handle)
}

#[tauri::command]
pub fn load_preset(app_handle: AppHandle, db: State<DbConn>, project_id: String, preset_name: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    let project_path = std::path::Path::new(&project.path);
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::load_preset(&app_dir, &project_id, project_path, &preset_name)
}

// Backward-compatible command aliases for the existing frontend wiring.
#[tauri::command]
pub fn load_template(app_handle: AppHandle, db: State<DbConn>, project_id: String, template_name: String) -> Result<(), String> {
    load_preset(app_handle, db, project_id, template_name)
}

#[tauri::command]
pub fn restore_project_backup(app_handle: AppHandle, db: State<DbConn>, project_id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    let project_path = std::path::Path::new(&project.path);
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::restore_backup(&app_dir, &project_id, project_path)
}

#[tauri::command]
pub fn open_presets_folder(app_handle: AppHandle) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = app_dir.join("presets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open_folder(&dir)
}

// Backward-compatible command aliases for the existing frontend wiring.
#[tauri::command]
pub fn open_templates_folder(app_handle: AppHandle) -> Result<(), String> {
    open_presets_folder(app_handle)
}

#[tauri::command]
pub fn list_skills(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::list_skills(&app_dir)
}

#[tauri::command]
pub fn list_agents(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::list_agents(&app_dir)
}

#[tauri::command]
pub fn install_skill(app_handle: AppHandle, db: State<DbConn>, project_id: String, skill_name: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::install_skill(&app_dir, std::path::Path::new(&project.path), &skill_name)
}

#[tauri::command]
pub fn install_agent(app_handle: AppHandle, db: State<DbConn>, project_id: String, agent_name: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::install_agent(&app_dir, std::path::Path::new(&project.path), &agent_name)
}

#[tauri::command]
pub fn open_skills_folder(app_handle: AppHandle) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = app_dir.join("skills");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open_folder(&dir)
}

#[tauri::command]
pub fn open_agents_folder(app_handle: AppHandle) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = app_dir.join("agents");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open_folder(&dir)
}

#[tauri::command]
pub fn import_claude_file(app_handle: AppHandle, file_path: String, file_type: String) -> Result<String, String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dest_dir = match file_type.as_str() {
        "skill" => app_dir.join("skills"),
        "agent" => app_dir.join("agents"),
        _ => return Err(format!("Unknown file type: {}", file_type)),
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let src = std::path::Path::new(&file_path);
    let filename = src.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file path".to_string())?;
    let dest = dest_dir.join(filename);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    Ok(filename.to_string())
}

#[tauri::command]
pub fn update_project_system_prompt(db: State<DbConn>, id: String, system_prompt: Option<String>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::update_project_system_prompt(&conn, &id, system_prompt.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_project_context(db: State<DbConn>, project_id: String) -> Result<String, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    let context = crate::context_generator::generate_context(std::path::Path::new(&project.path));

    db::update_project_context(&conn, &project_id, &context).map_err(|e| e.to_string())?;

    Ok(context)
}

#[tauri::command]
pub fn get_templates(db: State<DbConn>) -> Result<Vec<PromptTemplate>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_templates(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_template(db: State<DbConn>, name: String, content: String) -> Result<PromptTemplate, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_template(&conn, &name, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_template(db: State<DbConn>, id: String, name: String, content: String) -> Result<PromptTemplate, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::update_template(&conn, &id, &name, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_template(db: State<DbConn>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::delete_template(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task_max_turns(db: State<DbConn>, id: String, max_turns: Option<i32>) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::update_task_max_turns(&conn, &id, max_turns).map_err(|e| e.to_string())?;
    db::get_task_by_id(&conn, &id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
pub fn estimate_task_turns(description: String, tag: Option<String>) -> i32 {
    crate::context_generator::estimate_max_turns(&description, tag.as_deref())
}

#[derive(serde::Serialize)]
pub struct TokenEstimate {
    prompt_tokens: usize,
    context_tokens: usize,
    system_tokens: usize,
    total_tokens: usize,
}

#[tauri::command]
pub fn estimate_task_tokens(db: State<DbConn>, task_id: String) -> Result<TokenEstimate, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let task = db::get_task_by_id(&conn, &task_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())?;
    let project = db::get_project_by_id(&conn, &task.project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    let chars_per_token = 4;
    let prompt_tokens = task.description.len() / chars_per_token;
    let context_tokens = project.project_context
        .as_ref()
        .map(|c| c.len() / chars_per_token)
        .unwrap_or(0);
    let system_tokens = project.system_prompt
        .as_ref()
        .map(|s| s.len() / chars_per_token)
        .unwrap_or(0);

    Ok(TokenEstimate {
        prompt_tokens,
        context_tokens,
        system_tokens,
        total_tokens: prompt_tokens + context_tokens + system_tokens,
    })
}
