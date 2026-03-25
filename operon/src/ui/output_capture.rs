use std::io::{self, BufRead, BufReader};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::thread::JoinHandle;

type EmitFn = Box<dyn Fn(String) + Send + 'static>;

struct FdSpec {
    fd: RawFd,
    name: String,
    emit: EmitFn,
}

struct FdHandle {
    original: OwnedFd,
    target_fd: RawFd,
    thread: Option<JoinHandle<()>>,
}

pub struct FdRedirect {
    specs: Vec<FdSpec>,
}

pub struct FdRedirectHandle {
    handles: Vec<FdHandle>,
}

impl FdRedirectHandle {
    pub fn original_fd(&self, target: &impl AsRawFd) -> Option<std::fs::File> {
        self.handles
            .iter()
            .find(|h| h.target_fd == target.as_raw_fd())
            .map(|h| {
                let duped =
                    nix::unistd::dup(h.original.as_raw_fd()).expect("Failed to dup original fd");
                unsafe { std::fs::File::from_raw_fd(duped) }
            })
    }
}

impl FdRedirect {
    pub fn new() -> Self {
        Self { specs: Vec::new() }
    }

    pub fn with_fd(
        mut self,
        target: &impl AsRawFd,
        name: &str,
        emit: impl Fn(String) + Send + 'static,
    ) -> Self {
        self.specs.push(FdSpec {
            fd: target.as_raw_fd(),
            name: name.to_owned(),
            emit: Box::new(emit),
        });
        self
    }

    pub fn capture(self) -> io::Result<FdRedirectHandle> {
        let mut handles = Vec::with_capacity(self.specs.len());

        for spec in self.specs {
            match Self::capture_fd(spec) {
                Ok(handle) => handles.push(handle),
                Err(e) => {
                    drop(FdRedirectHandle { handles });
                    return Err(e);
                }
            }
        }

        Ok(FdRedirectHandle { handles })
    }

    fn capture_fd(spec: FdSpec) -> io::Result<FdHandle> {
        let original_raw = nix::unistd::dup(spec.fd).map_err(io::Error::from)?;
        let original = unsafe { OwnedFd::from_raw_fd(original_raw) };

        let (read_end, write_end) = nix::unistd::pipe().map_err(io::Error::from)?;

        nix::unistd::dup2(write_end.as_raw_fd(), spec.fd).map_err(io::Error::from)?;

        drop(write_end);

        let read_file: std::fs::File = read_end.into();
        let thread_name = format!("{}-capture", spec.name);
        let emit = spec.emit;

        let thread = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let reader = BufReader::new(read_file);
                for line in reader.lines() {
                    match line {
                        Ok(line) => emit(line),
                        Err(_) => break,
                    }
                }
            })?;

        Ok(FdHandle {
            original,
            target_fd: spec.fd,
            thread: Some(thread),
        })
    }
}

impl Drop for FdRedirectHandle {
    fn drop(&mut self) {
        for handle in &mut self.handles {
            let _ = nix::unistd::dup2(handle.original.as_raw_fd(), handle.target_fd);
            let Some(thread) = handle.thread.take() else {
                continue;
            };
            if let Err(panic) = thread.join() {
                tracing::error!(
                    "fd capture thread for fd {} panicked: {:?}",
                    handle.target_fd,
                    panic,
                );
            }
        }
    }
}

pub fn capture_std_outputs() -> io::Result<FdRedirectHandle> {
    FdRedirect::new()
        .with_fd(&std::io::stdout(), "stdout", |line| {
            tracing::warn!("!stdout {}", line);
        })
        .with_fd(&std::io::stderr(), "stderr", |line| {
            tracing::error!("!stderr {}", line);
        })
        .capture()
}
