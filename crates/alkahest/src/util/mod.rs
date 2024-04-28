pub mod changelog_diff;
pub mod consts;
pub mod dds;
pub mod error;
pub mod export;
pub mod image;
pub mod text;

pub use parking_lot::RwLock;
use tiger_parse::FnvHash;

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
            )
            .ok();
        }
    }
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

pub trait BoolExts {
    fn yes_no(self) -> &'static str;
}

impl BoolExts for bool {
    fn yes_no(self) -> &'static str {
        if self {
            "yes"
        } else {
            "no"
        }
    }
}

/// Fixes version/tag strings to be compatible with semver
pub fn version_fixup(version: &str) -> String {
    let v = version.replace('v', "");
    if v.chars().filter(|c| *c == '.').count() == 1 {
        format!("{}.0", v)
    } else {
        v
    }
}
