use std::fmt;
use std::str;

use aes::block_cipher_trait::generic_array::GenericArray;
use aes::block_cipher_trait::BlockCipher;
use aes::Aes256;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use derive_more::{Display, Error};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::tcn::Tcn;

const BROADCAST_KEY: &[u8] =
    &*b"\xe8\x8c^&\x87.\xb2\x05tJ\xedf-\xec\xf0'\x17:S\x0b*j\xc7\x01\x92x\x80\x18\x05\xe3w\xb0";

/// A compact representation of contact numbers.
#[derive(Default, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Ccn {
    bytes: [u8; 32],
}

impl fmt::Debug for Ccn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Ccn").field(&self.to_string()).finish()
    }
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

    /// Returns the bytes behind the CCN
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Generates a new CCN from this one.
    pub fn ratchet(&self) -> Ccn {
        let mut h = Sha256::new();
        h.input(self.bytes);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&h.result());
        Ccn { bytes }
    }

    /// Generates the successor CCNs.
    pub fn generate_ccns(&self) -> impl Iterator<Item = Ccn> {
        let mut current = self.ratchet();
        std::iter::from_fn(move || {
            let rv = current;
            current = current.ratchet();
            Some(rv)
        })
    }

    /// Generates a list of TCNs
    pub fn generate_tcns(&self) -> impl Iterator<Item = Tcn> {
        let mut hmac = Hmac::<Sha256>::new_varkey(BROADCAST_KEY).unwrap();
        hmac.input(self.as_bytes());
        let cipher = Aes256::new(&GenericArray::from_slice(&hmac.result().code()));
        let mut block = GenericArray::clone_from_slice(&[0u8; 16]);
        std::iter::from_fn(move || {
            cipher.encrypt_block(&mut block);
            Some(Tcn::from_bytes(&block[..]).unwrap())
        })
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
            serializer.serialize_bytes(self.as_bytes())
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

#[test]
fn test_ccn_and_tcn() {
    let ccn_0 = Ccn::from_bytes(&b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"[..]).unwrap();
    let ccn_1 = ccn_0.ratchet();
    assert_eq!(
        ccn_1,
        "xi5GFb054iJXLzob98ITLqHmWxfsgFBHvWsoQsWTST8"
            .parse()
            .unwrap()
    );

    let tcns: Vec<_> = ccn_1.generate_tcns().take(3).collect();
    assert_eq!(
        tcns,
        vec![
            "lCwTulzwiY0kYMjLXsZ73Q".parse().unwrap(),
            "SK3F0EX9piiOphM5a_d0-g".parse().unwrap(),
            "hX5kfy51PnOsQGWEmliaAw".parse().unwrap(),
        ]
    );
}
