mod db;
mod commands;
mod executor;

use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");

            let conn = db::init_db(&app_dir).expect("failed to initialize database");
            let db_conn: db::DbConn = Arc::new(Mutex::new(conn));

            app.manage(db_conn);
            app.manage(executor::new_running_processes());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::get_projects,
            commands::delete_project,
            commands::validate_project_path,
            commands::create_task,
            commands::get_tasks,
            commands::update_task,
            commands::delete_task,
            commands::move_task,
            commands::get_task_logs,
            commands::start_queue,
            commands::stop_queue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
