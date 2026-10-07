use doctor_core::{CommandResult, CommandStatus};
use std::{
    error::Error,
    ffi::OsString,
    fmt, fs,
    io::{self, Read},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub working_directory: PathBuf,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct CapturedCommand {
    /// Exact OS arguments, including non-UTF-8 strings when supported by the OS.
    pub request: CommandSpec,
    /// Text views use UTF-8 replacement; original bytes remain available below.
    pub record: CommandResult,
    pub stdout_bytes: Vec<u8>,
    pub stderr_bytes: Vec<u8>,
    pub spawn_error_kind: Option<io::ErrorKind>,
}

#[derive(Debug)]
pub enum RunnerError {
    InvalidTimeout,
    WorkingDirectory {
        path: PathBuf,
        source: io::Error,
    },
    Execution {
        stage: &'static str,
        source: io::Error,
        capture: Box<CapturedCommand>,
    },
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeout => f.write_str("command timeout must be greater than zero"),
            Self::WorkingDirectory { path, source } => {
                write!(f, "invalid working directory {}: {source}", path.display())
            }
            Self::Execution { stage, source, .. } => write!(f, "command {stage} failed: {source}"),
        }
    }
}

impl Error for RunnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTimeout => None,
            Self::WorkingDirectory { source, .. } | Self::Execution { source, .. } => Some(source),
        }
    }
}

type OutputBuffer = Arc<Mutex<Vec<u8>>>;

fn snapshot(buffer: &OutputBuffer) -> Vec<u8> {
    buffer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn capture(
    spec: &CommandSpec,
    start: Instant,
    status: CommandStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
) -> CapturedCommand {
    CapturedCommand {
        request: spec.clone(),
        record: CommandResult {
            program: spec.program.to_string_lossy().into_owned(),
            args: spec
                .args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            working_directory: spec.working_directory.clone(),
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            elapsed_millis: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
        stdout_bytes: stdout,
        stderr_bytes: stderr,
        spawn_error_kind: None,
    }
}

fn reader(
    pipe: impl Read + Send + 'static,
    buffer: OutputBuffer,
    cancel: Arc<AtomicBool>,
) -> io::Result<mpsc::Receiver<io::Result<()>>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("sortralis-output".into())
        .spawn(move || {
            let mut pipe = pipe;
            let mut chunk = [0; 8192];
            let result = loop {
                if cancel.load(Ordering::Relaxed) {
                    break Ok(());
                }
                match pipe.read(&mut chunk) {
                    Ok(0) => break Ok(()),
                    Ok(_) if cancel.load(Ordering::Relaxed) => break Ok(()),
                    Ok(size) => buffer
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .extend_from_slice(&chunk[..size]),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => break Err(error),
                }
            };
            let _ = sender.send(result);
        })?;
    Ok(receiver)
}

struct CancelReaders(Arc<AtomicBool>);

impl Drop for CancelReaders {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

struct ManagedChild {
    child: Child,
    reaped: bool,
    armed: bool,
}

impl ManagedChild {
    fn terminate(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use nix::{
                errno::Errno,
                sys::signal::{killpg, Signal},
                unistd::Pid,
            };
            // POSIX process IDs fit pid_t (i32); process_group(0) creates this PGID.
            match killpg(Pid::from_raw(self.child.id() as i32), Signal::SIGKILL) {
                Ok(()) | Err(Errno::ESRCH) => {}
                Err(error) => return Err(io::Error::from_raw_os_error(error as i32)),
            }
        }
        #[cfg(not(unix))]
        if !self.reaped {
            self.child.kill()?;
        }
        if !self.reaped {
            self.child.wait()?;
            self.reaped = true;
        }
        self.armed = false;
        Ok(())
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.armed {
            // Best-effort cleanup during error unwinding; explicit cleanup failures
            // are reported by execute before reaching this fallback.
            let _ = self.terminate();
        }
    }
}

fn exit_status(status: ExitStatus) -> CommandStatus {
    if let Some(code) = status.code() {
        CommandStatus::Exited { code }
    } else {
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;
        CommandStatus::Terminated { signal }
    }
}

/// Uses Command with separate arguments and null stdin, never a shell wrapper.
/// On Unix, timeout kills the process group. Other platforms kill the direct child.
pub fn execute(spec: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
    if spec.timeout.is_zero() {
        return Err(RunnerError::InvalidTimeout);
    }
    let metadata =
        fs::metadata(&spec.working_directory).map_err(|source| RunnerError::WorkingDirectory {
            path: spec.working_directory.clone(),
            source,
        })?;
    if !metadata.is_dir() {
        return Err(RunnerError::WorkingDirectory {
            path: spec.working_directory.clone(),
            source: io::Error::other("path is not a directory"),
        });
    }
    let start = Instant::now();
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let mut result = capture(
                spec,
                start,
                CommandStatus::FailedToStart {
                    message: error.to_string(),
                },
                vec![],
                vec![],
            );
            result.spawn_error_kind = Some(error.kind());
            return Ok(result);
        }
    };
    let mut managed = ManagedChild {
        child,
        reaped: false,
        armed: true,
    };
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let cancel = CancelReaders(Arc::new(AtomicBool::new(false)));
    let fail = |managed: &mut ManagedChild, stage, source: io::Error| -> RunnerError {
        let cleanup = managed.terminate().err();
        let source = match cleanup {
            Some(cleanup) => io::Error::other(format!("{source}; cleanup failed: {cleanup}")),
            None => source,
        };
        RunnerError::Execution {
            stage,
            capture: Box::new(capture(
                spec,
                start,
                CommandStatus::ExecutionFailed {
                    message: source.to_string(),
                },
                snapshot(&stdout),
                snapshot(&stderr),
            )),
            source,
        }
    };
    // Piped handles should exist; still return a typed error if this invariant fails.
    let out_pipe = managed
        .child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("stdout pipe unavailable"));
    let err_pipe = managed
        .child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("stderr pipe unavailable"));
    let (out_pipe, err_pipe) = match (out_pipe, err_pipe) {
        (Ok(out), Ok(err)) => (out, err),
        (Err(error), _) | (_, Err(error)) => return Err(fail(&mut managed, "pipe setup", error)),
    };
    let out_reader = reader(out_pipe, stdout.clone(), cancel.0.clone())
        .map_err(|error| fail(&mut managed, "stdout reader", error))?;
    let err_reader = reader(err_pipe, stderr.clone(), cancel.0.clone())
        .map_err(|error| fail(&mut managed, "stderr reader", error))?;
    let mut out_done = false;
    let mut err_done = false;
    let mut status = None;
    loop {
        for (receiver, done) in [(&out_reader, &mut out_done), (&err_reader, &mut err_done)] {
            if !*done {
                match receiver.try_recv() {
                    Ok(Ok(())) => *done = true,
                    Ok(Err(error)) => return Err(fail(&mut managed, "output capture", error)),
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(fail(
                            &mut managed,
                            "output reader",
                            io::Error::other("reader disconnected"),
                        ))
                    }
                }
            }
        }
        if status.is_none() {
            match managed.child.try_wait() {
                Ok(Some(value)) => {
                    managed.reaped = true;
                    status = Some(value);
                }
                Ok(None) => {}
                Err(error) => return Err(fail(&mut managed, "wait", error)),
            }
        }
        if let Some(value) = status.filter(|_| out_done && err_done) {
            managed.armed = false;
            return Ok(capture(
                spec,
                start,
                exit_status(value),
                snapshot(&stdout),
                snapshot(&stderr),
            ));
        }
        if start.elapsed() >= spec.timeout {
            managed
                .terminate()
                .map_err(|error| fail(&mut managed, "timeout cleanup", error))?;
            // Allow bounded pipe draining after killing the group, but never wait
            // indefinitely for an escaped descendant to close an inherited pipe.
            for (receiver, done) in [(&out_reader, out_done), (&err_reader, err_done)] {
                if !done {
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(Ok(())) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Ok(Err(error)) => {
                            return Err(fail(&mut managed, "timeout output capture", error))
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return Err(fail(
                                &mut managed,
                                "timeout output reader",
                                io::Error::other("reader disconnected"),
                            ))
                        }
                    }
                }
            }
            return Ok(capture(
                spec,
                start,
                CommandStatus::TimedOut,
                snapshot(&stdout),
                snapshot(&stderr),
            ));
        }
        thread::sleep(Duration::from_millis(5).min(spec.timeout.saturating_sub(start.elapsed())));
    }
}
