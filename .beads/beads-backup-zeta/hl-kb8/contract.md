# Contract Specification: CLI Lock Integration (hl-kb8)

- **bead_id**: hl-kb8
- **bead_title**: CLI: Integrate lock command into CLI
- **phase**: State 1
- **updated_at**: 2026-03-30T03:55:00Z

## Context
- **Feature**: Integrate LockManager into the SCP CLI.
- **Domain terms**: Session (locked resource), Agent (lock holder), TTL (Time-To-Live).
- **Assumptions**: The CLI will use a persistent SQLite-backed LockManager.

## Preconditions
- session_name: Must be non-empty and <= 255 characters.
- agent_id: Must be non-empty.
- ttl_seconds: Must be in range [0, 86400] (0 defaults to 300s).
- For unlock and heartbeat: The caller must be the current holder of an active lock.

## Postconditions
- **Lock**: An exclusive record exists in session_locks with expires_at = now + TTL. Audit entry created.
- **Unlock**: Record removed from session_locks. Audit entry created.
- **Heartbeat**: expires_at updated to now + default_ttl. Audit entry created.

## Invariants
- **Exclusivity**: All sessions have at most one entry in session_locks where expires_at >= now.
- **Integrity**: Every modification to session_locks must be reflected in session_lock_audit.
- **Cleanliness**: Expired locks are logically treated as non-existent and physically removed upon the next acquisition attempt for that session.

## Error Taxonomy
- Error::InvalidInput: Empty or too long session/agent names.
- Error::TtlOutOfRange: TTL exceeds 24 hours.
- Error::SessionLocked: Lock acquisition failed because another agent holds it.
- Error::NotLockHolder: Attempted to unlock or heartbeat a session held by another agent.
- Error::SessionNotFound: Attempted to lock a session that does not exist in the system.
- Error::DatabaseError: Persistence failure.

## Contract Signatures
- fn cmd_lock(session: &str, agent: &str, ttl: Option<u64>) -> Result<LockResponse, Error>
- fn cmd_unlock(session: &str, agent: &str) -> Result<(), Error>
- fn cmd_heartbeat(session: &str, agent: &str) -> Result<(), Error>
- fn cmd_lock_status(session: &str) -> Result<LockState, Error>
- fn cmd_lock_list() -> Result<Vec<LockInfo>, Error>
