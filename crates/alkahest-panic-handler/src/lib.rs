// TODO(cohae): Remove this once PanicHookInfo becomes stable (and update MSRV)
#![allow(deprecated)]

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    io::Write,
    panic::PanicInfo,
    sync::{Arc, OnceLock},
};

use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    static ref PANIC_FILE: Arc<Mutex<Option<fs_err::File>>> = Arc::new(Mutex::new(None));
    static ref PANIC_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    static ref PANIC_HEADER: OnceLock<String> = OnceLock::new();
}

pub fn install_hook(header: Option<String>) {
    std::panic::set_hook(Box::new(|info| {
        let _guard = PANIC_LOCK.lock();
        let this_thread = std::thread::current();

        // First call color-eyre's fancy CLI backtrace
        let (panic_hook, _) = color_eyre::config::HookBuilder::new().into_hooks();
        eprintln!(
            "Thread '{}' panicked:\n{}",
            this_thread
                .name()
                .map(|name| name.to_string())
                .unwrap_or(format!("{:?}", this_thread.id())),
            panic_hook.panic_report(info)
        );

        // Write a panic file
        match write_panic_to_file(info, Backtrace::force_capture()) {
            Ok(()) => {}
            Err(e) => eprintln!("Failed to create panic log: {e}"),
        }

        // Dont show dialog on debug builds
        if cfg!(debug_assertions) {
            return;
        }

        // Finally, show a dialog
        let panic_message_stripped = strip_ansi_codes(&format!("{info}"));
        if let Err(e) = native_dialog::MessageDialog::new()
            .set_type(native_dialog::MessageType::Error)
            .set_title("Alkahest crashed!")
            .set_text(&format!(
                "{}\n\nA full crash log has been written to panic.log",
                panic_message_stripped
            ))
            .show_alert()
        {
            eprintln!("Failed to show error dialog: {e}")
        }

        // Make sure the application exits
        std::process::exit(-1);
    }));

    if let Some(header) = header {
        PANIC_HEADER.set(header).expect("Panic header already set");
    }
}

fn write_panic_to_file(info: &PanicInfo<'_>, bt: Backtrace) -> std::io::Result<()> {
    let mut file_lock = PANIC_FILE.lock();
    if file_lock.is_none() {
        *file_lock = Some(fs_err::File::create("panic.log")?);
    }

    let f = file_lock.as_mut().unwrap();

    // Write panic header
    if let Some(header) = PANIC_HEADER.get() {
        writeln!(f, "{}", header)?;
    }

    writeln!(f, "{}", info)?;
    if bt.status() == BacktraceStatus::Captured {
        writeln!(f)?;
        writeln!(f, "Backtrace:")?;
        writeln!(f, "{}", bt)?;
    }

    Ok(())
}

pub fn strip_ansi_codes(input: &str) -> String {
    let ansi_escape_pattern = regex::Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    ansi_escape_pattern.replace_all(input, "").to_string()
}
