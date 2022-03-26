pub use terminal_size::{Height, Width};

#[cfg(windows)]
pub fn safe_terminal_size() -> Option<(Width, Height)> {
    use std::os::windows::io::RawHandle;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    let stdout = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } as RawHandle;
    if let Some(wh) = terminal_size::terminal_size_using_handle(stdout) {
        Some(wh)
    } else {
        let stderr = unsafe { GetStdHandle(STD_ERROR_HANDLE) } as RawHandle;
        terminal_size::terminal_size_using_handle(stderr)
    }
}

#[cfg(not(windows))]
pub fn safe_terminal_size() -> Option<(Width, Height)> {
    if let Some(wh) = terminal_size::terminal_size_using_fd(libc::STDOUT_FILENO) {
        Some(wh)
    } else {
        terminal_size::terminal_size_using_fd(libc::STDERR_FILENO)
    }
}
