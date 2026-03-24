mod db;
mod commands;
mod executor;
mod pty;

use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");

            let conn = db::init_db(&app_dir).expect("failed to initialize database");
            let db_conn: db::DbConn = Arc::new(Mutex::new(conn));

            app.manage(db_conn);
            app.manage(executor::new_running_processes());
            app.manage(executor::new_stop_flags());
            app.manage(executor::new_session_store());
            app.manage(pty::new_pty_sessions());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::get_projects,
            commands::delete_project,
            commands::validate_project_path,
            commands::check_git,
            commands::init_git,
            commands::create_task,
            commands::get_tasks,
            commands::update_task,
            commands::delete_task,
            commands::move_task,
            commands::get_task_logs,
            commands::start_queue,
            commands::pause_queue,
            commands::stop_queue,
            commands::cancel_and_revert,
            commands::reset_session,
            commands::check_claude,
            commands::open_pty,
            commands::write_pty,
            commands::resize_pty,
            commands::close_pty,
            commands::get_git_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
