use std::fmt::Display;

pub trait ErrorAlert<T, E> {
    /// Display the error to the user in an alert window, if the result is an error
    fn err_alert(self) -> Result<T, E>;
}

impl<T, E> ErrorAlert<T, E> for Result<T, E>
where
    E: Display + Send + Sync + 'static,
{
    fn err_alert(self) -> Result<T, E> {
        match self {
            Ok(o) => Ok(o),
            Err(e) => {
                error!("{e}");

                native_dialog::MessageDialog::new()
                    .set_title("Oh fiddlesticks, what now")
                    .set_text(&e.to_string())
                    .set_type(native_dialog::MessageType::Error)
                    .show_alert()
                    .ok();

                Err(e)
            }
        }
    }
}
