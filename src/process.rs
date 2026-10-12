use std::{
    io::Read,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

// Drain both pipes concurrently: large images/audio must not deadlock a waiting child.
pub fn output(command: &mut Command, cancel: &AtomicBool) -> Result<Output, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Search cancelled".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "Cannot start {}: {e}",
                command.get_program().to_string_lossy()
            )
        })?;
    let stdout = child.stdout.take().ok_or("Missing stdout")?;
    let stderr = child.stderr.take().ok_or("Missing stderr")?;
    let read = |stream: Box<dyn Read + Send>, limit| {
        std::thread::spawn(move || {
            let mut bytes = vec![];
            stream
                .take(limit + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > limit {
                return Err("Subprocess output exceeded limit".to_string());
            }
            Ok(bytes)
        })
    };
    let out = read(Box::new(stdout), 64_000_000);
    let err = read(Box::new(stderr), 1_000_000);
    let started = Instant::now();
    let mut interrupted = None;
    let status = loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(60) {
            interrupted = Some(if cancel.load(Ordering::Relaxed) {
                "Search cancelled"
            } else {
                "Subprocess exceeded 60 seconds"
            });
            let _ = child.kill();
            break child.wait().map_err(|e| e.to_string())?;
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    };
    let stdout = out.join().map_err(|_| "stdout reader failed")?;
    let stderr = err.join().map_err(|_| "stderr reader failed")?;
    if let Some(reason) = interrupted {
        return Err(reason.into());
    }
    Ok(Output {
        status,
        stdout: stdout?,
        stderr: stderr?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires ffmpeg on PATH"]
    fn cancellation_kills_running_media_process() {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            signal.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let result = output(
            Command::new("ffmpeg").args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "anullsrc",
                "-f",
                "null",
                "-",
            ]),
            &cancel,
        );
        thread.join().unwrap();
        assert_eq!(result.unwrap_err(), "Search cancelled");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn cancellation_prevents_process_start() {
        let error = output(
            &mut Command::new("nonexistent-digup-test-program"),
            &AtomicBool::new(true),
        )
        .unwrap_err();
        assert_eq!(error, "Search cancelled");
    }
}
