//! Schema envelope for array responses

use serde::{Deserialize, Serialize};

use crate::fix::Fix;
use crate::hints::NextAction;

use super::hateoas::{HateoasLink, RelatedResources};
use super::meta::ResponseMeta;

/// Schema envelope for array responses
///
/// Unlike `SchemaEnvelope` which uses flatten for single objects,
/// `SchemaEnvelopeArray` explicitly wraps array data because serde flatten
/// cannot serialize sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelopeArray<T> {
    /// JSON Schema reference (e.g., `scp://list-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Array data (cannot be flattened, so stored as explicit field)
    pub data: Vec<T>,
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

impl<T> SchemaEnvelopeArray<T> {
    /// Create a new array schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "list-response")
    /// * `data` - The array data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// let envelope = SchemaEnvelopeArray::new("list-response", items);
    /// ```
    #[must_use]
    pub fn new(schema_name: &str, data: Vec<T>) -> Self {
        Self {
            schema: format!("scp://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: "array".to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links
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

    /// Add next actions
    #[must_use]
    pub fn with_next(self, next: Vec<NextAction>) -> Self {
        Self {
            schema: self.schema,
            schema_version: self.schema_version,
            schema_type: self.schema_type,
            success: self.success,
            data: self.data,
            next,
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

    // ── SchemaEnvelopeArray::new ─────────────────────────────────────────────

    #[test]
    fn test_array_envelope_new_basics() {
        let env = SchemaEnvelopeArray::new("list-response", vec!["a", "b"]);
        assert!(env.success);
        assert_eq!(env.schema_type, "array");
        assert_eq!(env.data.len(), 2);
        assert!(env.next.is_empty());
        assert!(env.fixes.is_empty());
        assert!(env.links.is_empty());
        assert!(env.related.is_none());
        assert!(env.meta.is_none());
    }

    #[test]
    fn test_array_envelope_new_empty() {
        let env: SchemaEnvelopeArray<String> =
            SchemaEnvelopeArray::new("list-response", vec![]);
        assert!(env.data.is_empty());
    }

    #[test]
    fn test_array_envelope_new_schema_uri() {
        let env: SchemaEnvelopeArray<i32> =
            SchemaEnvelopeArray::new("bead-list", vec![1, 2, 3]);
        assert!(env.schema.contains("bead-list"));
        assert!(env.schema.starts_with("scp://"));
        assert_eq!(env.schema_version, "1.0");
    }

    // ── with_links ───────────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_with_links() {
        let link = HateoasLink::self_link("list");
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![])
            .with_links(vec![link]);
        assert_eq!(env.links.len(), 1);
    }

    // ── with_related ─────────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_with_related_nonempty() {
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec![],
            workspaces: vec![],
            commits: vec![],
            parent: None,
            children: vec![],
        };
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![])
            .with_related(related);
        assert!(env.related.is_some());
    }

    #[test]
    fn test_array_envelope_with_related_empty_becomes_none() {
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![])
            .with_related(RelatedResources::default());
        assert!(env.related.is_none());
    }

    // ── with_meta ────────────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_with_meta() {
        let meta = ResponseMeta::new("list").with_duration(100);
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![])
            .with_meta(meta);
        assert!(env.meta.is_some());
        assert_eq!(env.meta.as_ref().expect("meta").duration_ms, Some(100));
    }

    // ── with_next ────────────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_with_next() {
        let next = vec![NextAction {
            action: "next step".to_string(),
            commands: vec!["do next".to_string()],
            risk: Default::default(),
            description: None,
        }];
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![])
            .with_next(next);
        assert_eq!(env.next.len(), 1);
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_serde_roundtrip() {
        let env = SchemaEnvelopeArray::new("list", vec!["x", "y", "z"]);
        let json = serde_json::to_string(&env).expect("serialize ok");
        let deserialized: SchemaEnvelopeArray<String> =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(env.data, deserialized.data);
        assert_eq!(env.success, deserialized.success);
    }

    #[test]
    fn test_array_envelope_serde_data_not_flattened() {
        let env = SchemaEnvelopeArray::new("list", vec![1, 2]);
        let json_val = serde_json::to_value(&env).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        // data should be a field, not flattened at the top level
        assert!(obj.contains_key("data"));
        let data = obj["data"].as_array().expect("array");
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn test_array_envelope_serde_skip_empty_collections() {
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![]);
        let json_val = serde_json::to_value(&env).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("next"));
        assert!(!obj.contains_key("fixes"));
        assert!(!obj.contains_key("_links"));
    }

    // ── Debug / Clone ────────────────────────────────────────────────────────

    #[test]
    fn test_array_envelope_debug() {
        let env: SchemaEnvelopeArray<String> = SchemaEnvelopeArray::new("list", vec![]);
        let debug = format!("{env:?}");
        assert!(debug.contains("SchemaEnvelopeArray"));
    }

    #[test]
    fn test_array_envelope_clone() {
        let env = SchemaEnvelopeArray::new("list", vec!["a".to_string()]);
        let cloned = env.clone();
        assert_eq!(env.data, cloned.data);
    }
}
