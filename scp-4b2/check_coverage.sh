#!/bin/bash

declare -A UNTESTED

# List of files we know have thiserror enums
FILES=(
    "crates/core/src/error.rs"
    "crates/core/src/domain/identifiers.rs"
    "crates/core/src/output_jsonl/types.rs"
    "crates/core/src/domain/session_remove.rs"
    "crates/core/src/domain/session_focus.rs"
    "crates/core/src/domain/repository.rs"
    "crates/core/src/domain/aggregates/workspace.rs"
    "crates/core/src/domain/aggregates/bead.rs"
    "crates/core/src/domain/aggregates/session.rs"
    "crates/core/src/cli_contracts/error.rs"
    "crates/core/src/beads/types.rs"
    "crates/core/src/beads/domain.rs"
    "crates/core/src/coordination/domain_types.rs"
    "crates/core/src/domain/validation.rs"
    "crates/core/src/moon_gates.rs"
    "crates/core/src/session_sync.rs"
    "crates/core/src/dag/types.rs"
)

for file in "${FILES[@]}"; do
    if [ ! -f "$file" ]; then continue; fi
    
    # Extract enum names
    enums=$(grep -oP '(?:pub\s+)?enum\s+\K[A-Za-z0-9_]+' "$file")
    
    for enum in $enums; do
        if [[ "$enum" != *"Error" && "$enum" != "JjConflictType" && "$enum" != "GateError" && "$enum" != "SyncError" && "$enum" != "DagError" ]]; then
            # Not an error enum (likely)
            continue
        fi
        
        # Extract variants. This is a bit tricky with bash, so let's use perl.
        # Find the block between 'enum ENUM_NAME {' and '}'.
        # Then find all lines that look like a variant definition.
        variants=$(perl -0777 -ne "while (/(?:pub\s+)?enum\s+$enum\s*\{([^}]+)\}/g) {
            my \$body = \$1;
            while (\$body =~ m/^\s*([A-Z][a-zA-Z0-9_]*)/gm) {
                print \"\$1\n\";
            }
        }" "$file")
        
        for variant in $variants; do
            if [ -z "$variant" ]; then continue; fi
            # Check if this variant is tested.
            # We look for "enum::variant" or just "variant" in test files or blocks.
            # A simple heuristic: if the string "variant" appears in any test file
            # or in the same file but within a test context.
            
            # Count occurrences of the variant name across all files in crates/core
            total_occurrences=$(rg -w "$variant" crates/core | wc -l)
            
            # Count occurrences in test blocks/files
            # To be safe, we look for test files (*test*.rs, tests/*) or the word test near it.
            # A simpler way: if total_occurrences <= 1, it's definitively not covered.
            # If it's only found in the source file where it's defined and nowhere else, and not in the test module.
            test_occurrences=$(rg -w "$variant" crates/core | grep -E 'test|assert' | wc -l)
            
            # Another way: look for $enum::$variant or $variant in assert!, match, etc in tests.
            # Let's search specifically for test files or lines with assert/test.
            
            if [ "$test_occurrences" -eq 0 ]; then
                # Let's double check if it's used in any file ending with _test.rs, tests/, or within mod tests
                file_occurrences=$(rg -w "$variant" crates/core)
                if ! echo "$file_occurrences" | grep -qE '#\[test|mod tests|tests\.rs|tests/|assert'; then
                     UNTESTED["$enum::$variant"]=1
                fi
            fi
        done
    done
done

for k in "${!UNTESTED[@]}"; do
    echo "Missing test for $k"
done
