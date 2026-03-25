use tauri::{AppHandle, Manager, State};
use crate::db::{self, DbConn, Project, Task, TaskLog, PromptTemplate};
use crate::executor::{self, RunningProcesses, StopFlags, SessionStore, ActiveQueues};
use crate::pty::{self, PtySessions};

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
    sessions: State<SessionStore>,
    project_id: String,
) -> Result<(), String> {
    let mut store = sessions.lock().map_err(|e| e.to_string())?;
    store.remove(&project_id);
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

#[tauri::command]
pub fn load_preset(app_handle: AppHandle, db: State<DbConn>, project_id: String, preset_name: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    let project_path = std::path::Path::new(&project.path);
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::mode_manager::load_preset(&app_dir, &project_id, project_path, &preset_name)
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
pub fn get_installed_skills(db: State<DbConn>, project_id: String) -> Result<Vec<String>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    crate::mode_manager::list_installed_skills(std::path::Path::new(&project.path))
}

#[tauri::command]
pub fn get_installed_agents(db: State<DbConn>, project_id: String) -> Result<Vec<String>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let project = db::get_project_by_id(&conn, &project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;
    crate::mode_manager::list_installed_agents(std::path::Path::new(&project.path))
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
pub fn update_project_system_prompt(db: State<DbConn>, id: String, system_prompt: Option<String>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::update_project_system_prompt(&conn, &id, system_prompt.as_deref()).map_err(|e| e.to_string())
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
