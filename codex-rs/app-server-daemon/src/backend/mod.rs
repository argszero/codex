mod pid;

use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

pub(crate) use pid::PidBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendKind {
    Pid,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendPaths {
    pub(crate) codex_bin: PathBuf,
    pub(crate) pid_file: PathBuf,
    pub(crate) update_pid_file: PathBuf,
    pub(crate) remote_control_enabled: bool,
    pub(crate) stop_grace_period_secs: Option<u64>,
    pub(crate) stop_timeout_secs: Option<u64>,
}

pub(crate) fn pid_backend(paths: BackendPaths) -> PidBackend {
    PidBackend::with_stop_policy(
        paths.codex_bin,
        paths.pid_file,
        paths.remote_control_enabled,
        pid::resolve_stop_grace(paths.stop_grace_period_secs),
        pid::resolve_stop_timeout(paths.stop_timeout_secs),
    )
}

pub(crate) fn pid_update_loop_backend(paths: BackendPaths) -> PidBackend {
    // The update loop has no turns to drain — always use the default stop policy
    // so an operator's unbounded drain setting can never wedge disable-auto-update.
    PidBackend::new_update_loop(paths.codex_bin, paths.update_pid_file)
}

pub(crate) async fn append_stderr_log_tail_context(pid_file: &Path, context: &mut String) {
    match pid::read_stderr_log_tail(pid_file).await {
        Ok(Some(tail)) => tail.append_to_context(context),
        Ok(None) => {}
        Err(err) => {
            context.push_str(&format!(
                "\n\nFailed to read managed app-server stderr log: {err:#}"
            ));
        }
    }
}
