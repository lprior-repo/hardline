# ADR-015: Security & Access Control

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs security controls for:

1. **Agent authentication** - Verify agent identity
2. **Authorization** - What can each agent do
3. **Workspace isolation** - Prevent cross-workspace access
4. **Audit logging** - Who accessed what
5. **Secrets management** - Protect tokens and credentials

With 600+ concurrent agents, security is critical. This ADR defines the access control model.

---

## Decision

### Authentication

```rust
/// Agent authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToken {
    pub agent_id: AgentId,
    pub token_hash: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    pub resource: Resource,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    Workspace(WorkspaceId),
    WorkspaceType,
    Queue,
    Stack,
    Agent(AgentId),
    AgentType,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
    Admin,
}

impl AgentToken {
    pub fn new(agent_id: AgentId, scopes: Vec<Scope>) -> Self {
        Self {
            agent_id,
            token_hash: generate_token_hash(),
            issued_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            scopes,
        }
    }
    
    pub fn is_valid(&self) -> bool {
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }
        true
    }
    
    pub fn has_scope(&self, resource: &Resource, action: Action) -> bool {
        self.scopes.iter().any(|s| {
            match (&s.resource, &s.action) {
                (Resource::System, Action::Admin) => true,  // Admin has all
                (r, a) => r == resource && (a == action || a == Action::Admin),
            }
        })
    }
}
```

### Authorization

```rust
/// Authorization context
pub struct AuthContext {
    pub agent_id: AgentId,
    pub token: AgentToken,
    pub workspace_id: Option<WorkspaceId>,
}

impl AuthContext {
    pub fn can_access(&self, resource: &Resource, action: Action) -> bool {
        // System admin can do anything
        if self.token.has_scope(&Resource::System, Action::Admin) {
            return true;
        }
        
        // Check workspace-specific access
        if let Some(ws_id) = &self.workspace_id {
            match resource {
                Resource::Workspace(id) if id == ws_id => {
                    self.token.has_scope(resource, action)
                }
                _ => {
                    // Cross-workspace access requires System scope
                    self.token.has_scope(&Resource::System, action)
                }
            }
        } else {
            // No workspace context - check if resource is workspace-independent
            matches!(resource, Resource::System | Resource::WorkspaceType | Resource::AgentType)
        }
    }
}

/// Authorization middleware
pub struct AuthMiddleware;

impl AuthMiddleware {
    pub fn authenticate(req: &Request) -> Result<AuthContext, AuthError> {
        let token_header = req.headers()
            .get("Authorization")
            .ok_or(AuthError::MissingToken)?;
        
        let token = token_header
            .to_str()
            .map_err(|_| AuthError::InvalidToken)?;
        
        if !token.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken);
        }
        
        let token_hash = &token[7..];
        let token_record = lookup_token(token_hash)
            .ok_or(AuthError::InvalidToken)?;
        
        if !token_record.is_valid() {
            return Err(AuthError::TokenExpired);
        }
        
        Ok(AuthContext {
            agent_id: token_record.agent_id,
            token: token_record,
            workspace_id: req.workspace_id(),
        })
    }
    
    pub fn authorize(ctx: &AuthContext, resource: &Resource, action: Action) 
        -> Result<(), AuthError> 
    {
        if ctx.can_access(resource, action) {
            Ok(())
        } else {
            Err(AuthError::AccessDenied {
                agent_id: ctx.agent_id,
                resource: resource.clone(),
                action,
            })
        }
    }
}
```

### Workspace Isolation

```rust
/// Workspace access guard
pub struct WorkspaceGuard;

impl WorkspaceGuard {
    /// Verify agent can only access their own workspace
    pub fn verify_workspace_access(
        agent_id: &AgentId,
        workspace_id: &WorkspaceId,
        workspace_repo: &dyn WorkspaceRepository,
    ) -> Result<(), AuthError> {
        let workspace = workspace_repo.find_by_id(workspace_id)?
            .ok_or(AuthError::ResourceNotFound)?;
        
        // Check if agent owns this workspace
        if let Some(owner) = &workspace.agent_id {
            if owner != agent_id {
                return Err(AuthError::AccessDenied {
                    agent_id: *agent_id,
                    resource: Resource::Workspace(*workspace_id),
                    action: Action::Read,
                });
            }
        }
        
        Ok(())
    }
    
    /// Verify workspace is in valid state for operations
    pub fn verify_workspace_state(
        workspace: &Workspace,
        required_states: &[WorkspaceState],
    ) -> Result<(), AuthError> {
        if !required_states.contains(&workspace.state) {
            return Err(AuthError::InvalidState {
                resource: Resource::Workspace(workspace.id),
                current_state: workspace.state,
                required_states: required_states.to_vec(),
            });
        }
        Ok(())
    }
}
```

### Secrets Management

```rust
/// Secret storage
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &SecretKey) -> Result<SecretValue, SecretError>;
    fn set(&self, key: &SecretKey, value: SecretValue) -> Result<(), SecretError>;
    fn delete(&self, key: &SecretKey) -> Result<(), SecretError>;
    fn list(&self, prefix: &str) -> Result<Vec<SecretKey>, SecretError>;
}

#[derive(Debug, Clone)]
pub struct SecretKey {
    pub scope: SecretScope,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum SecretScope {
    Workspace(WorkspaceId),
    Agent(AgentId),
    System,
}

#[derive(Debug, Clone)]
pub struct SecretValue {
    pub value: String,
    pub encrypted: bool,
    pub updated_at: DateTime<Utc>,
}

/// Encrypted secret store using AES-256-GCM
pub struct EncryptedSecretStore {
    inner: HashMap<SecretKey, SecretValue>,
    encryption_key: Vec<u8>,
}

impl SecretStore for EncryptedSecretStore {
    fn get(&self, key: &SecretKey) -> Result<SecretValue, SecretError> {
        let secret = self.inner.get(key)
            .ok_or(SecretError::NotFound)?;
        
        if !secret.encrypted {
            return Err(SecretError::NotEncrypted);
        }
        
        // Decrypt
        let decrypted = self.decrypt(&secret.value)?;
        
        Ok(SecretValue {
            value: decrypted,
            encrypted: false,
            updated_at: secret.updated_at,
        })
    }
    
    fn set(&self, key: &SecretKey, value: SecretValue) -> Result<(), SecretError> {
        // Encrypt before storing
        let encrypted = self.encrypt(&value.value)?;
        
        self.inner.insert(key.clone(), SecretValue {
            value: encrypted,
            encrypted: true,
            updated_at: Utc::now(),
        });
        
        Ok(())
    }
}
```

### Audit Logging

```rust
/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: AuditEventId,
    pub timestamp: DateTime<Utc>,
    pub agent_id: AgentId,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub resource_id: String,
    pub result: AuditResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Login,
    Logout,
    Read,
    Create,
    Update,
    Delete,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResource {
    Workspace,
    Queue,
    Stack,
    Agent,
    Secret,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure { reason: String },
}

/// Audit logger
pub struct AuditLogger {
    event_sender: mpsc::Sender<AuditEvent>,
}

impl AuditLogger {
    pub fn log(&self, event: AuditEvent) {
        self.event_sender.send(event).ok();
    }
    
    pub fn log_access(
        &self,
        ctx: &AuthContext,
        resource: &Resource,
        action: Action,
        result: &Result<(), AuthError>,
    ) {
        let (audit_result, reason) = match result {
            Ok(()) => (AuditResult::Success, None),
            Err(e) => (AuditResult::Failure { reason: e.to_string() }, Some(e.to_string())),
        };
        
        let (resource_type, resource_id) = match resource {
            Resource::Workspace(id) => ("workspace", id.to_string()),
            Resource::Queue => ("queue", "queue".to_string()),
            Resource::Agent(id) => ("agent", id.to_string()),
            Resource::System => ("system", "system".to_string()),
            _ => ("unknown", "unknown".to_string()),
        };
        
        self.log(AuditEvent {
            id: AuditEventId::new(),
            timestamp: Utc::now(),
            agent_id: ctx.agent_id,
            action: match action {
                Action::Read => AuditAction::Read,
                Action::Write => AuditAction::Update,
                Action::Delete => AuditAction::Delete,
                Action::Execute => AuditAction::Execute,
                Action::Admin => AuditAction::Delete,
            },
            resource: match resource_type {
                "workspace" => AuditResource::Workspace,
                "queue" => AuditResource::Queue,
                "agent" => AuditResource::Agent,
                "system" => AuditResource::System,
                _ => AuditResource::System,
            },
            resource_id,
            result: audit_result,
            ip_address: None,
            user_agent: None,
            details: reason.map(|r| json!({ "reason": r })),
        });
    }
}
```

### Rate Limiting

```rust
/// Rate limiter
pub struct RateLimiter {
    redis: Arc<dyn Cache>,
    limits: HashMap<AgentId, RateLimit>,
}

pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub concurrent_requests: u32,
}

impl RateLimiter {
    pub fn check_rate_limit(&self, agent_id: &AgentId) -> Result<(), RateLimitError> {
        let key = format!("rate:{}", agent_id);
        
        let count: u32 = self.redis.get(&key).unwrap_or(0);
        let limit = self.limits.get(agent_id)
            .copied()
            .unwrap_or_default();
        
        if count >= limit.requests_per_minute {
            return Err(RateLimitError::Exceeded {
                agent_id: *agent_id,
                limit: limit.requests_per_minute,
                window: "minute".to_string(),
            });
        }
        
        self.redis.increment(&key, 1);
        self.redis.expire(&key, 60);  // 1 minute window
        
        Ok(())
    }
}
```

---

## Variants

### Variant A: RBAC (Role-Based Access Control) (CHOSEN)

```rust
// Agents have roles, roles have permissions
// Simple, well-understood
```

**Chosen because:**
- Simple to reason about
- Well-understood model
- Easy to audit

### Variant B: ABAC (Attribute-Based Access Control)

**Rejected because:**
- Complex policy engine
- Hard to audit
- Overkill for hardline

### Variant C: Capabilities-Based

```rust
// Tokens carry capabilities directly
// More flexible but harder to revoke
```

**Rejected because:**
- Revocation is hard
- Token lifetime issues

---

## Invariants

### Authentication Invariants

```rust
/// INVARIANT: Valid token has not expired
fn assert_token_not_expired(token: &AgentToken) {
    if let Some(expires) = token.expires_at {
        assert!(Utc::now() < expires, "Token {} is expired", token.agent_id);
    }
}

/// INVARIANT: Token hash is non-empty
fn assert_token_hash_valid(token: &AgentToken) {
    assert!(!token.token_hash.is_empty());
    assert!(token.token_hash.len() >= 32);
}
```

### Authorization Invariants

```rust
/// INVARIANT: Workspace agent can only access own workspace
fn assert_workspace_isolation(ctx: &AuthContext, resource: &Resource) -> bool {
    if let Resource::Workspace(ws_id) = resource {
        if let Some(ctx_ws) = ctx.workspace_id {
            return ws_id == &ctx_ws;  // Must match
        }
    }
    true  // Non-workspace resources are handled differently
}

/// INVARIANT: Admin scope grants all access
fn assert_admin_has_all(token: &AgentToken) {
    if token.has_scope(&Resource::System, Action::Admin) {
        // Admin should have access to everything
        assert!(token.scopes.iter().any(|s| s.action == Action::Admin));
    }
}
```

### Secrets Invariants

```rust
/// INVARIANT: Stored secrets are encrypted
fn assert_secrets_encrypted(store: &dyn SecretStore) {
    let keys = store.list("").unwrap();
    for key in keys {
        let secret = store.get(&key).unwrap();
        assert!(secret.encrypted, "Secret {} is not encrypted", key.name);
    }
}

/// INVARIANT: Secret values are non-empty
fn assert_secret_value_not_empty(secret: &SecretValue) {
    assert!(!secret.value.is_empty());
}
```

### Audit Invariants

```rust
/// INVARIANT: All access attempts are logged
fn assert_access_is_audited(event: &AuditEvent) {
    assert!(event.result.is_some());
}

/// INVARIANT: Audit timestamps are not in future
fn assert_audit_timestamp_not_future(event: &AuditEvent) {
    assert!(event.timestamp <= Utc::now());
}
```

---

## Consequences

### Positive

1. **Isolation** - Agents can't access each other's workspaces
2. **Auditability** - All actions logged
3. **Controlled access** - Scoped tokens limit blast radius
4. **Secrets protected** - Encrypted storage

### Negative

1. **Complexity** - Auth middleware adds overhead
2. **Token management** - Need to handle issuance/revocation
3. **Performance** - Auth checks on every request

### CLI Commands

```bash
hardline auth login <agent-name>           # Get token
hardline auth logout                       # Revoke token
hardline auth verify                       # Verify token validity
hardline auth scopes                        # List token scopes
hardline audit list --agent <id>           # View audit log
hardline secrets set <key> <value>        # Store secret
hardline secrets list                      # List secrets
hardline rate-limits show                  # View rate limits
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/security/auth.rs` | Authentication |
| `crates/core/src/security/authorization.rs` | Authorization |
| `crates/core/src/security/secrets.rs` | Secret storage |
| `crates/core/src/security/audit.rs` | Audit logging |

---

## Related ADRs

- ADR-001: CLI Architecture (auth commands)
- ADR-010: Agent Registry & Heartbeat (agent identity)
- ADR-005: Workspace Isolation Model (workspace access control)
