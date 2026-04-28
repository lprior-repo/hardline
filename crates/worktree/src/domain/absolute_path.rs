use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

/// Value object representing an absolute path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    /// Create a new absolute path with validation
    ///
    /// # Errors
    /// Returns error if path is not absolute or contains parent directory traversal.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, super::WorktreeDomainError> {
        let path = path.as_ref();

        if !path.is_absolute() {
            return Err(super::WorktreeDomainError::InvalidPath(format!(
                "Path is not absolute: {}",
                path.display()
            )));
        }

        // Reject path traversal components
        for component in path.components() {
            if component == std::path::Component::ParentDir {
                return Err(super::WorktreeDomainError::InvalidPath(format!(
                    "Path contains '..' traversal: {}",
                    path.display()
                )));
            }
        }

        Ok(Self(path.to_path_buf()))
    }

    /// Create an absolute path without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the path is absolute and does not contain
    /// parent directory traversal components (`..`).
    ///
    /// # Examples
    ///
    /// ```
    /// use worktree::domain::AbsolutePath;
    ///
    /// // Safe when using a known absolute path
    /// let path = unsafe { AbsolutePath::new_unchecked("/tmp") };
    /// ```
    #[must_use]
    pub unsafe fn new_unchecked<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_path_buf())
    }

    /// Create an absolute path from a string
    pub fn from_string(s: &str) -> Result<Self, super::WorktreeDomainError> {
        Self::new(s)
    }

    /// Get the path as a path buffer
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }

    /// Get the path as a path reference
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Get the path as a string reference
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        self.0.to_string_lossy()
    }

    /// Create a child path relative to this absolute path
    pub fn join<P: AsRef<Path>>(&self, child: P) -> AbsolutePath {
        // Joining two absolute paths always produces an absolute path.
        // Use new_unchecked to avoid expect/unwrap since validation is redundant here.
        // SAFETY: PathBuf::join of absolute path always produces absolute path.
        unsafe { AbsolutePath::new_unchecked(self.0.join(child)) }
    }

    /// Get the parent directory
    pub fn parent(&self) -> Option<AbsolutePath> {
        self.0.parent().and_then(|p| AbsolutePath::new(p).ok())
    }

    /// Get the file or directory name
    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name().and_then(|s| s.to_str())
    }

    /// Check if path exists
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Check if path is a directory
    pub fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    /// Check if path is a file
    pub fn is_file(&self) -> bool {
        self.0.is_file()
    }
}

impl Display for AbsolutePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0.display(), f)
    }
}

impl From<AbsolutePath> for PathBuf {
    fn from(path: AbsolutePath) -> Self {
        path.0
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn absolute_path_new_absolute_path_returns_instance() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        assert_eq!(path.as_str(), "/home/user/project");
    }

    #[test]
    fn absolute_path_new_relative_path_returns_invalid_path_error() {
        let result = AbsolutePath::new("relative/path");
        assert!(result.is_err());
    }

    #[test]
    fn absolute_path_from_string_returns_instance() {
        let path = AbsolutePath::from_string("/tmp/test").unwrap();
        assert_eq!(path.as_str(), "/tmp/test");
    }

    #[test]
    fn absolute_path_into_path_buf_returns_owned_path() {
        let path = AbsolutePath::new("/home/test").unwrap();
        let buf = path.into_path_buf();
        assert_eq!(buf.to_string_lossy(), "/home/test");
    }

    #[test]
    fn absolute_path_join_child_returns_combined_path() {
        let path = AbsolutePath::new("/home/user").unwrap();
        let child = path.join("project/src");
        assert_eq!(child.as_str(), "/home/user/project/src");
    }

    #[test]
    fn absolute_path_parent_returns_parent_directory() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.as_str(), "/home/user");
    }

    #[test]
    fn absolute_path_file_name_returns_filename() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        assert_eq!(path.file_name(), Some("project"));
    }

    #[test]
    fn absolute_path_root_parent_returns_none() {
        let root = AbsolutePath::new("/").unwrap();
        assert!(root.parent().is_none());
    }

    #[test]
    fn absolute_path_is_dir_returns_true_for_directory() {
        let current_dir = AbsolutePath::new(env::current_dir().unwrap()).unwrap();
        assert!(current_dir.is_dir());
    }

    #[test]
    fn absolute_path_display_impl_returns_string() {
        let path = AbsolutePath::new("/home/test").unwrap();
        assert_eq!(format!("{}", path), "/home/test");
    }

    #[test]
    fn absolute_path_new_rejects_parent_dir_traversal() {
        let result = AbsolutePath::new("/tmp/../../../etc/shadow");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{}", err).contains("'..'"));
    }

    #[test]
    fn absolute_path_new_rejects_embedded_parent_dir() {
        let result = AbsolutePath::new("/home/user/../other");
        assert!(result.is_err());
    }
}
