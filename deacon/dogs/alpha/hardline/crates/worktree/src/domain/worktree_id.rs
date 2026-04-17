use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

/// Value object representing a unique worktree identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorktreeId(Uuid);

impl WorktreeId {
    /// Create a new worktree ID from a UUID string
    pub fn from_string(s: &str) -> Result<Self, super::WorktreeDomainError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| super::WorktreeDomainError::InvalidPath(format!("Invalid UUID: {}", e)))
    }

    /// Create a new random worktree ID
    pub fn new_random() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a worktree ID from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Get the UUID as bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }

    /// Convert to string representation
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl Display for WorktreeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<WorktreeId> for Uuid {
    fn from(id: WorktreeId) -> Self {
        id.0
    }
}

impl From<Uuid> for WorktreeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_id_new_random_generates_unique_ids() {
        let id1 = WorktreeId::new_random();
        let id2 = WorktreeId::new_random();
        assert_ne!(id1, id2);
    }

    #[test]
    fn worktree_id_from_string_valid_uuid_returns_id() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = WorktreeId::from_string(uuid_str).unwrap();
        assert_eq!(id.as_string(), uuid_str);
    }

    #[test]
    fn worktree_id_from_string_invalid_uuid_returns_error() {
        let result = WorktreeId::from_string("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn worktree_id_from_bytes_returns_correct_id() {
        let bytes = [
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
            0x00, 0x00,
        ];
        let id = WorktreeId::from_bytes(bytes);
        assert_eq!(id.as_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_display_impl_returns_string() {
        let id = WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format!("{}", id), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_conversion_to_uuid_preserves_value() {
        let id = WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let uuid: Uuid = id.into();
        assert_eq!(uuid.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_conversion_from_uuid_preserves_value() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id: WorktreeId = uuid.into();
        assert_eq!(id.as_string(), "550e8400-e29b-41d4-a716-446655440000");
    }
}
