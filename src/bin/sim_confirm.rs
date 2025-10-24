use anyhow::Result;
use subnet_wcp_persistence::{KvStore, keys};
use std::env;

fn now_ms() -> i64 { (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()) as i64 }

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: sim_confirm <activity_id_hex_with_0x>");
        std::process::exit(1);
    }
    let activity_id = &args[1];
    let store = KvStore::open("./wcp.db")?;

    // Simulate tx record (already confirmed in chain in our test scenario)
    let tx = serde_json::json!({
        "activity_id": activity_id,
        "kind": "claim",
        "status": "submitted",
        "tx_hash": "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        "submitted_at_ms": now_ms(),
        "last_bump_at_ms": null,
    });
    store.put(keys::tx(activity_id), serde_json::to_vec(&tx)?)?;

    // Simulate inflight entry that confirmer would write after confirmation
    let inflight = serde_json::json!({
        "activity_id": activity_id,
        "queue": "video/1.0.0/processing",
        "claimed_at_ms": now_ms(),
        "assignment_status": "Pending",
    });
    store.put(keys::inflight(activity_id), serde_json::to_vec(&inflight)?)?;

    // Remove enqueue signal if present
    store.delete(keys::broadcast_claim(activity_id).as_bytes())?;

    println!("wrote tx:{} and inflight:{}", keys::tx(activity_id), keys::inflight(activity_id));
    Ok(())
}


