use std::fs;
use std::path::Path;
use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;
use pretty_assertions::assert_eq;

use super::CheckStatus;
use super::desktop::MAX_LOG_DAYS;
use super::desktop::inspect_desktop_session;
use super::desktop::recent_log_directories;

const CURRENT_SESSION: &str = "12345678-1234-1234-1234-123456789abc";
const OTHER_SESSION: &str = "87654321-4321-4321-4321-cba987654321";
const CURRENT_PROCESS: u32 = 12345;
const OTHER_PROCESS: u32 = 54321;

#[test]
fn current_desktop_session_reports_the_latest_local_handshake() {
    let logs = DesktopLogs::new();
    let directory = logs.day("2026/09/29");
    logs.write(
        &directory,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        concat!(
            "2026-08-07T12:00:00.000Z error [AppServerConnection] initialize_handshake_result outcome=failure transportKind=stdio\n",
            "2026-08-07T12:01:00.000Z info [AppServerConnection] Starting app-server connection hostId=local transport=websocket\n",
            "2026-08-07T12:02:00.000Z info [AppServerConnection] initialize_handshake_result outcome=success transportKind=websocket\n",
            "2026-08-07T12:03:00.000Z info [AppServerConnection] app_server_connection.state_changed hostId=remote transport=websocket\n",
            "2026-08-07T12:04:00.000Z error [AppServerConnection] initialize_handshake_result outcome=failure transportKind=websocket\n",
        ),
    );

    let (running, check) = inspect_desktop_session(&[directory], |pid| pid == CURRENT_PROCESS);

    assert!(running);
    assert_eq!(check.id, "desktop.app_server.handshake");
    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(
        check.summary,
        "the desktop app-server initialized successfully"
    );
}

#[test]
fn handshake_recorded_in_an_earlier_day_directory_is_detected() {
    let logs = DesktopLogs::new();
    let earlier = logs.day("2026/09/24");
    let current = logs.day("2026/09/29");
    logs.write(
        &earlier,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        "2026-09-24T00:38:44.000Z info [AppServerConnection] initialize_handshake_result outcome=success transportKind=stdio\n",
    );
    logs.write(
        &current,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        "2026-09-29T08:00:00.000Z info [AppServerConnection] app_server_connection.state_changed hostId=local transport=websocket\n",
    );

    let (running, check) =
        inspect_desktop_session(&[current, earlier], |pid| pid == CURRENT_PROCESS);

    assert!(running);
    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(
        check.summary,
        "the desktop app-server initialized successfully"
    );
}

#[test]
fn handshake_of_another_session_is_not_reported() {
    let logs = DesktopLogs::new();
    let earlier = logs.day("2026/09/24");
    let current = logs.day("2026/09/29");
    logs.write(
        &earlier,
        OTHER_SESSION,
        OTHER_PROCESS,
        "2026-09-24T00:38:44.000Z info [AppServerConnection] initialize_handshake_result outcome=failure transportKind=stdio\n",
    );
    logs.write(
        &current,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        "2026-09-29T08:00:00.000Z info [AppServerConnection] Starting app-server connection hostId=local transport=websocket\n",
    );

    let (running, check) =
        inspect_desktop_session(&[current, earlier], |pid| pid == CURRENT_PROCESS);

    assert!(running);
    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(
        check.summary,
        "no desktop app-server handshake was recorded"
    );
}

#[test]
fn recent_log_directories_start_at_the_current_day_and_are_bounded() {
    let root = Path::new("/logs/com.openai.codex");
    let now = DateTime::parse_from_rfc3339("2026-10-01T00:30:00Z")
        .expect("timestamp should parse")
        .with_timezone(&Utc);

    let directories = recent_log_directories(root, now);

    assert_eq!(directories.len(), MAX_LOG_DAYS as usize);
    assert_eq!(directories[0], root.join("2026/10/01"));
    assert_eq!(directories[1], root.join("2026/09/30"));
    assert_eq!(directories[2], root.join("2026/09/29"));
}

#[test]
fn failed_handshake_does_not_expose_sensitive_log_fields() {
    let logs = DesktopLogs::new();
    let directory = logs.day("2026/09/29");
    logs.write(
        &directory,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        "2026-08-07T12:00:00.000Z error [AppServerConnection] initialize_handshake_result errorMessage=\"desktop-secret outcome=success\" outcome=failure transportKind=stdio\n",
    );

    let (running, check) = inspect_desktop_session(&[directory], |_| true);

    assert!(running);
    assert_eq!(check.status, CheckStatus::Fail);
    assert_eq!(check.summary, "the desktop app-server failed to initialize");
    assert!(
        !serde_json::to_string(&check)
            .unwrap()
            .contains("desktop-secret")
    );
}

#[test]
fn handshake_failure_in_the_middle_of_a_log_segment_is_detected() {
    let logs = DesktopLogs::new();
    let directory = logs.day("2026/09/29");
    let padding = "x".repeat(96 * 1024);
    logs.write(
        &directory,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        &format!(
            "{padding}\n2026-08-07T12:00:00.000Z error [AppServerConnection] initialize_handshake_result outcome=failure transportKind=stdio\n{padding}"
        ),
    );

    let (running, check) = inspect_desktop_session(&[directory], |_| true);

    assert!(running);
    assert_eq!(check.status, CheckStatus::Fail);
}

#[test]
fn stopped_desktop_does_not_report_a_failure() {
    let logs = DesktopLogs::new();
    let directory = logs.day("2026/09/29");
    logs.write(
        &directory,
        CURRENT_SESSION,
        CURRENT_PROCESS,
        "2026-08-07T12:00:00.000Z error [AppServerConnection] initialize_handshake_result outcome=failure transportKind=stdio\n",
    );

    let (running, check) = inspect_desktop_session(&[directory], |_| false);

    assert!(!running);
    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(check.summary, "the desktop application is not running");
}

/// A temporary log root laid out the way the desktop app writes it:
/// `<root>/<year>/<month>/<day>/`.
struct DesktopLogs {
    root: tempfile::TempDir,
}

impl DesktopLogs {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("temporary directory should be created"),
        }
    }

    fn day(&self, day: &str) -> PathBuf {
        let directory = self.root.path().join(day);
        fs::create_dir_all(&directory).expect("day directory should be created");
        directory
    }

    fn write(&self, directory: &Path, session: &str, process: u32, contents: &str) {
        fs::write(
            directory.join(format!(
                "codex-desktop-{session}-{process}-t0-i0-120000-0.log"
            )),
            contents,
        )
        .expect("desktop log fixture should be created");
    }
}
