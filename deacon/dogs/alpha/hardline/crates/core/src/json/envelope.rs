//! Schema envelope wrappers for protocol-compliant JSON responses

use serde::{Deserialize, Serialize};

use crate::fix::Fix;
use crate::hints::NextAction;

use super::hateoas::{HateoasLink, RelatedResources};
use super::meta::ResponseMeta;
use super::schemas::{self, SCHEMA_VERSION};

/// Generic schema envelope for protocol-compliant JSON responses
///
/// Wraps response data with schema metadata (`$schema`, `_schema_version`) for AI-first CLI design.
/// All JSON outputs should be wrapped with this envelope to conform to `ResponseEnvelope` pattern.
///
/// Includes HATEOAS-style navigation with `_links`, `_related`, and `_meta` blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelope<T> {
    /// JSON Schema reference (e.g., `scp://status-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("single" for objects, "array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Response data (flattened into envelope at JSON level)
    #[serde(flatten)]
    pub data: T,
    /// Suggested next actions for AI agents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub next: Vec<NextAction>,
    /// Available fixes for errors (empty for success responses)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<Fix>,
    /// HATEOAS-style navigation links
    #[serde(rename = "_links", skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<HateoasLink>,
    /// Related resources for cross-referencing
    #[serde(rename = "_related", skip_serializing_if = "Option::is_none")]
    pub related: Option<RelatedResources>,
    /// Response metadata for debugging and tracing
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SchemaEnvelope<T> {
    /// Create a new schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "status-response") Should use a constant from
    ///   `schemas` module for conformance
    /// * `schema_type` - Response shape ("single" or "array")
    /// * `data` - The response data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// use scp_core::json::schemas;
    /// let envelope = SchemaEnvelope::new(schemas::STATUS_RESPONSE, "single", data);
    /// ```
    pub fn new(schema_name: &str, schema_type: &str, data: T) -> Self {
        Self {
            schema: schemas::uri(schema_name),
            schema_version: SCHEMA_VERSION.to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Create a schema envelope with next actions
    pub fn with_next(schema_name: &str, schema_type: &str, data: T, next: Vec<NextAction>) -> Self {
        Self {
            schema: format!("scp://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next,
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links to envelope
    #[must_use]
    pub fn with_links(self, links: Vec<HateoasLink>) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next: self.next,
            fixes: self.fixes,
            links,
            related: self.related,
            meta: self.meta,
        }
    }

    /// Add a single link
    #[must_use]
    pub fn add_link(self, link: HateoasLink) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next: self.next,
            fixes: self.fixes,
            links: {
                let mut new_links = self.links;
                new_links.push(link);
                new_links
            },
            related: self.related,
            meta: self.meta,
        }
    }

    /// Add related resources
    #[must_use]
    pub fn with_related(self, related: RelatedResources) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next: self.next,
            fixes: self.fixes,
            links: self.links,
            related: if related.is_empty() {
                None
            } else {
                Some(related)
            },
            meta: self.meta,
        }
    }

    /// Add response metadata
    #[must_use]
    pub fn with_meta(self, meta: ResponseMeta) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next: self.next,
            fixes: self.fixes,
            links: self.links,
            related: self.related,
            meta: Some(meta),
        }
    }

    /// Add fixes to envelope
    #[must_use]
    pub fn with_fixes(self, fixes: Vec<Fix>) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next: self.next,
            fixes,
            links: self.links,
            related: self.related,
            meta: self.meta,
        }
    }

    /// Mark as failed response
    #[must_use]
    pub fn as_error(self) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: false,
            data: self.data,
            next: self.next,
            fixes: self.fixes,
            links: self.links,
            related: self.related,
            meta: self.meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hints::NextAction;

    /// Helper struct for tests (serde flatten requires a struct/map, not a primitive)
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        text: String,
    }

    fn make_test_data(text: &str) -> TestData {
        TestData { text: text.to_string() }
    }

    // ── SchemaEnvelope::new ──────────────────────────────────────────────────

    #[test]
    fn test_envelope_new_basics() {
        let env = SchemaEnvelope::new("test-response", "single", make_test_data("hello"));
        assert!(env.success);
        assert_eq!(env.schema_type, "single");
        assert!(env.next.is_empty());
        assert!(env.fixes.is_empty());
        assert!(env.links.is_empty());
        assert!(env.related.is_none());
        assert!(env.meta.is_none());
        assert_eq!(env.schema_version, "1.0");
    }

    #[test]
    fn test_envelope_new_schema_uri() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("status-response", "single", make_test_data("data"));
        assert!(env.schema.contains("status-response"));
        assert!(env.schema.starts_with("scp://"));
    }

    #[test]
    fn test_envelope_new_data_flattened() {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct Data {
            name: String,
            value: i32,
        }
        let env = SchemaEnvelope::new("test", "single", Data {
            name: "test".to_string(),
            value: 42,
        });
        let json_val = serde_json::to_value(&env).expect("serialize ok");
        assert_eq!(json_val["name"], "test");
        assert_eq!(json_val["value"], 42);
    }

    // ── SchemaEnvelope::with_next ────────────────────────────────────────────

    #[test]
    fn test_envelope_with_next() {
        let next = vec![NextAction {
            action: "run tests".to_string(),
            commands: vec!["cargo test".to_string()],
            risk: Default::default(),
            description: None,
        }];
        let env = SchemaEnvelope::with_next("test", "single", make_test_data("data"), next.clone());
        assert_eq!(env.next.len(), 1);
        assert_eq!(env.next[0].action, "run tests");
    }

    // ── SchemaEnvelope::with_links ───────────────────────────────────────────

    #[test]
    fn test_envelope_with_links() {
        let link = HateoasLink::self_link("status");
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_links(vec![link]);
        assert_eq!(env.links.len(), 1);
        assert_eq!(env.links[0].rel, "self");
    }

    #[test]
    fn test_envelope_with_empty_links_keeps_empty() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_links(vec![]);
        assert!(env.links.is_empty());
    }

    // ── SchemaEnvelope::add_link ─────────────────────────────────────────────

    #[test]
    fn test_envelope_add_link() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .add_link(HateoasLink::self_link("self"))
                .add_link(HateoasLink::action("next", "do-thing", "Do Thing"));
        assert_eq!(env.links.len(), 2);
    }

    // ── SchemaEnvelope::with_related ─────────────────────────────────────────

    #[test]
    fn test_envelope_with_related_nonempty() {
        let related = RelatedResources {
            sessions: vec!["session-1".to_string()],
            beads: vec![],
            workspaces: vec![],
            commits: vec![],
            parent: None,
            children: vec![],
        };
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_related(related);
        assert!(env.related.is_some());
        assert_eq!(env.related.as_ref().expect("related").sessions.len(), 1);
    }

    #[test]
    fn test_envelope_with_related_empty_becomes_none() {
        let related = RelatedResources::default();
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_related(related);
        assert!(env.related.is_none());
    }

    // ── SchemaEnvelope::with_meta ────────────────────────────────────────────

    #[test]
    fn test_envelope_with_meta() {
        let meta = ResponseMeta::new("status").with_duration(42);
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_meta(meta);
        assert!(env.meta.is_some());
        assert_eq!(env.meta.as_ref().expect("meta").duration_ms, Some(42));
    }

    // ── SchemaEnvelope::with_fixes ───────────────────────────────────────────

    #[test]
    fn test_envelope_with_fixes() {
        use crate::fix::Fix;
        let fixes = vec![Fix {
            description: "Apply fix".to_string(),
            commands: vec!["fix it".to_string()],
            automatic: true,
            impact: crate::fix::FixImpact::Safe,
            rationale: None,
        }];
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .with_fixes(fixes);
        assert_eq!(env.fixes.len(), 1);
    }

    // ── SchemaEnvelope::as_error ─────────────────────────────────────────────

    #[test]
    fn test_envelope_as_error() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"))
                .as_error();
        assert!(!env.success);
    }

    // ── SchemaEnvelope serde roundtrip ───────────────────────────────────────

    #[test]
    fn test_envelope_serde_roundtrip() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("data"));
        let json = serde_json::to_string(&env).expect("serialize ok");
        let deserialized: SchemaEnvelope<TestData> =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(env.success, deserialized.success);
        assert_eq!(env.data, deserialized.data);
        assert_eq!(env.schema_type, deserialized.schema_type);
    }

    #[test]
    fn test_envelope_serde_skip_empty_next_and_fixes() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"));
        let json_val = serde_json::to_value(&env).expect("serialize ok");
        assert!(!json_val.as_object().expect("obj").contains_key("next"));
        assert!(!json_val.as_object().expect("obj").contains_key("fixes"));
        assert!(!json_val.as_object().expect("obj").contains_key("_links"));
        assert!(!json_val.as_object().expect("obj").contains_key("_related"));
        assert!(!json_val.as_object().expect("obj").contains_key("_meta"));
    }

    // ── Debug / Clone ────────────────────────────────────────────────────────

    #[test]
    fn test_envelope_debug() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("x"));
        let debug = format!("{env:?}");
        assert!(debug.contains("SchemaEnvelope"));
    }

    #[test]
    fn test_envelope_clone() {
        let env: SchemaEnvelope<TestData> =
            SchemaEnvelope::new("test", "single", make_test_data("data"));
        let cloned = env.clone();
        assert_eq!(env.data, cloned.data);
    }
}
