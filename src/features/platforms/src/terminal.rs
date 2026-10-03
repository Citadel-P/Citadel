//! Bounded interactive container sessions. Dropping output closes the session.
use crate::RuntimeCapabilityError;
use crate::RuntimeErrorKind;
use futures_util::{future::BoxFuture, stream::BoxStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub const MAX_TERMINAL_INPUT: usize = 16 * 1024;
pub const MAX_TERMINAL_OUTPUT: usize = 1024 * 1024;
pub const TERMINAL_INPUT_CAPACITY: usize = 8;

#[derive(Clone, Copy)]
pub enum TerminalShell {
    Sh,
    Bash,
}
impl TerminalShell {
    pub fn command(self) -> &'static str {
        match self {
            Self::Sh => "/bin/sh",
            Self::Bash => "/bin/bash",
        }
    }
}

#[derive(Debug)]
pub enum TerminalInput {
    Stdin(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}
impl TerminalInput {
    pub fn validate(&self) -> Result<(), RuntimeCapabilityError> {
        let valid = match self {
            Self::Stdin(bytes) => !bytes.is_empty() && bytes.len() <= MAX_TERMINAL_INPUT,
            Self::Resize { cols, rows } => (1..=1000).contains(cols) && (1..=1000).contains(rows),
        };
        if valid {
            Ok(())
        } else {
            Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "Invalid terminal input or dimensions.",
                false,
            ))
        }
    }
}

pub enum TerminalOutput {
    Data(Vec<u8>),
    Exit(i32),
}
pub type TerminalStream = BoxStream<'static, Result<TerminalOutput, RuntimeCapabilityError>>;

pub struct TerminalSession {
    pub input: TerminalInputSender,
    pub output: TerminalStream,
}
pub struct TerminalInputSender(mpsc::Sender<TerminalInput>);
impl TerminalInputSender {
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }
    // Fail explicitly on backpressure; never drop keystrokes or queue without bounds.
    pub fn try_send(&self, input: TerminalInput) -> Result<(), RuntimeCapabilityError> {
        input.validate()?;
        self.0.try_send(input).map_err(|_| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Unavailable,
                "Terminal input is closed or its queue is full.",
                false,
            )
        })
    }
}
pub fn input_channel() -> (TerminalInputSender, mpsc::Receiver<TerminalInput>) {
    let (sender, receiver) = mpsc::channel(TERMINAL_INPUT_CAPACITY);
    (TerminalInputSender(sender), receiver)
}

pub trait ContainerTerminalPort: Send + Sync {
    fn container_terminal<'a>(
        &'a self,
        id: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_is_bounded_and_invalid_input_does_not_consume_queue_space() {
        let (input, _receiver) = input_channel();
        assert!(
            input
                .try_send(TerminalInput::Stdin(vec![0; MAX_TERMINAL_INPUT + 1]))
                .is_err()
        );
        assert!(
            input
                .try_send(TerminalInput::Resize { cols: 0, rows: 24 })
                .is_err()
        );
        for _ in 0..TERMINAL_INPUT_CAPACITY {
            input.try_send(TerminalInput::Stdin(vec![1])).unwrap();
        }
        assert!(input.try_send(TerminalInput::Stdin(vec![1])).is_err());
    }
    #[test]
    fn closed_session_rejects_input() {
        let (input, receiver) = input_channel();
        drop(receiver);
        assert!(input.try_send(TerminalInput::Stdin(vec![1])).is_err());
    }
}
