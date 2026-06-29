use std::{
    io::{self, Read},
    thread::{self, JoinHandle},
};

type PipeReader = JoinHandle<io::Result<Vec<u8>>>;

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

    pub(super) fn join(self) -> Result<(Vec<u8>, Vec<u8>), ()> {
        let stdout = join_pipe_reader(self.stdout)?;
        let stderr = join_pipe_reader(self.stderr)?;
        Ok((stdout, stderr))
    }
}

fn spawn_pipe_reader<R>(mut pipe: R) -> PipeReader
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = Vec::new();
        pipe.read_to_end(&mut output)?;
        Ok(output)
    })
}

fn join_pipe_reader(reader: PipeReader) -> Result<Vec<u8>, ()> {
    match reader.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(_)) | Err(_) => Err(()),
    }
}
