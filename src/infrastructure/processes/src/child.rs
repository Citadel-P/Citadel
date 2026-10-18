//! Own a child and its Unix process group until output has drained and it is reaped.
use std::{io, process::ExitStatus};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

pub struct OwnedChild {
    // Drop the group before Child: the unreaped leader fences PID reuse.
    group: ProcessGroup,
    pub(crate) child: Child,
}
impl OwnedChild {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        command.kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command.spawn()?;
        Ok(Self {
            group: ProcessGroup::new(&child),
            child,
        })
    }
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }
    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }
    /// Call only after draining both output pipes, so cleanup retains the leader's identity.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = self.child.wait().await?;
        self.group.disarm();
        Ok(status)
    }
    pub async fn terminate(&mut self) -> io::Result<()> {
        self.group.kill();
        let _ = self.child.start_kill();
        self.child.wait().await?;
        Ok(())
    }
}
struct ProcessGroup {
    #[cfg(unix)]
    pid: Option<rustix::process::Pid>,
}
impl ProcessGroup {
    fn new(_child: &Child) -> Self {
        Self {
            #[cfg(unix)]
            pid: _child
                .id()
                .and_then(|id| i32::try_from(id).ok())
                .and_then(rustix::process::Pid::from_raw),
        }
    }
    fn kill(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.pid.take() {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
    fn disarm(&mut self) {
        #[cfg(unix)]
        {
            self.pid = None;
        }
    }
}
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.kill();
    }
}
