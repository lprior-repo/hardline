//! Data types for the completions command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use scp_core::Error;

/// Options for the completions command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct CompletionsOptions {
    /// Which shell to generate completions for.
    pub shell: Shell,
}

/// Supported shells for completion generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl FromStr for Shell {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "ps" | "pwsh" => Ok(Self::PowerShell),
            "elvish" => Ok(Self::Elvish),
            _ => Err(Error::validation_error(format!(
                "Unknown shell: '{s}'. Supported: bash, zsh, fish, powershell, elvish"
            ))),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Zsh => write!(f, "zsh"),
            Self::Fish => write!(f, "fish"),
            Self::PowerShell => write!(f, "powershell"),
            Self::Elvish => write!(f, "elvish"),
        }
    }
}

/// Output of the completions command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionsOutput {
    /// Shell type that was requested.
    pub shell: Shell,
    /// The generated completion script.
    pub script: String,
    /// Installation instructions for this shell.
    pub install_instructions: String,
}

/// Returns all supported shell variants.
pub fn supported_shells() -> Vec<Shell> {
    vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ]
}

/// Returns installation instructions for a given shell (pure function).
pub fn install_instructions(shell: Shell) -> String {
    match shell {
        Shell::Bash => {
            "Add to ~/.bashrc:\n  source <(scp completions bash)".to_string()
        }
        Shell::Zsh => {
            "Add to ~/.zshrc:\n  source <(scp completions zsh)\n\nOr save to ~/.zfunc/_scp".to_string()
        }
        Shell::Fish => {
            "Save to ~/.config/fish/completions/scp.fish:\n  scp completions fish > ~/.config/fish/completions/scp.fish".to_string()
        }
        Shell::PowerShell => {
            "Add to $PROFILE:\n  scp completions powershell | Out-String | Invoke-Expression".to_string()
        }
        Shell::Elvish => {
            "Save to ~/.elvish/lib/scp.elv:\n  scp completions elvish > ~/.elvish/lib/scp.elv".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_from_str_bash() {
        let shell: Shell = "bash".parse().expect("parse bash");
        assert_eq!(shell, Shell::Bash);
    }

    #[test]
    fn shell_from_str_zsh() {
        let shell: Shell = "zsh".parse().expect("parse zsh");
        assert_eq!(shell, Shell::Zsh);
    }

    #[test]
    fn shell_from_str_fish() {
        let shell: Shell = "fish".parse().expect("parse fish");
        assert_eq!(shell, Shell::Fish);
    }

    #[test]
    fn shell_from_str_powershell() {
        let shell: Shell = "powershell".parse().expect("parse powershell");
        assert_eq!(shell, Shell::PowerShell);
    }

    #[test]
    fn shell_from_str_pwsh_alias() {
        let shell: Shell = "pwsh".parse().expect("parse pwsh");
        assert_eq!(shell, Shell::PowerShell);
    }

    #[test]
    fn shell_from_str_ps_alias() {
        let shell: Shell = "ps".parse().expect("parse ps");
        assert_eq!(shell, Shell::PowerShell);
    }

    #[test]
    fn shell_from_str_elvish() {
        let shell: Shell = "elvish".parse().expect("parse elvish");
        assert_eq!(shell, Shell::Elvish);
    }

    #[test]
    fn shell_from_str_case_insensitive() {
        let shell: Shell = "BASH".parse().expect("parse BASH");
        assert_eq!(shell, Shell::Bash);

        let shell: Shell = "ZSH".parse().expect("parse ZSH");
        assert_eq!(shell, Shell::Zsh);
    }

    #[test]
    fn shell_from_str_invalid() {
        let result = "invalid".parse::<Shell>();
        assert!(result.is_err());
    }

    #[test]
    fn shell_display() {
        assert_eq!(format!("{}", Shell::Bash), "bash");
        assert_eq!(format!("{}", Shell::Zsh), "zsh");
        assert_eq!(format!("{}", Shell::Fish), "fish");
        assert_eq!(format!("{}", Shell::PowerShell), "powershell");
        assert_eq!(format!("{}", Shell::Elvish), "elvish");
    }

    #[test]
    fn supported_shells_contains_all_variants() {
        let shells = supported_shells();
        assert_eq!(shells.len(), 5);
        assert!(shells.contains(&Shell::Bash));
        assert!(shells.contains(&Shell::Zsh));
        assert!(shells.contains(&Shell::Fish));
        assert!(shells.contains(&Shell::PowerShell));
        assert!(shells.contains(&Shell::Elvish));
    }

    #[test]
    fn install_instructions_bash() {
        let instructions = install_instructions(Shell::Bash);
        assert!(instructions.contains("bashrc"));
        assert!(instructions.contains("scp completions bash"));
    }

    #[test]
    fn install_instructions_zsh() {
        let instructions = install_instructions(Shell::Zsh);
        assert!(instructions.contains("zshrc"));
        assert!(instructions.contains("scp completions zsh"));
    }

    #[test]
    fn install_instructions_fish() {
        let instructions = install_instructions(Shell::Fish);
        assert!(instructions.contains("fish"));
        assert!(instructions.contains("scp.fish"));
    }

    #[test]
    fn install_instructions_powershell() {
        let instructions = install_instructions(Shell::PowerShell);
        assert!(instructions.contains("PROFILE"));
        assert!(instructions.contains("scp completions powershell"));
    }

    #[test]
    fn install_instructions_elvish() {
        let instructions = install_instructions(Shell::Elvish);
        assert!(instructions.contains("elvish"));
        assert!(instructions.contains("scp.elv"));
    }

    #[test]
    fn completions_output_serialization_roundtrip() {
        let output = CompletionsOutput {
            shell: Shell::Bash,
            script: "test script".to_string(),
            install_instructions: "test instructions".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: CompletionsOutput =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.shell, Shell::Bash);
        assert_eq!(deserialized.script, "test script");
    }

    #[test]
    fn shell_serialization_roundtrip() {
        let json = serde_json::to_string(&Shell::Bash).expect("serialize");
        assert_eq!(json, "\"bash\"");
        let deserialized: Shell = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, Shell::Bash);
    }
}
