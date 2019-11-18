use std::collections::HashMap;

use super::errors::*;
use super::{HarResponder, Request, ResponderBehaviour, Response};

#[derive(Debug)]
/// Internal state associated to each request
struct StatefulResponses {
    /// All possible responses for this request
    responses: Vec<Response>,
    /// How to pick from the set of responses
    behaviour: ResponderBehaviour,
    /// Current
    last_index: Option<usize>,
}

#[derive(Debug)]
pub struct InMemoryResponder {
    responses: HashMap<Request, StatefulResponses>,
}

impl InMemoryResponder {
    pub fn new<RQ: Into<Request>, RP: Into<Response>>(
        behaviour: ResponderBehaviour,
        iter: impl Iterator<Item = (RQ, RP)>,
    ) -> Self {
        let mut responses: HashMap<Request, StatefulResponses> = HashMap::new();

        for (into_req, into_resp) in iter {
            let stateful_responses =
                responses
                    .entry(into_req.into())
                    .or_insert_with(|| StatefulResponses {
                        responses: Vec::with_capacity(1),
                        behaviour: behaviour.clone(),
                        last_index: None,
                    });
            stateful_responses.responses.push(into_resp.into());
        }

        Self { responses }
    }
}

impl HarResponder for InMemoryResponder {
    fn respond_to(&mut self, request: &Request) -> Result<Response, ResponderError> {
        let state = self
            .responses
            .get_mut(request)
            .ok_or(ResponderError::RequestNotFound)?;

        state
            .behaviour
            .choose_index(state.last_index, state.responses.len())
            .and_then(|index| state.responses.get(index))
            .map(Clone::clone)
            .ok_or(ResponderError::ResponseNotFound)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;
    use url::Url;

    use crate::req_resp::{
        HarResponder, InMemoryResponder, Request, ResponderBehaviour, ResponderError, Response,
    };

    fn reqs_resp_fixture() -> impl Iterator<Item = (Request, Response)> {
        (0..5).map(|i| {
            (
                Request {
                    method: "GET".into(),
                    url: Url::parse("http://harplay/path/").unwrap(),
                    original_url: "http://harplay/path/".into(),
                    headers: Vec::new(),
                },
                Response {
                    status_code: 200,
                    headers: Vec::new(),
                    body: Some(i.to_string().into()),
                },
            )
        })
    }

    use crate::req_resp::ResponderBehaviour::*;
    #[test_case(AlwaysFirst, Ok("0"))]
    #[test_case(AlwaysLast, Ok("4"))]
    #[test_case(Random, Ok(""))]
    #[test_case(SequentialClamping, Ok("0"))]
    #[test_case(SequentialOnce, Ok("0"))]
    #[test_case(SequentialWrapping, Ok("0"))]
    fn it_works(behaviour: ResponderBehaviour, content: Result<&'static str, ResponderError>) {
        let req: Request = Request {
            method: "GET".into(),
            url: Url::parse("http://harplay/path/").unwrap(),
            original_url: "http://harplay/path/".into(),
            headers: Vec::new(),
        };

        let mut responder = InMemoryResponder::new(behaviour.clone(), reqs_resp_fixture());

        if behaviour == Random {
            assert!(responder.respond_to(&req).is_ok());
        } else {
            assert_eq!(
                responder.respond_to(&req).unwrap().body,
                content
                    .map(|content| {
                        Response {
                            status_code: 200,
                            headers: Vec::new(),
                            body: Some(content.into()),
                        }
                    })
                    .unwrap()
                    .body
            );
        }
    }
}
