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
    fn new<RQ: Into<Request>, RP: Into<Response>>(
        iter: impl Iterator<Item = (RQ, RP)>,
        behaviour: ResponderBehaviour,
    ) -> Self {
        let mut responses: HashMap<Request, StatefulResponses> = HashMap::new();

        for (into_req, into_resp) in iter {
            let req = into_req.into();
            let resp = into_resp.into();

            let stateful_responses = responses.entry(req).or_insert_with(|| StatefulResponses {
                responses: Vec::with_capacity(1),
                behaviour: behaviour.clone(),
                last_index: None,
            });
            stateful_responses.responses.push(resp.into());
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

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
