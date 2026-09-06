/// Keeps only a possible secret prefix and an incomplete UTF-8 character
/// between chunks. Never publish a prefix that could complete in the next read.
pub struct SecretRedactor<'a> {
    secrets: &'a [&'a str],
    pending: Vec<u8>,
}
impl<'a> SecretRedactor<'a> {
    pub fn new(secrets: &'a [&'a str]) -> Self {
        Self {
            secrets,
            pending: Vec::new(),
        }
    }
    pub fn push(&mut self, input: &[u8], finish: bool) -> String {
        self.pending.extend_from_slice(input);
        let limit = if !finish {
            match std::str::from_utf8(&self.pending) {
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                _ => self.pending.len(),
            }
        } else {
            self.pending.len()
        };
        let mut output = Vec::new();
        let mut offset = 0;
        while offset < limit {
            let rest = &self.pending[offset..];
            if self.secrets.iter().any(|secret| {
                !secret.is_empty()
                    && secret.len() > rest.len()
                    && secret.as_bytes().starts_with(rest)
            }) {
                if finish {
                    output.extend_from_slice(b"[redacted]");
                    offset = self.pending.len();
                }
                break;
            }
            if let Some(length) = self
                .secrets
                .iter()
                .filter(|secret| !secret.is_empty() && rest.starts_with(secret.as_bytes()))
                .map(|secret| secret.len())
                .max()
            {
                output.extend_from_slice(b"[redacted]");
                offset += length;
            } else {
                output.push(self.pending[offset]);
                offset += 1;
            }
        }
        self.pending.drain(..offset);
        String::from_utf8_lossy(&output).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_secrets_across_every_chunk_boundary_and_preserves_utf8() {
        let input = "hello 🔐 token-long-value and multi\nline value".as_bytes();
        let secrets = ["token", "token-long-value", "multi\nline"];
        for split in 0..=input.len() {
            let mut redactor = SecretRedactor::new(&secrets);
            let mut output = redactor.push(&input[..split], false);
            output.push_str(&redactor.push(&input[split..], false));
            output.push_str(&redactor.push(&[], true));
            assert_eq!(
                output, "hello 🔐 [redacted] and [redacted] value",
                "split {split}"
            );
        }
    }
    #[test]
    fn retains_only_a_possible_secret_prefix() {
        let secrets = ["secret-value"];
        let mut redactor = SecretRedactor::new(&secrets);
        assert_eq!(
            redactor.push(b"ordinary output sec", false),
            "ordinary output "
        );
        assert_eq!(redactor.pending, b"sec");
        assert_eq!(redactor.push(b"ret-value!", true), "[redacted]!");
        assert!(redactor.pending.is_empty());
    }
}
