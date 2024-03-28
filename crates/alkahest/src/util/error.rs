use std::fmt::Display;

pub trait ErrorAlert<T, E> {
    /// Display the error to the user in an alert window, if the result is an error
    fn err_alert(self) -> Result<T, E>;
}

impl<T, E> ErrorAlert<T, E> for Result<T, E>
where
    E: Display + Send + Sync + AsRef<dyn std::error::Error> + 'static,
{
    fn err_alert(self) -> Result<T, E> {
        match self {
            Ok(o) => Ok(o),
            Err(e) => {
                error!("{e}");

                show_error_alert(&e);

                Err(e)
            }
        }
    }
}

pub fn show_error_alert<E: AsRef<dyn std::error::Error> + Display>(e: E) {
    native_dialog::MessageDialog::new()
        .set_title("Oh fiddlesticks, what now")
        .set_text(&if let Some(source) = e.as_ref().source() {
            format!("{e}\n\nCaused by: {source}")
        } else {
            format!("{e}")
        })
        .set_type(native_dialog::MessageType::Error)
        .show_alert()
        .ok();
}
