use super::{VerificationError, OUTPUT_LIMIT};
use doctor_core::{
    verification::*, Category, CommandResult, CommandStatus, Evidence, Finding, RuleId, Severity,
};
use doctor_runner::{
    redact, safe_command, CapturedCommand, CommandRunner, CommandSpec, RunnerError,
    OUTPUT_TRUNCATION_MARKER,
};
use std::{io, time::Instant};

/// Long/truncated streams retain complete prefix lines only. This avoids exposing
/// a partial secret token at the truncation boundary. Final display is byte-bounded.
pub fn bounded_text(text: &str) -> (String, bool) {
    let stream_truncated = text.as_bytes().ends_with(OUTPUT_TRUNCATION_MARKER);
    let truncated = stream_truncated || text.len() > OUTPUT_LIMIT;
    let text = if stream_truncated {
        &text[..text.len() - OUTPUT_TRUNCATION_MARKER.len()]
    } else {
        text
    };
    let mut end = text
        .len()
        .min(OUTPUT_LIMIT.saturating_sub(OUTPUT_TRUNCATION_MARKER.len()));
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = if truncated {
        let prefix = &text[..end];
        &prefix[..prefix.rfind('\n').map(|i| i + 1).unwrap_or(0)]
    } else {
        text
    };
    let mut result = redact(prefix);
    if truncated {
        result.push_str("\n[OUTPUT TRUNCATED]\n");
    }
    // Redaction markers can increase short-token output; retain complete lines.
    if result.len() > OUTPUT_LIMIT {
        let mut end = OUTPUT_LIMIT - OUTPUT_TRUNCATION_MARKER.len();
        while !result.is_char_boundary(end) {
            end -= 1;
        }
        result.truncate(result[..end].rfind('\n').map(|i| i + 1).unwrap_or(0));
        result.push_str("\n[OUTPUT TRUNCATED]\n");
        return (result, true);
    }
    (result, truncated)
}

pub(super) fn failed(status: VerificationStatus) -> bool {
    !matches!(
        status,
        VerificationStatus::Passed | VerificationStatus::Skipped
    )
}
fn finding(
    id: &str,
    category: Category,
    name: &str,
    command: Option<CommandResult>,
    reason: &str,
) -> Result<Finding, VerificationError> {
    Ok(Finding {
        id: RuleId::new(id).map_err(VerificationError::RuleId)?,
        title: redact(name),
        severity: Severity::Breaking,
        category,
        summary: bounded_text(reason).0,
        why_it_matters:
            "Verification did not complete successfully; review the actual command evidence.".into(),
        evidence: vec![Evidence {
            path: None,
            line: None,
            message: bounded_text(reason).0,
            command,
        }],
        recommendation: "Fix the reported project or tool issue, then rerun explicit verification."
            .into(),
        references: vec![],
    })
}
pub(super) fn record(
    report: &mut VerificationReport,
    name: &str,
    id: &str,
    category: Category,
    capture: CapturedCommand,
) -> Result<bool, VerificationError> {
    let status = match &capture.record.status {
        CommandStatus::Exited { code: 0 } => VerificationStatus::Passed,
        CommandStatus::Exited { .. } => VerificationStatus::Failed,
        CommandStatus::TimedOut => VerificationStatus::TimedOut,
        CommandStatus::FailedToStart { .. }
            if capture.spawn_error_kind == Some(io::ErrorKind::NotFound) =>
        {
            VerificationStatus::MissingTool
        }
        _ => VerificationStatus::ExecutionError,
    };
    let mut command = capture.record;
    let exit_code = match command.status {
        CommandStatus::Exited { code } => Some(code),
        _ => None,
    };
    let (stdout, stdout_truncated) = bounded_text(&command.stdout);
    let (stderr, stderr_truncated) = bounded_text(&command.stderr);
    command.stdout = stdout;
    command.stderr = stderr;
    if let CommandStatus::FailedToStart { message } | CommandStatus::ExecutionFailed { message } =
        &mut command.status
    {
        *message = bounded_text(message).0;
    }
    let displayed_command = safe_command(&mut command);
    let finding = if failed(status) {
        Some(finding(
            id,
            category,
            name,
            Some(command.clone()),
            &format!("{name}: {status:?}; review captured stdout/stderr"),
        )?)
    } else {
        None
    };
    let ok = !failed(status);
    report.steps.push(VerificationStep {
        name: redact(name),
        status,
        duration_millis: command.elapsed_millis,
        exit_code,
        displayed_command: Some(displayed_command),
        command: Some(command),
        stdout_truncated,
        stderr_truncated,
        reason: None,
        finding,
    });
    Ok(ok)
}
pub(super) fn error_step(
    report: &mut VerificationReport,
    name: &str,
    id: &str,
    category: Category,
    reason: &str,
    external: bool,
) -> Result<(), VerificationError> {
    report.steps.push(VerificationStep {
        name: redact(name),
        status: if external {
            VerificationStatus::ExecutionError
        } else {
            VerificationStatus::Failed
        },
        command: None,
        displayed_command: None,
        duration_millis: 0,
        exit_code: None,
        stdout_truncated: false,
        stderr_truncated: false,
        reason: Some(bounded_text(reason).0),
        finding: Some(finding(id, category, name, None, reason)?),
    });
    Ok(())
}
pub(super) fn skip(report: &mut VerificationReport, name: &str, reason: &str) {
    report.steps.push(VerificationStep {
        name: redact(name),
        status: VerificationStatus::Skipped,
        command: None,
        displayed_command: None,
        duration_millis: 0,
        exit_code: None,
        stdout_truncated: false,
        stderr_truncated: false,
        reason: Some(reason.into()),
        finding: None,
    });
}
pub(super) fn task(
    report: &mut VerificationReport,
    request: &CommandSpec,
    name: &str,
    id: &str,
    category: Category,
    runner: &impl CommandRunner,
) -> Result<bool, VerificationError> {
    let start = Instant::now();
    match runner.execute(request) {
        Ok(c) => record(report, name, id, category, c),
        Err(RunnerError::Execution { capture, .. }) => record(report, name, id, category, *capture),
        Err(e) => {
            let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            let capture = CapturedCommand {
                request: request.clone(),
                record: CommandResult {
                    program: request.program.to_string_lossy().into_owned(),
                    args: request
                        .args
                        .iter()
                        .map(|a| a.to_string_lossy().into_owned())
                        .collect(),
                    working_directory: request.working_directory.clone(),
                    status: CommandStatus::ExecutionFailed {
                        message: e.to_string(),
                    },
                    stdout: String::new(),
                    stderr: e.to_string(),
                    elapsed_millis: elapsed,
                },
                stdout_bytes: vec![],
                stderr_bytes: e.to_string().into_bytes(),
                spawn_error_kind: None,
            };
            record(report, name, id, category, capture)
        }
    }
}
