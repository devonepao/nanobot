use crate::cron::types::{CronSchedule, ScheduleKind};
use anyhow::{anyhow, Result};
use chrono::DateTime;
use cron::Schedule;
use std::str::FromStr;

pub fn compute_next_run(schedule: &CronSchedule, now_ms: i64) -> Option<i64> {
    match schedule.kind {
        ScheduleKind::At => {
            if let Some(at_ms) = schedule.at_ms {
                if at_ms > now_ms {
                    return Some(at_ms);
                }
            }
            None
        }
        ScheduleKind::Every => {
            if let Some(every_ms) = schedule.every_ms {
                if every_ms > 0 {
                    return Some(now_ms + every_ms);
                }
            }
            None
        }
        ScheduleKind::Cron => {
            if let Some(ref expr) = schedule.expr {
                compute_cron_next(expr, now_ms).ok()
            } else {
                None
            }
        }
    }
}

fn compute_cron_next(expr: &str, now_ms: i64) -> Result<i64> {
    // The cron crate expects 6 fields (sec min hour day month day_of_week)
    // Convert 5-field format to 6-field by prepending "0" for seconds
    let expr = if expr.split_whitespace().count() == 5 {
        format!("0 {}", expr)
    } else {
        expr.to_string()
    };
    
    let schedule = Schedule::from_str(&expr)
        .map_err(|e| anyhow!("Invalid cron expression '{}': {}", expr, e))?;
    
    let now_secs = now_ms / 1000;
    let now_dt = DateTime::from_timestamp(now_secs, 0)
        .ok_or_else(|| anyhow!("Invalid timestamp"))?;
    
    let next = schedule
        .after(&now_dt)
        .next()
        .ok_or_else(|| anyhow!("No next occurrence for cron expression"))?;
    
    Ok(next.timestamp() * 1000)
}

pub fn validate_cron_expression(expr: &str) -> Result<()> {
    // Convert 5-field format to 6-field by prepending "0" for seconds
    let expr = if expr.split_whitespace().count() == 5 {
        format!("0 {}", expr)
    } else {
        expr.to_string()
    };
    
    Schedule::from_str(&expr)
        .map_err(|e| anyhow!("Invalid cron expression '{}': {}", expr, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_schedule() {
        let now = 1000;
        let schedule = CronSchedule::at(2000);
        assert_eq!(compute_next_run(&schedule, now), Some(2000));
        
        // Past time should return None
        let schedule = CronSchedule::at(500);
        assert_eq!(compute_next_run(&schedule, now), None);
    }

    #[test]
    fn test_every_schedule() {
        let now = 1000;
        let schedule = CronSchedule::every(5000);
        assert_eq!(compute_next_run(&schedule, now), Some(6000));
    }

    #[test]
    fn test_cron_schedule() {
        // Every minute: "* * * * *"
        let schedule = CronSchedule::cron("* * * * *");
        let now = chrono::Utc::now().timestamp() * 1000;
        let next = compute_next_run(&schedule, now);
        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }

    #[test]
    fn test_validate_cron() {
        assert!(validate_cron_expression("* * * * *").is_ok());
        assert!(validate_cron_expression("0 9 * * *").is_ok());
        assert!(validate_cron_expression("invalid").is_err());
    }
}
