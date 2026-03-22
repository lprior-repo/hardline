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
