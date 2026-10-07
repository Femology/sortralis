//! Human-readable storage diff records. Full scan report formats are separate.
use doctor_core::storage::StorageDiff;
use std::io::{self, Write};

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
