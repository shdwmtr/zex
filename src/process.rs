use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct ProcessOutput {
    pub status_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run_with_timeout(
    binary: &str,
    dir: &Path,
    args: &[&str],
    stdin_data: Option<&[u8]>,
    timeout_ms: u64,
    cancel: &AtomicBool,
) -> Option<ProcessOutput> {
    let mut command = Command::new(binary);
    command
        .current_dir(dir)
        .args(args)
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().ok()?;

    if let Some(data) = stdin_data {
        let mut stdin = child.stdin.take()?;
        let data = data.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
    }

    let mut stdout = child.stdout.take()?;
    let (stdout_tx, stdout_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = stdout_tx.send(buf);
    });

    let mut stderr = child.stderr.take()?;
    let (stderr_tx, stderr_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    loop {
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                let stdout = stdout_rx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
                let stderr = stderr_rx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
                return Some(ProcessOutput {
                    status_code: exit_status.code(),
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}
