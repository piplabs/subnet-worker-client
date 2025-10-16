use anyhow::Result;
use rocksdb::{Options, DB};
use std::sync::Arc;

#[derive(Clone)]
pub struct KvStore { db: Arc<DB> }

impl KvStore {
    pub fn open(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<()> {
        Ok(self.db.put(key, value)?)
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?)
    }

    pub fn delete(&self, key: impl AsRef<[u8]>) -> Result<()> {
        Ok(self.db.delete(key)?)
    }

    pub fn scan_prefix(&self, prefix: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut out = Vec::new();
        let mut it = self.db.raw_iterator();
        let pref = prefix.as_bytes();
        it.seek(pref);
        while it.valid() {
            if let (Some(k), Some(v)) = (it.key(), it.value()) {
                if !k.starts_with(pref) { break; }
                out.push((k.to_vec(), v.to_vec()));
            } else {
                break;
            }
            it.next();
        }
        Ok(out)
    }
}

pub mod keys {
    pub fn inflight(activity_id: &str) -> String { format!("inflight:{}", activity_id) }
    pub fn tx(activity_id: &str) -> String { format!("tx:{}", activity_id) }
    pub fn done(activity_id: &str) -> String { format!("done:{}", activity_id) }
    pub fn claim_job(activity_id: &str) -> String { format!("claim_job:{}", activity_id) }
    pub const NONCE_LAST: &str = "nonce:last";
}


