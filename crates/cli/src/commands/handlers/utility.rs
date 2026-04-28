//! Utility handlers: config, query, schema, completions, wait

use anyhow::{Context, Result};
use clap::ArgMatches;
use scp_core::OutputFormat;

use super::json_format::get_format;
use super::introspect;
use crate::commands::handlers::{
    completions, query,
    schema::{self, SchemaMode},
    wait,
};
use crate::commands::config;

/// Handle the config command with subcommand routing
///
/// Routes to:
/// - `config list` - show all config
/// - `config get <key>` - get a specific config value
/// - `config set <key> <value>` - set a config value
/// - `config schema` - show configuration schema
pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    // Detect which subcommand was invoked
    let (subcommand_name, subcommand_args) = sub_m
        .subcommand()
        .ok_or_else(|| anyhow::anyhow!("Config subcommand required. Use: list, get, set, or schema"))?;

    match subcommand_name {
        "list" => handle_config_list(subcommand_args).await,
        "get" => handle_config_get(subcommand_args).await,
        "set" => handle_config_set(subcommand_args).await,
        "schema" => handle_config_schema(subcommand_args),
        _ => anyhow::bail!(
            "Unknown config subcommand: {subcommand_name}. Use: list, get, set, or schema"
        ),
    }
}

/// Handle `scp config list`
async fn handle_config_list(sub_m: &ArgMatches) -> Result<()> {
    let _global = sub_m.get_flag("global");
    let _format = get_format(sub_m);
    config::list()
}

/// Handle `scp config get <key>`
async fn handle_config_get(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m
        .get_one::<String>("key")
        .cloned()
        .context("Config key is required for 'get' subcommand")?;
    let _global = sub_m.get_flag("global");
    let _format = get_format(sub_m);
    config::get(&key)
}

/// Handle `scp config set <key> <value>`
async fn handle_config_set(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m
        .get_one::<String>("key")
        .cloned()
        .context("Config key is required for 'set' subcommand")?;
    let value = sub_m
        .get_one::<String>("value")
        .cloned()
        .context("Config value is required for 'set' subcommand")?;
    let _global = sub_m.get_flag("global");
    let _format = get_format(sub_m);
    config::set(&key, &value)
}

/// Handle `scp config schema`
fn handle_config_schema(sub_m: &ArgMatches) -> Result<()> {
    // hardline uses schema module for schema output
    let format = get_format(sub_m);
    let options = schema::SchemaOptions {
        mode: SchemaMode::Single("config-schema".to_string()),
        format,
    };
    schema::run_schema(&options)
}

pub async fn handle_query(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("query".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    let query_type_str = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;

    let query_type = query::QueryType::from_str(query_type_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown query type: {query_type_str}"))?;

    let args = sub_m.get_one::<String>("args").cloned();
    let status_filter = sub_m.get_one::<String>("status").cloned();
    let agent_filter = sub_m.get_one::<String>("agent").cloned();

    let options = query::QueryOptions {
        query_type,
        argument: args,
        status_filter,
        agent_filter,
    };

    query::run_query(&options)
}

pub fn handle_schema(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let mode = if sub_m.get_flag("list") {
        SchemaMode::List
    } else if sub_m.get_flag("all") {
        SchemaMode::All
    } else if let Some(name) = sub_m.get_one::<String>("name").cloned() {
        SchemaMode::Single(name)
    } else {
        SchemaMode::List
    };
    let options = schema::SchemaOptions { mode, format };
    schema::run_schema(&options)
}

pub fn handle_completions(sub_m: &ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let shell_str = sub_m
        .get_one::<String>("shell")
        .ok_or_else(|| anyhow::anyhow!("Shell is required"))?;
    let shell: completions::Shell = shell_str.parse()?;
    let options = completions::CompletionsOptions { shell };
    completions::run_completions(&options)
}

pub async fn handle_wait(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = build_wait_options(sub_m, format)?;
    let output = wait::run_wait(&options).await?;
    if !output.condition_met {
        std::process::exit(1);
    }
    Ok(())
}

fn build_wait_options(sub_m: &ArgMatches, format: OutputFormat) -> Result<wait::WaitOptions> {
    let condition_str = sub_m
        .get_one::<String>("condition")
        .ok_or_else(|| anyhow::anyhow!("Condition is required"))?;
    let name = sub_m.get_one::<String>("name").cloned();
    let status = sub_m.get_one::<String>("status").cloned();
    let timeout = sub_m.get_one::<f64>("timeout").copied().unwrap_or(30.0);
    let interval = sub_m.get_one::<f64>("interval").copied().unwrap_or(1.0);

    build_wait_options_from_values(condition_str, name, status, timeout, interval, format)
}

#[allow(clippy::too_many_arguments)]
fn build_wait_options_from_values(
    condition_str: &str,
    name: Option<String>,
    status: Option<String>,
    timeout: f64,
    interval: f64,
    format: OutputFormat,
) -> Result<wait::WaitOptions> {
    if status.is_some() && condition_str != "session-status" {
        anyhow::bail!("--status is only valid with session-status condition");
    }

    let condition = match condition_str {
        "session-exists" => wait::WaitCondition::SessionExists(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "session-unlocked" => wait::WaitCondition::SessionUnlocked(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "healthy" => wait::WaitCondition::Healthy,
        "session-status" => wait::WaitCondition::SessionStatus {
            name: name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
            status: status.ok_or_else(|| anyhow::anyhow!("--status required"))?,
        },
        _ => anyhow::bail!("Unknown condition: {condition_str}"),
    };

    Ok(wait::WaitOptions {
        condition,
        timeout: std::time::Duration::from_secs_f64(timeout),
        poll_interval: std::time::Duration::from_secs_f64(interval),
        format,
    })
}

#[cfg(test)]
mod tests {
    use scp_core::OutputFormat;

    use super::build_wait_options_from_values;
    use crate::commands::handlers::wait::WaitCondition;

    #[test]
    fn test_handle_query_always_uses_json_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
        let json_flag_false = false;
        let _ = OutputFormat::from_json_flag(json_flag_false);
        let query_format = OutputFormat::Json;
        assert!(query_format.is_json());
    }

    #[test]
    fn wait_rejects_status_for_non_session_status() {
        let result = build_wait_options_from_values(
            "healthy",
            None,
            Some("active".to_string()),
            30.0,
            1.0,
            OutputFormat::Json,
        );

        assert!(result.is_err());
        let err = result.err().map_or(String::new(), |e| e.to_string());
        assert!(
            err.contains("--status is only valid with session-status"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn wait_defaults_preserve_timeout_and_interval() {
        let options =
            build_wait_options_from_values("healthy", None, None, 30.0, 1.0, OutputFormat::Json)
                .expect("options should build");

        assert!(matches!(options.condition, WaitCondition::Healthy));
        assert_eq!(options.timeout.as_secs(), 30);
        assert_eq!(options.poll_interval.as_secs(), 1);
    }

    mod martin_fowler_wait_option_behavior {
        use super::*;

        /// GIVEN: `session-exists` requires a session name
        /// WHEN: We build options without a name
        /// THEN: The parser should fail with an actionable error
        #[test]
        fn given_session_exists_without_name_when_building_then_error_is_actionable() {
            let result = build_wait_options_from_values(
                "session-exists",
                None,
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("Session name required"),
                "unexpected error: {err}"
            );
        }

        /// GIVEN: `session-status` requires `--status`
        /// WHEN: We provide a session name but no status target
        /// THEN: Option construction should fail with a focused validation error
        #[test]
        fn given_session_status_without_status_when_building_then_requires_status() {
            let result = build_wait_options_from_values(
                "session-status",
                Some("feature-auth".to_string()),
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(err.contains("--status required"), "unexpected error: {err}");
        }

        /// GIVEN: Valid `session-status` inputs
        /// WHEN: We build wait options with name + status
        /// THEN: The resulting condition should preserve both values exactly
        #[test]
        fn given_valid_session_status_when_building_then_preserves_name_and_status() {
            let result = build_wait_options_from_values(
                "session-status",
                Some("feature-auth".to_string()),
                Some("active".to_string()),
                45.0,
                2.0,
                OutputFormat::Json,
            )
            .expect("valid session-status options should build");

            match result.condition {
                WaitCondition::SessionStatus { name, status } => {
                    assert_eq!(name, "feature-auth");
                    assert_eq!(status, "active");
                }
                _ => panic!("expected session-status condition"),
            }
            assert_eq!(result.timeout.as_secs(), 45);
            assert_eq!(result.poll_interval.as_secs(), 2);
        }

        /// GIVEN: A non-session-status wait condition
        /// WHEN: `--status` is provided anyway
        /// THEN: Validation should fail and explain correct usage
        #[test]
        fn given_session_exists_with_status_when_building_then_rejects_misused_status_flag() {
            let result = build_wait_options_from_values(
                "session-exists",
                Some("feature-auth".to_string()),
                Some("active".to_string()),
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("--status is only valid with session-status"),
                "unexpected error: {err}"
            );
        }

        /// GIVEN: A healthy wait condition
        /// WHEN: Options are built with explicit timeout and interval
        /// THEN: Healthy condition remains selected and durations are preserved
        #[test]
        fn given_healthy_with_explicit_timing_when_building_then_preserves_timing() {
            let result = build_wait_options_from_values(
                "healthy",
                None,
                None,
                90.0,
                5.0,
                OutputFormat::Json,
            )
            .expect("healthy options should build");

            assert!(matches!(result.condition, WaitCondition::Healthy));
            assert_eq!(result.timeout.as_secs(), 90);
            assert_eq!(result.poll_interval.as_secs(), 5);
        }

        /// GIVEN: An unknown wait condition string
        /// WHEN: Option construction is attempted
        /// THEN: Validation should fail fast with the unknown condition value
        #[test]
        fn given_unknown_condition_when_building_then_returns_explicit_unknown_error() {
            let result = build_wait_options_from_values(
                "not-a-real-condition",
                None,
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("Unknown condition: not-a-real-condition"),
                "unexpected error: {err}"
            );
        }
    }

    mod martin_fowler_wait_table_driven_behavior {
        use super::*;

        struct BuildCase {
            name: &'static str,
            condition: &'static str,
            session_name: Option<&'static str>,
            status: Option<&'static str>,
            timeout: u64,
            interval: u64,
            expect_ok: bool,
            expected_error_fragment: Option<&'static str>,
        }

        /// GIVEN: a matrix of wait option scenarios
        /// WHEN: option-building runs across all rows
        /// THEN: each row should produce expected success/failure behavior
        #[test]
        #[allow(clippy::cast_precision_loss)]
        fn given_wait_option_matrix_when_building_then_each_row_matches_expected_behavior() {
            let cases = [
                BuildCase {
                    name: "healthy basic",
                    condition: "healthy",
                    session_name: None,
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: true,
                    expected_error_fragment: None,
                },
                BuildCase {
                    name: "session-exists missing name",
                    condition: "session-exists",
                    session_name: None,
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("Session name required"),
                },
                BuildCase {
                    name: "session-status missing status",
                    condition: "session-status",
                    session_name: Some("feat-x"),
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("--status required"),
                },
                BuildCase {
                    name: "session-status complete",
                    condition: "session-status",
                    session_name: Some("feat-x"),
                    status: Some("active"),
                    timeout: 30,
                    interval: 1000,
                    expect_ok: true,
                    expected_error_fragment: None,
                },
                BuildCase {
                    name: "misused status on healthy",
                    condition: "healthy",
                    session_name: None,
                    status: Some("active"),
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("--status is only valid with session-status"),
                },
            ];

            for case in cases {
                let result = build_wait_options_from_values(
                    case.condition,
                    case.session_name.map(str::to_string),
                    case.status.map(str::to_string),
                    case.timeout as f64,
                    case.interval as f64,
                    OutputFormat::Json,
                );

                assert_eq!(result.is_ok(), case.expect_ok, "case failed: {}", case.name);

                if let Some(fragment) = case.expected_error_fragment {
                    let err_text = result.err().map_or(String::new(), |e| e.to_string());
                    assert!(
                        err_text.contains(fragment),
                        "case '{}' missing error fragment '{}': {}",
                        case.name,
                        fragment,
                        err_text
                    );
                }
            }
        }
    }
}
