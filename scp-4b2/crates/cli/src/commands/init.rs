//! Initialize command

use scp_core::Result;

/// Initialize SCP in current directory
pub fn run(vcs_type: &str) -> Result<()> {
    println!("Initializing Source Control Plane...");

    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::Io(e))?;

    match vcs_type {
        "jj" => {
            // Check if jj is installed
            std::process::Command::new("jj")
                .arg("--version")
                .output()
                .map_err(|e| scp_core::Error::Io(e))?;

            // Check if already initialized
            if cwd.join(".jj").exists() {
                println!("Already initialized with JJ");
                return Ok(());
            }

            // Initialize jj
            let output = std::process::Command::new("jj")
                .args(["init", "--name", "main"])
                .current_dir(&cwd)
                .output()
                .map_err(|e| scp_core::Error::Io(e))?;

            if !output.status.success() {
                return Err(scp_core::Error::Internal(format!(
                    "Failed to init jj: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            println!("✓ Initialized JJ in {:?}", cwd);
            Ok(())
        }
        "git" => {
            // Similar for git
            if cwd.join(".git").exists() {
                println!("Already initialized with Git");
                return Ok(());
            }

            let output = std::process::Command::new("git")
                .args(["init"])
                .current_dir(&cwd)
                .output()
                .map_err(|e| scp_core::Error::Io(e))?;

            if !output.status.success() {
                return Err(scp_core::Error::Internal(format!(
                    "Failed to init git: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            println!("✓ Initialized Git in {:?}", cwd);
            Ok(())
        }
        _ => Err(scp_core::Error::ConfigInvalid(format!(
            "Unknown VCS type: {}",
            vcs_type
        ))),
    }
}
