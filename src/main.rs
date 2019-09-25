mod cli_args;
mod errors;
mod har;
mod logging;
mod req_resp;

use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};

use http::Request as HttpRequest;
use warp::{Filter, Rejection, Reply};

use crate::cli_args::CliArgs;
use crate::errors::*;
use crate::har::Spec;
use crate::req_resp::{HarResponder, Request, Response};

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

fn replay<T>(
    http_request: HttpRequest<T>,
    request_map: Arc<RwLock<HashMap<Request, Vec<Response>>>>,
) -> Result<impl Reply, Rejection> {
    let request: Request = http_request.try_into().context(IncomingUrl)?;
    let read_lock = request_map.read().map_err(|_| AppError::DatabaseLock)?;
    let responses = read_lock.get(&request).ok_or(AppError::RequestLookup)?;

    responses
        .get(0)
        .ok_or(AppError::ResponseLookup)
        .map_err(Rejection::from)
        .map(Clone::clone)
}

fn respond<T, R>(
    http_request: HttpRequest<T>,
    responder: Arc<RwLock<impl HarResponder>>,
) -> Result<impl Reply, Rejection> {
    let request: Request = http_request.try_into().context(IncomingUrl)?;
    let mut write_lock = responder.write().map_err(|_| AppError::DatabaseLock)?;

    write_lock.respond_to(&request).map_err(|responder_error| {
        Rejection::from(AppError::from(responder_error))
    })

    // let read_lock = responder.read().map_err(|_| AppError::DatabaseLock)?;
    // let response = read_lock.respond_to(&request).map_err(From::from)?;
    // response
    //     .ok_or(AppError::ResponseLookup)
    //     .map_err(Rejection::from)
    //     .map(Clone::clone)
}

#[paw::main]
fn main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging(args.log_level)?;

    log::trace!("{} {}", "harPlay", env!("CARGO_PKG_VERSION"));
    log::trace!("Loading requests from {:?}", args.har_file);
    log::trace!("URL filtering by {:?}", &args.url_filter);

    let requests = Arc::new(RwLock::new({
        let har_file = har::from_path(args.har_file)?;
        let mut requests: HashMap<Request, Vec<Response>> =
            HashMap::with_capacity(match &har_file.log {
                Spec::V1_2(log) => log.entries.len(),
                Spec::V1_3(log) => log.entries.len(),
            });

        match har_file.log {
            Spec::V1_2(log) => {
                for entry in log.entries {
                    if let Some(regex) = &args.url_filter {
                        if !regex.is_match(&entry.request.url) {
                            log::trace!(
                                "Request excluded by filter: {} {}",
                                &entry.request.method,
                                &entry.request.url,
                            );
                            continue;
                        }
                    }

                    // Keep a clone of the URL around in case the conversion
                    // fails, so we are able to log the problem and the cause.
                    let url = entry.request.url.clone();

                    match entry.request.try_into() {
                        Ok(request) => {
                            log::info!("Adding {}", request);
                            let responses = requests
                                .entry(request)
                                .or_insert_with(|| Vec::with_capacity(1));
                            responses.push(entry.response.into());
                        }
                        Err(error) => {
                            log::error!("Entry dropped: Error parsing URL {}: {:?}", url, error);
                        }
                    }
                }
            }
            _ => {
                unimplemented!("V1_3 not yet supported");
            }
        }

        requests
    }));

    warp::serve(
        extract_request()
            .and(warp::any().map(move || requests.clone()))
            .and_then(replay),
    )
    .run(args.bind);

    Ok(())
}
