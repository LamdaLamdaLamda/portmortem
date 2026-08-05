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

// Direct WinAPI call rather than sysinfo's snapshot-based Process::kill():
// that requires re-enumerating the whole system process list and looking
// the PID up in it, which adds a snapshot-staleness window for no reason —
// OpenProcess + TerminateProcess targets the exact PID directly.
#[cfg(target_os = "windows")]
pub fn kill_process(pid: i32) -> io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }

        let result = TerminateProcess(handle, 1);
        CloseHandle(handle);

        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
