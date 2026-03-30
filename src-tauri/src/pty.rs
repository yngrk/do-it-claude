use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri::Emitter;

pub type PtySessions = Arc<Mutex<HashMap<String, PtySession>>>;

pub struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
}

pub fn new_pty_sessions() -> PtySessions {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn spawn_pty(
    app: AppHandle,
    sessions: PtySessions,
    session_id: String,
    cwd: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l"); // Login shell: sources .zprofile/.zshrc so PATH includes Homebrew, npm, etc.
    cmd.cwd(&cwd);

    // On macOS, Tauri apps inherit a minimal environment. Resolve the user's
    // shell PATH so commands like `claude`, `node`, `cargo` are available.
    if let Ok(output) = std::process::Command::new("/bin/zsh")
        .args(["-ilc", "echo $PATH"])
        .output()
    {
        if let Ok(shell_path) = String::from_utf8(output.stdout) {
            let shell_path = shell_path.trim();
            if !shell_path.is_empty() {
                cmd.env("PATH", shell_path);
            }
        }
    }

    pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    // Drop slave — we only need master
    drop(pair.slave);

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    {
        let mut s = sessions.lock().map_err(|e| e.to_string())?;
        s.insert(
            session_id.clone(),
            PtySession {
                writer,
                master: pair.master,
            },
        );
    }

    // Read thread: forward PTY output to frontend
    let sid = session_id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app.emit("pty-output", (&sid, &data));
                }
                Err(_) => break,
            }
        }
        let _ = app.emit("pty-exit", &sid);
    });

    Ok(())
}

pub fn write_to_pty(sessions: &PtySessions, session_id: &str, data: &str) -> Result<(), String> {
    let mut s = sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session) = s.get_mut(session_id) {
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        session.writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn resize_pty(
    sessions: &PtySessions,
    session_id: &str,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let s = sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session) = s.get(session_id) {
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn close_pty(sessions: &PtySessions, session_id: &str) -> Result<(), String> {
    let mut s = sessions.lock().map_err(|e| e.to_string())?;
    s.remove(session_id);
    Ok(())
}
