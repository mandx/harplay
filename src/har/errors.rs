use std::io::Error as IoError;

pub use snafu::{ensure, Backtrace, ErrorCompat, OptionExt, ResultExt, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum HarError {
    #[snafu(display("Reading error: {}", source))]
    Reading { source: serde_json::Error },

    #[snafu(display("File opening error: {}", source))]
    Opening { source: IoError },
}
