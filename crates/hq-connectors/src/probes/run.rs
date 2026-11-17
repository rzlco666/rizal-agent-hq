use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_STDOUT: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub success: bool,
}

pub fn run_timeout(program: &str, argv: &[&str], timeout_ms: u64) -> Option<CommandOutput> {
    let mut command = Command::new(program);
    command
        .args(argv)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(path) = search_path() {
        command.env("PATH", path);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut output = Vec::with_capacity(MAX_STDOUT.min(8192));
        let mut buffer = [0; 8192];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let remaining = MAX_STDOUT.saturating_sub(output.len());
                    output.extend_from_slice(&buffer[..count.min(remaining)]);
                }
            }
        }
        output
    });
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let bytes = reader.join().ok()?;
    let status = status?;
    Some(CommandOutput {
        stdout: String::from_utf8_lossy(&bytes).into_owned(),
        success: status.success(),
    })
}

fn search_path() -> Option<OsString> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let mut paths = vec![home.join(".bun/bin"), home.join(".local/bin")];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(paths).ok()
}
