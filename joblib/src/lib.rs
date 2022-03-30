mod actors;
pub mod errors;
mod events;
pub mod types;

// re-export the job coord handle as if it is the job coordinator itself.
pub use actors::coordinator::JobCoordinatorHandle as JobCoordinator;
pub use events::JobStatus;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic() {
        let coordinator = JobCoordinator::new();
        let jid = coordinator
            .start_job(
                "echo".into(),
                vec!["hello world!".into()],
                "/tmp".into(),
                vec![],
            )
            .await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("Started job {}", jid.expect("job start error"));
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}
