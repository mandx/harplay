#[derive(Debug)]
pub enum IntoRequestError {
    // I don't see too much value on implementing `snafu` for this type...
    NonHttpScheme,
    ParsingUrl,
    ReplacingHost,
    ReplacingScheme,
}

impl std::fmt::Display for IntoRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NonHttpScheme => "Scheme is not HTTP/HTTPS",
                Self::ParsingUrl => "Error parsing URL from HTTP request",
                Self::ReplacingHost => "Error normalizing URL's host",
                Self::ReplacingScheme => "Error normalizing URL's scheme",
            }
        )
    }
}

impl std::error::Error for IntoRequestError {}

#[derive(Debug)]
pub enum ResponderError {
    RequestNotFound,
    ResponseNotFound,
}

impl std::error::Error for ResponderError {}

impl std::fmt::Display for ResponderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::RequestNotFound => "Request not found",
                Self::ResponseNotFound => "Response not found",
            }
        )
    }
}
