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
        let coordinator = JobCoordinator::spawn();
        let echo_str = "hello world!";
        let no_trailing_newline = "-n";
        let job_id = coordinator
            .start_job(
                "echo".into(),
                vec![no_trailing_newline.to_string(), echo_str.to_string()],
                "/tmp".into(),
                vec![],
            )
            .await
            .expect("job start err");
        let mut output = coordinator
            .stream_all(job_id)
            .await
            .expect("failed to grab stdout/stderr for job");
        let mut output_bytes = vec![];
        while let Some(blob) = output.recv().await {
            output_bytes.extend(blob);
        }
        assert_eq!(String::from_utf8_lossy(&output_bytes), echo_str);
    }
}
