pub mod image;
pub mod lock;

use std::path::PathBuf;

pub use lock::{FilterDebugLockTarget, LockTracker, RwLock};

/// Enables ANSI color codes on older/weird command prompt versions
pub fn fix_windows_command_prompt() {
    use windows::Win32::System::Console::{
        GetStdHandle, SetConsoleMode, ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_OUTPUT_HANDLE,
    };

    unsafe {
        if let Ok(stdout) = GetStdHandle(STD_OUTPUT_HANDLE) {
            SetConsoleMode(
                stdout,
                ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            );
        }
    }
}

pub fn exe_directory() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn exe_relative_path(path: &str) -> PathBuf {
    exe_directory().join(path)
}
