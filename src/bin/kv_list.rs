use anyhow::Result;
use subnet_wcp_persistence::KvStore;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let prefix = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let store = KvStore::open("./wcp.db")?;
    for (k, v) in store.scan_prefix(prefix)? {
        let ks = String::from_utf8_lossy(&k);
        let vs = String::from_utf8_lossy(&v);
        println!("{} => {}", ks, vs);
    }
    Ok(())
}


