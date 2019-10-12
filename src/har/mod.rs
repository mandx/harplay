pub mod errors;
pub mod generic;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use errors::HarError;
use errors::*;
pub use generic::*;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Har {
    pub log: Log,
}

/// Deserialize a HAR from a path
pub fn from_path<P>(path: P) -> Result<Har, HarError>
where
    P: AsRef<Path>,
{
    from_reader(BufReader::new(File::open(path).context(Opening)?))
}

/// Deserialize a HAR from type which implements Read
pub fn from_reader<R>(read: R) -> Result<Har, HarError>
where
    R: Read,
{
    Ok(serde_json::from_reader::<R, Har>(read).context(Reading)?)
}
