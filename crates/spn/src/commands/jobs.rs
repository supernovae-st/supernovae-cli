//! Jobs CLI commands.
//!
//! Submit and manage background Nika workflow jobs.

use crate::daemon::jobs::{Job, JobScheduler, JobState, JobStore};
use crate::error::Result;
use crate::ux::design_system as ds;
use crate::JobsCommands;
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// Run a jobs command.
pub async fn run(command: JobsCommands) -> Result<()> {
    match command {
        JobsCommands::List { all, json } => list(all, json).await,
        JobsCommands::Submit {
            workflow,
            args,
            name,
            priority,
        } => submit(workflow, args, name, priority).await,
        JobsCommands::Status { id } => status(&id).await,
        JobsCommands::Cancel { id } => cancel(&id).await,
        JobsCommands::Output { id, follow } => output(&id, follow).await,
        JobsCommands::Clear { all } => clear(all).await,
    }
}

/// Get the job store path.
fn jobs_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".spn/jobs"))
}

/// List jobs.
async fn list(all: bool, json: bool) -> Result<()> {
    let store = JobStore::new(jobs_dir()?);
    store.init().await?;

    let jobs = store.list().await;

    // Filter to recent/active unless --all
    let jobs: Vec<_> = if all {
        jobs
    } else {
        jobs.into_iter()
            .filter(|j| !j.is_terminal() || is_recent(j.ended_at, 86400))
            .collect()
    };

    if json {
        let output = serde_json::to_string_pretty(&jobs)?;
        println!("{}", output);
        return Ok(());
    }

    if jobs.is_empty() {
        println!("{} No jobs found", ds::primary("→"));
        println!();
        println!(
            "Submit a workflow with: {}",
            ds::highlight("spn jobs submit <workflow.yaml>")
        );
        return Ok(());
    }

    println!("{}", ds::highlight("Background Jobs"));
    println!();

    // Header
    println!(
        "  {:<10} {:<12} {:<20} {:<15}",
        "ID", "STATE", "WORKFLOW", "CREATED"
    );
    println!("  {}", "─".repeat(60));

    for job_status in &jobs {
        let state_str = format_state(job_status.state);
        let workflow = job_status
            .job
            .workflow
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "?".to_string());
        let created = format_time(job_status.job.created_at);

        println!(
            "  {:<10} {:<12} {:<20} {:<15}",
            job_status.job.id.to_string(),
            state_str,
            truncate(&workflow, 18),
            created
        );
    }

    println!();

    // Summary
    let pending = jobs.iter().filter(|j| j.state == JobState::Pending).count();
    let running = jobs.iter().filter(|j| j.state == JobState::Running).count();
    let completed = jobs
        .iter()
        .filter(|j| j.state == JobState::Completed)
        .count();
    let failed = jobs.iter().filter(|j| j.state == JobState::Failed).count();

    println!(
        "  {} total: {} pending, {} running, {} completed, {} failed",
        jobs.len(),
        pending,
        running,
        completed,
        failed
    );

    Ok(())
}

/// Submit a new job.
async fn submit(
    workflow: PathBuf,
    args: Vec<String>,
    name: Option<String>,
    priority: i32,
) -> Result<()> {
    // Validate workflow exists
    if !workflow.exists() {
        println!(
            "{} Workflow not found: {}",
            ds::error("✗"),
            workflow.display()
        );
        return Ok(());
    }

    let store = Arc::new(JobStore::new(jobs_dir()?));
    store.init().await?;

    let scheduler = JobScheduler::new(store);

    if !scheduler.has_nika() {
        println!(
            "{} Nika not found. Install with: {}",
            ds::warning("⚠"),
            ds::highlight("spn setup nika")
        );
        return Ok(());
    }

    let mut job = Job::new(workflow.clone())
        .with_priority(priority)
        .with_args(args);

    if let Some(n) = name {
        job = job.with_name(n);
    }

    let status = scheduler.submit(job).await;

    println!("{} Job submitted", ds::success("✓"));
    println!();
    println!("  ID:        {}", status.job.id);
    println!("  Workflow:  {}", workflow.display());
    println!("  State:     {}", format_state(status.state));
    println!();
    println!(
        "Track with: {}",
        ds::highlight(format!("spn jobs status {}", status.job.id))
    );

    Ok(())
}

/// Show job status.
async fn status(id: &str) -> Result<()> {
    let store = JobStore::new(jobs_dir()?);
    store.init().await?;

    // Parse the ID (short or full)
    let job_status = find_job_by_prefix(&store, id).await?;

    match job_status {
        Some(status) => {
            println!("{}", ds::highlight("Job Details"));
            println!();
            println!("  ID:        {}", status.job.id);
            if let Some(ref name) = status.job.name {
                println!("  Name:      {}", name);
            }
            println!("  Workflow:  {}", status.job.workflow.display());
            println!("  State:     {}", format_state(status.state));
            println!("  Priority:  {}", status.job.priority);
            println!("  Created:   {}", format_datetime(status.job.created_at));

            if let Some(started) = status.started_at {
                println!("  Started:   {}", format_datetime(started));
            }
            if let Some(ended) = status.ended_at {
                println!("  Ended:     {}", format_datetime(ended));
            }

            if let Some(ref error) = status.error {
                println!();
                println!("  {} {}", ds::error("Error:"), error);
            }

            if let Some(ref output) = status.output {
                if !output.is_empty() {
                    println!();
                    println!("  Output:");
                    for line in output.lines().take(10) {
                        println!("    {}", line);
                    }
                    if output.lines().count() > 10 {
                        println!("    ... (use 'spn jobs output {}' for full output)", id);
                    }
                }
            }
        }
        None => {
            println!("{} Job not found: {}", ds::error("✗"), id);
        }
    }

    Ok(())
}

/// Cancel a job.
async fn cancel(id: &str) -> Result<()> {
    let store = Arc::new(JobStore::new(jobs_dir()?));
    store.init().await?;

    let scheduler = JobScheduler::new(store.clone());

    // Find the job
    let job_status = find_job_by_prefix(&store, id).await?;

    match job_status {
        Some(status) => {
            if status.is_terminal() {
                println!(
                    "{} Job {} is already {}",
                    ds::warning("⚠"),
                    status.job.id,
                    status.state
                );
                return Ok(());
            }

            if scheduler.cancel(&status.job.id).await {
                println!("{} Job {} cancelled", ds::success("✓"), status.job.id);
            } else {
                println!("{} Failed to cancel job", ds::error("✗"));
            }
        }
        None => {
            println!("{} Job not found: {}", ds::error("✗"), id);
        }
    }

    Ok(())
}

/// Show job output.
async fn output(id: &str, follow: bool) -> Result<()> {
    let store = JobStore::new(jobs_dir()?);
    store.init().await?;

    let job_status = find_job_by_prefix(&store, id).await?;

    match job_status {
        Some(status) => {
            if let Some(output) = &status.output {
                println!("{}", output);
            } else if status.state == JobState::Running || status.state == JobState::Pending {
                if follow {
                    // Follow mode: poll until job completes
                    follow_job_output(&store, &status.job.id).await?;
                } else {
                    println!("{} Job is still running...", ds::primary("→"));
                    println!(
                        "Use {} to wait for completion",
                        ds::highlight(format!("spn jobs output {} --follow", id))
                    );
                }
            } else {
                println!("{} No output available", ds::warning("⚠"));
            }
        }
        None => {
            println!("{} Job not found: {}", ds::error("✗"), id);
        }
    }

    Ok(())
}

/// Follow job output until completion.
async fn follow_job_output(
    store: &JobStore,
    job_id: &crate::daemon::jobs::JobId,
) -> Result<()> {
    use std::io::Write;
    use std::time::Duration;

    println!(
        "{} Following job {}... (Ctrl+C to stop)",
        ds::primary("→"),
        job_id
    );

    let poll_interval = Duration::from_millis(500);
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_idx = 0;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                // Clear spinner line and show stopped message
                print!("\r{}\r", " ".repeat(60));
                println!("{} Stopped following job", ds::info_line(""));
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                // Check job status
                if let Some(status) = store.get(job_id).await {
                    match status.state {
                        JobState::Running | JobState::Pending => {
                            // Show spinner
                            print!(
                                "\r{} {} {}",
                                spinner_chars[spinner_idx % spinner_chars.len()],
                                ds::highlight(status.state.to_string()),
                                status.job.name.as_deref().unwrap_or("unnamed")
                            );
                            std::io::stdout().flush().ok();
                            spinner_idx += 1;
                        }
                        JobState::Completed | JobState::Failed | JobState::Cancelled => {
                            // Clear spinner line
                            print!("\r{}\r", " ".repeat(60));

                            // Show final state
                            match status.state {
                                JobState::Completed => {
                                    println!("{} Job completed", ds::success("✓"));
                                }
                                JobState::Failed => {
                                    println!("{} Job failed", ds::error("✗"));
                                }
                                JobState::Cancelled => {
                                    println!("{} Job cancelled", ds::warning("⚠"));
                                }
                                _ => {}
                            }

                            // Show output if available
                            if let Some(output) = status.output {
                                println!();
                                println!("{}", output);
                            }

                            break;
                        }
                    }
                } else {
                    print!("\r{}\r", " ".repeat(60));
                    println!("{} Job no longer exists", ds::error("✗"));
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Clear completed/failed jobs.
async fn clear(all: bool) -> Result<()> {
    let store = JobStore::new(jobs_dir()?);
    store.init().await?;

    let max_age = if all { 0 } else { 86400 }; // 24 hours
    let removed = store.cleanup(max_age).await;

    if removed > 0 {
        println!("{} Cleared {} old jobs", ds::success("✓"), removed);
    } else {
        println!("{} No jobs to clear", ds::primary("→"));
    }

    Ok(())
}

/// Find a job by ID prefix.
async fn find_job_by_prefix(
    store: &JobStore,
    prefix: &str,
) -> Result<Option<crate::daemon::jobs::JobStatus>> {
    let jobs = store.list().await;
    let matches: Vec<_> = jobs
        .into_iter()
        .filter(|j| j.job.id.to_string().starts_with(prefix))
        .collect();

    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches.into_iter().next().unwrap())),
        _ => {
            println!(
                "{} Ambiguous ID '{}'. Matches: {}",
                ds::warning("⚠"),
                prefix,
                matches
                    .iter()
                    .map(|j| j.job.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Ok(None)
        }
    }
}

/// Format job state with color.
fn format_state(state: JobState) -> String {
    let s = state.to_string();
    match state {
        JobState::Pending => ds::primary(s).to_string(),
        JobState::Running => ds::highlight(s).to_string(),
        JobState::Completed => ds::success(s).to_string(),
        JobState::Failed => ds::error(s).to_string(),
        JobState::Cancelled => ds::warning(s).to_string(),
    }
}

/// Format time as relative.
fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    let now = Local::now();
    let duration = now.signed_duration_since(datetime);

    if duration.num_seconds() < 60 {
        "just now".into()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

/// Format time as datetime.
fn format_datetime(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Check if time is within seconds.
fn is_recent(time: Option<SystemTime>, seconds: u64) -> bool {
    match time {
        Some(t) => {
            let now = SystemTime::now();
            match now.duration_since(t) {
                Ok(duration) => duration.as_secs() < seconds,
                Err(_) => true, // Future time
            }
        }
        None => true, // No end time = still active
    }
}

/// Truncate string with ellipsis.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world", 8);
        assert!(result.len() <= 8 || result.ends_with('…'));
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_format_state_pending() {
        let state = format_state(JobState::Pending);
        // Contains the word Pending (possibly with ANSI colors)
        assert!(state.to_lowercase().contains("pending") || state.contains("Pending"));
    }

    #[test]
    fn test_format_state_running() {
        let state = format_state(JobState::Running);
        assert!(state.to_lowercase().contains("running") || state.contains("Running"));
    }

    #[test]
    fn test_format_state_completed() {
        let state = format_state(JobState::Completed);
        assert!(state.to_lowercase().contains("completed") || state.contains("Completed"));
    }

    #[test]
    fn test_format_state_failed() {
        let state = format_state(JobState::Failed);
        assert!(state.to_lowercase().contains("failed") || state.contains("Failed"));
    }

    #[test]
    fn test_jobs_dir_returns_valid_path() {
        let path = jobs_dir().unwrap();
        assert!(path.to_string_lossy().contains(".spn"));
        assert!(path.to_string_lossy().contains("jobs"));
    }
}
