# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project does not yet tag releases; entries under `[Unreleased]` describe
work completed on local branches that has not been merged to `main` or
released. No version below has been published to crates.io.

## [Unreleased]

### Fixed

- CI installs `sccache` and `clang`/`lld` in all jobs and pins toolchain
  1.99.1 consistently in `rust-toolchain.toml` and CI (gate-1 batch).
- CLI test binaries renamed from `scp-cli` to `hd`; `criterion_main`
  import restored in core benchmarks; `scp-core` clippy fixes (gate-1 batch).
- `scp-tui` branch refresh implemented (was a stub), duplicate-branch
  stack-tree overflow fixed, red-queen unhandled-keys list corrected
  (gate-1 leftovers).
- `scp-vcs` gix branch test fixture now builds a real root commit
  (gate-1 leftovers).
- Postgres integration tests skip unless `HARDLINE_TEST_POSTGRES_URL` is
  set; CI test job gains a `postgres:16` service container (gate-1 leftovers).
- Deny batch: `im` replaced with maintained fork `imbl`, dev-only
  `bincode` replaced with `postcard`, wildcard path dependencies pinned,
  empty `scp-core` default feature removed, `scp-fuzz` licensed
  (deny batch).
- Done handler rewritten from `jj` commands issued through the git binary
  to native git equivalents (conflict detection, dry-run previews)
  (done-handler batch).
- Session persistence parameterized with bind parameters; a pre-existing
  read-back bug that dropped optional session columns is fixed
  (session-SQL batch).
- Checkpoint create/restore/list and query session commands wired to the
  real session database (were TODO stubs behind live subcommands)
  (checkpoint/query batch).
- README installation instructions corrected (binary/package is `hd`,
  build from source) and license line corrected to Apache-2.0 to match
  `LICENSE` (release-story batch).

## [0.5.0] - 2026-05-02

- Last release on `main` (`a5f86577`). Detailed release notes for this
  version were not recorded at the time; see the git history for the
  change set.
