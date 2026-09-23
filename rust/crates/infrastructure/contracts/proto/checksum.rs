use sha2::{Digest, Sha256};

pub(crate) fn normalized_sha256(contents: &str) -> String {
    // source.json pins LF-normalized text so Git checkout settings do not
    // change the checksum. Preserve every other byte of the contract.
    let normalized = contents.replace("\r\n", "\n");
    Sha256::digest(normalized.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_windows_and_mixed_line_endings_have_the_same_checksum() {
        let unix = "syntax = \"proto3\";\nmessage Request {}\n";
        let windows = unix.replace('\n', "\r\n");
        let mixed = unix.replacen('\n', "\r\n", 1);
        assert_eq!(normalized_sha256(unix), normalized_sha256(&windows));
        assert_eq!(normalized_sha256(unix), normalized_sha256(&mixed));
    }

    #[test]
    fn contract_changes_still_change_the_checksum() {
        assert_ne!(
            normalized_sha256("message Request { string name = 1; }\n"),
            normalized_sha256("message Request { string name = 2; }\r\n"),
        );
    }

    #[test]
    fn only_crlf_pairs_are_normalized() {
        assert_ne!(
            normalized_sha256("message\rRequest {}"),
            normalized_sha256("messageRequest {}")
        );
        assert_ne!(
            normalized_sha256("message Request {}\n"),
            normalized_sha256("message Request {}")
        );
    }
}
