use anyhow::Result;

pub struct EventPoller;

impl EventPoller {
    pub async fn run(&self) -> Result<()> {
        // TODO: subscribe to TaskQueue/WorkflowEngine events and surface to scheduler
        Ok(())
    }
}


