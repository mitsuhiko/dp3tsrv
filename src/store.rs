use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use bytes::{Buf, BufMut, BytesMut};
use chrono::{DateTime, Utc};
use crc::crc32;

use crate::ccn::Ccn;

const DAYS_WINDOW: u64 = 21;

/// Abstracts over an append only file of CCNs
pub struct CcnStore {
    path: PathBuf,
    buckets: RwLock<BTreeMap<u64, HashSet<Ccn>>>,
}

impl fmt::Debug for CcnStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CcnStore")
            .field("path", &self.path)
            .finish()
    }
}

impl CcnStore {
    /// Opens a ccn store
    pub fn open<P: AsRef<Path>>(p: P) -> Result<CcnStore, io::Error> {
        let path = p.as_ref().to_path_buf();
        fs::create_dir_all(&path)?;
        Ok(CcnStore {
            path,
            buckets: RwLock::new(BTreeMap::new()),
        })
    }

    /// Returns the current bucket.
    pub fn current_bucket(&self) -> u64 {
        let now = Utc::now();
        (now.timestamp() as u64) / 3600
    }

    /// Ensure bucket is loaded from disk.
    fn ensure_bucket_loaded(&self, bucket: u64) -> Result<bool, io::Error> {
        // we only upsert so if the bucket was already loaded, we don't
        // need to do anything
        if self.buckets.read().unwrap().contains_key(&bucket) {
            return Ok(false);
        }

        let mut buckets = self.buckets.write().unwrap();
        let path = self.path.join(&format!("_{}.bucket", bucket));

        let mut set = HashSet::new();
        if let Ok(mut f) = fs::File::open(path).map(BufReader::new) {
            loop {
                let mut buf = [0u8; 36];
                match f.read(&mut buf)? {
                    0 => break,
                    x if x != buf.len() => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "something went very wrong",
                        ));
                    }
                    _ => {}
                }
                let ccn = Ccn::from_bytes(&buf[4..]).unwrap();
                let checksum = crc32::checksum_ieee(ccn.as_bytes());
                if (&buf[..4]).get_u32_le() != checksum {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "bad checksum, corrupted file",
                    ));
                }
                set.insert(ccn);
            }
        }

        buckets.insert(bucket, set);

        Ok(true)
    }

    /// Returns all buckets after a certain timestamp
    pub fn fetch_buckets(&self, timestamp: DateTime<Utc>) -> Result<Vec<Ccn>, io::Error> {
        let mut rv = vec![];
        let bucket_start = (timestamp.timestamp() as u64) / 3600;
        let bucket_end = self.current_bucket();

        match bucket_end.checked_sub(bucket_start) {
            None => return Ok(vec![]),
            Some(diff) if diff > 24 * DAYS_WINDOW => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "reading too far into the past",
                ))
            }
            _ => {}
        }

        for bucket in bucket_start..=bucket_end {
            self.ensure_bucket_loaded(bucket)?;
            if let Some(set) = self.buckets.read().unwrap().get(&bucket) {
                rv.extend(set);
            }
        }

        Ok(rv)
    }

    /// Checks if a CCN is already known.
    pub fn has_ccn(&self, ccn: Ccn) -> Result<bool, io::Error> {
        let now = self.current_bucket();
        for bucket in (now - DAYS_WINDOW)..now {
            self.ensure_bucket_loaded(bucket)?;
            if let Some(set) = self.buckets.read().unwrap().get(&bucket) {
                if set.contains(&ccn) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Adds a CCN at the current timestamp.
    pub fn add_ccn(&self, ccn: Ccn) -> Result<bool, io::Error> {
        // check if this ccn has already been seen in the last 21 days
        if self.has_ccn(ccn)? {
            return Ok(false);
        }

        let bucket = self.current_bucket();
        let path = self.path.join(&format!("_{}.bucket", bucket));
        let mut buckets = self.buckets.write().unwrap();
        let mut file = BufWriter::new(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?,
        );
        let mut msg = BytesMut::new();
        msg.put_u32_le(crc32::checksum_ieee(ccn.as_bytes()));
        msg.put_slice(ccn.as_bytes());
        file.write(&msg)?;
        buckets
            .entry(bucket)
            .or_insert_with(Default::default)
            .insert(ccn);
        Ok(true)
    }
}
