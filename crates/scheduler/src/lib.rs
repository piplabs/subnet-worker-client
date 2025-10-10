use anyhow::Result;
use subnet_wcp_persistence::KvStore;

pub struct Scheduler {
    #[allow(dead_code)]
    store: KvStore,
}

impl Scheduler {
    pub fn new(store: KvStore) -> Self { Self { store } }

    pub async fn run(&self) -> Result<()> {
        // TODO: poll queues, assign tasks, manage inflight
        Ok(())
    }
}


