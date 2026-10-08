//! Human-readable storage diff records. Full scan report formats are separate.
use doctor_core::storage::StorageDiff;
use std::io::{self, Write};

pub fn write_git_diff(
    writer: &mut impl Write,
    diff: &doctor_core::git_diff::GitDiff,
) -> io::Result<()> {
    writeln!(
        writer,
        "Git source diff: {} ({}) -> {} ({})",
        diff.from_ref, diff.before.commit, diff.to_ref, diff.after.commit
    )?;
    writeln!(
        writer,
        "Dirty active worktree: {} (committed snapshots compared; dirty files excluded)",
        diff.dirty_active_worktree
    )?;
    writeln!(
        writer,
        "Contract candidates: {} -> {}",
        diff.before.contracts.len(),
        diff.after.contracts.len()
    )?;
    writeln!(writer, "{}", diff.scope)?;
    writeln!(writer, "SDK changes: {}", diff.sdk_changes.len())?;
    for sdk in &diff.sdk_changes {
        writeln!(
            writer,
            "  {} requested: {:?} -> {:?}; lockfile recorded: {:?} -> {:?}",
            sdk.package.display(),
            sdk.before_requested,
            sdk.after_requested,
            sdk.before_locked,
            sdk.after_locked
        )?;
    }
    for (label, predicate) in [
        ("Functions added", 0),
        ("Functions removed", 1),
        ("Functions changed", 2),
    ] {
        let changes: Vec<_> = diff
            .functions
            .iter()
            .filter(|c| match predicate {
                0 => c.before.is_empty(),
                1 => c.after.is_empty(),
                _ => !c.before.is_empty() && !c.after.is_empty(),
            })
            .collect();
        writeln!(writer, "{label}: {}", changes.len())?;
        for c in changes {
            writeln!(
                writer,
                "  {}: {:?} -> {:?}",
                c.subject,
                c.before.iter().map(|f| &f.signature).collect::<Vec<_>>(),
                c.after.iter().map(|f| &f.signature).collect::<Vec<_>>()
            )?;
        }
    }
    writeln!(writer, "Types changed/added/removed: {}", diff.types.len())?;
    for change in &diff.types {
        writeln!(writer, "  {}", change.subject)?;
    }
    writeln!(
        writer,
        "Events changed/added/removed: {}",
        diff.events.len()
    )?;
    for change in &diff.events {
        writeln!(writer, "  {}", change.subject)?;
    }
    writeln!(
        writer,
        "Storage migration/review records: {}",
        diff.storage
            .iter()
            .map(|s| s.diff.records.len())
            .sum::<usize>()
    )?;
    for package in &diff.storage {
        for record in &package.diff.records {
            writeln!(
                writer,
                "  {} {} [{}]: {}",
                package.package.display(),
                record.id,
                record.classification,
                record.summary
            )?;
            for (side, evidence) in [("before", &record.before), ("after", &record.after)] {
                for e in evidence {
                    if let Some(path) = &e.path {
                        writeln!(
                            writer,
                            "    {side} {}{}: {}",
                            path.display(),
                            e.line.map(|l| format!(":{l}")).unwrap_or_default(),
                            e.message
                        )?;
                    }
                }
            }
        }
    }
    for (label, kind) in [
        ("Findings introduced", 0),
        ("Findings resolved", 1),
        ("Findings changed", 2),
    ] {
        let changes: Vec<_> = diff
            .findings
            .iter()
            .filter(|c| match kind {
                0 => c.before.is_empty(),
                1 => c.after.is_empty(),
                _ => !c.before.is_empty() && !c.after.is_empty(),
            })
            .collect();
        writeln!(writer, "{label}: {}", changes.len())?;
        for c in changes {
            writeln!(writer, "  {}", c.subject)?;
        }
    }
    let uncertainty = |a: &doctor_core::git_diff::RefAnalysis| {
        a.contracts
            .iter()
            .map(|p| p.source.uncertainties.len() + p.storage.uncertainties.len())
            .sum::<usize>()
    };
    writeln!(
        writer,
        "Unresolved source observations: {} -> {}",
        uncertainty(&diff.before),
        uncertainty(&diff.after)
    )?;
    for (side, analysis) in [("before", &diff.before), ("after", &diff.after)] {
        for p in &analysis.contracts {
            writeln!(
                writer,
                "  {side} {} SDK rule context: {}",
                p.package.manifest_path.display(),
                p.rule_context
            )?;
        }
    }
    Ok(())
}

pub fn write_storage_diff(writer: &mut impl Write, diff: &StorageDiff) -> io::Result<()> {
    writeln!(writer, "{}", diff.scope)?;
    if diff.records.is_empty() {
        writeln!(
            writer,
            "No differences detected within the source analysis scope."
        )?;
    }
    for record in &diff.records {
        writeln!(
            writer,
            "{} [{}] {}: {}",
            record.id, record.classification, record.subject, record.summary
        )?;
        for (label, evidence) in [("before", &record.before), ("after", &record.after)] {
            for item in evidence {
                if let Some(path) = &item.path {
                    writeln!(
                        writer,
                        "  {label} {}{}: {}",
                        path.display(),
                        item.line.map(|line| format!(":{line}")).unwrap_or_default(),
                        item.message
                    )?;
                }
            }
        }
    }
    writeln!(writer, "Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.")
}

pub fn write_wasm_inspection(
    output: &mut impl std::io::Write,
    report: &doctor_wasm::WasmInspection,
) -> std::io::Result<()> {
    writeln!(output, "Local Wasm: {:?}", report.path)?;
    writeln!(
        output,
        "Stellar CLI: {}; SHA-256: {}",
        report.stellar_cli_version, report.hash
    )?;
    for (name, section) in [
        ("Interface", &report.interface),
        ("Meta", &report.meta),
        ("Env-meta", &report.env_meta),
    ] {
        writeln!(
            output,
            "{name}: {:?} ({} entries)",
            section.status,
            section.entries.len()
        )?;
        serde_json::to_writer_pretty(&mut *output, &section.entries)?;
        writeln!(output)?;
    }
    writeln!(
        output,
        "Build info: {:?} — {}",
        report.build_info.status, report.build_info.reason
    )?;
    writeln!(output, "{}", report.scope)
}

pub fn write_verification(
    output: &mut impl std::io::Write,
    report: &doctor_core::verification::VerificationReport,
) -> std::io::Result<()> {
    writeln!(
        output,
        "Verification workspace: {:?}",
        report.repository_path
    )?;
    for step in &report.steps {
        writeln!(
            output,
            "{}: {:?}; exit {:?}; {} ms",
            step.name, step.status, step.exit_code, step.duration_millis
        )?;
        if let Some(command) = &step.displayed_command {
            writeln!(output, "  Command: {command}")?;
        }
        if let Some(reason) = &step.reason {
            writeln!(output, "  Reason: {reason:?}")?;
        }
        if let Some(command) = &step.command {
            if !command.stdout.is_empty() {
                writeln!(
                    output,
                    "  stdout (truncated={}): {:?}",
                    step.stdout_truncated, command.stdout
                )?;
            }
            if !command.stderr.is_empty() {
                writeln!(
                    output,
                    "  stderr (truncated={}): {:?}",
                    step.stderr_truncated, command.stderr
                )?;
            }
        }
        if let Some(finding) = &step.finding {
            writeln!(output, "  {}: {}", finding.id.as_str(), finding.summary)?;
        }
    }
    writeln!(output, "Verification exit: {}", report.exit_code.as_u8())?;
    writeln!(output, "{}", report.scope)
}

pub fn write_interface_diff(
    output: &mut impl Write,
    diff: &doctor_core::interface::InterfaceDiff,
) -> io::Result<()> {
    writeln!(
        output,
        "Contract interface diff [{}]:",
        diff.analysis_source
    )?;
    if diff.analysis_source == doctor_core::interface::AnalysisSource::SourceApproximation {
        writeln!(
            output,
            "NOTICE: Source-derived approximation: not an exact on-ledger contract specification comparison."
        )?;
    }
    writeln!(output, "{}", diff.scope)?;

    let breaking_count = diff
        .records
        .iter()
        .filter(|r| r.classification == doctor_core::interface::InterfaceClassification::Breaking)
        .count();
    let review_count = diff
        .records
        .iter()
        .filter(|r| {
            r.classification == doctor_core::interface::InterfaceClassification::ReviewRequired
        })
        .count();
    let non_breaking_count = diff
        .records
        .iter()
        .filter(|r| {
            r.classification == doctor_core::interface::InterfaceClassification::NonBreaking
        })
        .count();

    writeln!(
        output,
        "Diff summary: {} breaking, {} review required, {} non-breaking changes",
        breaking_count, review_count, non_breaking_count
    )?;

    if diff.records.is_empty() {
        writeln!(
            output,
            "Identical contract interface: zero public function, type, error, or event changes detected."
        )?;
    } else {
        for record in &diff.records {
            writeln!(
                output,
                "[{}] {} {}: {}",
                record.classification, record.id, record.subject, record.summary
            )?;
            if let Some(before) = &record.before_evidence {
                writeln!(output, "  before: {before}")?;
            }
            if let Some(after) = &record.after_evidence {
                writeln!(output, "  after:  {after}")?;
            }
        }
    }

    if diff.has_breaking_changes {
        writeln!(
            output,
            "Verdict: NOT READY (breaking interface changes detected)"
        )?;
    } else if review_count > 0 {
        writeln!(
            output,
            "Verdict: READY_FOR_MANUAL_REVIEW (manual review items required)"
        )?;
    } else {
        writeln!(
            output,
            "Verdict: CHECKS_COMPLETED (no breaking interface changes)"
        )?;
    }

    writeln!(
        output,
        "Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
    )
}
