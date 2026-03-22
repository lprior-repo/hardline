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
