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

#[cfg(test)]
mod tests {
    use super::*;

    // ── HateoasLink construction ─────────────────────────────────────────────

    #[test]
    fn test_self_link() {
        let link = HateoasLink::self_link("status");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "status");
        assert_eq!(link.method.as_deref(), Some("GET"));
        assert!(link.title.is_none());
    }

    #[test]
    fn test_related_link() {
        let link = HateoasLink::related("parent", "session show");
        assert_eq!(link.rel, "parent");
        assert_eq!(link.href, "session show");
        assert_eq!(link.method.as_deref(), Some("GET"));
    }

    #[test]
    fn test_action_link() {
        let link = HateoasLink::action("create", "session new", "Create Session");
        assert_eq!(link.rel, "create");
        assert_eq!(link.href, "session new");
        assert_eq!(link.method.as_deref(), Some("POST"));
        assert_eq!(link.title.as_deref(), Some("Create Session"));
    }

    #[test]
    fn test_with_title() {
        let link = HateoasLink::self_link("status").with_title("Get Status");
        assert_eq!(link.title.as_deref(), Some("Get Status"));
    }

    #[test]
    fn test_with_title_overwrites_none() {
        let link = HateoasLink::action("run", "exec", "").with_title("Execute");
        assert_eq!(link.title.as_deref(), Some("Execute"));
    }

    // ── HateoasLink serde ────────────────────────────────────────────────────

    #[test]
    fn test_hateoas_link_serde_roundtrip() {
        let link = HateoasLink::action("delete", "rm thing", "Delete Thing");
        let json = serde_json::to_string(&link).expect("serialize ok");
        let deserialized: HateoasLink =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(link, deserialized);
    }

    #[test]
    fn test_hateoas_link_serde_skips_none_fields() {
        let link = HateoasLink::self_link("status");
        let json_val = serde_json::to_value(&link).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(obj.contains_key("method"));
        assert!(!obj.contains_key("title"));
    }

    #[test]
    fn test_hateoas_link_serde_includes_title() {
        let link = HateoasLink::action("x", "y", "Z");
        let json_val = serde_json::to_value(&link).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(obj.contains_key("title"));
    }

    #[test]
    fn test_hateoas_link_equality() {
        let a = HateoasLink::self_link("test");
        let b = HateoasLink::self_link("test");
        assert_eq!(a, b);
    }

    #[test]
    fn test_hateoas_link_inequality() {
        let a = HateoasLink::self_link("a");
        let b = HateoasLink::self_link("b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_hateoas_link_clone() {
        let link = HateoasLink::action("x", "y", "Z");
        let cloned = link.clone();
        assert_eq!(link, cloned);
    }

    #[test]
    fn test_hateoas_link_debug() {
        let link = HateoasLink::self_link("status");
        let debug = format!("{link:?}");
        assert!(debug.contains("self"));
    }

    // ── RelatedResources ─────────────────────────────────────────────────────

    #[test]
    fn test_related_resources_default_is_empty() {
        let r = RelatedResources::default();
        assert!(r.is_empty());
    }

    #[test]
    fn test_related_resources_with_sessions() {
        let r = RelatedResources {
            sessions: vec!["s1".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_with_beads() {
        let r = RelatedResources {
            beads: vec!["hl-001".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_with_workspaces() {
        let r = RelatedResources {
            workspaces: vec!["ws-1".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_with_commits() {
        let r = RelatedResources {
            commits: vec!["abc123".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_with_parent() {
        let r = RelatedResources {
            parent: Some("parent-id".to_string()),
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_with_children() {
        let r = RelatedResources {
            children: vec!["child-1".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn test_related_resources_all_fields_populated() {
        let r = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["b1".to_string()],
            workspaces: vec!["w1".to_string()],
            commits: vec!["c1".to_string()],
            parent: Some("p1".to_string()),
            children: vec!["ch1".to_string()],
        };
        assert!(!r.is_empty());
        assert_eq!(r.sessions.len(), 1);
        assert_eq!(r.parent.as_deref(), Some("p1"));
    }

    // ── RelatedResources serde ───────────────────────────────────────────────

    #[test]
    fn test_related_resources_serde_roundtrip() {
        let r = RelatedResources {
            sessions: vec!["s1".to_string(), "s2".to_string()],
            beads: vec![],
            workspaces: vec![],
            commits: vec![],
            parent: Some("root".to_string()),
            children: vec![],
        };
        let json = serde_json::to_string(&r).expect("serialize ok");
        let deserialized: RelatedResources =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(r, deserialized);
    }

    #[test]
    fn test_related_resources_serde_skips_empty_vecs() {
        let r = RelatedResources::default();
        let json_val = serde_json::to_value(&r).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("sessions"));
        assert!(!obj.contains_key("beads"));
        assert!(!obj.contains_key("workspaces"));
        assert!(!obj.contains_key("commits"));
        assert!(!obj.contains_key("children"));
        assert!(!obj.contains_key("parent"));
    }

    #[test]
    fn test_related_resources_serde_includes_parent() {
        let r = RelatedResources {
            parent: Some("parent".to_string()),
            ..Default::default()
        };
        let json_val = serde_json::to_value(&r).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(obj.contains_key("parent"));
    }

    #[test]
    fn test_related_resources_equality() {
        let a = RelatedResources {
            sessions: vec!["x".to_string()],
            ..Default::default()
        };
        let b = RelatedResources {
            sessions: vec!["x".to_string()],
            ..Default::default()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_related_resources_clone() {
        let r = RelatedResources {
            beads: vec!["hl-001".to_string()],
            ..Default::default()
        };
        let cloned = r.clone();
        assert_eq!(r, cloned);
    }
}
