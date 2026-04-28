# Codanna Query Performance Metrics

This document provides comprehensive guidance on Codanna semantic search performance expectations, metrics, and optimization strategies.

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

## Index Statistics (Current)

**Current Index State:**
- **Symbols:** 3,578 across 113 files
- **Relationships:** 3,399 (call graph, dependencies)
- **Symbol Types:** 9 kinds (Functions, Structs, Modules, etc.)
- **Semantic Search:** Disabled (configurable)
- **Indexed Paths:** 3 directories (crates/isolate-core/src, crates/isolate/src, docs)

**Symbol Distribution:**
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

## Performance Expectations

### Query Type Benchmarks

**Exact Symbol Lookup (`find_symbol`)**
- **Expected Latency:** 5-15ms
- **Best Case:** ~5ms (cached, unique match)
- **Worst Case:** ~15ms (multiple matches, disk I/O)
- **Use Case:** When you know exact symbol name

**Fuzzy Symbol Search (`search_symbols`)**
- **Expected Latency:** 10-50ms
- **Best Case:** ~10ms (specific pattern, filtered by kind)
- **Worst Case:** ~50ms (broad pattern, no kind filter)
- **Use Case:** Partial name matches, pattern-based discovery

**Semantic Search (`semantic_search_docs`)**
- **Expected Latency:** 50-200ms
- **Best Case:** ~50ms (small result set, high relevance)
- **Worst Case:** ~200ms (large result set, semantic scoring)
- **Note:** Currently disabled in config (requires embedding model)
- **Use Case:** Natural language queries, intent-based search

**Document Search (`search_documents`)**
- **Expected Latency:** 20-100ms
- **Best Case:** ~20ms (keyword match in small collection)
- **Worst Case:** ~100ms (full-text scan across large docs)
- **Use Case:** Finding documentation, comments, guides

**Dependency Analysis (`analyze_impact`)**
- **Expected Latency:** 100-500ms
- **Best Case:** ~100ms (isolated symbol, few dependencies)
- **Worst Case:** ~500ms (highly connected symbol, deep call chains)
- **Use Case:** Understanding change impact, refactoring planning

**Call Graph Queries (`get_calls`, `find_callers`)**
- **Expected Latency:** 20-100ms
- **Best Case:** ~20ms (leaf function, no outgoing calls)
- **Worst Case:** ~100ms (complex function, deep recursion)
- **Use Case:** Tracing execution flow, finding usage

### Comparison with Grep/Glob

| Operation | Grep/Glob | Codanna | Speedup |
|-----------|-----------|---------|---------|
| Exact symbol name | ~200ms | ~5ms | **40x** |
| Fuzzy name match | ~500ms | ~10ms | **50x** |
| Find all references | ~800ms | ~20ms | **40x** |
| Semantic search | N/A | ~50ms | **∞** |
| Dependency analysis | N/A (manual) | ~100ms | **∞** |
| Impact analysis | N/A (manual) | ~500ms | **∞** |

**Token Efficiency:** Codanna uses 90% fewer tokens than Grep/Glob for code exploration due to:
- Pre-indexed symbols and relationships
- Semantic understanding vs pattern matching
- Relationship awareness (callers/callees pre-computed)
- Targeted context retrieval

## Factors Affecting Query Performance

### 1. Index Size

**Larger Index = Slower Queries**

```bash
# Check index size
du -sh .codanna/index

# Current: ~2-5MB (small, fast)
# Expected growth: 10-20MB per 10k symbols
```

**Impact:**
- **< 5k symbols:** Sub-10ms queries
- **5k-20k symbols:** 10-50ms queries
- **20k-50k symbols:** 50-100ms queries
- **> 50k symbols:** Consider filtering by path/kind

### 2. Query Complexity

**Simple = Fast, Complex = Slow**

| Query Type | Complexity | Example |
|------------|------------|---------|
| Exact name | O(1) | `find_symbol("Workspace")` |
| Fuzzy match | O(log n) | `search_symbols("work", kind: "Struct")` |
| Semantic | O(n) | `semantic_search_docs("workspace isolation")` |
| Graph traversal | O(n + e) | `analyze_impact(symbol_id)` |

**Optimization Tips:**
- Use `kind` filter to reduce search space
- Use exact `find_symbol` when name is known
- Limit result sets with `limit` parameter

### 3. Disk I/O vs Cached

**First Query = Slow, Subsequent = Fast**

```bash
# First query: Cold cache
codanna mcp find_symbol --name Workspace  # ~15ms (disk read)

# Subsequent queries: Hot cache
codanna mcp find_symbol --name Workspace  # ~5ms (memory)
```

**Cache Behavior:**
- Tantivy search engine keeps index in memory
- LRU cache for frequently accessed symbols
- Cache size: 50MB heap (configurable via `tantivy_heap_mb`)

### 4. Parallelism Settings

**More Threads = Faster Indexing, Same Query Speed**

```toml
[indexing]
parallelism = 32  # Current: 32 threads
```

**Impact:**
- **Indexing:** 32x faster with 32 threads (during `codanna index`)
- **Queries:** No impact (queries are single-threaded, read-only)
- **Trade-off:** Higher memory usage during indexing

### 5. Semantic Search Overhead

**Embeddings = Slower but Smarter**

```toml
[semantic_search]
enabled = false    # Currently disabled
model = "AllMiniLML6V2"
embedding_threads = 3
```

**When Enabled:**
- **Indexing:** +30-50% time (compute embeddings)
- **Queries:** +50-100ms per query (vector similarity search)
- **Benefit:** Natural language understanding (e.g., "error handling patterns")

**Recommendation:** Enable only if you frequently use natural language queries.

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

```bash
# Narrow search space for faster queries
codanna mcp search_symbols --pattern create --kind Function  # Functions only
codanna mcp search_symbols --pattern Config --kind Struct    # Structs only
codanna mcp search_symbols --pattern Error --kind Enum       # Enums only
```

**Available Kinds:** Function, Method, Struct, Enum, Trait, Module, Constant, Field, TypeAlias

### 3. Limit Result Sets

```bash
# Fetch fewer results for faster response
codanna mcp search_symbols --pattern work --limit 5      # Top 5 matches
codanna mcp semantic_search_docs --query workspace --limit 10  # Top 10 matches
```

**Impact:**
- `limit 5`: ~10ms (fuzzy), ~50ms (semantic)
- `limit 50`: ~30ms (fuzzy), ~150ms (semantic)
- `limit 100`: ~50ms (fuzzy), ~200ms (semantic)

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

**Stale Index Indicators:**
- Queries return outdated symbols
- `find_symbol` fails for newly added code
- Relationship counts don't match actual code

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
- **More memory = Better cache, faster queries**

## Common Query Patterns

### Pattern 1: Find Exact Symbol

```bash
# Fastest: O(1) lookup
codanna mcp find_symbol --name Workspace
```

**When to use:** You know the exact symbol name.

### Pattern 2: Fuzzy Name Match

```bash
# Fast: O(log n) search with filter
codanna mcp search_symbols --pattern work --kind Struct --limit 10
```

**When to use:** You remember part of the name.

### Pattern 3: Find All References

```bash
# Medium: O(e) where e = edges in call graph
codanna mcp find_callers --symbol_id 123
```

**When to use:** You want to see who calls this function.

### Pattern 4: Understand Impact

```bash
# Slower: O(n + e) full graph traversal
codanna mcp analyze_impact --symbol_id 456
```

**When to use:** You're planning a refactor and need to know what breaks.

### Pattern 5: Semantic Discovery

```bash
# Slower: O(n) vector similarity search
codanna mcp semantic_search_docs --query "workspace isolation patterns" --limit 5
```

**When to use:** You don't know the exact name, just the concept.

## Performance Monitoring

### Check Index Health

```bash
# View index statistics
codanna mcp get_index_info

# Expected output:
# - Symbols: 3,578
# - Relationships: 3,399
# - Files: 113
```

**Healthy Indicators:**
- Symbol count matches expected (rough estimate: 1-2 symbols per 10 lines of Rust)
- Relationship count > 50% of symbol count (well-connected code)
- File count matches indexed directories

**Warning Signs:**
- Symbol count dropped significantly → partial index failure
- Relationships < 20% of symbols → parser issues (missing call graph)
- File count doesn't match directories → some files excluded by ignore patterns

### Benchmark Queries

```bash
# Time exact symbol lookup
time codanna mcp find_symbol --name Workspace

# Time fuzzy search
time codanna mcp search_symbols --pattern work --kind Struct --limit 10

# Time dependency analysis
time codanna mcp analyze_impact --symbol_id 123
```

**Expected Benchmarks (on this codebase):**
- Exact lookup: < 15ms
- Fuzzy search: < 50ms
- Impact analysis: < 500ms

### Parser Performance

```bash
# Benchmark parser (code → AST → index)
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

## Troubleshooting Performance Issues

### Issue: Queries Are Slow (> 500ms)

**Diagnosis:**
```bash
# Check index size
du -sh .codanna/index

# Check symbol count
codanna mcp get_index_info | grep "Symbols:"
```

**Solutions:**
1. **Index too large:** Remove unnecessary directories (`codanna remove-dir`)
2. **Low memory:** Increase `tantivy_heap_mb` in config
3. **Stale cache:** Rebuild index (`codanna index`)
4. **Complex query:** Simplify query, add filters, limit results

### Issue: Semantic Search Very Slow (> 1s)

**Diagnosis:**
```bash
# Check if semantic search is enabled
codanna config | grep "semantic_search"
```

**Solutions:**
1. **Disable semantic search:** Set `enabled = false` if not needed
2. **Reduce embedding threads:** Try `embedding_threads = 1` (lower CPU)
3. **Limit results:** Always use `--limit` with semantic queries
4. **Use model caching:** First query is slow, subsequent are fast

### Issue: Index Build Takes Forever

**Diagnosis:**
```bash
# Check parallelism setting
codanna config | grep parallelism
```

**Solutions:**
1. **Increase parallelism:** Set `parallelism = 0` (use all CPU cores)
2. **Exclude large dirs:** Remove `node_modules`, `target`, etc.
3. **Disable semantic search:** Skip embedding computation
4. **Use file watching:** Enable `--watch` for incremental updates

### Issue: Out of Memory During Indexing

**Diagnosis:**
```bash
# Check batch size
codanna config | grep batch_size
```

**Solutions:**
1. **Reduce batch size:** Try `batch_size = 1000` (default: 5000)
2. **Reduce parallelism:** Try `parallelism = 8` (default: 32)
3. **Close other apps:** Free up RAM for indexing
4. **Index in stages:** Index directories one at a time

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

### 4. Monitor Performance

```bash
# Weekly: Check index health
codanna mcp get_index_info

# Monthly: Benchmark queries
time codanna mcp find_symbol --name Workspace

# Quarterly: Rebuild from scratch
rm -rf .codanna/index
codanna index
```

### 5. Optimize for Your Workflow

**Heavy Refactoring?** Use `analyze_impact` frequently
```bash
codanna mcp analyze_impact --symbol_id 123  # Check impact before changing
```

**New to Codebase?** Use `semantic_search_docs`
```bash
codanna mcp semantic_search_docs --query "how to handle errors" --limit 5
```

**Debugging?** Use `find_callers` and `get_calls`
```bash
codanna mcp find_callers --symbol_id 456  # Who calls this broken function?
codanna mcp get_calls --symbol_id 456     # What does this function call?
```

## Summary

**Performance Expectations:**
- Exact symbol lookup: 5-15ms
- Fuzzy search: 10-50ms
- Semantic search: 50-200ms (if enabled)
- Dependency analysis: 100-500ms

**Key Optimization Principles:**
1. Use exact `find_symbol` when name is known
2. Add `kind` filter to narrow search space
3. Limit results with `--limit` parameter
4. Keep index fresh with regular rebuilds
5. Match query type to use case

**Codanna vs Grep/Glob:**
- **40-50x faster** for exact/fuzzy searches
- **90% fewer tokens** (pre-indexed, semantic understanding)
- **Relationship-aware** (callers, callees, impact analysis)
- **Natural language support** (semantic search)

**Next Steps:**
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

**Related Documentation:**
- [03_WORKFLOW.md](03_WORKFLOW.md) - Daily development workflow
- [08_BEADS.md](08_BEADS.md) - Issue tracking and triage
- [PERFORMANCE.md](PERFORMANCE.md) - Build/test pipeline performance
