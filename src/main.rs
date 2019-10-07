mod cli_args;
mod errors;
mod har;
mod logging;
mod req_resp;

use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use http::Request as HttpRequest;
use warp::{Filter, Rejection, Reply};

use crate::cli_args::CliArgs;
use crate::errors::*;
use crate::req_resp::{HarResponder, InMemoryResponder, Request, ResponderBehaviour, Response};

fn extract_request(
) -> impl Filter<Extract = (http::Request<warp::body::BodyStream>,), Error = warp::Rejection> + Copy
{
    warp::method()
        .and(warp::path::full())
        .and(warp::filters::header::headers_cloned())
        .and(warp::body::stream())
        .map(
            |method: http::Method,
             path: warp::path::FullPath,
             headers: http::HeaderMap,
             body: warp::body::BodyStream| {
                let mut req = http::Request::builder()
                    .method(method)
                    .uri(path.as_str())
                    .body(body)
                    .expect("request builder");
                *req.headers_mut() = headers;
                req
            },
        )
}

fn respond<T>(
    http_request: HttpRequest<T>,
    responder: Arc<Mutex<impl HarResponder>>,
) -> Result<impl Reply, Rejection> {
    let request: Request = http_request.try_into().context(IncomingUrl)?;

    responder
        // .write()
        .lock()
        .map_err(|_| AppError::DatabaseLock)?
        .respond_to(&request)
        .map_err(|responder_error| Rejection::from(AppError::from(responder_error)))
}

#[paw::main]
fn main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging(args.log_level)?;

    log::trace!("{} {}", "harPlay", env!("CARGO_PKG_VERSION"));
    log::trace!("Loading requests from {:?}", args.har_file);
    log::trace!("URL filtering by {:?}", &args.url_filter);

    let responder = Arc::new(Mutex::new({
        let har_file = har::from_path(&args.har_file)?;

        InMemoryResponder::new(
            ResponderBehaviour::SequentialWrapping,
            har_file
                .log
                .entries
                .into_iter()
                .filter(|entry| {
                    args.url_filter
                        .as_ref()
                        .map(|regex| {
                            let is_match = !regex.is_match(&entry.request.url);

                            if !is_match {
                                log::trace!(
                                    "Request excluded by filter: {} {}",
                                    &entry.request.method,
                                    &entry.request.url,
                                );
                            }

                            is_match
                        })
                        .unwrap_or(true)
                })
                .filter_map(|entry| {
                    let url = entry.request.url.clone();
                    let req: Request = match entry.request.try_into() {
                        Ok(req) => req,
                        Err(error) => {
                            log::error!("Entry dropped: Error parsing URL {}: {:?}", url, error);
                            return None;
                        }
                    };
                    let resp: Response = entry.response.into();
                    Some((req, resp))
                }),
        )
    }));

    warp::serve(
        extract_request()
            .and(warp::any().map(move || responder.clone()))
            .and_then(respond),
    )
    .run(args.network_bind);

    Ok(())
}
