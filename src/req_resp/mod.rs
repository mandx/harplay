pub mod in_memory;
pub mod errors;

use std::convert::TryFrom;
use std::fmt::Debug;

use url::Url;

pub use errors::*;
pub use in_memory::InMemoryResponder;

// Maybe rename these generic names into more specific ones,
// since we are also dealing with `http`'s and `warp`'s types.

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl From<crate::har::v1_2::Headers> for Header {
    fn from(header: crate::har::v1_2::Headers) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

impl From<crate::har::v1_3::Headers> for Header {
    fn from(header: crate::har::v1_3::Headers) -> Self {
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
    pub body: Option<String>,
}

impl From<crate::har::v1_2::Response> for Response {
    fn from(response: crate::har::v1_2::Response) -> Self {
        Self {
            status_code: response.status as u16,
            body: response.content.text,
            headers: response.headers.iter().cloned().map(From::from).collect(),
        }
    }
}

impl From<crate::har::v1_3::Response> for Response {
    fn from(response: crate::har::v1_3::Response) -> Self {
        Self {
            status_code: response.status as u16,
            body: response.content.text,
            headers: response.headers.iter().cloned().map(From::from).collect(),
        }
    }
}

impl warp::reply::Reply for Response {
    fn into_response(self) -> warp::reply::Response {
        let mut resp_builder = http::Response::builder();
        resp_builder.status(self.status_code);
        for header in self.headers {
            if header.name == "content-length" {
                continue;
            }

            if header.name == "transfer-encoding" {
                continue;
            }

            resp_builder.header(&header.name, &header.value);
        }
        resp_builder
            .body(self.body.unwrap_or_else(String::new).into())
            .unwrap()
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
        use http::HttpTryFrom;
        let original_uri = req.uri();

        let mut url = UriBuilder::new()
            .authority(
                original_uri
                    .authority_part()
                    .map(Clone::clone)
                    .unwrap_or_else(|| Authority::from_static("harplay")),
            )
            .scheme(
                original_uri
                    .scheme_part()
                    .map(Clone::clone)
                    .unwrap_or_else(|| HttpTryFrom::try_from("http").unwrap()),
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

impl TryFrom<crate::har::v1_2::Request> for Request {
    type Error = IntoRequestError;

    fn try_from(req: crate::har::v1_2::Request) -> Result<Self, Self::Error> {
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

impl TryFrom<crate::har::v1_3::Request> for Request {
    type Error = IntoRequestError;

    fn try_from(req: crate::har::v1_3::Request) -> Result<Self, Self::Error> {
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

#[derive(Debug, Clone)]
pub enum ResponderBehaviour {
    SequentialWrapping,
    SequentialClamping,
    SequentialOnce,
    Random,
    AlwaysFirst,
    AlwaysLast,
}

impl ResponderBehaviour {
    pub fn choose_index(&self, last: Option<usize>, length: usize) -> Option<usize> {
        if length < 1 {
            return None;
        }

        match self {
            Self::SequentialWrapping | Self::SequentialClamping | Self::SequentialOnce => {
                match last {
                    Some(mut last) => {
                        last += 1;
                        if last >= length {
                            match self {
                                Self::SequentialWrapping => Some(0),
                                Self::SequentialClamping => Some(length - 1),
                                Self::SequentialOnce => None,
                                _ => unreachable!(),
                            }
                        } else {
                            Some(last)
                        }
                    }
                    None => Some(0),
                }
            }
            Self::Random => {
                use rand::prelude::*;
                let mut rng = thread_rng();
                Some(rng.gen_range(0, length))
            }
            Self::AlwaysFirst => Some(0),
            Self::AlwaysLast => Some(length - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::ResponderBehaviour::{self, *};

    // Zero-length
    #[test_case(SequentialWrapping, Some(0), 0, None)]
    #[test_case(SequentialClamping, Some(0), 0, None)]
    #[test_case(SequentialOnce, Some(0), 0, None)]
    #[test_case(Random, Some(0), 0, None)]
    #[test_case(AlwaysFirst, Some(0), 0, None)]
    #[test_case(AlwaysLast, Some(0), 0, None)]
    #[test_case(SequentialWrapping, Some(1), 0, None)]
    #[test_case(SequentialClamping, Some(1), 0, None)]
    #[test_case(SequentialOnce, Some(1), 0, None)]
    #[test_case(Random, Some(1), 0, None)]
    #[test_case(AlwaysFirst, Some(1), 0, None)]
    #[test_case(AlwaysLast, Some(1), 0, None)]
    // AlwaysFirst
    #[test_case(AlwaysFirst, None, 1, Some(0))]
    #[test_case(AlwaysFirst, Some(0), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(1), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(2), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(3), 1, Some(0))]
    // AlwaysLast
    #[test_case(AlwaysLast, None, 1, Some(0))]
    #[test_case(AlwaysLast, Some(3), 1, Some(0))]
    #[test_case(AlwaysLast, Some(3), 2, Some(1))]
    #[test_case(AlwaysLast, Some(3), 3, Some(2))]
    #[test_case(AlwaysLast, Some(3), 4, Some(3))]
    #[test_case(AlwaysLast, Some(3), 5, Some(4))]
    // SequentialWrapping
    #[test_case(SequentialWrapping, None, 4, Some(0))]
    #[test_case(SequentialWrapping, Some(0), 4, Some(1))]
    #[test_case(SequentialWrapping, Some(1), 4, Some(2))]
    #[test_case(SequentialWrapping, Some(2), 4, Some(3))]
    #[test_case(SequentialWrapping, Some(3), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(5), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(6), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(7), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(8), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(9), 4, Some(0))]
    // SequentialClamping
    #[test_case(SequentialClamping, None, 4, Some(0))]
    #[test_case(SequentialClamping, Some(0), 4, Some(1))]
    #[test_case(SequentialClamping, Some(1), 4, Some(2))]
    #[test_case(SequentialClamping, Some(2), 4, Some(3))]
    #[test_case(SequentialClamping, Some(3), 4, Some(3))]
    #[test_case(SequentialClamping, Some(4), 4, Some(3))]
    #[test_case(SequentialClamping, Some(5), 4, Some(3))]
    #[test_case(SequentialClamping, Some(6), 4, Some(3))]
    #[test_case(SequentialClamping, Some(7), 4, Some(3))]
    #[test_case(SequentialClamping, Some(8), 4, Some(3))]
    #[test_case(SequentialClamping, Some(9), 4, Some(3))]
    // SequentialOnce
    #[test_case(SequentialOnce, None, 4, Some(0))]
    #[test_case(SequentialOnce, Some(0), 4, Some(1))]
    #[test_case(SequentialOnce, Some(1), 4, Some(2))]
    #[test_case(SequentialOnce, Some(2), 4, Some(3))]
    #[test_case(SequentialOnce, Some(3), 4, None)]
    #[test_case(SequentialOnce, Some(4), 4, None)]
    #[test_case(SequentialOnce, Some(5), 4, None)]
    #[test_case(SequentialOnce, Some(6), 4, None)]
    #[test_case(SequentialOnce, Some(7), 4, None)]
    #[test_case(SequentialOnce, Some(8), 4, None)]
    #[test_case(SequentialOnce, Some(9), 4, None)]
    fn behaviour_tests(
        variant: ResponderBehaviour,
        last: Option<usize>,
        length: usize,
        expected: Option<usize>,
    ) {
        assert_eq!(variant.choose_index(last, length), expected);
    }

    // #[test]
    // fn it_works() {
    //     assert_eq!(2 + 2, 4);
    // }
}
