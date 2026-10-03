use sha2::{Digest, Sha256};

pub fn code_hash(code: &str) -> String {
    let bytes = Sha256::digest(code.as_bytes());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn joined_logs(stdout: &[u8], stderr: &[u8], truncated: bool) -> String {
    let mut output = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&String::from_utf8_lossy(stderr));
    }
    if truncated {
        output.push_str("\n[output truncated]");
    }
    output
}

pub(crate) fn allow_net_authority(base_url: &str) -> &str {
    base_url
        .split_once("://")
        .map_or(base_url, |(_, authority)| authority)
        .trim_end_matches('/')
}

pub(crate) fn redact_run_logs(value: &str, token: &str) -> String {
    if token.is_empty() {
        return redact_logs(value);
    }
    redact_logs(&citadel_execution::SecretRedactor::new(&[token]).push(value.as_bytes(), true))
}

pub fn redact_logs(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut redact_next = false;
    for segment in value.split_inclusive(char::is_whitespace) {
        let token_end = segment.find(char::is_whitespace).unwrap_or(segment.len());
        let (part, whitespace) = segment.split_at(token_end);
        let lower = part.to_ascii_lowercase();
        if redact_next {
            output.push_str("[redacted]");
            redact_next = false;
        } else if lower == "bearer" {
            output.push_str(part);
            redact_next = true;
        } else if lower.starts_with("token=")
            || lower.starts_with("api_key=")
            || lower.starts_with("apikey=")
            || lower.starts_with("password=")
            || lower.starts_with("secret=")
        {
            output.push_str(part.split_once('=').map_or(part, |(key, _)| key));
            output.push_str("=[redacted]");
        } else {
            output.push_str(part);
        }
        output.push_str(whitespace);
    }
    output
}
