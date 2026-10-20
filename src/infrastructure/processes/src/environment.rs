//! Deliberate host configuration shared by command builders. Tool credentials,
//! HOME, Docker/Git configuration and workload variables must be supplied by callers.
use std::ffi::OsString;

pub fn tool_environment() -> Vec<(OsString, OsString)> {
    [
        "PATH",
        "SystemRoot",
        "PATHEXT",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "TZ",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ]
    .into_iter()
    .filter_map(|name| std::env::var_os(name).map(|value| (name.into(), value)))
    .collect()
}
