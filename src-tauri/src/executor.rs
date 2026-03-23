use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use crate::db::{self, DbConn};

pub type RunningProcesses = Arc<Mutex<HashMap<String, tokio::process::Child>>>;

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
    project_id: String,
) {
    loop {
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
        }

        let _ = app.emit("task-started", TaskStartedPayload {
            task_id: task.id.clone(),
            project_id: project_id.clone(),
        });

        let project_path = {
            let conn = db.lock().unwrap();
            let projects = db::get_projects(&conn).unwrap_or_default();
            projects.into_iter().find(|p| p.id == project_id).map(|p| p.path.clone())
        };

        let project_path = match project_path {
            Some(p) => p,
            None => break,
        };

        let mut child = match Command::new("claude")
            .arg("-p")
            .arg(&task.description)
            .current_dir(&project_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!("Failed to spawn claude: {}", e);
                {
                    let conn = db.lock().unwrap();
                    let _ = db::add_task_log(&conn, &task.id, &error_msg, "stderr");
                    let _ = db::set_task_completed(&conn, &task.id, 1);
                }
                let _ = app.emit("task-output", TaskOutputPayload {
                    task_id: task.id.clone(),
                    content: error_msg,
                    log_type: "stderr".to_string(),
                });
                let _ = app.emit("task-completed", TaskCompletedPayload {
                    task_id: task.id.clone(),
                    project_id: project_id.clone(),
                    exit_code: 1,
                });
                continue;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        {
            let mut procs = processes.lock().await;
            procs.insert(project_id.clone(), child);
        }

        let task_id = task.id.clone();

        if let Some(stdout) = stdout {
            let app_clone = app.clone();
            let db_clone = db.clone();
            let task_id_clone = task_id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    {
                        let conn = db_clone.lock().unwrap();
                        let _ = db::add_task_log(&conn, &task_id_clone, &line, "stdout");
                    }
                    let _ = app_clone.emit("task-output", TaskOutputPayload {
                        task_id: task_id_clone.clone(),
                        content: line,
                        log_type: "stdout".to_string(),
                    });
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
                    let _ = app_clone.emit("task-output", TaskOutputPayload {
                        task_id: task_id_clone.clone(),
                        content: line,
                        log_type: "stderr".to_string(),
                    });
                }
            });
        }

        let exit_code = {
            let mut procs = processes.lock().await;
            if let Some(mut child) = procs.remove(&project_id) {
                match child.wait().await {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(_) => 1,
                }
            } else {
                {
                    let conn = db.lock().unwrap();
                    let _ = db::set_task_completed(&conn, &task_id, 130);
                }
                let _ = app.emit("task-completed", TaskCompletedPayload {
                    task_id: task_id.clone(),
                    project_id: project_id.clone(),
                    exit_code: 130,
                });
                break;
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        {
            let conn = db.lock().unwrap();
            let _ = db::set_task_completed(&conn, &task_id, exit_code);
        }

        let _ = app.emit("task-completed", TaskCompletedPayload {
            task_id: task_id.clone(),
            project_id: project_id.clone(),
            exit_code,
        });
    }
}

pub async fn stop_queue(processes: RunningProcesses, project_id: &str) {
    let mut procs = processes.lock().await;
    if let Some(mut child) = procs.remove(project_id) {
        let _ = child.kill().await;
    }
}
