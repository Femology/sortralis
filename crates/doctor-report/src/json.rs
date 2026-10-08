//! JSON report renderer.
use doctor_core::report::Report;
use std::io::{self, Write};

pub fn write_json_report(writer: &mut impl Write, report: &Report) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, report).map_err(io::Error::other)?;
    writeln!(writer)?;
    Ok(())
}
