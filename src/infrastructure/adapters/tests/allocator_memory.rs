//! Opt-in process-memory regression for the packaged glibc allocation settings.
#![cfg(target_os = "linux")]

use citadel_adapters::security::identity::crypto::Argon2PasswordHasher;
use citadel_identity::PasswordHasher;

fn footprint_kib() -> u64 {
    // Count swapped pages too: pressure on the test host must not hide growth.
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmSwap:"))
        .map(|line| {
            line.split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum()
}

#[test]
#[ignore = "isolated RSS regression; set MALLOC_MMAP_THRESHOLD_=131072 and run with --ignored --nocapture"]
fn repeated_hashing_and_cross_thread_buffer_frees_have_bounded_idle_memory() {
    let hasher = Argon2PasswordHasher::default();
    let password = "allocator-regression-fixture-password";
    let hash = hasher.hash(password).unwrap();
    let baseline = footprint_kib();
    let mut peak = baseline;
    for _ in 0..20 {
        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    for _ in 0..5 {
                        assert!(hasher.verify(password, &hash));
                    }
                });
            }
        });
        for mib in [1, 4, 12, 16] {
            let buffer = vec![42_u8; mib * 1024 * 1024];
            let transferred = buffer.clone();
            std::thread::spawn(move || {
                std::hint::black_box(&transferred);
            })
            .join()
            .unwrap();
            std::hint::black_box(&buffer);
        }
        peak = peak.max(footprint_kib());
    }
    std::thread::sleep(std::time::Duration::from_secs(15));
    let idle = footprint_kib();
    eprintln!("RSS + swap KiB: baseline={baseline}, sampled_peak={peak}, idle={idle}");
    // This mixed workload includes two concurrent 19 MiB hashes and copied
    // 16 MiB buffers. Permit bounded reuse, not accumulation across rounds.
    assert!(
        idle <= baseline + 64 * 1024,
        "idle allocator growth exceeds 64 MiB"
    );
}
