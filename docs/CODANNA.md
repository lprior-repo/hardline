# Codanna - Semantic Code Search

Codanna is the project's semantic code search engine. It provides fast, intelligent code exploration with pre-indexed symbols, relationships, and semantic understanding.

**Key Benefits:**
- **40-50x faster** than Grep/Glob for exact/fuzzy searches
- **90% fewer tokens** (pre-indexed, semantic understanding)
- **Relationship-aware** (callers, callees, impact analysis)
- **Natural language support** (semantic search)

---

## Quick Start

```bash
# Check current index statistics
codanna mcp get_index_info

# Benchmark parser performance
codanna benchmark

# Rebuild index if performance degrades
codanna index

# Verify indexed paths
codanna list-dirs
```

### Essential Commands (AI Agent)

```bash
# Natural language query → full context
mcp__codanna__semantic_search_with_context(query="your intent", limit: 5)

# Know exact name
mcp__codanna__find_symbol(name="ExactName")

# Fuzzy pattern search
mcp__codanna__search_symbols(query="pattern", kind:"Struct|Function|Trait", lang:"rust", limit: 10)

# Search markdown docs
mcp__codanna__search_documents(query="topic", limit: 5)

# Dependency graph
mcp__codanna__analyze_impact(symbol_id: 123)
```

---

## Index Statistics

### Current Index State

| Metric | Value |
|--------|-------|
| **Symbols** | 3,578 across 113 files |
| **Relationships** | 3,399 (call graph, dependencies) |
| **Symbol Types** | 9 kinds |
| **Semantic Search** | Configurable (currently disabled) |
| **Indexed Paths** | 3 directories |

### Symbol Distribution

| Kind | Count | Percentage |
|------|-------|------------|
| Functions | 1,457 | 40.7% |
| Fields | 938 | 26.2% |
| Modules | 223 | 6.2% |
| Structs | 254 | 7.1% |
| Constants | 277 | 7.7% |
| Methods | 362 | 10.1% |
| Enums | 50 | 1.4% |
| Traits | 6 | 0.2% |
| Type Aliases | 11 | 0.3% |

---

## Query Types & Performance

### Exact Symbol Lookup (`find_symbol`)

- **Expected Latency:** 5-15ms
- **Best Case:** ~5ms (cached, unique match)
- **Worst Case:** ~15ms (multiple matches, disk I/O)
- **Use Case:** When you know exact symbol name

```bash
codanna mcp find_symbol --name Workspace
```

### Fuzzy Symbol Search (`search_symbols`)

- **Expected Latency:** 10-50ms
- **Best Case:** ~10ms (specific pattern, filtered by kind)
- **Worst Case:** ~50ms (broad pattern, no kind filter)
- **Use Case:** Partial name matches, pattern-based discovery

```bash
# Fuzzy match with filter
codanna mcp search_symbols --pattern work --kind Struct --limit 10
```

### Semantic Search (`semantic_search_docs`)

- **Expected Latency:** 50-200ms
- **Best Case:** ~50ms (small result set, high relevance)
- **Worst Case:** ~200ms (large result set, semantic scoring)
- **Note:** Currently disabled in config (requires embedding model)
- **Use Case:** Natural language queries, intent-based search

```bash
# Natural language discovery
codanna mcp semantic_search_docs --query "workspace isolation patterns" --limit 5
```

### Document Search (`search_documents`)

- **Expected Latency:** 20-100ms
- **Use Case:** Finding documentation, comments, guides

```bash
codanna mcp search_documents --query error_handling
```

### Dependency Analysis (`analyze_impact`)

- **Expected Latency:** 100-500ms
- **Best Case:** ~100ms (isolated symbol, few dependencies)
- **Worst Case:** ~500ms (highly connected symbol, deep call chains)
- **Use Case:** Understanding change impact, refactoring planning

```bash
codanna mcp analyze_impact --symbol_id 123
```

### Call Graph Queries (`get_calls`, `find_callers`)

- **Expected Latency:** 20-100ms
- **Use Case:** Tracing execution flow, finding usage

```bash
# Who calls this function?
codanna mcp find_callers --symbol_id 123

# What does this function call?
codanna mcp get_calls --symbol_id 456
```

---

## Codanna vs Grep/Glob

| Operation | Grep/Glob | Codanna | Speedup |
|-----------|-----------|---------|---------|
| Exact symbol name | ~200ms | ~5ms | **40x** |
| Fuzzy name match | ~500ms | ~10ms | **50x** |
| Find all references | ~800ms | ~20ms | **40x** |
| Semantic search | N/A | ~50ms | **∞** |
| Dependency analysis | N/A (manual) | ~100ms | **∞** |
| Impact analysis | N/A (manual) | ~500ms | **∞** |

---

## Optimization Strategies

### 1. Use the Right Tool for the Job

```bash
# ✅ GOOD: Exact name known
codanna mcp find_symbol --name Workspace

# ❌ BAD: Unnecessary fuzzy search
codanna mcp search_symbols --pattern Workspace

# ✅ GOOD: Partial name, specific type
codanna mcp search_symbols --pattern work --kind Struct --limit 10

# ❌ BAD: No kind filter, broad pattern
codanna mcp search_symbols --pattern w  # Returns everything with 'w'

# ✅ GOOD: Natural language intent
codanna mcp semantic_search_docs --query "workspace isolation patterns"

# ❌ BAD: Exact name for semantic query
codanna mcp semantic_search_docs --query Workspace  # Use find_symbol instead
```

### 2. Filter by Symbol Kind

**Available Kinds:** Function, Method, Struct, Enum, Trait, Module, Constant, Field, TypeAlias

```bash
# Narrow search space for faster queries
codanna mcp search_symbols --pattern create --kind Function  # Functions only
codanna mcp search_symbols --pattern Config --kind Struct    # Structs only
codanna mcp search_symbols --pattern Error --kind Enum       # Enums only
```

### 3. Limit Result Sets

| Limit | Fuzzy Search | Semantic Search |
|-------|--------------|-----------------|
| 5 | ~10ms | ~50ms |
| 50 | ~30ms | ~150ms |
| 100 | ~50ms | ~200ms |

```bash
codanna mcp search_symbols --pattern work --limit 5
codanna mcp semantic_search_docs --query workspace --limit 10
```

### 4. Target Specific Directories

```bash
# Index only what you need
codanna remove-dir node_modules      # Exclude large dependencies
codanna remove-dir target            # Exclude build artifacts
codanna add-dir crates/isolate-core/src  # Index core library only
```

**Current Indexed Paths:**
- `crates/isolate-core/src` (core library)
- `crates/isolate/src` (CLI binary)
- `crates/isolate/tests` (integration tests)
- `docs` (documentation)

### 5. Keep Index Fresh

```bash
# Rebuild after significant changes
codanna index

# Or use file watching (automatic updates)
codanna serve --watch
```

### 6. Adjust Memory Settings

```toml
[indexing]
tantivy_heap_mb = 50  # Default: 50MB

[semantic_search]
embedding_threads = 3  # Default: 3 threads
```

**Tuning Guidelines:**
- **< 4GB RAM:** Keep `tantivy_heap_mb = 50`
- **4-8GB RAM:** Try `tantivy_heap_mb = 100`
- **> 8GB RAM:** Try `tantivy_heap_mb = 200`

---

## Factors Affecting Performance

### 1. Index Size

```bash
# Check index size
du -sh .codanna/index
```

| Symbols | Query Time |
|---------|------------|
| < 5k | Sub-10ms |
| 5k-20k | 10-50ms |
| 20k-50k | 50-100ms |
| > 50k | Consider filtering |

### 2. Query Complexity

| Query Type | Complexity | Example |
|------------|------------|---------|
| Exact name | O(1) | `find_symbol("Workspace")` |
| Fuzzy match | O(log n) | `search_symbols("work", kind: "Struct")` |
| Semantic | O(n) | `semantic_search_docs("workspace isolation")` |
| Graph traversal | O(n + e) | `analyze_impact(symbol_id)` |

### 3. Disk I/O vs Cached

- **First query:** Cold cache (~15ms)
- **Subsequent queries:** Hot cache (~5ms)

### 4. Semantic Search Overhead

When enabled:
- **Indexing:** +30-50% time (compute embeddings)
- **Queries:** +50-100ms per query (vector similarity search)
- **Benefit:** Natural language understanding

**Recommendation:** Enable only if you frequently use natural language queries.

---

## Common Query Patterns

### Pattern 1: Find Exact Symbol

```bash
# Fastest: O(1) lookup
codanna mcp find_symbol --name Workspace
```

### Pattern 2: Fuzzy Name Match

```bash
# Fast: O(log n) search with filter
codanna mcp search_symbols --pattern work --kind Struct --limit 10
```

### Pattern 3: Find All References

```bash
# Medium: O(e) where e = edges in call graph
codanna mcp find_callers --symbol_id 123
```

### Pattern 4: Understand Impact

```bash
# Slower: O(n + e) full graph traversal
codanna mcp analyze_impact --symbol_id 456
```

### Pattern 5: Semantic Discovery

```bash
# Slower: O(n) vector similarity search
codanna mcp semantic_search_docs --query "workspace isolation patterns" --limit 5
```

---

## Performance Monitoring

### Check Index Health

```bash
codanna mcp get_index_info
```

**Healthy Indicators:**
- Symbol count matches expected (1-2 symbols per 10 lines of Rust)
- Relationship count > 50% of symbol count
- File count matches indexed directories

**Warning Signs:**
- Symbol count dropped → partial index failure
- Relationships < 20% of symbols → parser issues
- File count mismatch → files excluded by ignore patterns

### Benchmark Queries

```bash
# Time exact symbol lookup
time codanna mcp find_symbol --name Workspace

# Time fuzzy search
time codanna mcp search_symbols --pattern work --kind Struct --limit 10

# Time dependency analysis
time codanna mcp analyze_impact --symbol_id 123
```

### Parser Performance

```bash
codanna benchmark
```

**Expected Output:**
```
Parser Benchmark:
  Files parsed: 113
  Parse time: 450ms
  Throughput: 251 files/sec
  Indexing time: 120ms
  Total: 570ms
```

---

## Troubleshooting

### Queries Are Slow (> 500ms)

**Diagnosis:**
```bash
du -sh .codanna/index
codanna mcp get_index_info | grep "Symbols:"
```

**Solutions:**
1. Remove unnecessary directories (`codanna remove-dir`)
2. Increase `tantivy_heap_mb` in config
3. Rebuild index (`codanna index`)
4. Simplify query, add filters, limit results

### Semantic Search Very Slow (> 1s)

**Solutions:**
1. Disable semantic search if not needed
2. Reduce embedding threads: `embedding_threads = 1`
3. Always use `--limit` with semantic queries
4. First query is slow, subsequent are fast (model caching)

### Index Build Takes Forever

**Solutions:**
1. Increase parallelism: `parallelism = 0` (use all cores)
2. Exclude large dirs: `node_modules`, `target`
3. Disable semantic search (skip embedding computation)
4. Enable `--watch` for incremental updates

### Out of Memory During Indexing

**Solutions:**
1. Reduce batch size: `batch_size = 1000`
2. Reduce parallelism: `parallelism = 8`
3. Close other apps to free RAM
4. Index directories one at a time

---

## When to Reindex

**Reindex when:**
- Codanna returns no results
- Search seems outdated
- After large code changes (>100 files)

```bash
codanna index && codanna documents index --collection docs
```

---

## Best Practices

### 1. Use Codanna for Code Exploration

**✅ DO:**
```bash
# Fast, semantic, relationship-aware
codanna mcp find_symbol --name Workspace
codanna mcp semantic_search_docs --query "error handling"
codanna mcp analyze_impact --symbol_id 123
```

**❌ DON'T:**
```bash
# Slow, token-heavy, no relationships
grep -r "Workspace" crates/
rg "struct Workspace" .
find . -name "*.rs" -exec grep "Workspace" {} \;
```

### 2. Match Query Type to Use Case

| Use Case | Query Type | Example |
|----------|------------|---------|
| Know exact name | `find_symbol` | Find `Workspace` struct |
| Know partial name | `search_symbols` | Find `*Workspace*` with kind filter |
| Don't know name | `semantic_search_docs` | Find "workspace isolation" concept |
| Understand impact | `analyze_impact` | What breaks if I change `Workspace`? |
| Find usage | `find_callers` | Who calls `workspace::create`? |
| Trace execution | `get_calls` | What does `workspace::create` call? |

### 3. Keep Index Clean

```bash
# Regularly rebuild index
codanna index

# Remove unnecessary paths
codanna remove-dir target
codanna remove-dir node_modules

# Verify indexed paths
codanna list-dirs
```

### 4. Optimize for Your Workflow

**Heavy Refactoring?**
```bash
codanna mcp analyze_impact --symbol_id 123  # Check impact before changing
```

**New to Codebase?**
```bash
codanna mcp semantic_search_docs --query "how to handle errors" --limit 5
```

**Debugging?**
```bash
codanna mcp find_callers --symbol_id 456  # Who calls this broken function?
codanna mcp get_calls --symbol_id 456     # What does this function call?
```

---

## Summary

### Performance Expectations

| Query Type | Latency |
|------------|---------|
| Exact symbol lookup | 5-15ms |
| Fuzzy search | 10-50ms |
| Semantic search | 50-200ms (if enabled) |
| Dependency analysis | 100-500ms |

### Key Optimization Principles

1. Use exact `find_symbol` when name is known
2. Add `kind` filter to narrow search space
3. Limit results with `--limit` parameter
4. Keep index fresh with regular rebuilds
5. Match query type to use case

### Quick Reference

```bash
# 1. Check current index health
codanna mcp get_index_info

# 2. Benchmark common queries
time codanna mcp find_symbol --name Workspace
time codanna mcp search_symbols --pattern work --kind Struct --limit 10

# 3. Rebuild if performance degraded
codanna index

# 4. Optimize config if needed
edit .codanna/settings.toml
```

---

## Related Documentation

- [03_WORKFLOW.md](03_WORKFLOW.md) - Daily development workflow
- [08_BEADS.md](08_BEADS.md) - Issue tracking and triage
- [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) - AI agent rules and workflows
- [CI-CD-PERFORMANCE.md](CI-CD-PERFORMANCE.md) - Build/test pipeline performance
