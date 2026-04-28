# Introduction

Isolate is a workspace isolation tool built on top of [JJ (Jujutsu)](https://github.com/jj-vcs/jj) version control. It provides clean, isolated workspaces for AI agents working in parallel.

## The Problem

Running 8-12 agents in parallel is chaos:

- **Lost code** — changes overwritten, gone forever
- **Duplicate work** — the same feature re-implemented 3-4x
- **Bead stealing** — agents claiming work already in progress
- **Detached HEAD** — constantly stuck in broken states
- **Broken main** — always blocked, always broken

## The Solution

**Workspace isolation.** Each agent gets their own isolated JJ workspace. No shared state to corrupt, no coordination needed between agents.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Development Phase (Isolate)                            │
│  - Agent spawns workspace                              │
│  - Agent works on feature                              │
│  - Agent runs isolate sync as main advances            │
│  - Feature validated                                   │
└─────────────────────── Hand off ────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────┐
│  Queue/Stacking Phase (External Tool)                  │
│  - Gets feature from Isolate                          │
│  - Handles queue, stacking, final rebase              │
│  - Merges to main                                      │
└─────────────────────────────────────────────────────────┘
```

## Why JJ?

JJ is fundamentally better for multi-agent workflows:

- **Lock-free concurrency** — agents don't corrupt each other's work
- **Operation log** — undo ANY operation, recover from mistakes
- **Anonymous commits** — no branch name pollution at 8-12 agents
- **First-class conflicts** — no blocking on merges

Git Worktrees work at 1-3 agents. They break at 4+. We know because we lived it.

## Quick Start

```bash
# Check where you are
isolate whereami

# Start work on a feature
isolate work feature-123

# Sync with main as it advances
isolate sync

# Complete work when done
isolate done
```

## Requirements

- **JJ (Jujutsu)** must be installed
- Install via: `cargo install jj-cli` or `brew install jj`
