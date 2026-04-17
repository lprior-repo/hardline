import re
import os

# Find all error variants
error_files = [
    "crates/core/src/error.rs",
    "crates/core/src/domain/identifiers.rs",
    "crates/core/src/output_jsonl/types.rs",
    "crates/core/src/domain/session_remove.rs",
    "crates/core/src/domain/session_focus.rs",
    "crates/core/src/domain/repository.rs",
    "crates/core/src/domain/aggregates/workspace.rs",
    "crates/core/src/domain/aggregates/bead.rs",
    "crates/core/src/domain/aggregates/session.rs",
    "crates/core/src/cli_contracts/error.rs",
    "crates/core/src/beads/types.rs",
    "crates/core/src/beads/domain.rs",
    "crates/core/src/coordination/domain_types.rs",
    "crates/core/src/domain/validation.rs",
    "crates/core/src/moon_gates.rs",
    "crates/core/src/session_sync.rs",
    "crates/core/src/dag/types.rs"
]

error_definitions = {}

for filepath in error_files:
    if not os.path.exists(filepath):
        continue
    with open(filepath, "r") as f:
        content = f.read()
    
    # Simple regex to find enum Error types and their variants
    # Look for pub enum Name { ... }
    matches = re.finditer(r'(?:pub\s+)?enum\s+([A-Za-z0-9_]+)\s*\{([^}]*)\}', content, re.MULTILINE)
    for match in matches:
        enum_name = match.group(1)
        # Check if it has thiserror derive before it
        # Actually it's simpler to just look at all enums in these files and filter manually,
        # but let's assume all enums in these files are error enums if they have Name ending in Error
        # Wait, some are not ending in Error (like JjConflictType), let's rely on the ones we know
        if "Error" in enum_name or enum_name in ["IdentifierError", "OutputLineError", "SessionRemoveError", "SessionFocusError", "RepositoryError", "WorkspaceError", "BeadError", "SessionError", "ContractError", "BeadsError", "DomainError", "ValidationError", "GateError", "SyncError", "DagError"]:
            variants_text = match.group(2)
            # Find variants
            variants = []
            for line in variants_text.split('\n'):
                line = line.strip()
                if line.startswith('//') or line.startswith('#'): continue
                if line == '': continue
                vmatch = re.match(r'([A-Za-z0-9_]+)', line)
                if vmatch:
                    v = vmatch.group(1)
                    # Ignore pub, crate, etc if any, but they shouldn't be inside enum
                    variants.append(v)
            if enum_name not in error_definitions:
                error_definitions[enum_name] = []
            error_definitions[enum_name].extend(variants)

# Now scan all tests in crates/core
# We will search across all .rs files in crates/core for the variants inside test functions or test modules
# Actually just grep for `EnumName::Variant` or `Variant` if imported.

import subprocess

def check_coverage():
    missing = []
    
    for enum, variants in error_definitions.items():
        for variant in variants:
            # grep for the variant in crates/core
            # looking for test files or mod tests
            # we'll use ripgrep
            try:
                # search for "EnumName::Variant"
                rg_cmd = f"rg -l '{enum}::{variant}' crates/core"
                output = subprocess.check_output(rg_cmd, shell=True, text=True)
                
                # Check if any of the files are tests or contain #[test]
                # A simple approximation: if it's used anywhere besides its definition, it's covered.
                # Let's count occurrences
                rg_cmd2 = f"rg '{enum}::{variant}' crates/core | wc -l"
                count = int(subprocess.check_output(rg_cmd2, shell=True, text=True).strip())
                
                # if count <= 1 (only definition), it might be missing
                # wait, definition is just `Variant` inside the enum, not `EnumName::Variant`.
                # So any occurrence of `EnumName::Variant` is a usage.
                if count == 0:
                    missing.append((enum, variant))
            except subprocess.CalledProcessError:
                missing.append((enum, variant))
                
    return missing

missing_tests = check_coverage()
for enum, variant in missing_tests:
    print(f"Missing test for {enum}::{variant}")

