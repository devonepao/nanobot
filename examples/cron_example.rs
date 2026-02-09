use nanobot::cron::{CronSchedule, CronService};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Cron Service Example ===\n");

    // Create a temporary file for storing cron jobs
    let temp_dir = std::env::temp_dir();
    let store_path = temp_dir.join("example_cron.json");

    // Create the cron service with a callback
    let service = Arc::new(
        CronService::new(&store_path).with_callback(|job| async move {
            println!("🔔 Executing job: {}", job.name);
            println!("   Message: {}", job.payload.message);
            Some(format!("Job '{}' completed successfully", job.name))
        }),
    );

    // Start the service
    service.start().await?;
    println!("✓ Cron service started\n");

    // Spawn the timer loop
    let handle = service.clone().spawn();

    // Add an interval-based job (every 3 seconds)
    println!("Adding interval job (every 3 seconds)...");
    let schedule = CronSchedule::every(3000);
    let job1 = service
        .add_job(
            "interval-task",
            schedule,
            "This runs every 3 seconds",
            false,
            None,
            None,
            false,
        )
        .await?;
    println!("✓ Added job: {} (ID: {})\n", job1.name, job1.id);

    // Add a one-time job (5 seconds from now)
    println!("Adding one-time job (5 seconds from now)...");
    let now_ms = chrono::Utc::now().timestamp_millis();
    let schedule = CronSchedule::at(now_ms + 5000);
    let job2 = service
        .add_job(
            "one-time-task",
            schedule,
            "This runs once after 5 seconds",
            false,
            None,
            None,
            true, // Delete after run
        )
        .await?;
    println!("✓ Added job: {} (ID: {})\n", job2.name, job2.id);

    // Add a cron job (every minute at :00 seconds)
    println!("Adding cron job (every minute)...");
    let schedule = CronSchedule::cron("* * * * *");
    let job3 = service
        .add_job(
            "cron-task",
            schedule,
            "This runs every minute",
            false,
            None,
            None,
            false,
        )
        .await?;
    println!("✓ Added job: {} (ID: {})\n", job3.name, job3.id);

    // List all jobs
    println!("Current jobs:");
    let jobs = service.list_jobs(false).await;
    for job in &jobs {
        println!(
            "  • {} (ID: {}) - Next run: {:?}",
            job.name, job.id, job.state.next_run_at_ms
        );
    }
    println!();

    // Get service status
    let status = service.status().await;
    println!("Service status:");
    println!("  • Enabled: {}", status.enabled);
    println!("  • Total jobs: {}", status.jobs);
    println!("  • Next wake: {:?}\n", status.next_wake_at_ms);

    // Wait for 10 seconds to see jobs execute
    println!("Waiting 10 seconds to observe job executions...\n");
    sleep(Duration::from_secs(10)).await;

    // Disable the interval job
    println!("\nDisabling interval job...");
    service.enable_job(&job1.id, false).await?;
    println!("✓ Job '{}' disabled\n", job1.name);

    // List jobs again
    println!("Active jobs after disabling:");
    let jobs = service.list_jobs(false).await;
    for job in &jobs {
        println!("  • {} (ID: {})", job.name, job.id);
    }
    println!();

    // Manually run a job
    println!("Manually running cron job...");
    service.run_job_now(&job3.id, false).await?;
    println!("✓ Job '{}' executed manually\n", job3.name);

    // Clean up
    println!("Stopping cron service...");
    service.stop().await;
    handle.await?;
    println!("✓ Cron service stopped");

    // Clean up the temporary file
    if store_path.exists() {
        tokio::fs::remove_file(&store_path).await?;
    }

    Ok(())
}
