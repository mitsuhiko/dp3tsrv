use std::fmt;
use std::str;

use derive_more::{Display, Error};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// A compact representation of contact numbers.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Ccn {
    bytes: [u8; 32],
}

impl Ccn {
    /// Creates a CCN from raw bytes.
    pub fn from_bytes(b: &[u8]) -> Result<Ccn, InvalidCcn> {
        if b.len() != 32 {
            return Err(InvalidCcn);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(b);
        Ok(Ccn { bytes })
    }

    /// Returns true if this is the nil CCN
    pub fn is_nil(&self) -> bool {
        self.bytes.iter().all(|&x| x == 0)
    }

    /// Returns the bytes behind the CCN
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Raised if a CCN is invalid.
#[derive(Error, Display, Debug)]
#[display(fmt = "invalid ccn")]
pub struct InvalidCcn;

impl str::FromStr for Ccn {
    type Err = InvalidCcn;

    fn from_str(value: &str) -> Result<Ccn, InvalidCcn> {
        let mut bytes = [0u8; 32];
        if value.len() != 43 {
            return Err(InvalidCcn);
        }
        base64::decode_config_slice(value, base64::URL_SAFE_NO_PAD, &mut bytes[..])
            .map_err(|_| InvalidCcn)?;
        Ok(Ccn { bytes })
    }
}

impl fmt::Display for Ccn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 50];
        let len = base64::encode_config_slice(self.bytes, base64::URL_SAFE_NO_PAD, &mut buf);
        f.write_str(unsafe { std::str::from_utf8_unchecked(&buf[..len]) })
    }
}

impl Serialize for Ccn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_bytes(&self.bytes)
        }
    }
}

impl<'de> Deserialize<'de> for Ccn {
    fn deserialize<D>(deserializer: D) -> Result<Ccn, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer).map_err(D::Error::custom)?;
            s.parse().map_err(D::Error::custom)
        } else {
            let buf = Vec::<u8>::deserialize(deserializer).map_err(D::Error::custom)?;
            Ccn::from_bytes(&buf).map_err(D::Error::custom)
        }
    }
}
