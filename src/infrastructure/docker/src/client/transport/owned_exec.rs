use super::{DockerClient, DockerError};
use std::time::Duration;
use tokio::io::AsyncReadExt;
// The launcher cannot run the requested command until an identity-checked helper
// approves it. A lost/late startup response therefore leaves only a bounded
// launcher, not an unowned application command. Keep this longer than Core's
// opening + approval budgets. POSIX sleep is also required in the target image.
pub(super) const LAUNCH: &str = r#"
start_time() {
    IFS= read -r stat < "/proc/$1/stat" 2>/dev/null || return 1
    set -- ${stat##*) }
    [ "$4" = "$leader" ] || return 1
    shift 19
    printf '%s' "$1"
}
signal_owned() {
    [ -n "$2" ] && [ -n "$3" ] || return 1
    current=$(start_time "$2") || return 1
    [ "$current" = "$3" ] || return 1
    kill -"$1" "$2"
}
leader=$$
leader_started=$(start_time "$leader") || exit 125
approved=
(
    timer=
    timer_started=
    trap 'signal_owned TERM "$timer" "$timer_started" 2>/dev/null; wait; exit' TERM
    sleep 10 &
    timer=$!
    timer_started=$(start_time "$timer")
    wait "$timer"
    signal_owned ALRM "$leader" "$leader_started"
) </dev/null >/dev/null 2>&1 &
watchdog=$!
watchdog_started=$(start_time "$watchdog")
stop_watchdog() {
    [ -n "$watchdog" ] || return 0
    signal_owned TERM "$watchdog" "$watchdog_started" 2>/dev/null || :
    wait "$watchdog" 2>/dev/null || :
    watchdog=
}
trap 'stop_watchdog' 0
trap 'exit 125' ALRM
trap 'trap "" ALRM; approved=yes; stop_watchdog' USR1
printf 'CITADEL_TERMINAL %s %s\n' "$leader" "$leader_started"
if [ -z "$approved" ]; then
    wait "$watchdog" 2>/dev/null || :
fi
[ "$approved" = yes ] || exit 125
stop_watchdog
trap - 0 USR1 ALRM
exec "$@"
"#;

// First collect every member's PID and start time while the original leader is
// still identifiable. Killing a child may make its waiting parent exit, so the
// kill pass must use each captured identity rather than require a living leader.
// Deliberately detached sessions are outside this ownership boundary.
const CONTROL: &str = r#"
leader=$1
started=$2
action=$3
matches() {
    [ "$1" -gt 1 ] || return 1
    IFS= read -r stat < "/proc/$1/stat" 2>/dev/null || return 1
    expected=$2
    set -- ${stat##*) }
    [ "$4" = "$leader" ] || return 1
    shift 19
    [ "$1" = "$expected" ]
}
matches "$leader" "$started" || exit 1
if [ "$action" = approve ]; then
    kill -USR1 "$leader"
    exit $?
fi
members=
for proc in /proc/[0-9]*/stat; do
    matches "$leader" "$started" || exit 1
    IFS= read -r stat < "$proc" 2>/dev/null || continue
    pid=${stat%% *}
    [ "$pid" = "$leader" ] && continue
    [ "$pid" -gt 1 ] || continue
    set -- ${stat##*) }
    [ "$4" = "$leader" ] || continue
    shift 19
    members="$members $pid:$1"
done
failed=0
for member in $members; do
    pid=${member%:*}
    start=${member#*:}
    if matches "$pid" "$start"; then
        kill -KILL "$pid" 2>/dev/null || { matches "$pid" "$start" && failed=1; }
    fi
done
if matches "$leader" "$started"; then
    kill -KILL "$leader" 2>/dev/null || { matches "$leader" "$started" && failed=1; }
fi
exit "$failed"
"#;

#[derive(Debug)]
pub(super) struct ProcessIdentity {
    pub(super) pid: u32,
    pub(super) started: u64,
}

impl DockerClient {
    pub(super) async fn approve_exec(
        &self,
        id: &str,
        identity: &ProcessIdentity,
        user: Option<&str>,
    ) -> Result<(), DockerError> {
        self.control_exec(id, identity, user, "approve").await
    }

    async fn control_exec(
        &self,
        id: &str,
        identity: &ProcessIdentity,
        user: Option<&str>,
        action: &str,
    ) -> Result<(), DockerError> {
        let control_id = self.create_exec(id, serde_json::json!({
            "AttachStdout":true,"AttachStderr":true,"Tty":true,"User":user,
            "Cmd":["/bin/sh","-c",CONTROL,"citadel-exec-control",identity.pid.to_string(),identity.started.to_string(),action]
        })).await?;
        let mut socket = self.start_terminal_socket(&control_id).await?;
        let mut buffer = [0; 1024];
        while socket
            .read(&mut buffer)
            .await
            .map_err(DockerError::ProtocolIo)?
            != 0
        {}
        loop {
            let state = self.exec_inspect(&control_id).await?;
            if state.running == Some(false) {
                return if state.exit_code == Some(0) {
                    Ok(())
                } else {
                    Err(DockerError::Api {
                        status: super::StatusCode::BAD_GATEWAY,
                        message: "Docker exec ownership control failed.".into(),
                    })
                };
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub(super) async fn cleanup_exec(
        &self,
        id: &str,
        exec_id: &str,
        identity: &ProcessIdentity,
        user: Option<&str>,
    ) -> Result<(), DockerError> {
        if self.exec_inspect(exec_id).await?.running == Some(false) {
            return Ok(());
        }
        self.control_exec(id, identity, user, "cleanup").await?;
        loop {
            if self.exec_inspect(exec_id).await?.running == Some(false) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

pub(super) fn parse_identity(line: &[u8]) -> Option<ProcessIdentity> {
    let mut fields = std::str::from_utf8(line).ok()?.split_whitespace();
    if fields.next()? != "CITADEL_TERMINAL" {
        return None;
    }
    let pid = fields.next()?.parse::<u32>().ok()?;
    let started = fields.next()?.parse::<u64>().ok()?;
    if pid <= 1 || started == 0 || fields.next().is_some() {
        return None;
    }
    Some(ProcessIdentity { pid, started })
}
