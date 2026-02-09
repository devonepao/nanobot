# Cron Module

The cron module provides scheduled task execution with support for cron expressions and interval-based scheduling.

## Features

- **Multiple Schedule Types**:
  - `at`: One-time execution at a specific timestamp
  - `every`: Recurring execution at fixed intervals
  - `cron`: Cron expression-based scheduling

- **Job Management**:
  - Add, remove, enable/disable jobs
  - Manual job execution
  - Persistent storage in JSON format
  - Automatic job cleanup for one-shot tasks

- **Async Execution**:
  - Built on Tokio for efficient async operations
  - Callback-based job execution
  - Non-blocking timer implementation

## Usage

### Creating a Cron Service

```rust
use nanobot::cron::{CronService, CronSchedule};
use std::sync::Arc;

// Create service with storage path
let service = Arc::new(CronService::new("/path/to/cron.json"));

// Optional: Add callback for job execution
let service = Arc::new(
    CronService::new("/path/to/cron.json")
        .with_callback(|job| async move {
            println!("Executing job: {}", job.name);
            Some("Job completed".to_string())
        })
);

// Start the service
service.start().await?;

// Spawn the timer loop
let handle = service.clone().spawn();
```

### Schedule Types

#### At (One-time)

```rust
// Run at a specific timestamp (milliseconds)
let schedule = CronSchedule::at(timestamp_ms);

service.add_job(
    "one-time-task",
    schedule,
    "Execute this message",
    false,
    None,
    None,
    true  // Delete after run
).await?;
```

#### Every (Interval)

```rust
// Run every 5 seconds
let schedule = CronSchedule::every(5000);

service.add_job(
    "recurring-task",
    schedule,
    "Execute this message",
    false,
    None,
    None,
    false
).await?;
```

#### Cron Expression

The cron module supports standard cron expressions (5 fields) which are automatically converted to 6-field format (including seconds):

```rust
// Run every day at 9 AM
let schedule = CronSchedule::cron("0 9 * * *");

// Run every minute
let schedule = CronSchedule::cron("* * * * *");

// Run every Monday at 2:30 PM
let schedule = CronSchedule::cron("30 14 * * 1");

service.add_job(
    "daily-task",
    schedule,
    "Execute this message",
    false,
    None,
    None,
    false
).await?;
```

**Cron Expression Format** (5 fields):
- `minute (0-59)`
- `hour (0-23)`
- `day of month (1-31)`
- `month (1-12)`
- `day of week (0-6, Sunday=0)`

Special characters: `*` (any), `,` (list), `-` (range), `/` (step)

### Job Management

```rust
// List all enabled jobs
let jobs = service.list_jobs(false).await;

// List all jobs including disabled
let jobs = service.list_jobs(true).await;

// Enable/disable a job
service.enable_job(&job_id, false).await?;

// Remove a job
service.remove_job(&job_id).await?;

// Manually run a job
service.run_job_now(&job_id, false).await?;

// Get service status
let status = service.status().await;
println!("Jobs: {}, Next wake: {:?}", status.jobs, status.next_wake_at_ms);
```

### Job Payload

Jobs can carry additional information in their payload:

```rust
use nanobot::cron::CronPayload;

// The payload is used by the callback to determine what to do
// - kind: "agent_turn" or "system_event"
// - message: The message to execute
// - deliver: Whether to deliver the response
// - channel: Delivery channel (e.g., "whatsapp")
// - to: Recipient address
```

### Stopping the Service

```rust
// Stop the cron service
service.stop().await;

// Wait for the timer loop to finish
handle.await?;
```

## Persistence

Jobs are automatically persisted to the specified JSON file. The format is:

```json
{
  "version": 1,
  "jobs": [
    {
      "id": "abc123",
      "name": "my-job",
      "enabled": true,
      "schedule": {
        "kind": "cron",
        "expr": "0 9 * * *"
      },
      "payload": {
        "kind": "agent_turn",
        "message": "Good morning!",
        "deliver": false
      },
      "state": {
        "nextRunAtMs": 1234567890000,
        "lastRunAtMs": 1234567880000,
        "lastStatus": "ok"
      },
      "createdAtMs": 1234567800000,
      "updatedAtMs": 1234567890000,
      "deleteAfterRun": false
    }
  ]
}
```

## Error Handling

The cron service uses Rust's `Result` type for error handling:

```rust
use anyhow::Result;

async fn setup_cron() -> Result<()> {
    let service = Arc::new(CronService::new("cron.json"));
    service.start().await?;
    
    // Validate cron expression before use
    validate_cron_expression("0 9 * * *")?;
    
    Ok(())
}
```

## Differences from Python Implementation

1. **Cron Expression Format**: The Rust `cron` crate uses 6-field format internally (including seconds), but this module automatically converts 5-field expressions for compatibility.

2. **Async/Await**: Uses Tokio's async runtime instead of Python's asyncio.

3. **Type Safety**: Strong typing with Rust's type system provides compile-time guarantees.

4. **Memory Safety**: No garbage collector; deterministic memory management.

5. **Callback Type**: Uses trait objects (`dyn Fn`) instead of Python's callable protocol.

## Testing

Run the cron module tests:

```bash
cargo test cron::
```

## Examples

See the `examples/` directory for complete examples of using the cron service.
