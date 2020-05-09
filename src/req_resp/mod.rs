mod behaviour;
mod errors;
mod in_memory;

use std::convert::TryFrom;
use url::Url;

pub use behaviour::ResponderBehaviour;
pub use errors::*;
pub use in_memory::InMemoryResponder;

// Maybe rename these generic names into more specific ones,
// since we are also dealing with `http`'s types.

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl From<crate::har::Headers> for Header {
    fn from(header: crate::har::Headers) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

/// Very simple representation of a recorded response
#[derive(Debug, Clone)]
pub struct Response {
    pub status_code: u16,
    pub headers: Vec<Header>,
    pub body: Option<Vec<u8>>,
}

impl From<crate::har::Response> for Response {
    fn from(response: crate::har::Response) -> Self {
        Self {
            status_code: response.status as u16,
            body: response.content.text.map(Into::into),
            headers: response.headers.iter().cloned().map(From::from).collect(),
        }
    }
}

/// Very simple representation of a recorded request
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub url: Url,
    pub original_url: String,
    pub headers: Vec<Header>,
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Request {} {}", self.method, self.original_url)
    }
}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for Request {}

impl std::hash::Hash for Request {
    fn hash<'a, H: std::hash::Hasher>(&'a self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        for header in self.headers.iter() {
            header.name.hash(state);
            header.value.hash(state);
        }
    }
}

impl<B> TryFrom<http::Request<B>> for Request {
    type Error = IntoRequestError;

    fn try_from(req: http::Request<B>) -> Result<Self, Self::Error> {
        use http::uri::{Authority, Builder as UriBuilder, PathAndQuery};
        let original_uri = req.uri();

        let mut url = UriBuilder::new()
            .authority(
                original_uri
                    .authority()
                    .map(Clone::clone)
                    .unwrap_or_else(|| Authority::from_static("harplay")),
            )
            .scheme(
                original_uri
                    .scheme()
                    .map(Clone::clone)
                    .unwrap_or_else(|| TryFrom::try_from("http").unwrap()),
            )
            .path_and_query(
                original_uri
                    .path_and_query()
                    .map(Clone::clone)
                    .unwrap_or_else(|| PathAndQuery::from_static("/")),
            )
            .build()
            .map_err(|_| IntoRequestError::ParsingUrl)
            .and_then(|uri| {
                Url::parse(&uri.to_string()).map_err(|_| IntoRequestError::ParsingUrl)
            })?;

        let current_scheme = url.scheme();
        if !(current_scheme == "http" || current_scheme == "https") {
            return Err(IntoRequestError::NonHttpScheme);
        }

        url.set_host(Some("harplay"))
            .map_err(|_| IntoRequestError::ReplacingHost)?;

        url.set_scheme("http")
            .map_err(|_| IntoRequestError::ReplacingScheme)?;

        let mut headers: Vec<_> = req
            .headers()
            .iter()
            .filter_map(|(k, v)| match v.to_str() {
                Ok(v) => Some(Header {
                    name: k.to_string(),
                    value: v.to_string(),
                }),
                Err(error) => {
                    log::warn!("Ignoring header {:?} from {:?}: {:?}", k, req.uri(), error);
                    None
                }
            })
            .collect();
        headers.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Request {
            method: req.method().as_str().into(),
            url,
            original_url: original_uri.to_string(),
            headers,
        })
    }
}

impl TryFrom<crate::har::Request> for Request {
    type Error = IntoRequestError;

    fn try_from(req: crate::har::Request) -> Result<Self, Self::Error> {
        let original_url = req.url;
        let mut url = Url::parse(&original_url).map_err(|_| IntoRequestError::ParsingUrl)?;

        let current_scheme = url.scheme();
        if !(current_scheme == "http" || current_scheme == "https") {
            return Err(IntoRequestError::NonHttpScheme);
        }

        url.set_host(Some("harplay"))
            .map_err(|_| IntoRequestError::ReplacingHost)?;

        url.set_scheme("http")
            .map_err(|_| IntoRequestError::ReplacingScheme)?;

        let mut headers: Vec<_> = req.headers.iter().cloned().map(Header::from).collect();
        headers.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Request {
            method: req.method,
            url,
            original_url,
            headers,
        })
    }
}

pub trait HarResponder {
    fn respond_to(&mut self, request: &Request) -> Result<Response, ResponderError>;
}

impl From<Response> for http::Response<hyper::Body> {
    fn from(response: Response) -> Self {
        use http::header::{HeaderName, HeaderValue};
        let mut resp_builder = http::Response::builder();

        if let Some(resp_headers) = resp_builder.headers_mut() {
            for header in response.headers.iter() {
                if let (Ok(header_name), Ok(header_value)) = (
                    header.name.parse::<HeaderName>(),
                    header.value.parse::<HeaderValue>(),
                ) {
                    resp_headers.append(header_name, header_value);
                }
                // TODO: Log header parsing failures?
            }
        }

        resp_builder
            .status(http::StatusCode::from_u16(response.status_code).unwrap())
            .body(
                response
                    .body
                    .map(hyper::Body::from)
                    .unwrap_or_else(hyper::Body::empty),
            )
            .unwrap()
    }
}
