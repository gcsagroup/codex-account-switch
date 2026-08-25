use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::error::{AppError, AppResult};

/// Restart the Codex desktop app so it reloads `auth.json`.
pub fn restart_codex_app() -> AppResult<String> {
    #[cfg(target_os = "macos")]
    {
        restart_macos()
    }
    #[cfg(target_os = "windows")]
    {
        restart_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(AppError::msg("当前系统暂不支持自动重启 Codex"))
    }
}

#[cfg(target_os = "macos")]
fn restart_macos() -> AppResult<String> {
    let app_path = resolve_macos_app_path()
        .ok_or_else(|| AppError::msg("未找到 ChatGPT/Codex 应用（/Applications）"))?;
    let app_name = app_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ChatGPT")
        .to_string();

    // 1) Ask it to quit politely.
    let _ = Command::new("osascript")
        .args(["-e", &format!("tell application \"{app_name}\" to quit")])
        .status();

    // 2) Wait until the main process is actually gone. ChatGPT often takes several seconds.
    if !wait_until_not_running(&app_name, Duration::from_secs(12)) {
        // 3) Force-kill leftovers so relaunch is not swallowed by a dying process.
        let _ = Command::new("killall").arg(&app_name).status();
        let _ = Command::new("killall").arg("ChatGPT").status();
        if !wait_until_not_running(&app_name, Duration::from_secs(5)) {
            return Err(AppError::msg(format!(
                "无法结束 {app_name}，请手动退出后再试"
            )));
        }
    }

    // 4) Brief settle time — LaunchServices ignores open() during teardown.
    thread::sleep(Duration::from_millis(1200));

    // 5) Relaunch by absolute path; retry if process doesn't appear.
    let mut last_err = String::new();
    for attempt in 1..=4 {
        match Command::new("open").arg(&app_path).output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                last_err = format!("open 失败: {}", String::from_utf8_lossy(&out.stderr).trim());
            }
            Err(e) => last_err = format!("open 错误: {e}"),
        }

        if wait_until_running(&app_name, Duration::from_secs(4)) {
            return Ok(format!("已重启 {app_name}"));
        }

        // Alternate launch styles on retry.
        if attempt == 2 {
            let _ = Command::new("open").args(["-a", &app_name]).status();
        } else if attempt >= 3 {
            let _ = Command::new("open").args(["-n", "-a", &app_name]).status();
        }
        thread::sleep(Duration::from_millis(800));
    }

    Err(AppError::msg(format!(
        "已退出 {app_name}，但重新打开失败。{}请手动打开 /Applications/{app_name}.app",
        if last_err.is_empty() {
            String::new()
        } else {
            format!("{last_err}。")
        }
    )))
}

#[cfg(target_os = "macos")]
fn resolve_macos_app_path() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "/Applications/ChatGPT.app",
        "/Applications/ChatGPT Classic.app",
        "/Applications/Codex.app",
        "/System/Applications/ChatGPT.app",
    ];
    for path in CANDIDATES {
        let p = PathBuf::from(path);
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn process_running(name: &str) -> bool {
    // Prefer pgrep — more reliable than System Events during app teardown.
    Command::new("pgrep")
        .args(["-x", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn wait_until_not_running(name: &str, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if !process_running(name) {
            // Double-check after a short pause; ChatGPT can briefly respawn helpers.
            thread::sleep(Duration::from_millis(400));
            if !process_running(name) {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    !process_running(name)
}

#[cfg(target_os = "macos")]
fn wait_until_running(name: &str, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if process_running(name) {
            return true;
        }
        // Also accept the binary path form used by ChatGPT.app.
        if Path::new("/Applications/ChatGPT.app/Contents/MacOS/ChatGPT").exists() {
            let running = Command::new("pgrep")
                .args(["-f", "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if running {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

#[cfg(target_os = "windows")]
fn restart_windows() -> AppResult<String> {
    for image in ["ChatGPT.exe", "Codex.exe"] {
        let _ = Command::new("taskkill").args(["/IM", image, "/F"]).status();
    }
    thread::sleep(Duration::from_millis(1200));

    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let candidates = [
        format!(r"{local}\Programs\ChatGPT\ChatGPT.exe"),
        r"C:\Program Files\ChatGPT\ChatGPT.exe".to_string(),
    ];

    for path in candidates {
        if Path::new(&path).is_file() {
            let status = Command::new("cmd")
                .args(["/C", "start", "", &path])
                .status()?;
            if status.success() {
                return Ok("已重启 ChatGPT".into());
            }
        }
    }

    Err(AppError::msg(
        "已切换账号，但未能自动重启 ChatGPT；请手动重启",
    ))
}
