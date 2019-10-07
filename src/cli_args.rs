use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use log::Level as LogLevel;
use regex::Regex;
use structopt::StructOpt;

use crate::req_resp::ResponderBehaviour;

#[derive(Debug, StructOpt)]
#[structopt(name = "harPlay", about = "Run a webserver out of a HAR file")]
pub struct CliArgs {
    #[structopt(parse(from_os_str))]
    pub har_file: PathBuf,

    #[structopt(
        short,
        long,
        parse(try_from_str),
        default_value = "sequential-once",
        possible_values = ResponderBehaviour::variants()
    )]
    pub behaviour: ResponderBehaviour,

    #[structopt(
        short,
        long,
        default_value = "127.0.0.1:3030",
        parse(try_from_str = SocketAddr::from_str)
    )]
    pub network_bind: SocketAddr,

    #[structopt(short, long, parse(try_from_str = Regex::new))]
    pub url_filter: Option<Regex>,

    #[structopt(short, long)]
    pub log_level: Option<LogLevel>,
}
