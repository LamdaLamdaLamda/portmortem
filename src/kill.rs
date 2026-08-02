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
