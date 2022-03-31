mod actors;
pub mod error;
mod events;
pub mod types;

// re-export the job coord handle as if it is the job coordinator itself.
pub use actors::coordinator::JobCoordinatorHandle as JobCoordinator;
pub use events::JobStatus;

#[cfg(test)]
mod joblib_tests {
    use futures::future::join_all;

    use super::*;
    use crate::error::Error as JobError;

    #[tokio::test]
    async fn basic() {
        // spawn a job coordinator with capacity for 32 concurrent messages
        let coordinator = JobCoordinator::spawn(32);
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

    #[tokio::test]
    async fn job_status() {
        // spawn a job coordinator with capacity for 32 concurrent messages
        let coordinator = JobCoordinator::spawn(32);
        let sleep_cmd = "sleep".to_string();

        // spawn a long sleep and short sleep
        let long_sleep_id = coordinator
            .start_job(
                sleep_cmd.clone(),
                vec!["1000".into()],
                "/tmp".into(),
                vec![],
            )
            .await
            .expect("start job err");
        let short_sleep_id = coordinator
            .start_job(sleep_cmd.clone(), vec!["2".into()], "/tmp".into(), vec![])
            .await
            .expect("start job err");

        // get status of jobs
        let long_sleep_status = coordinator
            .get_job_status(long_sleep_id)
            .await
            .expect("job id doesnt exist");
        let short_sleep_status = coordinator
            .get_job_status(short_sleep_id)
            .await
            .expect("job id doesnt exist");
        assert!(matches!(long_sleep_status, JobStatus::Running));
        assert!(matches!(short_sleep_status, JobStatus::Running));

        // wait for the short job to exit
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // get status of jobs again
        let long_sleep_status = coordinator
            .get_job_status(long_sleep_id)
            .await
            .expect("job id doesnt exist");
        let short_sleep_status = coordinator
            .get_job_status(short_sleep_id)
            .await
            .expect("job id doesnt exist");
        assert!(matches!(long_sleep_status, JobStatus::Running));
        assert!(matches!(short_sleep_status, JobStatus::Exited { code: 0 }));

        // kill the long sleeping job
        match coordinator.stop_job(long_sleep_id).await {
            Err(JobError::DoesNotExist) => panic!("job coordinator dropped the job"),
            Err(JobError::AlreadyStopped) => panic!("long sleep job exited already"),
            Ok(()) => {
                // give the child process some time to be reaped
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // get status of the long sleep job
                let long_sleep_status = coordinator
                    .get_job_status(long_sleep_id)
                    .await
                    .expect("job id doesnt exist");
                assert!(matches!(long_sleep_status, JobStatus::Killed { signal: 9 }));
            }
        }
        assert!(matches!(
            coordinator.stop_job(long_sleep_id).await,
            Err(JobError::AlreadyStopped)
        ));
    }

    #[tokio::test]
    async fn concurrent_output() {
        let coordinator = JobCoordinator::spawn(32);
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
        // get output for 3600 clients.
        let num_subs = 3600;
        let mut subscribers = Vec::with_capacity(num_subs);
        for _ in 0..num_subs {
            let mut output = coordinator
                .stream_all(job_id)
                .await
                .expect("failed to grab stdout/stderr for job");
            subscribers.push(tokio::spawn(async move {
                let mut output_bytes = vec![];
                while let Some(blob) = output.recv().await {
                    output_bytes.extend(blob);
                }
                // Each client will wait 1 second.
                // This is just a sanity check that the futures are being driven to completion concurrently
                // If they didn't run concurrently, this would take an hour to run!
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                String::from_utf8_lossy(&output_bytes).into_owned()
            }));
        }
        // give 10 seconds to get all the output or fail test
        match tokio::time::timeout(std::time::Duration::from_secs(10), join_all(subscribers)).await
        {
            Err(_) => panic!("subscribers didnt get output concurrently"),
            Ok(join_handles) => {
                let received_strs: Vec<_> = join_handles
                    .into_iter()
                    .filter_map(|result| result.ok())
                    .collect();
                assert_eq!(received_strs.len(), num_subs);
                for res in received_strs {
                    assert_eq!(res, echo_str);
                }
            }
        }
    }
}
