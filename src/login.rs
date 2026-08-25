use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::identity;
use crate::paths;

/// Shared child handle so both the worker thread and the UI cancel button
/// can observe/kill the same `codex login` process.
type SharedChild = Arc<Mutex<Option<Child>>>;
pub type LoginWait =
    Box<dyn FnMut(&mut LoginCancel) -> AppResult<Option<PathBuf>> + Send + 'static>;

/// Handle to cancel an in-progress `codex login` flow.
pub struct LoginCancel {
    flag: Arc<AtomicBool>,
    child: SharedChild,
}

impl LoginCancel {
    /// A lightweight handle that only signals cancellation (no ownership of `Child`).
    pub fn clone_for_ui(&self) -> LoginCancelHandle {
        LoginCancelHandle {
            flag: Arc::clone(&self.flag),
            child: Arc::clone(&self.child),
        }
    }

    /// Signal cancellation. Best-effort: kills the child if we still own it.
    pub fn cancel(&mut self) {
        self.flag.store(true, Ordering::Relaxed);
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for LoginCancel {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// UI-side cancel handle (no Child ownership, just kill signal).
pub struct LoginCancelHandle {
    flag: Arc<AtomicBool>,
    child: SharedChild,
}

impl LoginCancelHandle {
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Launch `codex login` and wait for it to write `auth.json`.
/// Returns a cancel handle immediately; poll `wait` until it resolves.
pub fn start_codex_login() -> AppResult<(LoginCancel, LoginWait)> {
    let live = paths::live_auth_path();
    let before = std::fs::read(&live).ok();

    let child = Command::new("codex")
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            AppError::msg(format!(
                "无法启动 `codex login`（确认已安装 codex CLI）: {e}"
            ))
        })?;

    let cancel = LoginCancel {
        flag: Arc::new(AtomicBool::new(false)),
        child: Arc::new(Mutex::new(Some(child))),
    };

    let start = Instant::now();
    let timeout = Duration::from_secs(300);

    let wait = move |cancel: &mut LoginCancel| -> AppResult<Option<PathBuf>> {
        if cancel.flag.load(Ordering::Relaxed) {
            return Err(AppError::msg("登录已取消"));
        }
        if start.elapsed() > timeout {
            cancel.cancel();
            return Err(AppError::msg("登录超时，未完成授权"));
        }

        {
            let mut guard = cancel.child.lock().unwrap();
            if let Some(child) = guard.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() && auth_changed(&live, before.as_deref()) {
                            guard.take();
                            return Ok(Some(live.clone()));
                        }
                        drop(guard);
                        cancel.cancel();
                        return Err(AppError::msg(format!(
                            "`codex login` 已退出（{status}），未获取到新的 auth.json"
                        )));
                    }
                    Ok(None) => {}
                    Err(_) => {
                        drop(guard);
                        cancel.cancel();
                        return Err(AppError::msg("无法检查 `codex login` 状态"));
                    }
                }
            } else {
                // Child already reaped (cancel raced or external kill).
                drop(guard);
                if live.is_file() {
                    return Ok(Some(live.clone()));
                }
                return Err(AppError::msg("登录已取消"));
            }
        }

        // Some codex versions keep running after writing auth.json; accept that.
        if auth_changed(&live, before.as_deref()) {
            // Give codex a moment to finish flushing the file.
            std::thread::sleep(Duration::from_millis(500));
            if let Some(mut child) = cancel.child.lock().unwrap().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            return Ok(Some(live.clone()));
        }

        Ok(None)
    };

    Ok((cancel, Box::new(wait)))
}

fn auth_changed(path: &std::path::Path, before: Option<&[u8]>) -> bool {
    std::fs::read(path)
        .ok()
        .filter(|bytes| before != Some(bytes.as_slice()))
        .is_some_and(|bytes| identity::parse_auth_bytes(&bytes).is_ok())
}
