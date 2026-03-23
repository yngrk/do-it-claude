use tauri::{AppHandle, State};
use crate::db::{self, DbConn, Project, Task, TaskLog};
use crate::executor::{self, RunningProcesses};

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
pub fn create_task(db: State<DbConn>, project_id: String, title: String, description: String) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::create_task(&conn, &project_id, &title, &description).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks(db: State<DbConn>, project_id: String) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::get_tasks(&conn, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(db: State<DbConn>, id: String, title: Option<String>, description: Option<String>) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    db::update_task(&conn, &id, title.as_deref(), description.as_deref()).map_err(|e| e.to_string())
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
    project_id: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    let processes = processes.inner().clone();

    tokio::spawn(async move {
        executor::start_queue(app, db, processes, project_id).await;
    });

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
