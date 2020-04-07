use std::fmt;
use std::str;

use derive_more::{Display, Error};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// A temporary contact number.
#[derive(Default, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Tcn {
    bytes: [u8; 16],
}

impl fmt::Debug for Tcn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Tcn").field(&self.to_string()).finish()
    }
}

impl Tcn {
    /// Creates a CCN from raw bytes.
    pub fn from_bytes(b: &[u8]) -> Result<Tcn, InvalidTcn> {
        if b.len() != 16 {
            return Err(InvalidTcn);
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(b);
        Ok(Tcn { bytes })
    }

    /// Returns the bytes behind the TCN
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Raised if a TCN is invalid.
#[derive(Error, Display, Debug)]
#[display(fmt = "invalid tcn")]
pub struct InvalidTcn;

impl str::FromStr for Tcn {
    type Err = InvalidTcn;

    fn from_str(value: &str) -> Result<Tcn, InvalidTcn> {
        let mut bytes = [0u8; 16];
        if value.len() != 22 {
            return Err(InvalidTcn);
        }
        base64::decode_config_slice(value, base64::URL_SAFE_NO_PAD, &mut bytes[..])
            .map_err(|_| InvalidTcn)?;
        Ok(Tcn { bytes })
    }
}

impl fmt::Display for Tcn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 50];
        let len = base64::encode_config_slice(self.bytes, base64::URL_SAFE_NO_PAD, &mut buf);
        f.write_str(unsafe { std::str::from_utf8_unchecked(&buf[..len]) })
    }
}

impl Serialize for Tcn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

impl<'de> Deserialize<'de> for Tcn {
    fn deserialize<D>(deserializer: D) -> Result<Tcn, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer).map_err(D::Error::custom)?;
            s.parse().map_err(D::Error::custom)
        } else {
            let buf = Vec::<u8>::deserialize(deserializer).map_err(D::Error::custom)?;
            Tcn::from_bytes(&buf).map_err(D::Error::custom)
        }
    }
}
