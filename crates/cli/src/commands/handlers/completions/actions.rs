//! Action functions for the completions command handler (Tier 3).
//!
//! I/O operations that generate and display shell completions.

use scp_core::output::Output;
use scp_core::Result;

use super::data::{install_instructions, CompletionsOptions, CompletionsOutput, Shell};

/// Execute the completions command with the given options.
///
/// Generates a shell completion script and prints it to stdout.
///
/// # Errors
///
/// Returns error if the completion script cannot be generated.
pub fn run_completions(options: &CompletionsOptions) -> Result<()> {
    let script = generate_completions(options.shell);
    let instructions = install_instructions(options.shell);

    Output::info(&script);
    Output::info("");
    Output::info(&format!("Installation ({}):", options.shell));
    Output::info(&instructions);

    Ok(())
}

/// Generate completions and return the output struct (for programmatic use).
///
/// # Errors
///
/// Currently always succeeds, but returns Result for API consistency.
pub fn generate_completions_output(shell: Shell) -> Result<CompletionsOutput> {
    let script = generate_completions(shell);
    let instructions = install_instructions(shell);

    Ok(CompletionsOutput {
        shell,
        script,
        install_instructions: instructions,
    })
}

/// Dispatch to the correct shell completion generator (pure dispatch).
fn generate_completions(shell: Shell) -> String {
    match shell {
        Shell::Bash => generate_bash_completions(),
        Shell::Zsh => generate_zsh_completions(),
        Shell::Fish => generate_fish_completions(),
        Shell::PowerShell => generate_powershell_completions(),
        Shell::Elvish => generate_elvish_completions(),
    }
}

fn generate_bash_completions() -> String {
    r#"# scp bash completion
_scp() {
    local cur prev commands
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    commands="init add list remove focus status sync done undo revert spawn work abort \
              agents ai checkpoint clean config context diff doctor introspect \
              query whereami whoami contract examples validate whatif claim yield events \
              batch completions export import rename pause resume clone integrity \
              recover bookmark schema backup task wait can_i prune"

    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    # Session name completion for commands that take session names
    case "${prev}" in
        focus|remove|status|sync|diff|claim|yield|rename|pause|resume|clone)
            local sessions=$(scp list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null)
            COMPREPLY=( $(compgen -W "${sessions}" -- "${cur}") )
            return 0
            ;;
        --shell)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell elvish" -- "${cur}") )
            return 0
            ;;
    esac

    # Flag completion
    case "${COMP_WORDS[1]}" in
        add|work)
            COMPREPLY=( $(compgen -W "--no-hooks --no-open --json --idempotent --dry-run" -- "${cur}") )
            ;;
        remove)
            COMPREPLY=( $(compgen -W "--force --merge --keep-branch --json --idempotent" -- "${cur}") )
            ;;
        done)
            COMPREPLY=( $(compgen -W "--message --keep-workspace --squash --dry-run --no-bead-update --json" -- "${cur}") )
            ;;
        list)
            COMPREPLY=( $(compgen -W "--all --json --bead --agent" -- "${cur}") )
            ;;
        *)
            COMPREPLY=( $(compgen -W "--json --help" -- "${cur}") )
            ;;
    esac
}

complete -F _scp scp
"#.to_string()
}

fn generate_zsh_completions() -> String {
    r#"#compdef scp

_scp() {
    local line state

    _arguments -C \
        '1: :->command' \
        '*::arg:->args'

    case $state in
        command)
            _values 'scp commands' \
                'init[Initialize scp in a repository]' \
                'add[Create session for manual work]' \
                'list[List all sessions]' \
                'remove[Remove a session]' \
                'focus[Switch to session workspace]' \
                'status[Show detailed session status]' \
                'sync[Sync workspace with main]' \
                'done[Complete work and merge]' \
                'undo[Revert last done operation]' \
                'revert[Revert specific session merge]' \
                'spawn[Create session for automated agent work]' \
                'work[Start working on a task]' \
                'abort[Abandon workspace without merging]' \
                'agents[List and manage agents]' \
                'ai[AI-first entry point]' \
                'checkpoint[Save and restore session state]' \
                'clean[Remove stale sessions]' \
                'config[View or modify configuration]' \
                'context[Show complete environment context]' \
                'diff[Show diff between session and main]' \
                'doctor[Run system health checks]' \
                'introspect[Discover scp capabilities]' \
                'query[Query system state]' \
                'whereami[Quick location query]' \
                'whoami[Agent identity query]' \
                'contract[Show command contracts]' \
                'examples[Show usage examples]' \
                'validate[Pre-validate inputs]' \
                'whatif[Preview what a command would do]' \
                'claim[Claim a session lock]' \
                'yield[Release a session lock]' \
                'events[Show or stream events]' \
                'batch[Execute multiple commands]' \
                'completions[Generate shell completions]' \
                'integrity[Check workspace integrity]' \
                'recover[Recover workspace state]' \
                'bookmark[Manage bookmarks]' \
                'schema[Show schema information]' \
                'backup[Backup workspace]' \
                'task[Manage tasks]' \
                'wait[Wait for condition]' \
                'can_i[Check permissions]' \
                'prune[Prune stale resources]'
            ;;
        args)
            case $line[1] in
                focus|remove|status|sync|diff|claim|yield|rename|pause|resume|clone)
                    _scp_sessions
                    ;;
                *)
                    _files
                    ;;
            esac
            ;;
    esac
}

_scp_sessions() {
    local sessions
    sessions=(${(f)"$(scp list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null)"})
    _describe 'sessions' sessions
}

_scp "$@"
"#
    .to_string()
}

fn generate_fish_completions() -> String {
    r#"# scp fish completion

# Disable file completion by default
complete -c scp -f

# Commands
complete -c scp -n "__fish_use_subcommand" -a init -d "Initialize scp"
complete -c scp -n "__fish_use_subcommand" -a add -d "Create session"
complete -c scp -n "__fish_use_subcommand" -a list -d "List sessions"
complete -c scp -n "__fish_use_subcommand" -a remove -d "Remove session"
complete -c scp -n "__fish_use_subcommand" -a focus -d "Switch to session"
complete -c scp -n "__fish_use_subcommand" -a status -d "Show status"
complete -c scp -n "__fish_use_subcommand" -a sync -d "Sync with main"
complete -c scp -n "__fish_use_subcommand" -a done -d "Complete and merge"
complete -c scp -n "__fish_use_subcommand" -a undo -d "Revert last done"
complete -c scp -n "__fish_use_subcommand" -a revert -d "Revert specific merge"
complete -c scp -n "__fish_use_subcommand" -a spawn -d "Spawn agent"
complete -c scp -n "__fish_use_subcommand" -a work -d "Start working"
complete -c scp -n "__fish_use_subcommand" -a abort -d "Abandon workspace"
complete -c scp -n "__fish_use_subcommand" -a agents -d "Manage agents"
complete -c scp -n "__fish_use_subcommand" -a ai -d "AI entry point"
complete -c scp -n "__fish_use_subcommand" -a checkpoint -d "Manage checkpoints"
complete -c scp -n "__fish_use_subcommand" -a clean -d "Remove stale sessions"
complete -c scp -n "__fish_use_subcommand" -a config -d "Manage config"
complete -c scp -n "__fish_use_subcommand" -a context -d "Show context"
complete -c scp -n "__fish_use_subcommand" -a diff -d "Show diff"
complete -c scp -n "__fish_use_subcommand" -a doctor -d "Health checks"
complete -c scp -n "__fish_use_subcommand" -a introspect -d "Discover capabilities"
complete -c scp -n "__fish_use_subcommand" -a query -d "Query state"
complete -c scp -n "__fish_use_subcommand" -a whereami -d "Location query"
complete -c scp -n "__fish_use_subcommand" -a whoami -d "Identity query"
complete -c scp -n "__fish_use_subcommand" -a contract -d "Show contracts"
complete -c scp -n "__fish_use_subcommand" -a examples -d "Show examples"
complete -c scp -n "__fish_use_subcommand" -a validate -d "Validate inputs"
complete -c scp -n "__fish_use_subcommand" -a whatif -d "Preview command"
complete -c scp -n "__fish_use_subcommand" -a claim -d "Claim lock"
complete -c scp -n "__fish_use_subcommand" -a yield -d "Release lock"
complete -c scp -n "__fish_use_subcommand" -a events -d "Show events"
complete -c scp -n "__fish_use_subcommand" -a batch -d "Batch execute"
complete -c scp -n "__fish_use_subcommand" -a completions -d "Generate completions"
complete -c scp -n "__fish_use_subcommand" -a integrity -d "Check integrity"
complete -c scp -n "__fish_use_subcommand" -a recover -d "Recover state"
complete -c scp -n "__fish_use_subcommand" -a bookmark -d "Manage bookmarks"
complete -c scp -n "__fish_use_subcommand" -a schema -d "Show schema"
complete -c scp -n "__fish_use_subcommand" -a backup -d "Backup workspace"
complete -c scp -n "__fish_use_subcommand" -a task -d "Manage tasks"
complete -c scp -n "__fish_use_subcommand" -a wait -d "Wait for condition"
complete -c scp -n "__fish_use_subcommand" -a can_i -d "Check permissions"
complete -c scp -n "__fish_use_subcommand" -a prune -d "Prune stale resources"

# Session name completion for relevant commands
function __fish_scp_sessions
    scp list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null
end

complete -c scp -n "__fish_seen_subcommand_from focus remove status sync diff claim yield" -a "(__fish_scp_sessions)"

# Global flags
complete -c scp -l json -d "Output as JSON"
complete -c scp -l help -d "Show help"
"#.to_string()
}

fn generate_powershell_completions() -> String {
    r#"# scp PowerShell completion

$script:scpCommands = @(
    'init', 'add', 'list', 'remove', 'focus', 'status', 'sync', 'done',
    'undo', 'revert', 'spawn', 'work', 'abort', 'agents', 'ai', 'checkpoint',
    'clean', 'config', 'context', 'diff', 'doctor', 'introspect',
    'query', 'whereami', 'whoami', 'contract', 'examples', 'validate', 'whatif',
    'claim', 'yield', 'events', 'batch', 'completions', 'integrity', 'recover',
    'bookmark', 'schema', 'backup', 'task', 'wait', 'can_i', 'prune'
)

Register-ArgumentCompleter -CommandName scp -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $words = $commandAst.CommandElements

    if ($words.Count -eq 1) {
        # Complete commands
        $script:scpCommands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
    elseif ($words.Count -ge 2) {
        $command = $words[1].Extent.Text

        # Complete session names for relevant commands
        if ($command -in @('focus', 'remove', 'status', 'sync', 'diff', 'claim', 'yield')) {
            $sessions = scp list --json 2>$null | ConvertFrom-Json | Select-Object -ExpandProperty data | Select-Object -ExpandProperty name
            $sessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        }

        # Complete flags
        @('--json', '--help', '--dry-run') | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
}
"#.to_string()
}

fn generate_elvish_completions() -> String {
    r"# scp elvish completion

set edit:completion:arg-completer[scp] = {|@words|
    var commands = [
        init add list remove focus status sync done undo revert spawn work abort
        agents ai checkpoint clean config context diff doctor introspect
        query whereami whoami contract examples validate whatif claim yield events
        batch completions integrity recover bookmark schema backup task wait can_i prune
    ]

    if (eq (count $words) 1) {
        # Complete commands
        for cmd $commands {
            put $cmd
        }
    } elif (eq (count $words) 2) {
        var cmd = $words[1]
        if (has-value [focus remove status sync diff claim yield] $cmd) {
            # Complete session names
            try {
                var sessions = (scp list --json 2>/dev/null | from-json)[data]
                for sess $sessions {
                    put $sess[name]
                }
            } catch { }
        }
    }
}
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_completions_bash() {
        let options = CompletionsOptions { shell: Shell::Bash };
        assert!(run_completions(&options).is_ok());
    }

    #[test]
    fn run_completions_zsh() {
        let options = CompletionsOptions { shell: Shell::Zsh };
        assert!(run_completions(&options).is_ok());
    }

    #[test]
    fn run_completions_fish() {
        let options = CompletionsOptions { shell: Shell::Fish };
        assert!(run_completions(&options).is_ok());
    }

    #[test]
    fn run_completions_powershell() {
        let options = CompletionsOptions {
            shell: Shell::PowerShell,
        };
        assert!(run_completions(&options).is_ok());
    }

    #[test]
    fn run_completions_elvish() {
        let options = CompletionsOptions {
            shell: Shell::Elvish,
        };
        assert!(run_completions(&options).is_ok());
    }

    #[test]
    fn generate_completions_output_bash() {
        let output = generate_completions_output(Shell::Bash).expect("bash output");
        assert_eq!(output.shell, Shell::Bash);
        assert!(output.script.contains("_scp()"));
        assert!(output.script.contains("complete -F _scp scp"));
        assert!(output.install_instructions.contains("bashrc"));
    }

    #[test]
    fn generate_completions_output_zsh() {
        let output = generate_completions_output(Shell::Zsh).expect("zsh output");
        assert_eq!(output.shell, Shell::Zsh);
        assert!(output.script.contains("#compdef scp"));
        assert!(output.script.contains("_scp()"));
        assert!(output.install_instructions.contains("zshrc"));
    }

    #[test]
    fn generate_completions_output_fish() {
        let output = generate_completions_output(Shell::Fish).expect("fish output");
        assert_eq!(output.shell, Shell::Fish);
        assert!(output.script.contains("complete -c scp"));
        assert!(output.install_instructions.contains("scp.fish"));
    }

    #[test]
    fn generate_completions_output_powershell() {
        let output = generate_completions_output(Shell::PowerShell).expect("powershell output");
        assert_eq!(output.shell, Shell::PowerShell);
        assert!(output.script.contains("Register-ArgumentCompleter"));
        assert!(output.install_instructions.contains("PROFILE"));
    }

    #[test]
    fn generate_completions_output_elvish() {
        let output = generate_completions_output(Shell::Elvish).expect("elvish output");
        assert_eq!(output.shell, Shell::Elvish);
        assert!(output.script.contains("arg-completer[scp]"));
        assert!(output.install_instructions.contains("scp.elv"));
    }

    #[test]
    fn bash_script_contains_all_commands() {
        let output = generate_completions_output(Shell::Bash).expect("bash output");
        assert!(output.script.contains("completions"));
        assert!(output.script.contains("integrity"));
        assert!(output.script.contains("recover"));
    }

    #[test]
    fn zsh_script_contains_all_commands() {
        let output = generate_completions_output(Shell::Zsh).expect("zsh output");
        assert!(output.script.contains("completions"));
        assert!(output.script.contains("integrity"));
        assert!(output.script.contains("recover"));
    }
}
