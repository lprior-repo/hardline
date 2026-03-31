# Codebase Map: hl-0g4 Port CLI: config command

## Source (isolate)
- config.rs: ConfigOptions struct, run(), run_with_port(), load_config_for_scope(), show_all_config(), show_config_value(), get_nested_value(), set_config_value(), set_nested_value(), parse_value()
- Dependencies: toml_edit, fs4 (file locking), SchemaEnvelope JSON wrapper
- Features: dot notation, file locking, JSON/TOML output, global/project scopes

## Target (hardline)
- CLI handlers: crates/cli/src/commands/handlers/ (has mod.rs, backup, session, workspace, etc.)
- Config system: crates/core/src/config/ (ConfigManager, Config, ConfigScope, ConfigValue)
- Existing config: crates/cli/src/commands/config.rs (basic get/set/list)
- Object config: crates/cli/src/commands/object_commands/config.rs (subcommands, JSON)
- Config keys: vcs.type, vcs.default_branch, workspace.directory, queue.default, logging.level, editor, remote.push/fetch, workspace.auto_rebase/auto_push

## Key Gaps
- No dot notation support
- No file locking
- No SchemaEnvelope JSON wrapper
- No toml_edit dependency
- No nested value support

## File Paths
- isolate: ~/.config/isolate/config.toml, .isolate/config
- hardline: ~/.config/scp/config.toml, .scp/config
- Env prefix: ISOLATE_ → SCP_
