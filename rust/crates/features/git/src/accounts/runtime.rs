use super::*;

pub trait GitCredentialProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<String, GitAccountError>;
    fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, GitAccountError>;
}
