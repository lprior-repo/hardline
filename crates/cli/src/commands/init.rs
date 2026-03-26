//! Initialize command

use scp_core::Result;
use scp_vcs::gix::repository;

/// Initialize SCP in current directory
pub fn run(vcs_type: &str) -> Result<()> {
    println!("Initializing Source Control Plane...");

    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::io_error(e.to_string()))?;

    match vcs_type {
        "jj" => {
            // Check if jj is installed
            std::process::Command::new("jj")
                .arg("--version")
                .output()
                .map_err(|e| scp_core::Error::io_error(e.to_string()))?;

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
                .map_err(|e| scp_core::Error::io_error(e.to_string()))?;

            if !output.status.success() {
                return Err(scp_core::Error::internal(format!(
                    "Failed to init jj: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            println!("✓ Initialized JJ in {:?}", cwd);
            Ok(())
        }
        "git" => {
            // Check if already initialized using gix
            match repository::open(&cwd) {
                Ok(_) => {
                    println!("Already initialized with Git");
                    return Ok(());
                }
                Err(_) => {
                    // Repository doesn't exist, proceed with initialization
                }
            }

            // Initialize git using gix
            repository::init(&cwd).map_err(|e| scp_core::Error::internal(e.to_string()))?;

            println!("✓ Initialized Git in {:?}", cwd);
            Ok(())
        }
        _ => Err(scp_core::Error::config_invalid(format!(
            "Unknown VCS type: {}",
            vcs_type
        ))),
    }
}
