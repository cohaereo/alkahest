pub mod image;
pub mod lock;

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
