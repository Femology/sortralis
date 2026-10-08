//! Terminal report renderer with optional ANSI colors and structured, readable layout.
use doctor_core::report::Report;
use doctor_core::Severity;
use std::io::{self, Write};

pub fn write_terminal_report(
    writer: &mut impl Write,
    report: &Report,
    use_color: bool,
) -> io::Result<()> {
    let (c_bold, c_red, c_yellow, c_green, c_cyan, c_reset) = if use_color {
        (
            "\x1b[1m",
            "\x1b[1;31m",
            "\x1b[1;33m",
            "\x1b[1;32m",
            "\x1b[36m",
            "\x1b[0m",
        )
    } else {
        ("", "", "", "", "", "")
    };

    // 1. Header & Project Location
    writeln!(
        writer,
        "{c_bold}Workspace:{} {}",
        c_reset,
        report.repository_path.display()
    )?;

    // 2. Summary & Counts
    writeln!(
        writer,
        "Summary: {} breaking, {} warning, {} manual review, {} info; {} steps ({} passed, {} failed, {} other)",
        report.summary.breaking_count,
        report.summary.warning_count,
        report.summary.manual_review_count,
        report.summary.info_count,
        report.summary.total_steps,
        report.summary.passed_steps,
        report.summary.failed_steps,
        report.summary.other_steps,
    )?;

    // 3. Project & Tool versions
    if !report.packages.is_empty() {
        write!(writer, "Packages: ")?;
        for (i, pkg) in report.packages.iter().enumerate() {
            if i > 0 {
                write!(writer, ", ")?;
            }
            write!(writer, "{}", pkg.name)?;
            if let Some(sdk) = &pkg.soroban_sdk_requirement {
                write!(writer, " (SDK: {sdk})")?;
            }
        }
        writeln!(writer)?;
    }

    if !report.tools.is_empty() {
        write!(writer, "Tools: ")?;
        for (i, tool) in report.tools.iter().enumerate() {
            if i > 0 {
                write!(writer, ", ")?;
            }
            if let Some(v) = &tool.version {
                write!(writer, "{} {v}", tool.name)?;
            } else {
                write!(writer, "{}: {}", tool.name, tool.status)?;
            }
        }
        writeln!(writer)?;
    }

    // 4. Verification steps
    for step in &report.steps {
        let status_color = if step.status.to_ascii_lowercase().contains("pass") {
            c_green
        } else if step.status.to_ascii_lowercase().contains("fail") {
            status_color_for_failure(use_color)
        } else {
            c_cyan
        };

        if let Some(dur) = step.duration_millis {
            writeln!(
                writer,
                "{}: {status_color}{}{c_reset} ({} ms)",
                step.name, step.status, dur
            )?;
        } else {
            writeln!(
                writer,
                "{}: {status_color}{}{c_reset}",
                step.name, step.status
            )?;
        }

        if let Some(desc) = &step.description {
            writeln!(writer, "  {desc}")?;
        }
    }

    // 5. Findings with location, evidence, remediation
    if !report.findings.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "{c_bold}Findings:{c_reset}")?;
        for finding in &report.findings {
            let (sev_color, sev_tag) = match finding.severity {
                Severity::Breaking => (c_red, "[BREAKING]"),
                Severity::Warning => (c_yellow, "[WARNING]"),
                Severity::ManualReview => (c_yellow, "[MANUAL_REVIEW]"),
                Severity::Info => (c_cyan, "[INFO]"),
            };

            writeln!(
                writer,
                "{sev_color}{sev_tag}{c_reset} {}: {}",
                finding.id.as_str(),
                finding.summary
            )?;

            for evidence in &finding.evidence {
                if let Some(path) = &evidence.path {
                    let line_str = evidence.line.map(|l| format!(":{l}")).unwrap_or_default();
                    writeln!(writer, "  Location: {}{line_str}", path.display())?;
                }
                if !evidence.message.is_empty() {
                    writeln!(writer, "  Evidence: {}", evidence.message)?;
                }
            }

            if !finding.why_it_matters.is_empty() {
                writeln!(writer, "  Why it matters: {}", finding.why_it_matters)?;
            }
            if !finding.recommendation.is_empty() {
                writeln!(writer, "  Remediation: {}", finding.recommendation)?;
            }
            for reference in &finding.references {
                writeln!(writer, "  Reference: {reference}")?;
            }
        }
    }

    // 6. Command executions (retained output)
    for cmd in &report.command_results {
        writeln!(
            writer,
            "Command: {} {} ({:?})",
            cmd.program,
            cmd.args.join(" "),
            cmd.status
        )?;
        if (cmd.args.first().is_none_or(|arg| arg != "metadata")
            || cmd.status != (doctor_core::CommandStatus::Exited { code: 0 }))
            && !cmd.stdout.is_empty()
        {
            writeln!(writer, "{}", cmd.stdout.trim_end())?;
        }
        if !cmd.stderr.is_empty() {
            writeln!(writer, "{}", cmd.stderr.trim_end())?;
        }
    }

    // 7. Verdict & Disclaimer
    let verdict_color = match report.verdict {
        doctor_core::Verdict::NotReady => c_red,
        doctor_core::Verdict::ReadyForManualReview => c_yellow,
        doctor_core::Verdict::ChecksCompletedWithWarnings => c_green,
    };
    writeln!(
        writer,
        "Verdict: {verdict_color}{:?}{c_reset}; exit {}",
        report.verdict, report.exit_code
    )?;
    writeln!(writer, "{}", report.disclaimer)?;

    Ok(())
}

fn status_color_for_failure(use_color: bool) -> &'static str {
    if use_color {
        "\x1b[1;31m"
    } else {
        ""
    }
}
