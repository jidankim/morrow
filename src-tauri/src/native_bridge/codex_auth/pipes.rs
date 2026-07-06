use std::{
    io::Read,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

const PIPE_OUTPUT_BYTE_LIMIT: usize = 1024 * 1024;

type PipeReader = Receiver<Result<Vec<u8>, PipeReadError>>;

pub(super) enum PipeReadError {
    ReadFailed,
    OutputLimitExceeded,
    TimedOut,
}

pub(super) struct PipeReaders {
    stdout: PipeReader,
    stderr: PipeReader,
}

impl PipeReaders {
    pub(super) fn spawn<S, E>(stdout: S, stderr: E) -> Self
    where
        S: Read + Send + 'static,
        E: Read + Send + 'static,
    {
        Self {
            stdout: spawn_pipe_reader(stdout),
            stderr: spawn_pipe_reader(stderr),
        }
    }

    pub(super) fn join(self, timeout: Duration) -> Result<(Vec<u8>, Vec<u8>), PipeReadError> {
        let stdout = join_pipe_reader(self.stdout, timeout)?;
        let stderr = join_pipe_reader(self.stderr, timeout)?;
        Ok((stdout, stderr))
    }
}

fn spawn_pipe_reader<R>(mut pipe: R) -> PipeReader
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        drop(sender.send(read_limited_pipe(&mut pipe)));
    });
    receiver
}

fn read_limited_pipe<R>(pipe: &mut R) -> Result<Vec<u8>, PipeReadError>
where
    R: Read,
{
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = pipe
            .read(&mut buffer)
            .map_err(|_error| PipeReadError::ReadFailed)?;
        if read == 0 {
            return Ok(output);
        }
        let next_len = output
            .len()
            .checked_add(read)
            .ok_or(PipeReadError::OutputLimitExceeded)?;
        if next_len > PIPE_OUTPUT_BYTE_LIMIT {
            return Err(PipeReadError::OutputLimitExceeded);
        }
        output.extend_from_slice(&buffer[..read]);
    }
}

fn join_pipe_reader(reader: PipeReader, timeout: Duration) -> Result<Vec<u8>, PipeReadError> {
    match reader.recv_timeout(timeout) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(PipeReadError::TimedOut),
        Err(RecvTimeoutError::Disconnected) => Err(PipeReadError::ReadFailed),
    }
}
