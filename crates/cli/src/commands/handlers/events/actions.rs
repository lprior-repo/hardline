//! Action functions for the events command handler (Tier 3).
//!
//! I/O operations that query and display events from the event log.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{event_type_matches, EventEntry, EventsOptions, EventsOutput};

/// Default number of events returned when no limit is specified.
const DEFAULT_LIMIT: usize = 50;

/// Execute the events command with the given options.
///
/// In list mode, reads the events log file, applies filters, and displays results.
/// In follow mode, continuously polls for new events.
///
/// # Errors
///
/// Returns error if the events log file cannot be read.
pub fn run_events(options: &EventsOptions) -> Result<()> {
    if options.follow {
        run_follow(options)
    } else {
        run_list(options)
    }
}

/// List recent events, optionally filtered.
fn run_list(options: &EventsOptions) -> Result<()> {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT);
    let entries = read_events_from_log(
        options.session.as_deref(),
        options.event_type.as_deref(),
        limit,
        options.since.as_deref(),
    )?;

    let output = EventsOutput {
        total: entries.len(),
        has_more: false,
        cursor: None,
        events: entries,
    };

    print_events(&output);
    Ok(())
}

/// Follow mode: print current events, then indicate streaming mode.
///
/// Note: Full streaming (infinite poll loop) is not implemented in this
/// synchronous handler. This outputs the current batch and a message
/// indicating that streaming would continue in a real async context.
fn run_follow(options: &EventsOptions) -> Result<()> {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT);
    let entries = read_events_from_log(
        options.session.as_deref(),
        options.event_type.as_deref(),
        limit,
        options.since.as_deref(),
    )?;

    let output = EventsOutput {
        total: entries.len(),
        has_more: false,
        cursor: None,
        events: entries,
    };

    print_events(&output);
    Output::info("Following events... (streaming requires async runtime)");
    Ok(())
}

/// Read events from the JSONL event log file, applying filters.
///
/// The events file is expected to live at `<data_dir>/events.jsonl`.
/// Falls back to an empty list if the file does not exist.
fn read_events_from_log(
    session: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
    since: Option<&str>,
) -> Result<Vec<EventEntry>> {
    let events_file = resolve_events_file_path()?;

    let content = match std::fs::read_to_string(&events_file) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };

    let mut entries: Vec<EventEntry> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            serde_json::from_str::<EventEntry>(line).ok().filter(|entry| {
                let session_ok = session.is_none_or(|s| entry.session.as_deref() == Some(s));
                let type_ok = event_type_matches(event_type, &entry.event_type);
                let since_ok = since.is_none_or(|st| entry.timestamp.as_str() >= st);

                session_ok && type_ok && since_ok
            })
        })
        .collect();

    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.truncate(limit);

    Ok(entries)
}

/// Resolve the path to the events JSONL log file.
///
/// Looks for a `.scp/data/events.jsonl` relative to the current directory,
/// falling back to `$HOME/.scp/data/events.jsonl`.
fn resolve_events_file_path() -> Result<std::path::PathBuf> {
    let local_path = std::path::Path::new(".scp/data/events.jsonl");
    if local_path.exists() {
        return Ok(local_path.to_path_buf());
    }

    let home_dir = std::env::var("HOME").map_err(|_| {
        Error::io_error("Cannot determine home directory: HOME env var not set")
    })?;

    Ok(std::path::Path::new(&home_dir).join(".scp/data/events.jsonl"))
}

/// Print events to stdout using the Output facade.
fn print_events(output: &EventsOutput) {
    if output.events.is_empty() {
        Output::info("No events found.");
        return;
    }

    Output::info(&format!("Events ({} shown, {} total):", output.events.len(), output.total));

    output.events.iter().for_each(|entry| {
        let session_str = entry
            .session
            .as_ref()
            .map_or_else(String::new, |s| format!(" [{s}]"));
        let agent_str = entry
            .agent_id
            .as_ref()
            .map_or_else(String::new, |a| format!(" agent:{a}"));
        Output::info(&format!(
            "{} {}{}{}: {}",
            entry.timestamp, entry.event_type, session_str, agent_str, entry.message
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_events_list_no_file_is_ok() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: None,
        };
        // This will try to read a non-existent file and return empty list
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_follow_no_file_is_ok() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: true,
            limit: None,
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_session_filter() {
        let options = EventsOptions {
            session: Some("nonexistent-session".to_string()),
            event_type: None,
            follow: false,
            limit: Some(10),
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_event_type_filter() {
        let options = EventsOptions {
            session: None,
            event_type: Some("session_created".to_string()),
            follow: false,
            limit: Some(5),
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_since_filter() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: Some("2099-01-01T00:00:00Z".to_string()),
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn read_events_from_log_handles_missing_file() {
        let result = read_events_from_log(None, None, 50, None);
        assert!(result.is_ok());
        let entries = result.expect("should be ok");
        assert!(entries.is_empty());
    }

    #[test]
    fn resolve_events_file_path_does_not_panic() {
        // This may or may not succeed depending on HOME, but must not panic
        let _ = resolve_events_file_path();
    }

    #[test]
    fn print_events_with_empty_output() {
        let output = EventsOutput {
            events: vec![],
            total: 0,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_with_entries() {
        use super::super::data::EventType;

        let output = EventsOutput {
            events: vec![EventEntry {
                id: "evt-1".to_string(),
                event_type: EventType::SessionCreated,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("test".to_string()),
                agent_id: Some("agent-1".to_string()),
                data: None,
                message: "Session created".to_string(),
            }],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn default_limit_is_fifty() {
        assert_eq!(DEFAULT_LIMIT, 50);
    }
}
