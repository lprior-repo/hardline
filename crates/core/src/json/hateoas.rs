//! HATEOAS link structures for API discoverability

use serde::{Deserialize, Serialize};

/// HATEOAS-style link for API discoverability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HateoasLink {
    /// Link relation type (e.g., "self", "next", "parent")
    pub rel: String,
    /// The command or action to take
    pub href: String,
    /// HTTP-like method hint ("GET" for read, "POST" for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl HateoasLink {
    /// Create a self-reference link
    #[must_use]
    pub fn self_link(command: impl Into<String>) -> Self {
        Self {
            rel: "self".to_string(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create a related resource link
    #[must_use]
    pub fn related(rel: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create an action link (mutating operation)
    #[must_use]
    pub fn action(
        rel: impl Into<String>,
        command: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("POST".to_string()),
            title: Some(title.into()),
        }
    }

    /// Add a title to this link
    #[must_use]
    pub fn with_title(self, title: impl Into<String>) -> Self {
        Self {
            rel: self.rel,
            href: self.href,
            method: self.method,
            title: Some(title.into()),
        }
    }
}

/// Related resource information for cross-referencing
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedResources {
    /// Related sessions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sessions: Vec<String>,
    /// Related beads/issues
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub beads: Vec<String>,
    /// Related workspaces
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub workspaces: Vec<String>,
    /// Related commits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub commits: Vec<String>,
    /// Parent resource (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Child resources
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<String>,
}

impl RelatedResources {
    /// Check if there are any related resources
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sessions.is_empty()
            && self.beads.is_empty()
            && self.workspaces.is_empty()
            && self.commits.is_empty()
            && self.parent.is_none()
            && self.children.is_empty()
    }
}
