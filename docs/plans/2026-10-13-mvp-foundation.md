# Lingoes RS MVP Foundation Implementation Plan

> **For Hermes:** Implement task-by-task with strict RED-GREEN-REFACTOR TDD.

**Goal:** Deliver a runnable Rust-only foundation for a Windows-first portable dictionary application.

**Architecture:** A Cargo workspace separates domain contracts, dictionary persistence/import, and the desktop entry point. The first vertical slice imports tab-separated entries, performs normalized offline lookup, and exposes it through a runnable CLI shell; native GUI and Windows capture are subsequent milestones built over the same core.

**Tech Stack:** Stable Rust, Cargo workspace, `rusqlite` with bundled SQLite/FTS5, `thiserror`; later milestones add Slint and `windows-rs` behind platform crates.

---

### Task 1: Define dictionary domain behavior

**Files:**
- Modify: `crates/lingoes-core/src/lib.rs`

1. Write tests for non-empty entries and normalized queries.
2. Run `cargo test -p lingoes-core` and confirm failure.
3. Implement the smallest domain types and validation.
4. Re-run tests and confirm pass.

### Task 2: Implement persistent lookup

**Files:**
- Modify: `crates/lingoes-dictionary/Cargo.toml`
- Modify: `crates/lingoes-dictionary/src/lib.rs`

1. Write tests for inserting entries, exact case-insensitive lookup, prefix ordering, and empty-query rejection.
2. Confirm tests fail before implementation.
3. Implement SQLite-backed storage using a repository interface.
4. Run crate and workspace tests.

### Task 3: Implement TSV import

**Files:**
- Modify: `crates/lingoes-dictionary/src/lib.rs`

1. Write tests for valid rows, blank/comment rows, malformed rows, and duplicate updates.
2. Confirm failure.
3. Implement streaming TSV import with line-specific errors.
4. Verify all tests.

### Task 4: Build runnable desktop-shell binary

**Files:**
- Modify: `apps/lingoes-desktop/Cargo.toml`
- Modify: `apps/lingoes-desktop/src/main.rs`

1. Add command-level tests for creating a sample dictionary and querying it.
2. Confirm failure.
3. Implement `import` and `lookup` commands as the first executable vertical slice.
4. Run a real import and lookup smoke test.

### Task 5: Quality gate

1. Run `cargo fmt --all -- --check`.
2. Run `cargo clippy --workspace --all-targets -- -D warnings`.
3. Run `cargo test --workspace`.
4. Record the next milestones: Slint GUI, Windows UI Automation text capture, tray/hotkey/popup, LD2/MDX/StarDict import, OCR, TTS, and translation providers.
