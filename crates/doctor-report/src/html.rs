//! Self-contained, offline-readable HTML report renderer with strict HTML escaping.
use doctor_core::report::Report;
use doctor_core::Severity;
use std::io::{self, Write};

/// Strict HTML entity escaping for user- and repository-supplied strings.
/// Escapes: & -> &amp;, < -> &lt;, > -> &gt;, " -> &quot;, ' -> &#39;.
pub fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

pub fn write_html_report(writer: &mut impl Write, report: &Report) -> io::Result<()> {
    let verdict_str = format!("{:?}", report.verdict);
    let verdict_class = match report.verdict {
        doctor_core::Verdict::NotReady => "badge-danger",
        doctor_core::Verdict::ReadyForManualReview => "badge-warning",
        doctor_core::Verdict::ChecksCompletedWithWarnings => "badge-success",
    };

    writeln!(writer, "<!DOCTYPE html>")?;
    writeln!(writer, "<html lang=\"en\">")?;
    writeln!(writer, "<head>")?;
    writeln!(writer, "  <meta charset=\"utf-8\">")?;
    writeln!(
        writer,
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    )?;
    writeln!(
        writer,
        "  <title>Sortralis Upgrade Report - {}</title>",
        html_escape(&report.repository_path.display().to_string())
    )?;
    writeln!(writer, "  <style>")?;
    writeln!(
        writer,
        r#"
    :root {{
      --bg: #0d1117;
      --card-bg: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --text-muted: #8b949e;
      --heading: #f0f6fc;
      --danger: #f85149;
      --danger-bg: rgba(248, 81, 73, 0.15);
      --warning: #d29922;
      --warning-bg: rgba(210, 153, 34, 0.15);
      --success: #3fb950;
      --success-bg: rgba(63, 185, 80, 0.15);
      --info: #58a6ff;
      --info-bg: rgba(88, 166, 255, 0.15);
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      line-height: 1.5;
      padding: 24px;
      max-width: 1200px;
      margin: 0 auto;
    }}
    header {{
      margin-bottom: 24px;
      border-bottom: 1px solid var(--border);
      padding-bottom: 16px;
    }}
    h1 {{ color: var(--heading); font-size: 24px; margin-bottom: 8px; }}
    h2 {{ color: var(--heading); font-size: 18px; margin: 24px 0 12px 0; }}
    .subtitle {{ color: var(--text-muted); font-size: 14px; }}
    .cards {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 16px;
      margin-bottom: 24px;
    }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 16px;
    }}
    .card-title {{ font-size: 12px; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px; }}
    .card-value {{ font-size: 24px; font-weight: 600; color: var(--heading); }}
    .badge {{
      display: inline-block;
      padding: 2px 8px;
      font-size: 12px;
      font-weight: 600;
      border-radius: 12px;
      text-transform: uppercase;
    }}
    .badge-danger {{ background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger); }}
    .badge-warning {{ background: var(--warning-bg); color: var(--warning); border: 1px solid var(--warning); }}
    .badge-success {{ background: var(--success-bg); color: var(--success); border: 1px solid var(--success); }}
    .badge-info {{ background: var(--info-bg); color: var(--info); border: 1px solid var(--info); }}
    .filters {{
      margin-bottom: 16px;
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }}
    .filter-btn {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      color: var(--text);
      padding: 6px 12px;
      border-radius: 6px;
      cursor: pointer;
      font-size: 13px;
    }}
    .filter-btn:hover, .filter-btn.active {{
      background: #21262d;
      border-color: #8b949e;
      color: var(--heading);
    }}
    .finding-card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 16px;
      margin-bottom: 12px;
    }}
    .finding-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 8px;
    }}
    .finding-title {{
      font-size: 16px;
      font-weight: 600;
      color: var(--heading);
    }}
    .finding-meta {{
      font-size: 13px;
      color: var(--text-muted);
      margin-bottom: 8px;
    }}
    .finding-section {{
      margin-top: 8px;
      font-size: 14px;
    }}
    .finding-label {{
      font-weight: 600;
      color: var(--text-muted);
      font-size: 12px;
      text-transform: uppercase;
    }}
    details {{
      margin-top: 8px;
      cursor: pointer;
    }}
    summary {{
      color: var(--info);
      font-size: 13px;
      outline: none;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 8px;
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
    }}
    th, td {{
      padding: 10px 14px;
      text-align: left;
      border-bottom: 1px solid var(--border);
      font-size: 13px;
    }}
    th {{
      background: #21262d;
      color: var(--heading);
      font-weight: 600;
    }}
    footer {{
      margin-top: 40px;
      padding-top: 16px;
      border-top: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-muted);
      text-align: center;
    }}
    "#
    )?;
    writeln!(writer, "  </style>")?;
    writeln!(writer, "</head>")?;
    writeln!(writer, "<body>")?;

    // Header
    writeln!(writer, "  <header>")?;
    writeln!(writer, "    <h1>Sortralis Upgrade Report</h1>")?;
    writeln!(
        writer,
        "    <div class=\"subtitle\">Workspace: <code>{}</code> | Generated by {} v{}</div>",
        html_escape(&report.repository_path.display().to_string()),
        html_escape(&report.tool_name),
        html_escape(&report.tool_version)
    )?;
    writeln!(writer, "  </header>")?;

    // Summary KPI Cards
    writeln!(writer, "  <section class=\"cards\">")?;
    writeln!(
        writer,
        "    <div class=\"card\"><div class=\"card-title\">Verdict</div><div class=\"card-value\"><span class=\"badge {}\">{}</span></div></div>",
        verdict_class,
        html_escape(&verdict_str)
    )?;
    writeln!(
        writer,
        "    <div class=\"card\"><div class=\"card-title\">Breaking</div><div class=\"card-value\" style=\"color: var(--danger);\">{}</div></div>",
        report.summary.breaking_count
    )?;
    writeln!(
        writer,
        "    <div class=\"card\"><div class=\"card-title\">Warnings & Review</div><div class=\"card-value\" style=\"color: var(--warning);\">{}</div></div>",
        report.summary.warning_count + report.summary.manual_review_count
    )?;
    writeln!(
        writer,
        "    <div class=\"card\"><div class=\"card-title\">Total Findings</div><div class=\"card-value\">{}</div></div>",
        report.summary.total_findings
    )?;
    writeln!(
        writer,
        "    <div class=\"card\"><div class=\"card-title\">Verification Steps</div><div class=\"card-value\">{}/{} Passed</div></div>",
        report.summary.passed_steps,
        report.summary.total_steps
    )?;
    writeln!(writer, "  </section>")?;

    // Filter Buttons
    if !report.findings.is_empty() {
        writeln!(writer, "  <h2>Findings ({})</h2>", report.findings.len())?;
        writeln!(writer, "  <nav class=\"filters\">")?;
        writeln!(
            writer,
            "    <button class=\"filter-btn active\" data-filter=\"ALL\" onclick=\"filterSeverity('ALL')\">All ({})</button>",
            report.findings.len()
        )?;
        if report.summary.breaking_count > 0 {
            writeln!(
                writer,
                "    <button class=\"filter-btn\" data-filter=\"BREAKING\" onclick=\"filterSeverity('BREAKING')\">Breaking ({})</button>",
                report.summary.breaking_count
            )?;
        }
        if report.summary.warning_count > 0 {
            writeln!(
                writer,
                "    <button class=\"filter-btn\" data-filter=\"WARNING\" onclick=\"filterSeverity('WARNING')\">Warning ({})</button>",
                report.summary.warning_count
            )?;
        }
        if report.summary.manual_review_count > 0 {
            writeln!(
                writer,
                "    <button class=\"filter-btn\" data-filter=\"MANUAL_REVIEW\" onclick=\"filterSeverity('MANUAL_REVIEW')\">Manual Review ({})</button>",
                report.summary.manual_review_count
            )?;
        }
        if report.summary.info_count > 0 {
            writeln!(
                writer,
                "    <button class=\"filter-btn\" data-filter=\"INFO\" onclick=\"filterSeverity('INFO')\">Info ({})</button>",
                report.summary.info_count
            )?;
        }
        writeln!(writer, "  </nav>")?;

        // Findings List
        writeln!(writer, "  <section id=\"findings-container\">")?;
        for finding in &report.findings {
            let sev_str = match finding.severity {
                Severity::Breaking => "BREAKING",
                Severity::Warning => "WARNING",
                Severity::ManualReview => "MANUAL_REVIEW",
                Severity::Info => "INFO",
            };
            let badge_class = match finding.severity {
                Severity::Breaking => "badge-danger",
                Severity::Warning => "badge-warning",
                Severity::ManualReview => "badge-warning",
                Severity::Info => "badge-info",
            };

            writeln!(
                writer,
                "    <article class=\"finding-card\" data-severity=\"{}\">",
                sev_str
            )?;
            writeln!(writer, "      <div class=\"finding-header\">")?;
            writeln!(
                writer,
                "        <span class=\"finding-title\">{}: {}</span>",
                html_escape(finding.id.as_str()),
                html_escape(&finding.title)
            )?;
            writeln!(
                writer,
                "        <span class=\"badge {}\">{}</span>",
                badge_class, sev_str
            )?;
            writeln!(writer, "      </div>")?;
            writeln!(
                writer,
                "      <div class=\"finding-summary\">{}</div>",
                html_escape(&finding.summary)
            )?;

            // Locations and evidence
            for ev in &finding.evidence {
                if let Some(path) = &ev.path {
                    let line_str = ev.line.map(|l| format!(":{l}")).unwrap_or_default();
                    writeln!(
                        writer,
                        "      <div class=\"finding-meta\">Location: <code>{}{line_str}</code></div>",
                        html_escape(&path.display().to_string())
                    )?;
                }
                if !ev.message.is_empty() {
                    writeln!(
                        writer,
                        "      <div class=\"finding-meta\">Evidence: {}</div>",
                        html_escape(&ev.message)
                    )?;
                }
            }

            // Collapsible details (accessible without JS)
            writeln!(writer, "      <details open>")?;
            writeln!(writer, "        <summary>Remediation & Details</summary>")?;
            if !finding.why_it_matters.is_empty() {
                writeln!(
                    writer,
                    "        <div class=\"finding-section\"><span class=\"finding-label\">Why it matters:</span> {}</div>",
                    html_escape(&finding.why_it_matters)
                )?;
            }
            if !finding.recommendation.is_empty() {
                writeln!(
                    writer,
                    "        <div class=\"finding-section\"><span class=\"finding-label\">Remediation:</span> {}</div>",
                    html_escape(&finding.recommendation)
                )?;
            }
            if !finding.references.is_empty() {
                writeln!(writer, "        <div class=\"finding-section\"><span class=\"finding-label\">References:</span>")?;
                writeln!(writer, "          <ul>")?;
                for r in &finding.references {
                    writeln!(
                        writer,
                        "            <li><code>{}</code></li>",
                        html_escape(r)
                    )?;
                }
                writeln!(writer, "          </ul>")?;
                writeln!(writer, "        </div>")?;
            }
            writeln!(writer, "      </details>")?;
            writeln!(writer, "    </article>")?;
        }
        writeln!(writer, "  </section>")?;
    }

    // Verification Steps
    if !report.steps.is_empty() {
        writeln!(writer, "  <h2>Verification Steps</h2>")?;
        writeln!(writer, "  <table>")?;
        writeln!(writer, "    <thead><tr><th>Step</th><th>Status</th><th>Exit Code</th><th>Duration</th></tr></thead>")?;
        writeln!(writer, "    <tbody>")?;
        for step in &report.steps {
            let status_badge = if step.status.to_ascii_lowercase().contains("pass") {
                "badge-success"
            } else if step.status.to_ascii_lowercase().contains("fail") {
                "badge-danger"
            } else {
                "badge-info"
            };
            let exit_str = step
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".into());
            let dur_str = step
                .duration_millis
                .map(|d| format!("{d} ms"))
                .unwrap_or_else(|| "-".into());

            writeln!(
                writer,
                "      <tr><td>{}</td><td><span class=\"badge {}\">{}</span></td><td>{}</td><td>{}</td></tr>",
                html_escape(&step.name),
                status_badge,
                html_escape(&step.status),
                exit_str,
                dur_str
            )?;
        }
        writeln!(writer, "    </tbody>")?;
        writeln!(writer, "  </table>")?;
    }

    // Packages & Tools
    if !report.packages.is_empty() {
        writeln!(writer, "  <h2>Detected Packages</h2>")?;
        writeln!(writer, "  <table>")?;
        writeln!(
            writer,
            "    <thead><tr><th>Package</th><th>Manifest</th><th>SDK Requirement</th></tr></thead>"
        )?;
        writeln!(writer, "    <tbody>")?;
        for pkg in &report.packages {
            let sdk_str = pkg.soroban_sdk_requirement.as_deref().unwrap_or("-");
            writeln!(
                writer,
                "      <tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                html_escape(&pkg.name),
                html_escape(&pkg.manifest_path.display().to_string()),
                html_escape(sdk_str)
            )?;
        }
        writeln!(writer, "    </tbody>")?;
        writeln!(writer, "  </table>")?;
    }

    if !report.tools.is_empty() {
        writeln!(writer, "  <h2>Tools</h2>")?;
        writeln!(writer, "  <table>")?;
        writeln!(
            writer,
            "    <thead><tr><th>Tool</th><th>Version</th><th>Status</th></tr></thead>"
        )?;
        writeln!(writer, "    <tbody>")?;
        for tool in &report.tools {
            let ver_str = tool.version.as_deref().unwrap_or("-");
            writeln!(
                writer,
                "      <tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                html_escape(&tool.name),
                html_escape(ver_str),
                html_escape(&tool.status)
            )?;
        }
        writeln!(writer, "    </tbody>")?;
        writeln!(writer, "  </table>")?;
    }

    // Footer
    writeln!(writer, "  <footer>")?;
    writeln!(writer, "    <p>{}</p>", html_escape(&report.disclaimer))?;
    writeln!(writer, "  </footer>")?;

    // Minimal Embedded JS for Filtering (Safe & Offline)
    writer.write_all(
        br#"
  <script>
    function filterSeverity(sev) {
      var cards = document.querySelectorAll('.finding-card');
      cards.forEach(function(card) {
        if (sev === 'ALL' || card.getAttribute('data-severity') === sev) {
          card.style.display = 'block';
        } else {
          card.style.display = 'none';
        }
      });
      var buttons = document.querySelectorAll('.filter-btn');
      buttons.forEach(function(btn) {
        if (btn.getAttribute('data-filter') === sev) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      });
    }
  </script>
"#,
    )?;

    writeln!(writer, "</body>")?;
    writeln!(writer, "</html>")?;

    Ok(())
}
