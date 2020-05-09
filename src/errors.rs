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

impl From<ResponderError> for AppError {
    fn from(error: ResponderError) -> Self {
        match error {
            ResponderError::RequestNotFound => Self::RequestLookup,
            ResponderError::ResponseNotFound => Self::ResponseLookup,
        }
    }
}

use hyper::{Body as HttpBody, Response as HttpResponse};

impl From<AppError> for HttpResponse<HttpBody> {
    fn from(_error: AppError) -> Self {
        HttpResponse::builder()
            .status(418)
            .body(HttpBody::empty())
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_responder_app_error_conversions() {
        assert_matches!(
            AppError::from(ResponderError::RequestNotFound),
            AppError::RequestLookup
        );
        assert_matches!(
            AppError::from(ResponderError::ResponseNotFound),
            AppError::ResponseLookup
        );
    }

    #[test]
    fn test_response_app_error_conversions() {
        let responses = &[
            HttpResponse::from(AppError::IncomingUrl {
                source: IntoRequestError::NonHttpScheme,
            }),
            HttpResponse::from(AppError::IncomingUrl {
                source: IntoRequestError::ParsingUrl,
            }),
            HttpResponse::from(AppError::IncomingUrl {
                source: IntoRequestError::ReplacingHost,
            }),
            HttpResponse::from(AppError::IncomingUrl {
                source: IntoRequestError::ReplacingScheme,
            }),
            HttpResponse::from(AppError::DatabaseLock),
            HttpResponse::from(AppError::RequestLookup),
            HttpResponse::from(AppError::ResponseLookup),
        ];

        for resp in responses {
            assert_eq!(resp.status().as_u16(), 418);
        }
    }
}
