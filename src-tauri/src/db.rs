use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLog {
    pub id: String,
    pub task_id: String,
    pub content: String,
    pub log_type: String,
    pub created_at: String,
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
    ")?;

    // Migration: add tag column to existing databases
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN tag TEXT", []);

    Ok(conn)
}

pub fn create_project(conn: &Connection, name: &str, path: &str) -> Result<Project> {
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        path: path.to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    conn.execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![project.id, project.name, project.path, project.created_at],
    )?;
    Ok(project)
}

pub fn get_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, path, created_at FROM projects ORDER BY created_at DESC")?;
    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(projects)
}

pub fn delete_project(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM task_logs WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1)", params![id])?;
    conn.execute("DELETE FROM tasks WHERE project_id = ?1", params![id])?;
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
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
    };
    conn.execute(
        "INSERT INTO tasks (id, project_id, title, description, tag, status, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![task.id, task.project_id, task.title, task.description, task.tag, task.status, task.sort_order, task.created_at],
    )?;
    Ok(task)
}

pub fn get_tasks(conn: &Connection, project_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at FROM tasks WHERE project_id = ?1 ORDER BY sort_order ASC"
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
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(tasks)
}

pub fn update_task(conn: &Connection, id: &str, title: Option<&str>, description: Option<&str>) -> Result<Task> {
    if let Some(t) = title {
        conn.execute("UPDATE tasks SET title = ?1 WHERE id = ?2", params![t, id])?;
    }
    if let Some(d) = description {
        conn.execute("UPDATE tasks SET description = ?1 WHERE id = ?2", params![d, id])?;
    }
    let mut stmt = conn.prepare("SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at FROM tasks WHERE id = ?1")?;
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
        })
    })
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM task_logs WHERE task_id = ?1", params![id])?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn move_task(conn: &Connection, id: &str, new_status: &str, new_sort_order: i32) -> Result<Task> {
    conn.execute(
        "UPDATE tasks SET status = ?1, sort_order = ?2 WHERE id = ?3",
        params![new_status, new_sort_order, id],
    )?;
    let mut stmt = conn.prepare("SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at FROM tasks WHERE id = ?1")?;
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
        "SELECT id, project_id, title, description, tag, status, sort_order, exit_code, created_at, started_at, completed_at FROM tasks WHERE project_id = ?1 AND status = 'queued' ORDER BY sort_order ASC LIMIT 1"
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
