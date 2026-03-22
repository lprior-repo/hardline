//! Pure validation functions for AbsolutePath.

use std::path::Path;

use super::path_errors::{PathValidationError, ShellMetacharacterError};

const SHELL_METACHARACTERS: &[(char, fn(&str, usize) -> ShellMetacharacterError)] = &[
    ('$', |_, pos| ShellMetacharacterError::ContainsDollar {
        position: pos,
    }),
    ('`', |_, pos| ShellMetacharacterError::ContainsBacktick {
        position: pos,
    }),
    (';', |_, pos| ShellMetacharacterError::ContainsSemicolon {
        position: pos,
    }),
    ('|', |_, pos| ShellMetacharacterError::ContainsPipe {
        position: pos,
    }),
    ('&', |_, pos| ShellMetacharacterError::ContainsAmpersand {
        position: pos,
    }),
];

/// Find the first shell metacharacter in the path string, if any.
/// Returns the error for the first metacharacter found (left-to-right).
pub fn find_first_metacharacter(path: &str) -> Option<ShellMetacharacterError> {
    SHELL_METACHARACTERS
        .iter()
        .filter_map(|&(c, make_err)| {
            path.match_indices(c)
                .next()
                .map(|(pos, _)| make_err(path, pos))
        })
        .min_by_key(|err| match err {
            ShellMetacharacterError::ContainsDollar { position } => *position,
            ShellMetacharacterError::ContainsBacktick { position } => *position,
            ShellMetacharacterError::ContainsSemicolon { position } => *position,
            ShellMetacharacterError::ContainsPipe { position } => *position,
            ShellMetacharacterError::ContainsAmpersand { position } => *position,
        })
}

/// Check if the path is valid UTF-8, returning InvalidUtf8 error if not.
pub fn validate_utf8(path: &Path) -> Result<(), PathValidationError> {
    match path.to_str() {
        Some(_) => Ok(()),
        None => {
            let invalid_bytes = path.as_os_str().as_encoded_bytes().to_vec();
            Err(PathValidationError::InvalidUtf8 { invalid_bytes })
        }
    }
}

/// Check if the path is absolute using Path::is_absolute().
pub fn validate_is_absolute(path: &Path) -> Result<(), PathValidationError> {
    if path.is_absolute() {
        Ok(())
    } else {
        let input = path.to_string_lossy().into_owned();
        Err(PathValidationError::NotAbsolute { input })
    }
}

/// Check if path contains any shell metacharacters.
pub fn validate_no_metacharacters(path: &str) -> Result<(), ShellMetacharacterError> {
    find_first_metacharacter(path).map_or(Ok(()), Err)
}
