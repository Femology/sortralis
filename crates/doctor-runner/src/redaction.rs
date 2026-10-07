//! Conservative presentation redaction, not a guarantee that arbitrary secrets
//! can be recognized. Public Stellar G/C/M addresses are deliberately preserved.
use doctor_core::CommandResult;
const MASK: &str = "[REDACTED]";
fn sensitive(name: &str) -> bool {
    let name = name
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
        .to_ascii_lowercase()
        .replace('-', "_");
    [
        "api_key",
        "apikey",
        "access_token",
        "auth_token",
        "token",
        "authorization",
        "password",
        "private_key",
        "secret_key",
        "secret",
        "seed",
        "mnemonic",
    ]
    .contains(&name.as_str())
}
fn token_secret(token: &str) -> bool {
    (token.len() == 56
        && token.starts_with('S')
        && token
            .bytes()
            .all(|b| b.is_ascii_uppercase() || (b'2'..=b'7').contains(&b)))
        || ["ghp_", "gho_", "ghs_", "github_pat_", "sk-"]
            .iter()
            .any(|p| token.starts_with(p) && token.len() >= p.len() + 5)
        || (token.starts_with("eyJ")
            && token.trim_end_matches('.').matches('.').count() == 2
            && token.len() > 20)
}
fn tokens(line: &str) -> String {
    let mut result = String::new();
    let mut token = String::new();
    let flush = |token: &mut String, result: &mut String| {
        if token_secret(token) {
            result.push_str(MASK);
        } else {
            result.push_str(token);
        }
        token.clear();
    };
    for c in line.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
            token.push(c);
        } else {
            flush(&mut token, &mut result);
            result.push(c);
        }
    }
    flush(&mut token, &mut result);
    if let Some(scheme) = result.find("://") {
        let rest = &result[scheme + 3..];
        if let Some(end) = rest.find('@').filter(|e| !rest[..*e].contains('/')) {
            if rest[..end].contains(':') {
                result.replace_range(scheme + 3..scheme + 3 + end, MASK);
            }
        }
    }
    result
}
pub fn redact(text: &str) -> String {
    let mut result = String::new();
    let mut pem = false;
    for line in text.split_inclusive('\n') {
        if line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----") {
            pem = true;
            result.push_str("[REDACTED PRIVATE KEY]\n");
            continue;
        }
        if pem {
            if line.contains("-----END ") && line.contains("PRIVATE KEY-----") {
                pem = false;
            }
            continue;
        }
        let mut cursor = 0;
        let mut masked = String::new();
        for (index, c) in line.char_indices() {
            if index < cursor || !matches!(c, '=' | ':') {
                continue;
            }
            let before = line[..index].trim_end();
            let name = before
                .rsplit(|c: char| c.is_whitespace() || c == ':' || c == '{' || c == ',')
                .next()
                .unwrap_or(before);
            if !sensitive(name) {
                continue;
            }
            let rest = &line[index + 1..];
            let start = index + 1 + rest.len() - rest.trim_start_matches([' ', '\t']).len();
            let value = &line[start..];
            let end = if value.starts_with(['\"', '\'']) {
                let quote = value.chars().next().unwrap_or('\"');
                let mut escaped = false;
                let mut end = line.len();
                for (offset, ch) in value.char_indices().skip(1) {
                    if !escaped && ch == quote {
                        end = start + offset + ch.len_utf8();
                        break;
                    }
                    escaped = !escaped && ch == '\\';
                }
                end
            } else if sensitive(name) && name.to_ascii_lowercase().contains("mnemonic") {
                line.trim_end_matches('\n').len()
            } else {
                let value = value.strip_prefix("Bearer ").unwrap_or(value);
                let offset = line[start..].len() - value.len();
                start
                    + offset
                    + value
                        .find(|c: char| c.is_whitespace() || matches!(c, ',' | '}' | ';'))
                        .unwrap_or(value.len())
            };
            masked.push_str(&line[cursor..start]);
            masked.push_str(MASK);
            cursor = end;
        }
        masked.push_str(&line[cursor..]);
        result.push_str(&tokens(&masked));
    }
    result
}
/// Redact stored display fields as well as the escaped terminal command.
pub fn safe_command(command: &mut CommandResult) -> String {
    command.program = redact(&command.program);
    command.working_directory =
        std::path::PathBuf::from(redact(&command.working_directory.to_string_lossy()));
    let mut mask_next = false;
    for arg in &mut command.args {
        let next = arg.starts_with('-') && sensitive(arg);
        *arg = if mask_next { MASK.into() } else { redact(arg) };
        mask_next = next;
    }
    command.stdout = redact(&command.stdout);
    command.stderr = redact(&command.stderr);
    format!("{:?} {:?}", command.program, command.args)
}
