//! Authentication and authorization types per ADR-015.
//!
//! Provides scoped authorization via token-based authentication.
//! Zero panic, zero unwrap - all operations return Result.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agent::AgentId;
use crate::error::Result;
use crate::error_internal::InternalErrorKind;

// ========================================================================
// Scope
// ========================================================================

/// Scoped authorization per ADR-015.
///
/// Defines what actions an authenticated agent is permitted to perform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// Read access to workspace resources
    ReadWorkspace,
    /// Write access to workspace resources
    WriteWorkspace,
    /// Manage agent sessions
    ManageSessions,
    /// Manage the work queue
    ManageQueue,
    /// Perform VCS operations (push, pull, etc.)
    VcsOperations,
    /// Full administrative access
    Admin,
}

impl Scope {
    /// Returns a human-readable label for this scope.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadWorkspace => "read:workspace",
            Self::WriteWorkspace => "write:workspace",
            Self::ManageSessions => "manage:sessions",
            Self::ManageQueue => "manage:queue",
            Self::VcsOperations => "vcs:operations",
            Self::Admin => "admin",
        }
    }
}

// ========================================================================
// AgentToken
// ========================================================================

/// Agent authentication token.
///
/// Stores a SHA-256 hash of the actual token and an expiration time.
/// The raw token is never stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentToken {
    /// SHA-256 hash of the actual token (hex-encoded)
    pub token_hash: String,
    /// When this token expires
    pub expires_at: DateTime<Utc>,
}

impl AgentToken {
    /// Create a new token by hashing the raw token with SHA-256.
    ///
    /// # Errors
    ///
    /// Returns an error if the raw token is empty.
    pub fn new(raw_token: &str, ttl: Duration) -> Result<Self> {
        if raw_token.is_empty() {
            return Err(
                InternalErrorKind::InvalidOperation(
                    "Token must not be empty".into(),
                )
                .into(),
            );
        }

        let token_hash = hash_token(raw_token);
        let expires_at = Utc::now() + ttl;

        Ok(Self {
            token_hash,
            expires_at,
        })
    }

    /// Verify a raw token against the stored hash.
    #[must_use]
    pub fn verify(&self, raw_token: &str) -> bool {
        hash_token(raw_token) == self.token_hash
    }

    /// Check whether this token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

/// Compute SHA-256 hash of a token string, returning hex-encoded digest.
fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let result = hasher.finalize();
    hex_encode(&result)
}

/// Convert a byte slice to lowercase hex without using formatting traits
/// that could panic.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX_CHARS[usize::from(byte >> 4)] as char);
        output.push(HEX_CHARS[usize::from(byte & 0x0F)] as char);
    }
    output
}

// ========================================================================
// AuthContext
// ========================================================================

/// Authentication context for request-scoped identity.
///
/// Carries the authenticated agent's identity, token, and granted scopes
/// through the request lifecycle.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The authenticated agent
    pub agent_id: AgentId,
    /// The agent's authentication token
    pub token: AgentToken,
    /// Scopes granted to this agent
    pub scopes: Vec<Scope>,
    /// When authentication was established
    pub authenticated_at: DateTime<Utc>,
}

impl AuthContext {
    /// Create a new authentication context.
    ///
    /// # Errors
    ///
    /// Returns an error if the token is already expired.
    pub fn new(
        agent_id: AgentId,
        token: AgentToken,
        scopes: Vec<Scope>,
    ) -> Result<Self> {
        if token.is_expired() {
            return Err(
                InternalErrorKind::InvalidOperation(
                    "Cannot create AuthContext with expired token".into(),
                )
                .into(),
            );
        }

        Ok(Self {
            agent_id,
            token,
            scopes,
            authenticated_at: Utc::now(),
        })
    }

    /// Check whether this context has a specific scope.
    #[must_use]
    pub fn has_scope(&self, scope: &Scope) -> bool {
        // Admin scope grants all access
        if self.scopes.contains(&Scope::Admin) {
            return true;
        }
        self.scopes.contains(scope)
    }

    /// Check whether the authentication is still valid (token not expired).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.token.is_expired()
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    #[test]
    fn test_agent_token_new_hashes_correctly() {
        let token = AgentToken::new("secret123", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");

        // Same input should produce same hash
        let token2 = AgentToken::new("secret123", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");
        assert_eq!(token.token_hash, token2.token_hash);

        // Different input should produce different hash
        let token3 = AgentToken::new("secret456", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");
        assert_ne!(token.token_hash, token3.token_hash);
    }

    #[test]
    fn test_agent_token_new_rejects_empty() {
        let result = AgentToken::new("", TimeDelta::try_seconds(3600).expect("valid duration"));
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_token_verify() {
        let token = AgentToken::new("my-token", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");

        assert!(token.verify("my-token"));
        assert!(!token.verify("wrong-token"));
        assert!(!token.verify(""));
    }

    #[test]
    fn test_agent_token_is_expired() {
        let token = AgentToken::new("tok", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");
        assert!(!token.is_expired());

        let expired = AgentToken::new("tok", TimeDelta::zero())
            .expect("should create token");
        // Tokens with 0 TTL expire immediately (or very soon)
        assert!(expired.is_expired());
    }

    #[test]
    fn test_token_hash_is_hex_sha256() {
        let token = AgentToken::new("test", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");
        // SHA-256 hex digest is 64 characters
        assert_eq!(token.token_hash.len(), 64);
        assert!(token.token_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_auth_context_new_valid() {
        let agent_id = AgentId::new("agent-1");
        let token = AgentToken::new("tok", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");
        let scopes = vec![Scope::ReadWorkspace];

        let ctx = AuthContext::new(agent_id.clone(), token, scopes.clone())
            .expect("should create context");

        assert_eq!(ctx.agent_id, agent_id);
        assert!(ctx.is_valid());
    }

    #[test]
    fn test_auth_context_new_rejects_expired() {
        let agent_id = AgentId::new("agent-1");
        let expired = AgentToken::new("tok", TimeDelta::zero())
            .expect("should create token");

        let result = AuthContext::new(agent_id, expired, vec![Scope::ReadWorkspace]);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_context_has_scope() {
        let agent_id = AgentId::new("agent-1");
        let token = AgentToken::new("tok", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");

        let ctx = AuthContext::new(
            agent_id,
            token,
            vec![Scope::ReadWorkspace, Scope::ManageSessions],
        )
        .expect("should create context");

        assert!(ctx.has_scope(&Scope::ReadWorkspace));
        assert!(ctx.has_scope(&Scope::ManageSessions));
        assert!(!ctx.has_scope(&Scope::WriteWorkspace));
        assert!(!ctx.has_scope(&Scope::Admin));
    }

    #[test]
    fn test_auth_context_admin_grants_all() {
        let agent_id = AgentId::new("admin");
        let token = AgentToken::new("tok", TimeDelta::try_seconds(3600).expect("valid duration"))
            .expect("should create token");

        let ctx = AuthContext::new(agent_id, token, vec![Scope::Admin])
            .expect("should create context");

        assert!(ctx.has_scope(&Scope::Admin));
        assert!(ctx.has_scope(&Scope::ReadWorkspace));
        assert!(ctx.has_scope(&Scope::WriteWorkspace));
        assert!(ctx.has_scope(&Scope::ManageSessions));
        assert!(ctx.has_scope(&Scope::ManageQueue));
        assert!(ctx.has_scope(&Scope::VcsOperations));
    }

    #[test]
    fn test_scope_as_str() {
        assert_eq!(Scope::ReadWorkspace.as_str(), "read:workspace");
        assert_eq!(Scope::WriteWorkspace.as_str(), "write:workspace");
        assert_eq!(Scope::ManageSessions.as_str(), "manage:sessions");
        assert_eq!(Scope::ManageQueue.as_str(), "manage:queue");
        assert_eq!(Scope::VcsOperations.as_str(), "vcs:operations");
        assert_eq!(Scope::Admin.as_str(), "admin");
    }
}
