# ADR-003: Scheduler/Cron in spn-daemon

**Status:** Accepted
**Date:** 2026-03-08
**Authors:** Thibaut, Claude
**Supersedes:** Nika scheduler (to be deprecated)
**Related:** ADR-001, ADR-002

---

## Context

Currently, Nika has preliminary scheduler/cron functionality for running workflows on a schedule. However, this creates architectural issues:

1. **Nika must be a daemon** — Scheduler needs long-running process
2. **Mixed responsibilities** — WHEN to run (infra) vs HOW to run (execution)
3. **Limited scope** — Can only schedule Nika workflows, not other commands

Per ADR-001, infrastructure concerns (WHEN, WHERE) belong in spn, not Nika.

---

## Decision

Move scheduler functionality from Nika to spn-daemon.

### New Commands

```bash
# Add a scheduled job
spn schedule add "nika run daily-report.yaml" --cron "0 9 * * *"
spn schedule add "spn model pull llama3.2" --cron "0 0 * * 0"
spn schedule add "curl https://healthcheck.io/ping/xxx" --every 5m
spn schedule add "spn upgrade --check" --daily

# List scheduled jobs
spn schedule list
spn schedule list --json

# Manage jobs
spn schedule remove <id>
spn schedule pause <id>
spn schedule resume <id>
spn schedule run <id>          # Run now (manual trigger)

# View history
spn schedule logs <id>
spn schedule logs <id> --last 10
spn schedule history           # All jobs
```

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  SCHEDULER IN spn-daemon                                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn-daemon                                                                     │
│  ├── Secret Manager (existing)                                                  │
│  ├── Model Manager (existing)                                                   │
│  └── Scheduler (NEW)                                                            │
│      ├── Job Store (~/.spn/scheduler/jobs.yaml)                                │
│      ├── Cron Parser (tokio-cron-scheduler)                                    │
│      ├── Executor (spawns commands)                                            │
│      └── History Store (~/.spn/scheduler/history/)                             │
│                                                                                 │
│  Flow:                                                                          │
│  1. User: spn schedule add "nika run ..." --cron "0 9 * * *"                   │
│  2. Daemon: Stores job in jobs.yaml                                            │
│  3. Daemon: Cron tick at 9:00 AM                                               │
│  4. Daemon: Spawns "nika run ..." as subprocess                                │
│  5. Daemon: Captures stdout/stderr to history                                  │
│  6. Daemon: Updates last_run, next_run                                         │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Job Schema

```yaml
# ~/.spn/scheduler/jobs.yaml
jobs:
  - id: "a1b2c3d4"
    name: "Daily Report"
    command: "nika run /path/to/daily-report.yaml"
    schedule:
      type: cron
      expression: "0 9 * * *"
    enabled: true
    created_at: "2026-03-08T10:00:00Z"
    last_run: "2026-03-08T09:00:00Z"
    next_run: "2026-03-09T09:00:00Z"
    run_count: 42
    fail_count: 1

  - id: "e5f6g7h8"
    name: "Model Update"
    command: "spn model pull llama3.2"
    schedule:
      type: interval
      every: "7d"
    enabled: true
```

### History Schema

```
~/.spn/scheduler/history/
├── a1b2c3d4/
│   ├── 2026-03-08T09-00-00.log
│   ├── 2026-03-07T09-00-00.log
│   └── ...
└── e5f6g7h8/
    └── ...
```

---

## Implementation

### Daemon Changes

```rust
// crates/spn/src/daemon/scheduler.rs

use tokio_cron_scheduler::{Job, JobScheduler};

pub struct Scheduler {
    scheduler: JobScheduler,
    jobs: Vec<ScheduledJob>,
    history_dir: PathBuf,
}

impl Scheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        let jobs = Self::load_jobs()?;

        for job in &jobs {
            if job.enabled {
                scheduler.add(job.to_cron_job()?).await?;
            }
        }

        Ok(Self { scheduler, jobs, history_dir })
    }

    pub async fn add_job(&mut self, job: ScheduledJob) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.scheduler.add(job.to_cron_job()?).await?;
        self.jobs.push(job);
        self.save_jobs()?;
        Ok(id)
    }

    pub async fn run_job(&self, id: &str) -> Result<()> {
        let job = self.find_job(id)?;
        let output = Command::new("sh")
            .arg("-c")
            .arg(&job.command)
            .output()
            .await?;

        self.save_history(id, &output)?;
        Ok(())
    }
}
```

### Protocol Extension

```rust
// crates/spn/src/daemon/protocol.rs

#[derive(Serialize, Deserialize)]
pub enum Request {
    // Existing
    Ping,
    GetSecret { provider: String },
    ListProviders,

    // NEW: Scheduler
    ScheduleAdd { command: String, schedule: Schedule },
    ScheduleList,
    ScheduleRemove { id: String },
    SchedulePause { id: String },
    ScheduleResume { id: String },
    ScheduleRun { id: String },
    ScheduleLogs { id: String, limit: Option<usize> },
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    // Existing...

    // NEW: Scheduler
    ScheduleAdded { id: String },
    ScheduleList { jobs: Vec<ScheduledJob> },
    ScheduleLogs { logs: Vec<LogEntry> },
    ScheduleOk,
}
```

### CLI Commands

```rust
// crates/spn/src/commands/schedule.rs

#[derive(Subcommand)]
pub enum ScheduleCommands {
    /// Add a scheduled job
    Add {
        /// Command to run
        command: String,

        /// Cron expression
        #[arg(long)]
        cron: Option<String>,

        /// Interval (e.g., "5m", "1h", "7d")
        #[arg(long)]
        every: Option<String>,

        /// Run daily at midnight
        #[arg(long)]
        daily: bool,

        /// Job name (optional)
        #[arg(long)]
        name: Option<String>,
    },

    /// List scheduled jobs
    List {
        #[arg(long)]
        json: bool,
    },

    /// Remove a scheduled job
    Remove { id: String },

    /// Pause a scheduled job
    Pause { id: String },

    /// Resume a paused job
    Resume { id: String },

    /// Run a job immediately
    Run { id: String },

    /// View job logs
    Logs {
        id: String,

        #[arg(long, default_value = "20")]
        last: usize,
    },

    /// View execution history
    History {
        #[arg(long, default_value = "50")]
        limit: usize,
    },
}
```

---

## Migration from Nika

### Phase 1: Implement in spn (v0.17.0)
- Add scheduler to daemon
- Add CLI commands
- Documentation

### Phase 2: Deprecate in Nika (v0.18.0)
- Mark Nika scheduler as deprecated
- Add migration warnings
- Provide migration script

### Phase 3: Remove from Nika (v0.19.0)
- Remove scheduler code from Nika
- Update documentation

### Migration Script

```bash
#!/bin/bash
# migrate-schedules.sh

# Export Nika schedules
nika schedule export > /tmp/nika-schedules.json

# Import to spn
jq -r '.schedules[] | "spn schedule add \"\(.command)\" --cron \"\(.cron)\" --name \"\(.name)\""' \
    /tmp/nika-schedules.json | bash

echo "Migration complete! Verify with: spn schedule list"
```

---

## Consequences

### Positive

1. **Single scheduler** — One place for all scheduled tasks
2. **General purpose** — Can schedule any command, not just Nika
3. **Daemon efficiency** — Reuses existing long-running process
4. **Clear separation** — WHEN (spn) vs HOW (Nika)
5. **Simplified Nika** — One less responsibility

### Negative

1. **Migration effort** — Users must migrate existing schedules
2. **spn dependency** — Scheduling requires spn-daemon running
3. **More daemon code** — Daemon grows in responsibility

### Mitigations

- **Migration**: Provide automated migration script
- **Dependency**: Document clearly, daemon auto-starts
- **Code growth**: Keep scheduler module isolated

---

## Testing Strategy

### Unit Tests
- Cron expression parsing
- Job serialization/deserialization
- Schedule calculations (next_run)

### Integration Tests
- Add/remove/pause/resume jobs
- Job execution and history capture
- Daemon restart preserves jobs

### End-to-End Tests
- Full workflow: add → wait → verify execution
- Error handling: command fails, history recorded

---

## Alternatives Considered

### Alternative A: Keep in Nika
Keep scheduler in Nika, make Nika a proper daemon.

**Rejected because:**
- Mixes WHEN and HOW responsibilities
- Nika must be running always
- Can't schedule non-Nika commands

### Alternative B: Separate scheduler binary
Create `spn-scheduler` as standalone daemon.

**Rejected because:**
- Yet another daemon to run
- Duplicates daemon infrastructure
- Coordination complexity

### Alternative C: Use system cron
Rely on system cron/launchd/systemd timers.

**Rejected because:**
- Platform-specific configuration
- No unified interface
- Hard to manage across machines

---

## Dependencies

### New Crates
```toml
tokio-cron-scheduler = "0.10"  # Cron scheduling
```

### Files to Create
```
crates/spn/src/daemon/scheduler.rs
crates/spn/src/commands/schedule.rs
```

### Files to Modify
```
crates/spn/src/daemon/mod.rs        # Add scheduler module
crates/spn/src/daemon/handler.rs    # Handle schedule requests
crates/spn/src/commands/mod.rs      # Add schedule subcommand
crates/spn/src/main.rs              # Wire up schedule command
```

---

## References

- [ADR-001: Ecosystem Role Distribution](./ADR-001-ecosystem-role-distribution.md)
- [tokio-cron-scheduler](https://github.com/mvniekerk/tokio-cron-scheduler)
- [Cron Expression Format](https://crontab.guru/)

---

## Changelog

| Date | Change |
|------|--------|
| 2026-03-08 | Initial version |
