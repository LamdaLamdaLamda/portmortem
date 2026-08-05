use std::io;

#[cfg(unix)]
pub fn kill_process(pid: i32) -> io::Result<()> {
    use libc::{kill, SIGTERM};

    let result = unsafe { kill(pid, SIGTERM) };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

// Shells out to taskkill rather than calling OpenProcess/TerminateProcess
// directly: a real-world CI run hit ERROR_ACCESS_DENIED from the raw WinAPI
// call even for a same-user process with a correctly-resolved PID — almost
// certainly security software heuristically blocking an unsigned binary's
// direct process-handle manipulation. taskkill.exe is a signed, trusted
// system tool and doesn't run into that, and this matches the existing
// pattern of shelling out to native tools on macOS (lsof/ps).
#[cfg(target_os = "windows")]
pub fn kill_process(pid: i32) -> io::Result<()> {
    use std::process::Command;

    let output = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if !stderr.trim().is_empty() { stderr.trim() } else { stdout.trim() };
        Err(io::Error::other(msg.to_string()))
    }
}
