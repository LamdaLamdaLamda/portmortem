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

#[cfg(target_os = "windows")]
pub fn kill_process(pid: i32) -> io::Result<()> {
    let sys = sysinfo::System::new_all();

    match sys.process(sysinfo::Pid::from_u32(pid as u32)) {
        Some(process) if process.kill() => Ok(()),
        Some(_) => Err(io::Error::other("failed to terminate process")),
        None => Err(io::Error::other("process not found")),
    }
}
