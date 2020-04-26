mod cli_args;
mod errors;
mod har;
mod logging;
mod req_resp;

use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use hyper::{
    service::{make_service_fn, service_fn},
    Body as HttpBody, Error as HttpError, Request as HttpRequest, Response as HttpResponse, Server,
};
use tokio::runtime::Runtime;

use crate::cli_args::CliArgs;
use crate::errors::*;
use crate::req_resp::{HarResponder, InMemoryResponder, Request, ResponderBehaviour, Response};

async fn respond<T>(
    http_request: HttpRequest<T>,
    responder: Arc<Mutex<impl HarResponder>>,
) -> Result<HttpResponse<HttpBody>, HttpError> {
    let request: Request = match http_request.try_into().context(IncomingUrl) {
        Ok(request) => request,
        Err(error) => return Ok(error.into()),
    };

    let mut responder = match responder.lock() {
        Ok(lock) => lock,
        Err(_) => return Ok(AppError::DatabaseLock.into()),
    };

    Ok(responder
        .respond_to(&request)
        .map(HttpResponse::<HttpBody>::from)
        .map_err(AppError::from)
        .unwrap_or_else(HttpResponse::<HttpBody>::from))
}

#[paw::main]
fn main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging(args.log_level)?;

    log::trace!("{} {}", "harPlay", env!("CARGO_PKG_VERSION"));
    log::trace!("Loading requests from {:?}", args.har_file);

    if let Some(regex) = &args.url_filter {
        log::trace!("URL filtering by {:?}", regex);
    } else {
        log::trace!("URL filtering disabled");
    }

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
                        Ok(req) => {
                            log::trace!("Adding {}", req);
                            req
                        }
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

    let service = make_service_fn(move |_| {
        let responder = responder.clone();

        async {
            Ok::<_, HttpError>(service_fn(move |request| {
                respond(request, responder.clone())
            }))
        }
    });

    let server = Server::bind(&args.network_bind).serve(service);

    Runtime::new()?.block_on(server)?;

    Ok(())
}
