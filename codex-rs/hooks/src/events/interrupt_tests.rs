use codex_protocol::protocol::HookEventName;
use codex_protocol::protocol::HookOutputEntry;
use codex_protocol::protocol::HookOutputEntryKind;
use codex_protocol::protocol::HookRunStatus;
use codex_utils_absolute_path::test_support::PathBufExt;
use codex_utils_absolute_path::test_support::test_path_buf;
use pretty_assertions::assert_eq;

use super::InterruptHandlerData;
use super::parse_completed;
use crate::engine::ConfiguredHandler;
use crate::engine::ConfiguredHandlerKind;
use crate::engine::HandlerRunResult;

#[test]
fn empty_stdout_succeeds() {
    let parsed = parse_completed(
        &handler(),
        run_result(Some(0), "", ""),
        /*turn_id*/ None,
    );

    assert_eq!(parsed.data, InterruptHandlerData);
    assert_eq!(parsed.completed.run.status, HookRunStatus::Completed);
    assert!(parsed.completed.run.entries.is_empty());
}

#[test]
fn system_message_becomes_warning() {
    let parsed = parse(r#"{"systemMessage":"watch the tide"}"#, Some(0));

    assert_eq!(parsed.data, InterruptHandlerData);
    assert_eq!(parsed.completed.run.status, HookRunStatus::Completed);
    assert_eq!(
        parsed.completed.run.entries,
        vec![warning("watch the tide")]
    );
}

#[test]
fn invalid_json_outputs_fail() {
    // Every rejected stdout keeps the same prefix but now carries its own
    // reason, so the cases assert their own message instead of sharing one.
    for (stdout, expected) in [
        (
            r#"{"continue":true}"#,
            "hook returned invalid interrupt hook JSON output: unknown field `continue`, expected `systemMessage`",
        ),
        (
            r#"{"stopReason":null}"#,
            "hook returned invalid interrupt hook JSON output: unknown field `stopReason`, expected `systemMessage`",
        ),
        (
            r#"{"suppressOutput":false}"#,
            "hook returned invalid interrupt hook JSON output: unknown field `suppressOutput`, expected `systemMessage`",
        ),
        (
            r#"{"decision":"block"}"#,
            "hook returned invalid interrupt hook JSON output: unknown field `decision`, expected `systemMessage`",
        ),
        (
            r#"{"systemMessage":"watch the tide""#,
            "hook returned invalid interrupt hook JSON output: stdout is not valid JSON: EOF while parsing an object at line 1 column 33",
        ),
    ] {
        assert_failed(stdout, Some(0), expected);
    }
}

#[test]
fn other_failures_use_standard_errors() {
    for (exit_code, stdout, expected) in [
        (Some(0), "aloha", "Interrupt hook returned non-JSON stdout"),
        (Some(2), "", "hook exited with code 2"),
    ] {
        assert_failed(stdout, exit_code, expected);
    }
}

fn assert_failed(stdout: &str, exit_code: Option<i32>, expected: &str) {
    let parsed = parse(stdout, exit_code);
    assert_eq!(parsed.data, InterruptHandlerData);
    assert_eq!(parsed.completed.run.status, HookRunStatus::Failed);
    assert_eq!(parsed.completed.run.entries, vec![error(expected)]);
}

fn parse(
    stdout: &str,
    exit_code: Option<i32>,
) -> crate::engine::dispatcher::ParsedHandler<InterruptHandlerData> {
    parse_completed(
        &handler(),
        run_result(exit_code, stdout, "ignored"),
        Some("turn-1".to_string()),
    )
}

fn warning(text: &str) -> HookOutputEntry {
    HookOutputEntry {
        kind: HookOutputEntryKind::Warning,
        text: text.to_string(),
    }
}

fn error(text: &str) -> HookOutputEntry {
    HookOutputEntry {
        kind: HookOutputEntryKind::Error,
        text: text.to_string(),
    }
}

fn handler() -> ConfiguredHandler {
    ConfiguredHandler {
        builtin: false,
        event_name: HookEventName::Interrupt,
        matcher: None,
        timeout_sec: 600,
        status_message: None,
        additional_context_limit: Default::default(),
        source_path: test_path_buf("/tmp/hooks.json").abs().into(),
        source: codex_protocol::protocol::HookSource::User,
        display_order: 0,
        kind: ConfiguredHandlerKind::Command {
            command: "echo hook".to_string(),
            r#async: false,
            env: std::collections::HashMap::new(),
        },
    }
}

fn run_result(exit_code: Option<i32>, stdout: &str, stderr: &str) -> HandlerRunResult {
    HandlerRunResult {
        started_at: 1,
        completed_at: 2,
        duration_ms: 1,
        exit_code,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        error: None,
    }
}
