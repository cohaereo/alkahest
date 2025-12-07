use std::{backtrace::Backtrace, fmt::Write, panic::PanicHookInfo};

use nu_ansi_term::{Color, Style};

pub fn hook(panic: &PanicHookInfo) {
    let message = if let Some(s) = panic.payload().downcast_ref::<&str>() {
        Some(s.to_string())
    } else {
        panic.payload().downcast_ref::<String>().cloned()
    };
    let location = panic.location().unwrap();
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("Unknown thread");

    let style = Style::new().fg(Color::LightMagenta).bold();
    let mut msg = String::new();
    writeln!(
        &mut msg,
        "Thread '{}' panicked at {}:\n{}",
        thread_name,
        location,
        message.unwrap_or("Unknown panic".to_owned())
    )
    .ok();

    let bt = Backtrace::capture();
    match bt.status() {
        std::backtrace::BacktraceStatus::Unsupported => {
            writeln!(&mut msg, "Backtrace is not supported").ok();
        }
        std::backtrace::BacktraceStatus::Disabled => {
            writeln!(
                &mut msg,
                "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace"
            )
            .ok();
        }
        std::backtrace::BacktraceStatus::Captured => {
            writeln!(&mut msg, "Backtrace:\n{bt}").ok();
        }
        _ => unimplemented!(),
    }

    eprint!("{}", style.paint(&msg));
    std::fs::write("panic.log", msg).ok();
}
