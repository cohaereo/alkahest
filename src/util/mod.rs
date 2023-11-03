pub mod image;
pub mod lock;

use std::path::PathBuf;

pub use lock::{FilterDebugLockTarget, LockTracker, RwLock};

use crate::types::FnvHash;

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

#[allow(non_snake_case)]
#[inline]
pub fn D3D11CalcSubresource(MipSlice: usize, ArraySlice: usize, MipLevels: usize) -> usize {
    MipSlice + ArraySlice * MipLevels
}

pub const FNV1_BASE: u32 = 0x811c9dc5;
pub const FNV1_PRIME: u32 = 0x01000193;
pub fn fnv1(data: &[u8]) -> FnvHash {
    data.iter().fold(FNV1_BASE, |acc, b| {
        acc.wrapping_mul(FNV1_PRIME) ^ (*b as u32)
    })
}
