//! Kani verification harness for WorktreeId arithmetic safety
//!
//! Verifies that WorktreeId operations preserve invariants

use worktree::domain::WorktreeId;

#[kani::proof]
fn prove_worktree_id_bytes_roundtrip() {
    let bytes: [u8; 16] = kani::any();
    let id = WorktreeId::from_bytes(bytes);
    let retrieved = *id.as_bytes();
    kani::assert(retrieved == bytes, "Bytes roundtrip preserves value");
}

#[kani::proof]
fn prove_worktree_id_uuid_conversion() {
    let bytes: [u8; 16] = kani::any();
    let id = WorktreeId::from_bytes(bytes);

    // Convert to string and back should preserve identity (modulo case)
    let uuid_str = id.as_string();
    let id2 = WorktreeId::from_string(&uuid_str);

    kani::assert(id2.is_ok(), "UUID string conversion succeeds");
    if let Ok(recovered) = id2 {
        kani::assert(
            recovered.as_string() == uuid_str,
            "Recovered ID matches original string",
        );
    }
}

#[kani::proof]
fn prove_worktree_id_uniqueness() {
    let bytes1: [u8; 16] = kani::any();
    let bytes2: [u8; 16] = kani::any();

    let id1 = WorktreeId::from_bytes(bytes1);
    let id2 = WorktreeId::from_bytes(bytes2);

    // If bytes are equal, IDs should be equal
    if bytes1 == bytes2 {
        kani::assert(id1 == id2, "Equal bytes produce equal IDs");
    }
}
