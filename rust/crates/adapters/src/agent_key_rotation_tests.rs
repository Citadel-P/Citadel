use super::*;

#[test]
fn rotation_persists_restricted_key_and_updates_every_signer_clone() {
    let root = std::env::temp_dir().join(format!("citadel-key-{}", uuid::Uuid::now_v7()));
    let path = root.join("signing-key");
    let signer = AgentRequestSigner::load_or_create(&path).unwrap();
    let clone = signer.clone();
    let previous = signer.public_key_base64();
    let public = signer.rotate().unwrap();
    assert_ne!(public, previous);
    assert_eq!(clone.public_key_base64(), public);
    assert_eq!(
        AgentRequestSigner::from_file(&path)
            .unwrap()
            .public_key_base64(),
        public
    );
    let request = clone.sign_with((), "/test", None, 1, [0; 16]).unwrap();
    let signature = request
        .metadata()
        .get_bin("x-signature-bin")
        .unwrap()
        .to_bytes()
        .unwrap();
    let mut payload = 1_i64.to_le_bytes().to_vec();
    payload.extend_from_slice(&[0; 16]);
    payload.extend_from_slice(b"/test");
    payload.extend_from_slice(&Sha256::digest([]));
    use ed25519_dalek::Verifier;
    signer
        .key
        .read()
        .unwrap()
        .verifying_key()
        .verify(
            &payload,
            &ed25519_dalek::Signature::from_slice(&signature).unwrap(),
        )
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_file(path).unwrap();
    fs::remove_dir(root).unwrap();
}

#[test]
fn rotation_without_durable_storage_does_not_change_the_active_key() {
    let signer = AgentRequestSigner::from_bytes(&[11; 32]);
    let before = signer.public_key_base64();
    assert!(signer.rotate().is_err());
    assert_eq!(before, signer.public_key_base64());
}

#[test]
fn rotation_write_failure_preserves_the_active_and_persisted_key() {
    let root = std::env::temp_dir().join(format!("citadel-key-failure-{}", uuid::Uuid::now_v7()));
    let moved = root.with_extension("moved");
    let signer = AgentRequestSigner::load_or_create(&root.join("signing-key")).unwrap();
    let clone = signer.clone();
    let before = signer.public_key_base64();
    fs::rename(&root, &moved).unwrap();
    assert!(signer.rotate().is_err());
    assert_eq!(clone.public_key_base64(), before);
    assert_eq!(
        AgentRequestSigner::from_file(&moved.join("signing-key"))
            .unwrap()
            .public_key_base64(),
        before
    );
    fs::remove_file(moved.join("signing-key")).unwrap();
    fs::remove_dir(moved).unwrap();
}

#[test]
fn concurrent_rotations_leave_disk_and_all_clones_on_the_same_key() {
    let root =
        std::env::temp_dir().join(format!("citadel-key-concurrent-{}", uuid::Uuid::now_v7()));
    let path = root.join("signing-key");
    let signer = AgentRequestSigner::load_or_create(&path).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let signer = signer.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                signer.rotate().unwrap()
            })
        })
        .collect();
    let keys: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_ne!(keys[0], keys[1]);
    let active = signer.public_key_base64();
    assert!(keys.contains(&active));
    assert_eq!(
        AgentRequestSigner::from_file(&path)
            .unwrap()
            .public_key_base64(),
        active
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_file(path).unwrap();
    fs::remove_dir(root).unwrap();
}
