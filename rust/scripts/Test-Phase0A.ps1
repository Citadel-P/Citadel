$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$mount = "type=bind,source=$repoRoot,target=/repo"
$dockerArgs = @(
    'run', '--rm',
    '--mount', $mount,
    '--volume', 'citadel-rust-cargo-registry:/usr/local/cargo/registry',
    '--volume', 'citadel-rust-cargo-git:/usr/local/cargo/git',
    '--volume', 'citadel-rust-target:/repo/rust/target',
    '--workdir', '/repo/rust',
    'rust:1.97.1-bookworm'
)

& docker @dockerArgs cargo run --locked -p xtask -- docker --check
if ($LASTEXITCODE -ne 0) { throw 'Docker subset verification failed.' }

& docker @dockerArgs cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { throw 'rustfmt failed.' }

& docker @dockerArgs cargo clippy --workspace --all-targets --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'Clippy failed.' }

& docker @dockerArgs cargo test --workspace --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
