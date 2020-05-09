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
#[cfg_attr(tarpaulin, skip)]
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Har, HarError> {
    from_reader(BufReader::new(File::open(path).context(Opening)?))
}

/// Deserialize a HAR from type which implements Read
pub fn from_reader<R: Read>(read: R) -> Result<Har, HarError> {
    Ok(serde_json::from_reader::<R, Har>(read).context(Reading)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn load_from_not_json_or_har() {
        let json = br#"{"some": "json"}"#;
        assert_matches!(from_reader(&json[..]), Err(_));
        let json = br#"{"log": {}}"#;
        assert_matches!(from_reader(&json[..]), Err(_));
        let json = br#"{"log":{"version":"1.2","pages":[],"entries":[]}}"#;
        assert_matches!(from_reader(&json[..]), Err(_));
    }

    #[test]
    fn load_from_har() {
        let json =
            br#"{"log":{"creator":{"name":"Creator?","version":"0.1"},"pages":[],"entries":[]}}"#;
        assert_matches!(from_reader(&json[..]), Ok(_));
        let json = br#"{"log":{"version":"1.2","creator":{"name":"Creator?","version":"0.1"},"pages":[],"entries":[]}}"#;
        assert_matches!(from_reader(&json[..]), Ok(_));
    }
}
