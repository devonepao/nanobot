use nanobot::HeartbeatService;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Heartbeat Service Example ===\n");

    // Create a temporary workspace
    let temp_dir = std::env::temp_dir();
    let workspace = temp_dir.join("heartbeat_example");
    std::fs::create_dir_all(&workspace)?;

    // Create the heartbeat service with a callback
    let service = Arc::new(
        HeartbeatService::new(&workspace)
            .with_interval(5) // Check every 5 seconds (normally 30 minutes)
            .with_callback(|prompt| async move {
                println!("🔔 Heartbeat triggered!");
                println!("   Prompt: {}", prompt.lines().next().unwrap_or(""));
                
                // Simulate agent processing
                sleep(Duration::from_millis(500)).await;
                
                // Return response
                Ok("HEARTBEAT_OK".to_string())
            }),
    );

    // Create a HEARTBEAT.md file with some tasks
    let heartbeat_file = workspace.join("HEARTBEAT.md");
    std::fs::write(
        &heartbeat_file,
        "# Heartbeat Tasks\n\n- Check for new emails\n- Review pending notifications\n",
    )?;

    println!("✓ Created HEARTBEAT.md with tasks\n");

    // Start the service
    service.start().await?;
    println!("✓ Heartbeat service started (checking every 5 seconds)\n");

    // Let it run for a bit
    println!("Waiting for heartbeats (will run for 15 seconds)...\n");
    sleep(Duration::from_secs(15)).await;

    // Trigger a manual heartbeat
    println!("\n📢 Triggering manual heartbeat...");
    if let Some(response) = service.trigger_now().await? {
        println!("   Response: {}", response);
    }

    // Clear the HEARTBEAT.md file
    println!("\n✓ Clearing HEARTBEAT.md (removing tasks)");
    std::fs::write(&heartbeat_file, "# Heartbeat Tasks\n\n")?;

    // Wait a bit more
    println!("   Waiting 10 more seconds (should not trigger)...\n");
    sleep(Duration::from_secs(10)).await;

    // Stop the service
    service.stop().await;
    println!("✓ Heartbeat service stopped");

    // Clean up
    std::fs::remove_dir_all(&workspace)?;

    Ok(())
}
