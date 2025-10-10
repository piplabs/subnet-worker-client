use anyhow::Result;
use rocksdb::{Options, DB};

pub struct KvStore {
    db: DB,
}

impl KvStore {
    pub fn open(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
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
}

pub mod keys {
    pub fn inflight(activity_id: &str) -> String { format!("inflight:{}", activity_id) }
    pub fn tx(activity_id: &str) -> String { format!("tx:{}", activity_id) }
    pub fn done(activity_id: &str) -> String { format!("done:{}", activity_id) }
    pub const NONCE_LAST: &str = "nonce:last";
}


