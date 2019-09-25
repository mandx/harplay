pub use snafu::{ensure, Backtrace, ErrorCompat, OptionExt, ResultExt, Snafu};

use crate::req_resp::{IntoRequestError, ResponderError};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AppError {
    #[snafu(display("Could parse incoming URL: {}", source))]
    IncomingUrl { source: IntoRequestError },
    #[snafu(display("Error locking database (probably lock poisoning)."))]
    DatabaseLock,
    #[snafu(display("Request not found"))]
    RequestLookup,
    #[snafu(display("Response not found"))]
    ResponseLookup,
}

impl From<AppError> for warp::reject::Rejection {
    fn from(error: AppError) -> Self {
        warp::reject::custom(error)
    }
}

impl From<ResponderError> for AppError {
    fn from(error: ResponderError) -> Self {
        match error {
            ResponderError::RequestNotFound => Self::RequestLookup,
            ResponderError::ResponseNotFound => Self::ResponseLookup,
        }
    }
}
