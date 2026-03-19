//! Tag commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, tag};

pub fn list(pattern: Option<&str>, _sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| {
                Error::VcsConflict(format!("Failed to open repo: {}", e), e.to_string())
            })?;

            let tags = tag::list(&repo, pattern)
                .map_err(|e| Error::VcsConflict("list tags".to_string(), e.to_string()))?;

            if tags.is_empty() {
                Output::info("No tags found");
            } else {
                for t in tags {
                    println!("{}", t);
                }
            }
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => Err(Error::VcsConflict(
            "Jujutsu tags not supported".to_string(),
            "Jujutsu".to_string(),
        )),
    }
}

pub fn create(name: &str, message: Option<&str>, _commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| {
                Error::VcsConflict(format!("Failed to open repo: {}", e), e.to_string())
            })?;

            let msg = message.unwrap_or("");
            tag::create(&repo, name, msg, force)
                .map_err(|e| Error::VcsConflict("create tag".to_string(), e.to_string()))?;

            Output::success(&format!("Created tag: {}", name));
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => Err(Error::VcsConflict(
            "Jujutsu tags not supported".to_string(),
            "Jujutsu".to_string(),
        )),
    }
}

pub fn delete(name: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            if remote {
                return Err(Error::VcsConflict(
                    "Remote tag delete not yet implemented".to_string(),
                    "remote".to_string(),
                ));
            }

            let repo = repository::open(&cwd).map_err(|e| {
                Error::VcsConflict(format!("Failed to open repo: {}", e), e.to_string())
            })?;

            tag::delete(&repo, name, false)
                .map_err(|e| Error::VcsConflict("delete tag".to_string(), e.to_string()))?;

            Output::success(&format!("Deleted local tag: {}", name));
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => Err(Error::VcsConflict(
            "Jujutsu tags not supported".to_string(),
            "Jujutsu".to_string(),
        )),
    }
}

pub fn push(tag: Option<&str>, remote: &str, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            if tag.is_none() {
                return Err(Error::VcsConflict(
                    "Push all tags not yet implemented".to_string(),
                    "all tags".to_string(),
                ));
            }

            let repo = repository::open(&cwd).map_err(|e| {
                Error::VcsConflict(format!("Failed to open repo: {}", e), e.to_string())
            })?;

            let t = tag.unwrap();
            tag::push(&repo, remote, t).map_err(|e| Error::VcsPushFailed(e.to_string()))?;
            Output::success(&format!("Pushed tag {} to {}", t, remote));
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => Err(Error::VcsConflict(
            "Jujutsu tags not supported".to_string(),
            "Jujutsu".to_string(),
        )),
    }
}
