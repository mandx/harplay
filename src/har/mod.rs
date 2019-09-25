pub mod errors;
pub mod v1_2;
pub mod v1_3;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use errors::HarError;
use errors::*;

/// Supported versions of HAR.
///
/// Note that point releases require adding here (as they must other wise they wouldn't need a new version)
/// Using untagged can avoid that but the errors on incompatible documents become super hard to debug.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "version")]
pub enum Spec {
    /// Version 1.2 of the HAR specification.
    ///
    /// Refer to the official
    /// [specification](https://w3c.github.io/web-performance/specs/HAR/Overview.html)
    /// for more information.
    #[allow(non_camel_case_types)]
    #[serde(rename = "1.2")]
    V1_2(v1_2::Log),

    // Version 1.3 of the HAR specification.
    //
    // Refer to the draft
    // [specification](https://github.com/ahmadnassri/har-spec/blob/master/versions/1.3.md)
    // for more information.
    #[allow(non_camel_case_types)]
    #[serde(rename = "1.3")]
    V1_3(v1_3::Log),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Har {
    pub log: Spec,
}

/// Deserialize a HAR from a path
pub fn from_path<P>(path: P) -> Result<Har, HarError>
where
    P: AsRef<Path>,
{
    from_reader(File::open(path).context(Opening)?)
}

/// Deserialize a HAR from type which implements Read
pub fn from_reader<R>(read: R) -> Result<Har, HarError>
where
    R: Read,
{
    Ok(serde_json::from_reader::<R, Har>(read).context(Reading)?)
}
